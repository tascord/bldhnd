use crate::ui::components::scroll::{ScrollText, Scroller};
use ratatui::prelude::Text;
use std::sync::Arc;
use tracing::{
    Level,
    field::{Field, Visit},
};
use tracing_subscriber::layer::{Context, Layer};

static LOG_SCROLLER: std::sync::LazyLock<Arc<Scroller>> = std::sync::LazyLock::new(|| Arc::new(Scroller::new()));

pub fn scroller() -> Arc<Scroller> {
    LOG_SCROLLER.clone()
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
            self.0 = format!("{:?}", value);
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some('[') = chars.peek() {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    let is_final = ('@'..='~').contains(&nc);
                    chars.next();
                    if is_final {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

pub struct LogsLayer {
    min_level: Level,
}

impl Default for LogsLayer {
    fn default() -> Self {
        Self { min_level: Level::TRACE }
    }
}

impl<S> Layer<S> for LogsLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();
        if level > self.min_level {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut text = visitor.0;
        if text.is_empty() {
            text = event.metadata().name().to_string();
        }

        let parsed = strip_ansi(&text);
        let sc = scroller();

        let idx = sc.items.lock_ref().len();
        let numbered = format!("{:03} {}", idx + 1, parsed);

        let t: ScrollText = ScrollText::new(Text::from(numbered));
        sc.item_ref(t);

        let mut items_lock = sc.items.lock_mut();
        while items_lock.len() > 200 {
            items_lock.remove(0);
        }
    }
}

pub fn install_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(crate::ui::components::sonner::SonnerLayer::default())
        .with(LogsLayer::default())
        .init();
}