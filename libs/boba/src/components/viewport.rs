//! Scrollable viewport that crops a larger virtual area into a visible window.
//!
//! ```rust
//! use boba::components::viewport::Viewport;
//! let vp = Viewport::new(80, 40); // virtual 80x40
//! ```

use {
    crate::components::{Component, block::BobaBlock, style::BobaStyle},
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        widgets::Widget,
    },
};

/// A scrollable viewport into a larger virtual canvas.
pub struct Viewport {
    virtual_width: u16,
    virtual_height: u16,
    scroll_x: Mutable<u16>,
    scroll_y: Mutable<u16>,
    focused: Mutable<bool>,
    show_borders: bool,
    show_scrollbars: bool,
}

impl Viewport {
    pub fn new(vw: u16, vh: u16) -> Self {
        Self {
            virtual_width: vw,
            virtual_height: vh,
            scroll_x: Mutable::new(0),
            scroll_y: Mutable::new(0),
            focused: Mutable::new(false),
            show_borders: true,
            show_scrollbars: true,
        }
    }

    pub fn without_borders(mut self) -> Self {
        self.show_borders = false;
        self
    }

    pub fn without_scrollbars(mut self) -> Self {
        self.show_scrollbars = false;
        self
    }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn scroll_to(&self, x: u16, y: u16) {
        self.scroll_x.set(x);
        self.scroll_y.set(y);
    }

    pub fn on_key(&self, code: KeyCode, area: Rect) {
        if !self.focused.get() {
            return;
        }
        let mut sx = self.scroll_x.get();
        let mut sy = self.scroll_y.get();
        let max_x = self.virtual_width.saturating_sub(area.width.saturating_sub(2));
        let max_y = self.virtual_height.saturating_sub(area.height.saturating_sub(2));
        match code {
            KeyCode::Left => sx = sx.saturating_sub(1),
            KeyCode::Right => sx = (sx + 1).min(max_x),
            KeyCode::Up => sy = sy.saturating_sub(1),
            KeyCode::Down => sy = (sy + 1).min(max_y),
            KeyCode::Home => sx = 0,
            KeyCode::End => sx = max_x,
            KeyCode::PageUp => sy = sy.saturating_sub(area.height.saturating_sub(2)),
            KeyCode::PageDown => sy = (sy + area.height.saturating_sub(2)).min(max_y),
            _ => {}
        }
        self.scroll_x.set(sx);
        self.scroll_y.set(sy);
    }

    /// Return the visible sub-rect inside the virtual canvas.
    pub fn visible_area(&self, container: Rect) -> Rect {
        let inner_w = container.width.saturating_sub(if self.show_borders { 2 } else { 0 });
        let inner_h = container.height.saturating_sub(if self.show_borders { 2 } else { 0 });
        Rect {
            x: self.scroll_x.get(),
            y: self.scroll_y.get(),
            width: inner_w.min(self.virtual_width),
            height: inner_h.min(self.virtual_height),
        }
    }

    pub fn render_to_buf<F: FnOnce(Rect, &mut Buffer)>(&self, area: Rect, buf: &mut Buffer, content: F) {
        let focused = self.focused.get();
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // draw content
        let visible = self.visible_area(area);
        let mut scratch = Buffer::empty(inner);
        content(visible, &mut scratch);
        for y in inner.top()..inner.bottom() {
            for x in inner.left()..inner.right() {
                buf[(x, y)] = scratch[(x, y)].clone();
            }
        }

        // borders
        if self.show_borders {
            let style = if focused { BobaStyle::new().accent() } else { BobaStyle::new().rounded() };
            let block: ratatui::widgets::Block<'_> = BobaBlock::new().rounded().border_style(style).into();
            block.render(area, buf);
        }

        // scrollbars
        if self.show_scrollbars {
            if self.virtual_width > inner.width {
                let thumb_w = (inner.width as f64 / self.virtual_width as f64 * inner.width as f64) as u16;
                let thumb_x = (self.scroll_x.get() as f64 / (self.virtual_width - inner.width) as f64
                    * (inner.width - thumb_w) as f64) as u16;
                for x in 0..inner.width {
                    let pos = inner.x + x;
                    let ch = if x >= thumb_x && x < thumb_x + thumb_w { '█' } else { '░' };
                    buf[(pos, area.bottom().saturating_sub(1))].set_char(ch);
                }
            }
            if self.virtual_height > inner.height {
                let thumb_h = (inner.height as f64 / self.virtual_height as f64 * inner.height as f64) as u16;
                let thumb_y = (self.scroll_y.get() as f64 / (self.virtual_height - inner.height) as f64
                    * (inner.height - thumb_h) as f64) as u16;
                for y in 0..inner.height {
                    let pos = inner.y + y;
                    let ch = if y >= thumb_y && y < thumb_y + thumb_h { '█' } else { '░' };
                    buf[(area.right().saturating_sub(1), pos)].set_char(ch);
                }
            }
        }
    }
}

impl Component for Viewport {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        let buf = ctx.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }
        self.render_to_buf(area, buf, |_view, _buf| {
            // default no-op; users usually wrap with a closure
        });
    }
}
