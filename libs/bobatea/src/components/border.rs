//! Custom borders for styled strings and surfaces.
//!
//! ```rust
//! use bobatea::components::border::Border;
//! let b = Border::rounded().top('═').bottom('─');
//! ```

use {
    crate::surface::{Cell, Surface},
    ratatui::style::Style,
};

/// A fully-customizable border.
///
/// Each edge is a single grapheme rune (`char`), and the struct tracks
/// whether each side should be drawn at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Border {
    pub top: char,
    pub bottom: char,
    pub left: char,
    pub right: char,
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub has_top: bool,
    pub has_bottom: bool,
    pub has_left: bool,
    pub has_right: bool,
}

impl Border {
    pub fn new() -> Self {
        Self {
            top: '─',
            bottom: '─',
            left: '│',
            right: '│',
            top_left: '┌',
            top_right: '┐',
            bottom_left: '└',
            bottom_right: '┘',
            has_top: true,
            has_bottom: true,
            has_left: true,
            has_right: true,
        }
    }

    /// Rounded-corners look (Unicode box-drawing).
    pub fn rounded() -> Self {
        Self {
            top: '─',
            bottom: '─',
            left: '│',
            right: '│',
            top_left: '╭',
            top_right: '╮',
            bottom_left: '╰',
            bottom_right: '╯',
            has_top: true,
            has_bottom: true,
            has_left: true,
            has_right: true,
        }
    }

    /// Normal (sharp corners) box-drawing.
    pub fn normal() -> Self { Self::new() }

    pub fn thick() -> Self {
        Self {
            top: '━',
            bottom: '━',
            left: '┃',
            right: '┃',
            top_left: '┏',
            top_right: '┓',
            bottom_left: '┗',
            bottom_right: '┛',
            has_top: true,
            has_bottom: true,
            has_left: true,
            has_right: true,
        }
    }

    pub fn double() -> Self {
        Self {
            top: '═',
            bottom: '═',
            left: '║',
            right: '║',
            top_left: '╔',
            top_right: '╗',
            bottom_left: '╚',
            bottom_right: '╝',
            has_top: true,
            has_bottom: true,
            has_left: true,
            has_right: true,
        }
    }

    pub fn top(mut self, c: char) -> Self {
        self.top = c;
        self
    }

    pub fn bottom(mut self, c: char) -> Self {
        self.bottom = c;
        self
    }

    pub fn left(mut self, c: char) -> Self {
        self.left = c;
        self
    }

    pub fn right(mut self, c: char) -> Self {
        self.right = c;
        self
    }

    pub fn top_left(mut self, c: char) -> Self {
        self.top_left = c;
        self
    }

    pub fn top_right(mut self, c: char) -> Self {
        self.top_right = c;
        self
    }

    pub fn bottom_left(mut self, c: char) -> Self {
        self.bottom_left = c;
        self
    }

    pub fn bottom_right(mut self, c: char) -> Self {
        self.bottom_right = c;
        self
    }

    pub fn no_top(mut self) -> Self {
        self.has_top = false;
        self
    }

    pub fn no_bottom(mut self) -> Self {
        self.has_bottom = false;
        self
    }

    pub fn no_left(mut self) -> Self {
        self.has_left = false;
        self
    }

    pub fn no_right(mut self) -> Self {
        self.has_right = false;
        self
    }

    /// Horizontal border size contributed by this border (0, 1, or 2).
    pub fn horizontal_size(&self) -> usize { (if self.has_top { 1 } else { 0 }) + (if self.has_bottom { 1 } else { 0 }) }

    pub fn vertical_size(&self) -> usize { (if self.has_left { 1 } else { 0 }) + (if self.has_right { 1 } else { 0 }) }

    /// Draw this border around an existing surface, using `style` for the border lines.
    pub fn draw(&self, surf: &Surface, style: Style) -> Surface {
        let inner_w = surf.columns();
        let inner_h = surf.height();
        let bw = self.vertical_size();
        let bh = self.horizontal_size();
        let out_w = inner_w + bw;
        let out_h = inner_h + bh;
        let blank = Cell::blank(style);
        let mut out = Surface::new(out_w, out_h, &blank);

        // Draw corners
        if self.has_top && self.has_left {
            if let Some(c) = out.cell_mut(0, 0) {
                *c = Cell::new(self.top_left.to_string(), style);
            }
        }
        if self.has_top && self.has_right {
            if let Some(c) = out.cell_mut(out_w - 1, 0) {
                *c = Cell::new(self.top_right.to_string(), style);
            }
        }
        if self.has_bottom && self.has_left {
            if let Some(c) = out.cell_mut(0, out_h - 1) {
                *c = Cell::new(self.bottom_left.to_string(), style);
            }
        }
        if self.has_bottom && self.has_right {
            if let Some(c) = out.cell_mut(out_w - 1, out_h - 1) {
                *c = Cell::new(self.bottom_right.to_string(), style);
            }
        }

        // Top & bottom edges
        if self.has_top {
            for x in 1..(out_w - 1) {
                if let Some(c) = out.cell_mut(x, 0) {
                    *c = Cell::new(self.top.to_string(), style);
                }
            }
        }
        if self.has_bottom {
            for x in 1..(out_w - 1) {
                if let Some(c) = out.cell_mut(x, out_h - 1) {
                    *c = Cell::new(self.bottom.to_string(), style);
                }
            }
        }

        // Left & right edges
        if self.has_left {
            for y in 1..(out_h - 1) {
                if let Some(c) = out.cell_mut(0, y) {
                    *c = Cell::new(self.left.to_string(), style);
                }
            }
        }
        if self.has_right {
            for y in 1..(out_h - 1) {
                if let Some(c) = out.cell_mut(out_w - 1, y) {
                    *c = Cell::new(self.right.to_string(), style);
                }
            }
        }

        // Copy inner surface
        let off_x = if self.has_left { 1 } else { 0 };
        let off_y = if self.has_top { 1 } else { 0 };
        for (dy, row) in surf.rows.iter().enumerate() {
            let mut dx = 0usize;
            for cell in row.iter() {
                if let Some(dst) = out.cell_mut(off_x + dx, off_y + dy) {
                    *dst = cell.clone();
                }
                dx += 1; // cell count - each cell is 1 position
            }
        }

        out
    }
}

impl Default for Border {
    fn default() -> Self { Self::new() }
}
