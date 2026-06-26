use {
    crate::components::{Component, anim::Spinner as SpinnerAnim},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        widgets::{Paragraph, Widget},
    },
};

/// A loading spinner component.
///
/// ```rust
/// use boba::components::spinner::Spinner;
/// let spinner = Spinner::dots();
/// ```
pub struct Spinner {
    anim: SpinnerAnim,
    label: Mutable<Option<String>>,
}

impl Spinner {
    pub fn dots() -> Self { Self { anim: SpinnerAnim::dots(), label: Mutable::new(None) } }

    pub fn line() -> Self { Self { anim: SpinnerAnim::line(), label: Mutable::new(None) } }

    pub fn mini() -> Self { Self { anim: SpinnerAnim::mini(), label: Mutable::new(None) } }

    pub fn with_label(self, label: impl Into<String>) -> Self {
        self.label.set(Some(label.into()));
        self
    }

    pub fn set_label(&self, label: impl Into<String>) { self.label.set(Some(label.into())); }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let frame = self.anim.frame();
        let text = match self.label.get_cloned() {
            Some(l) => format!("{} {}", frame, l),
            None => frame.to_string(),
        };

        Paragraph::new(text).style(ratatui::style::Style::default().fg(theme.spinner.fg)).render(area, buf);
    }
}

impl Component for Spinner {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
