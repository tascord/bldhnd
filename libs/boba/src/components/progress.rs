use {
    crate::components::{Component, block::BobaBlock},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        widgets::{Gauge, Widget},
    },
};

/// A progress bar component.
///
/// ```rust
/// use boba::components::progress::Progress;
/// let bar = Progress::new().label("Loading...");
/// bar.set(0.5);
/// ```
pub struct Progress {
    value: Mutable<f64>,
    label: Mutable<String>,
}

impl Progress {
    pub fn new() -> Self { Self { value: Mutable::new(0.0), label: Mutable::new(String::new()) } }

    pub fn label(self, label: impl Into<String>) -> Self {
        self.label.set(label.into());
        self
    }

    pub fn set(&self, v: f64) { self.value.set(v.clamp(0.0, 1.0)); }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let value = self.value.get();
        let label = self.label.get_cloned();

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let style = ratatui::style::Style::default().fg(theme.progress.filled).bg(theme.progress.empty);
        let block: ratatui::widgets::Block<'_> =
            BobaBlock::new().rounded().border_style(ratatui::style::Style::default().fg(theme.progress.label_fg)).into();

        Gauge::default().ratio(value).label(label).style(style).block(block).render(area, buf);
    }
}

impl Component for Progress {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
