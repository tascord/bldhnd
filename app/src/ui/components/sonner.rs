use std::{
    fmt::Display,
    sync::{
        Arc, LazyLock, RwLock,
        atomic::{AtomicU64, Ordering::SeqCst},
    },
    time::{Duration, Instant},
};

use ratatui::{
    prelude::*,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget, WidgetRef},
};
use tracing::{
    Level,
    field::{Field, Visit},
};
use tracing_subscriber::layer::{Context, Layer};

/* =========================================================
   Toast level
========================================================= */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Warn,
    Error,
}

impl ToastLevel {
    fn color(self) -> Color {
        match self {
            ToastLevel::Info => Color::Cyan,
            ToastLevel::Warn => Color::Yellow,
            ToastLevel::Error => Color::Red,
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            ToastLevel::Info => "i",
            ToastLevel::Warn => "!",
            ToastLevel::Error => "x",
        }
    }

    fn default_duration(self) -> Duration {
        match self {
            ToastLevel::Info => Duration::from_secs(3),
            ToastLevel::Warn => Duration::from_secs(5),
            ToastLevel::Error => Duration::from_secs(8),
        }
    }
}

/* =========================================================
   Toast
========================================================= */

#[derive(Debug, Clone)]
struct Toast {
    id: u64,
    level: ToastLevel,
    message: String,
    created: Instant,
    duration: Duration,
}

impl Toast {
    fn expired(&self) -> bool {
        self.created.elapsed() >= self.duration
    }
}

/* =========================================================
   Sonner: the toast queue + widget
========================================================= */

pub struct Sonner {
    queue: RwLock<Vec<Toast>>,
    next_id: AtomicU64,
    max_visible: usize,
    max_queued: usize,
}

static SONNER: LazyLock<Arc<Sonner>> = LazyLock::new(|| Arc::new(Sonner::new(4, 50)));

pub fn sonner() -> Arc<Sonner> {
    SONNER.clone()
}

impl Sonner {
    fn new(max_visible: usize, max_queued: usize) -> Self {
        Self { queue: RwLock::new(Vec::new()), next_id: AtomicU64::new(0), max_visible, max_queued }
    }

    pub fn push(&self, level: ToastLevel, message: impl Display) -> u64 {
        self.push_for(level, message, level.default_duration())
    }

    pub fn push_for(&self, level: ToastLevel, message: impl Display, duration: Duration) -> u64 {
        let id = self.next_id.fetch_add(1, SeqCst);

        let mut q = self.queue.write().unwrap();
        q.push(Toast { id, level, message: message.to_string(), created: Instant::now(), duration });

        while q.len() > self.max_queued {
            q.remove(0);
        }

        id
    }

    pub fn info(&self, message: impl Display) -> u64 {
        self.push(ToastLevel::Info, message)
    }

    pub fn warn(&self, message: impl Display) -> u64 {
        self.push(ToastLevel::Warn, message)
    }

    pub fn error(&self, message: impl Display) -> u64 {
        self.push(ToastLevel::Error, message)
    }

    pub fn dismiss(&self, id: u64) {
        self.queue.write().unwrap().retain(|t| t.id != id);
    }

    pub fn clear(&self) {
        self.queue.write().unwrap().clear();
    }

    fn gc(&self) {
        self.queue.write().unwrap().retain(|t| !t.expired());
    }
}

impl WidgetRef for Sonner {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.gc();

        if area.width == 0 || area.height == 0 {
            return;
        }

        let visible: Vec<Toast> = {
            let q = self.queue.read().unwrap();
            // newest first, capped to max_visible
            q.iter().rev().take(self.max_visible).cloned().collect()
        };

        if visible.is_empty() {
            return;
        }

        let box_width = area.width.min(40);
        let mut y = area.bottom();

        // Stack bottom-right, growing upward as more toasts are shown.
        for toast in &visible {
            let lines = wrap(&toast.message, box_width.saturating_sub(4) as usize);
            let height = (lines.len() as u16 + 2).min(area.height); // +2 for the border

            if y < area.top() + height {
                break;
            }
            y -= height;

            let rect = Rect { x: area.right().saturating_sub(box_width), y, width: box_width, height };
            let color = toast.level.color();

            Paragraph::new(Text::from_iter(lines.into_iter().map(Line::raw)))
                .style(Style::new().fg(color))
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::new().fg(color))
                        .title(format!(" {} ", toast.level.glyph())),
                )
                .render(rect, buf);
        }
    }
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }

    lines
}

/* =========================================================
   Tracing integration
========================================================= */

pub struct SonnerLayer {
    min_level: Level,
}

impl SonnerLayer {
    pub fn with_min_level(min_level: Level) -> Self {
        Self { min_level }
    }
}

impl Default for SonnerLayer {
    fn default() -> Self {
        Self { min_level: Level::WARN }
    }
}

#[derive(Default)]
struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && self.0.is_empty() {
            self.0 = format!("{value:?}");
        }
    }
}

impl<S> Layer<S> for SonnerLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();

        // tracing::Level orders ERROR < WARN < INFO < DEBUG < TRACE, so
        // "at least as severe as min_level" means "<= min_level".
        if level > self.min_level {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        if visitor.0.is_empty() {
            visitor.0 = event.metadata().name().to_string();
        }

        let toast_level = match level {
            Level::ERROR => ToastLevel::Error,
            Level::WARN => ToastLevel::Warn,
            _ => ToastLevel::Info,
        };

        sonner().push(toast_level, visitor.0);
    }
}

pub fn install_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry().with(SonnerLayer::default()).init();
}