//! Raw canvas — direct access to a [`Buffer`] for custom drawing.
//!
//! ```rust
//! use boba::components::canvas::Canvas;
//! let canvas = Canvas::new(|area, buf| {
//!    for y in area.top()..area.bottom() {
//!        for x in area.left()..area.right() {
//!            buf[(x, y)].set_char('█');
//!        }
//!    }
//! });
//! ```

use {
    crate::components::Component,
    ratatui::prelude::{Buffer, Frame, Rect},
};

/// A component that exposes direct buffer access.
pub struct Canvas<F: Fn(Rect, &mut Buffer)> {
    draw: F,
    clear: bool,
}

impl<F: Fn(Rect, &mut Buffer)> Canvas<F> {
    pub fn new(draw: F) -> Self { Self { draw, clear: false } }

    pub fn with_clear(mut self) -> Self {
        self.clear = true;
        self
    }
}

impl<F: Fn(Rect, &mut Buffer)> Component for Canvas<F> {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        let buf = ctx.buffer_mut();
        if self.clear {
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    buf[(x, y)].reset();
                    buf[(x, y)].set_bg(theme.global_bg);
                }
            }
        }
        (self.draw)(area, buf);
    }
}
