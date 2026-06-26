use {
    crate::components::{Component, block::BobaBlock},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Clear, Paragraph, Widget},
    },
    std::{
        fmt::Display,
        time::{Duration, Instant},
    },
};

#[derive(Debug, Clone)]
pub struct Toast {
    pub level: ToastLevel,
    pub message: String,
    pub created: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy)]
pub enum ToastLevel {
    Info,
    Warn,
    Error,
}

impl ToastLevel {
    pub fn color(self) -> ratatui::style::Color {
        match self {
            ToastLevel::Info => ratatui::style::Color::Cyan,
            ToastLevel::Warn => ratatui::style::Color::Yellow,
            ToastLevel::Error => ratatui::style::Color::Red,
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            ToastLevel::Info => "i",
            ToastLevel::Warn => "!",
            ToastLevel::Error => "x",
        }
    }

    pub fn default_duration(self) -> Duration {
        match self {
            ToastLevel::Info => Duration::from_secs(3),
            ToastLevel::Warn => Duration::from_secs(5),
            ToastLevel::Error => Duration::from_secs(8),
        }
    }
}

/// A toast notification component.
pub struct Toaster {
    queue: Mutable<Vec<Toast>>,
    max_visible: usize,
}

impl Clone for Toaster {
    fn clone(&self) -> Self { Self { queue: self.queue.clone(), max_visible: self.max_visible } }
}

impl Toaster {
    pub fn new(max_visible: usize) -> Self { Self { queue: Mutable::new(Vec::new()), max_visible } }

    pub fn push(&self, level: ToastLevel, message: impl Display) {
        let mut q = self.queue.lock_mut();
        q.push(Toast { level, message: message.to_string(), created: Instant::now(), duration: level.default_duration() });
    }

    pub fn info(&self, message: impl Display) { self.push(ToastLevel::Info, message); }

    pub fn warn(&self, message: impl Display) { self.push(ToastLevel::Warn, message); }

    pub fn error(&self, message: impl Display) { self.push(ToastLevel::Error, message); }

    pub fn dismiss(&self, idx: usize) { self.queue.lock_mut().remove(idx); }

    pub fn clear(&self) { self.queue.lock_mut().clear(); }

    fn gc(&self) { self.queue.lock_mut().retain(|t| t.created.elapsed() < t.duration); }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        self.gc();

        let visible: Vec<Toast> = self.queue.get_cloned().iter().rev().take(self.max_visible).cloned().collect();

        if visible.is_empty() {
            return;
        }

        let width = area.width.min(40);
        let mut y = area.top();

        for toast in &visible {
            let lines = wrap(&toast.message, width.saturating_sub(4) as usize);
            let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(y));

            if y + height > area.bottom() {
                break;
            }

            let rect = Rect { x: area.right().saturating_sub(width), y, width, height };
            let (style, border_color) = match toast.level {
                ToastLevel::Info => (theme.toast.info, theme.toast.info_border),
                ToastLevel::Warn => (theme.toast.warn, theme.toast.warn_border),
                ToastLevel::Error => (theme.toast.error, theme.toast.error_border),
            };
            let block = BobaBlock::new()
                .rounded()
                .border_style(ratatui::style::Style::default().fg(border_color))
                .title(format!("| {} |", toast.level.glyph()));

            Clear.render(rect, buf);
            Paragraph::new(lines.into_iter().map(Line::raw).collect::<Vec<_>>())
                .style(style)
                .block(block.into())
                .render(rect, buf);

            y += height;
        }
    }
}

impl Component for Toaster {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
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
