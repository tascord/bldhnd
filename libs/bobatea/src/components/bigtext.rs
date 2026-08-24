//! Large ASCII-art text using figlet fonts.
//!
//! ```rust
//! use bobatea::components::bigtext::BigText;
//! let bt = BigText::new("BOBA").color(ratatui::style::Color::Cyan);
//! ```

use {
    crate::components::{Component, style::BobaStyle},
    figlet_rs::FIGfont,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Color,
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

/// Renders large decorative text via figlet.
pub struct BigText {
    text: String,
    font: FIGfont,
    color: Color,
}

impl BigText {
    /// Load the Pagga font from embedded assets.
    pub fn new(text: impl Display) -> Self {
        let font_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/_assets/Pagga.tlf"));
        let font = FIGfont::from_content(font_content)
            .unwrap_or_else(|_| FIGfont::standard().unwrap_or_else(|_| FIGfont::from_content("").unwrap()));
        Self { text: text.to_string(), font, color: Color::White }
    }

    pub fn font(mut self, ff: FIGfont) -> Self {
        self.font = ff;
        self
    }

    pub fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }

    pub fn lines(&self) -> Vec<String> {
        match self.font.convert(self.text.as_str()) {
            Some(figure) => figure.to_string().lines().map(|s| s.to_string()).collect(),
            None => vec![self.text.clone()],
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let lines = self.lines();
        let style = BobaStyle::new().fg(self.color);
        Paragraph::new(lines.join("\n")).style(style).render(area, buf);
    }
}

impl Component for BigText {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
