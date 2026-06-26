//! Opinionated block wrappers with a fluent builder API.

use {
    crate::components::style::BobaStyle,
    ratatui::{
        layout::Alignment,
        style::Style,
        widgets::{Block, BorderType, Borders, Padding},
    },
    std::ops::Deref,
};

/// Newtype around [`ratatui::widgets::Block`] with a fluent builder API.
#[derive(Debug, Clone)]
pub struct BobaBlock(pub Block<'static>);

impl BobaBlock {
    pub fn new() -> Self { Self(Block::new()) }

    /// Use a rounded border style.
    pub fn rounded(self) -> Self { Self(self.0.border_type(BorderType::Rounded).borders(Borders::ALL)) }

    /// Use a double-line border style.
    pub fn double(self) -> Self { Self(self.0.border_type(BorderType::Double).borders(Borders::ALL)) }

    /// Use a thick border style.
    pub fn thick(self) -> Self { Self(self.0.border_type(BorderType::Thick).borders(Borders::ALL)) }

    /// Use a plain border style.
    pub fn plain(self) -> Self { Self(self.0.border_type(BorderType::Plain).borders(Borders::ALL)) }

    /// No borders.
    pub fn clear(self) -> Self { Self(self.0.borders(Borders::NONE)) }

    /// Only top/bottom borders.
    pub fn horizontal(self) -> Self { Self(self.0.borders(Borders::TOP | Borders::BOTTOM)) }

    /// Only left/right borders.
    pub fn vertical(self) -> Self { Self(self.0.borders(Borders::LEFT | Borders::RIGHT)) }

    /// Set the border style.
    pub fn border_style(self, style: impl Into<Style>) -> Self { Self(self.0.border_style(style.into())) }

    /// Shorthand for `border_style(style.danger())`.
    pub fn danger(self) -> Self { self.border_style(BobaStyle::new().danger()) }

    /// Shorthand for `border_style(style.success())`.
    pub fn success(self) -> Self { self.border_style(BobaStyle::new().success()) }

    /// Shorthand for `border_style(style.warn())`.
    pub fn warn(self) -> Self { self.border_style(BobaStyle::new().warn()) }

    /// Shorthand for `border_style(style.accent())`.
    pub fn accent(self) -> Self { self.border_style(BobaStyle::new().accent()) }

    /// Set a title.
    pub fn title(self, title: impl Into<ratatui::text::Line<'static>>) -> Self { Self(self.0.title(title.into())) }

    /// Set a title with alignment.
    pub fn title_aligned(self, title: impl Into<ratatui::text::Line<'static>>, align: Alignment) -> Self {
        Self(self.0.title(title.into()).title_alignment(align))
    }

    /// Set inner padding.
    pub fn padding(self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self(self.0.padding(Padding::new(left, right, top, bottom)))
    }

    pub fn padding_all(self, v: u16) -> Self { self.padding(v, v, v, v) }
}

impl Default for BobaBlock {
    fn default() -> Self { Self::new() }
}

impl Deref for BobaBlock {
    type Target = Block<'static>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl From<Block<'static>> for BobaBlock {
    fn from(b: Block<'static>) -> Self { Self(b) }
}

impl From<BobaBlock> for Block<'static> {
    fn from(b: BobaBlock) -> Self { b.0 }
}
