//! Help / keybind hint bar — contextual shortcuts display.
//!
//! ```rust
//! use boba::components::help::{HelpBar, Keybind};
//! let bar = HelpBar::new(vec![
//!     Keybind::new("q", "quit"),
//!     Keybind::new("space", "click"),
//!     Keybind::new("Ctrl+P", "palette"),
//! ]);
//! ```

use {
    crate::components::Component,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::{Line, Span},
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

/// A single keybind hint.
pub struct Keybind {
    key: String,
    desc: String,
}

impl Keybind {
    pub fn new(key: impl Display, desc: impl Display) -> Self { Self { key: key.to_string(), desc: desc.to_string() } }
}

/// A help bar that displays keybinds like vim's command bar.
pub struct HelpBar {
    binds: Vec<Keybind>,
    separator: String,
}

impl HelpBar {
    pub fn new(binds: Vec<Keybind>) -> Self { Self { binds, separator: "  ".into() } }

    pub fn with_separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let help = &theme.help;
        let spans: Vec<Span> = self
            .binds
            .iter()
            .flat_map(|b| {
                vec![
                    Span::styled(format!(" {} ", b.key), ratatui::style::Style::default().fg(help.key_fg).bg(help.key_bg)),
                    Span::styled(format!(" {} ", b.desc), ratatui::style::Style::default().fg(help.desc_fg)),
                    Span::styled(self.separator.clone(), ratatui::style::Style::default().fg(help.separator_fg)),
                ]
            })
            .collect();

        let line = Line::from(spans);
        Paragraph::new(line).style(ratatui::style::Style::default().bg(theme.global_bg)).render(area, buf);
    }
}

impl Component for HelpBar {
    fn height(&self) -> Option<usize> { Some(1) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
