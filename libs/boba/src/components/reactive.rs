//! Reactive text elements that auto-update when [`Mutable`] content changes.
//!
//! ```rust
//! use boba::components::reactive::{BobaParagraph, BobaSpan};
//! use futures_signals::signal::Mutable;
//!
//! let label = Mutable::new("Hello".to_string());
//! let p = BobaParagraph::new(&label).accent();
//! ```

use {
    crate::components::style::BobaStyle,
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Rect},
        text::{Line, Span},
        widgets::{Paragraph, Widget},
    },
};

/// A [`Paragraph`] that reads its text from a [`Mutable<String>`].
pub struct BobaParagraph<'a> {
    text: &'a Mutable<String>,
    style: BobaStyle,
    alignment: ratatui::layout::Alignment,
}

impl<'a> BobaParagraph<'a> {
    pub fn new(text: &'a Mutable<String>) -> Self {
        Self { text, style: BobaStyle::new(), alignment: ratatui::layout::Alignment::Left }
    }

    pub fn style(mut self, s: BobaStyle) -> Self {
        self.style = s;
        self
    }

    pub fn accent(mut self) -> Self {
        self.style = BobaStyle::new().accent();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style = BobaStyle::new().danger();
        self
    }

    pub fn success(mut self) -> Self {
        self.style = BobaStyle::new().success();
        self
    }

    pub fn warn(mut self) -> Self {
        self.style = BobaStyle::new().warn();
        self
    }

    pub fn info(mut self) -> Self {
        self.style = BobaStyle::new().info();
        self
    }

    pub fn muted(mut self) -> Self {
        self.style = BobaStyle::new().muted();
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    pub fn alignment(mut self, a: ratatui::layout::Alignment) -> Self {
        self.alignment = a;
        self
    }

    pub fn centered(self) -> Self { self.alignment(ratatui::layout::Alignment::Center) }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }
        let text = self.text.get_cloned();
        Paragraph::new(text).alignment(self.alignment).style(self.style).render(area, buf);
    }
}

/// A [`Span`] that reads its text from a [`Mutable<String>`].
pub struct BobaSpan<'a> {
    text: &'a Mutable<String>,
    style: BobaStyle,
}

impl<'a> BobaSpan<'a> {
    pub fn new(text: &'a Mutable<String>) -> Self { Self { text, style: BobaStyle::new() } }

    pub fn style(mut self, s: BobaStyle) -> Self {
        self.style = s;
        self
    }

    pub fn accent(mut self) -> Self {
        self.style = BobaStyle::new().accent();
        self
    }

    pub fn danger(mut self) -> Self {
        self.style = BobaStyle::new().danger();
        self
    }

    pub fn success(mut self) -> Self {
        self.style = BobaStyle::new().success();
        self
    }

    pub fn warn(mut self) -> Self {
        self.style = BobaStyle::new().warn();
        self
    }

    pub fn info(mut self) -> Self {
        self.style = BobaStyle::new().info();
        self
    }

    pub fn muted(mut self) -> Self {
        self.style = BobaStyle::new().muted();
        self
    }

    pub fn bold(mut self) -> Self {
        self.style = self.style.bold();
        self
    }

    pub fn to_span(&self) -> Span<'_> { Span::styled(self.text.get_cloned(), self.style) }
}

/// A [`Line`] built from reactive spans.
pub struct BobaLine<'a> {
    spans: Vec<BobaSpan<'a>>,
}

impl<'a> BobaLine<'a> {
    pub fn new(spans: Vec<BobaSpan<'a>>) -> Self { Self { spans } }

    pub fn to_line(&self) -> Line<'_> { Line::from(self.spans.iter().map(|s| s.to_span()).collect::<Vec<_>>()) }
}

/// Shorthand: create a reactive paragraph.
pub fn reactive(text: &Mutable<String>) -> BobaParagraph<'_> { BobaParagraph::new(text) }
