//! Small badge / pill labels.
//!
//! ```rust
//! use bobatea::components::badge::{Badge, BadgeStyle};
//! let badge = Badge::new("v1.0.0").style(BadgeStyle::Success);
//! ```

use {
    crate::components::Component,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Color,
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

pub enum BadgeStyle {
    Default,
    Primary,
    Success,
    Warn,
    Danger,
    Info,
}

impl BadgeStyle {
    pub fn colors(&self) -> (Color, Color) {
        match self {
            BadgeStyle::Default => (Color::White, Color::DarkGray),
            BadgeStyle::Primary => (Color::White, Color::Blue),
            BadgeStyle::Success => (Color::White, Color::Green),
            BadgeStyle::Warn => (Color::Black, Color::Yellow),
            BadgeStyle::Danger => (Color::White, Color::Red),
            BadgeStyle::Info => (Color::White, Color::Cyan),
        }
    }
}

/// A small pill-shaped badge label.
pub struct Badge {
    text: String,
    style: BadgeStyle,
}

impl Badge {
    pub fn new(text: impl Display) -> Self { Self { text: text.to_string(), style: BadgeStyle::Default } }

    pub fn style(mut self, s: BadgeStyle) -> Self {
        self.style = s;
        self
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let style = match self.style {
            BadgeStyle::Default => theme.badge_primary,
            BadgeStyle::Primary => theme.badge_primary,
            BadgeStyle::Success => theme.badge_success,
            BadgeStyle::Warn => theme.badge_warn,
            BadgeStyle::Danger => theme.badge_error,
            BadgeStyle::Info => theme.badge_info,
        };
        Paragraph::new(format!(" {} ", self.text)).style(style).render(area, buf);
    }
}

impl Component for Badge {
    fn height(&self) -> Option<usize> { Some(1) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
