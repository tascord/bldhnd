//! Layout primitives and off-screen surface compositing.
//!
//! Provides a `Surface` abstraction roughly equivalent to Lip Gloss's
//! styled-string + layout engine: paint text into an in-memory grid,
//! then blit it onto a [`ratatui::prelude::Buffer`] at an arbitrary
//! offset.

use {
    ratatui::{
        layout::Rect,
        prelude::Buffer,
        style::Style,
        text::{Line, Span},
        widgets::Widget,
    },
    unicode_width::UnicodeWidthStr,
};

/// A single cell in a [`Surface`] — a grapheme cluster + style.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub symbol: String,
    pub style: Style,
}

impl Cell {
    pub fn new(symbol: impl Into<String>, style: Style) -> Self {
        let symbol = symbol.into();
        debug_assert!(!symbol.is_empty(), "Cell symbol must not be empty; use ' ' for blank");
        Self { symbol, style }
    }

    pub fn blank(style: Style) -> Self { Self::new(" ", style) }

    /// Visual width of this cell (accounts for wide chars / CJK).
    pub fn width(&self) -> usize { self.symbol.width() }
}

/// An off-screen drawable grid.
#[derive(Debug, Clone)]
pub struct Surface {
    pub rows: Vec<Vec<Cell>>,
}

impl Surface {
    /// Create an empty surface of the given dimensions filled with blank cells.
    pub fn new(width: usize, height: usize, fill: &Cell) -> Self {
        let rows = (0..height).map(|_| (0..width).map(|_| fill.clone()).collect()).collect();
        Self { rows }
    }

    /// Visual width of the surface (widest row in display columns).
    pub fn width(&self) -> usize {
        self.rows.iter().map(|r| r.iter().map(|c| c.width().max(1)).sum::<usize>()).max().unwrap_or(0)
    }

    pub fn height(&self) -> usize { self.rows.len() }

    /// Get a mutable reference to a cell, if it exists.
    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> { self.rows.get_mut(y)?.get_mut(x) }

    /// Build a surface from pre-wrapped lines of styled text.
    pub fn from_lines(lines: &[Line<'_>]) -> Self {
        let rows: Vec<Vec<Cell>> = lines
            .iter()
            .map(|line| {
                let mut row = Vec::new();
                for span in line.spans.iter() {
                    let style = span.style;
                    for ch in span.content.chars() {
                        let sym = ch.to_string();
                        row.push(Cell::new(sym, style));
                    }
                }
                row
            })
            .collect();
        Self { rows }
    }

    /// Build a surface from raw styled text, splitting on `\n`.
    pub fn from_text(text: &str, style: Style) -> Self {
        let lines: Vec<Line<'_>> = text.lines().map(|l| Line::from(Span::styled(l, style))).collect();
        Self::from_lines(&lines)
    }

    /// Fill the entire surface with a background style (overwriting only blank-looking cells
    /// or all cells — here we naively overwrite all cells for simplicity).
    pub fn fill_bg(&mut self, style: Style) {
        for row in &mut self.rows {
            for cell in row.iter_mut() {
                cell.style = cell.style.patch(style);
            }
        }
    }

    /// Blit this surface into a `ratatui::Buffer` at `(x, y)`.
    pub fn blit(&self, dst: &mut Buffer, x: u16, y: u16) {
        let base_x = x;
        let base_y = y;
        let buf = dst.area();
        let buf_right = buf.x + buf.width;
        let buf_bottom = buf.y + buf.height;
        for (row_dy, row) in self.rows.iter().enumerate() {
            let dst_y = base_y + row_dy as u16;
            if dst_y >= buf_bottom {
                break;
            }
            let mut dst_x = base_x;
            for cell in row.iter() {
                let w = cell.width() as u16;
                if w == 0 {
                    continue;
                }
                if dst_x >= buf_right {
                    break;
                }
                // Handle multi-width chars safely
                let end_x = dst_x + w;
                if end_x > buf_right {
                    break;
                }
                dst.set_string(dst_x, dst_y, &cell.symbol, cell.style);
                dst_x += w;
            }
        }
    }

    /// Render this surface directly into a `ratatui::Frame` area.
    /// This makes `Surface` usable anywhere you'd normally use a ` ratatui::widgets::Widget`.
    pub fn render_to_area(&self, area: Rect, buf: &mut Buffer) { self.blit(buf, area.x, area.y); }
}

impl Widget for &Surface {
    fn render(self, area: Rect, buf: &mut Buffer) { self.render_to_area(area, buf); }
}

/// Horizontal alignment when joining surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

/// Join surfaces horizontally, aligning them vertically.
pub fn join_horizontal(align: Position, surfaces: &[Surface]) -> Surface {
    if surfaces.is_empty() {
        return Surface { rows: Vec::new() };
    }

    let total_width = surfaces.iter().map(|s| s.width()).sum();
    let max_height = surfaces.iter().map(|s| s.height()).max().unwrap_or(0);
    let blank = Cell::blank(Style::default());
    let mut out = Surface::new(total_width, max_height, &blank);

    let mut x_offset = 0;
    for surf in surfaces {
        let surf_w = surf.width();
        let surf_h = surf.height();
        let y_offset = match align {
            Position::Top => 0,
            Position::Bottom => max_height.saturating_sub(surf_h),
            _ => (max_height.saturating_sub(surf_h)) / 2,
        };

        for (dy, row) in surf.rows.iter().enumerate() {
            let y = y_offset + dy;
            if y >= max_height {
                break;
            }
            let mut dx = 0usize;
            for cell in row.iter() {
                if let Some(dst) = out.cell_mut(x_offset + dx, y) {
                    *dst = cell.clone();
                }
                dx += cell.width().max(1);
            }
        }
        x_offset += surf_w;
    }

    out
}

/// Join surfaces vertically, aligning them horizontally.
pub fn join_vertical(align: Position, surfaces: &[Surface]) -> Surface {
    if surfaces.is_empty() {
        return Surface { rows: Vec::new() };
    }

    let total_height = surfaces.iter().map(|s| s.height()).sum();
    let max_width = surfaces.iter().map(|s| s.width()).max().unwrap_or(0);
    let blank = Cell::blank(Style::default());
    let mut out = Surface::new(max_width, total_height, &blank);

    let mut y_offset = 0;
    for surf in surfaces {
        let surf_w = surf.width();
        let surf_h = surf.height();
        let x_offset = match align {
            Position::Left => 0,
            Position::Right => max_width.saturating_sub(surf_w),
            _ => (max_width.saturating_sub(surf_w)) / 2,
        };

        for (dy, row) in surf.rows.iter().enumerate() {
            let y = y_offset + dy;
            if y >= total_height {
                break;
            }
            let mut dx = 0usize;
            for cell in row.iter() {
                if let Some(dst) = out.cell_mut(x_offset + dx, y) {
                    *dst = cell.clone();
                }
                dx += cell.width().max(1);
            }
        }
        y_offset += surf_h;
    }

    out
}

/// Place a surface inside a larger box with the given alignment.
pub fn place(width: usize, height: usize, h_align: Position, v_align: Position, surf: &Surface, fill: &Cell) -> Surface {
    let mut out = Surface::new(width, height, fill);
    let sw = surf.width();
    let sh = surf.height();

    let x = match h_align {
        Position::Left => 0,
        Position::Right => width.saturating_sub(sw),
        _ => (width.saturating_sub(sw)) / 2,
    };
    let y = match v_align {
        Position::Top => 0,
        Position::Bottom => height.saturating_sub(sh),
        _ => (height.saturating_sub(sh)) / 2,
    };

    for (dy, row) in surf.rows.iter().enumerate() {
        let dst_y = y + dy;
        if dst_y >= height {
            break;
        }
        let mut dx = 0usize;
        for cell in row.iter() {
            let dst_x = x + dx;
            if dst_x >= width {
                break;
            }
            if let Some(dst) = out.cell_mut(dst_x, dst_y) {
                *dst = cell.clone();
            }
            dx += cell.width().max(1);
        }
    }

    out
}

/// Truncate or pad a surface horizontally to an exact width using a fill cell.
pub fn set_width(surf: &mut Surface, width: usize, fill: &Cell) {
    for row in &mut surf.rows {
        let current_width: usize = row.iter().map(|c| c.width().max(1)).sum();
        if current_width > width {
            // truncate
            let mut new_row = Vec::new();
            let mut w = 0usize;
            for cell in row.drain(..) {
                let cw = cell.width().max(1);
                if w + cw > width {
                    break;
                }
                new_row.push(cell);
                w += cw;
            }
            *row = new_row;
        } else if current_width < width {
            let pad = width - current_width;
            for _ in 0..pad {
                row.push(fill.clone());
            }
        }
    }
}

/// Update the surface vertical height by padding or truncating rows.
pub fn set_height(surf: &mut Surface, height: usize, fill: &Cell) {
    let current_height = surf.height();
    if current_height > height {
        surf.rows.truncate(height);
    } else if current_height < height {
        let width = surf.width();
        for _ in 0..(height - current_height) {
            surf.rows.push((0..width).map(|_| fill.clone()).collect());
        }
    }
}

/// Convenience: compute the combined width of a slice of surfaces (used for status bars etc).
pub fn width(surfaces: &[Surface]) -> usize { surfaces.iter().map(|s| s.width()).sum() }

/// Like [`place`], but fills empty space with characters from `chars` (cycled).
/// Equivalent to lipgloss's `WithWhitespaceChars` option.
pub fn place_with_whitespace(
    width: usize,
    height: usize,
    h_align: Position,
    v_align: Position,
    surf: &Surface,
    fill_style: Style,
    chars: &str,
) -> Surface {
    let chars_vec: Vec<char> = chars.chars().collect();
    let mut out = Surface::new(width, height, &Cell::blank(fill_style));

    // Fill background with cycling chars
    let mut char_idx = 0;
    for y in 0..height {
        let mut x = 0usize;
        for _ in 0..width {
            if x >= width {
                break;
            }
            let ch = chars_vec[char_idx % chars_vec.len()];
            if let Some(dst) = out.cell_mut(x, y) {
                *dst = Cell::new(ch.to_string(), fill_style);
            }
            x += ch.to_string().width().max(1);
            char_idx += 1;
        }
    }

    // Paste content on top
    let sw = surf.width();
    let sh = surf.height();
    let x = match h_align {
        Position::Left => 0,
        Position::Right => width.saturating_sub(sw),
        _ => (width.saturating_sub(sw)) / 2,
    };
    let y = match v_align {
        Position::Top => 0,
        Position::Bottom => height.saturating_sub(sh),
        _ => (height.saturating_sub(sh)) / 2,
    };

    for (dy, row) in surf.rows.iter().enumerate() {
        let dst_y = y + dy;
        if dst_y >= height {
            break;
        }
        let mut dx = 0usize;
        for cell in row.iter() {
            let dst_x = x + dx;
            if dst_x >= width {
                break;
            }
            if let Some(dst) = out.cell_mut(dst_x, dst_y) {
                *dst = cell.clone();
            }
            dx += cell.width().max(1);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_new_and_dimensions() {
        let cell = Cell::blank(Style::default());
        let s = Surface::new(10, 5, &cell);
        assert_eq!(s.width(), 10);
        assert_eq!(s.height(), 5);
    }

    #[test]
    fn surface_from_text() {
        let s = Surface::from_text("hello\nworld", Style::default());
        assert_eq!(s.width(), 5);
        assert_eq!(s.height(), 2);
        assert_eq!(s.rows[0][0].symbol, "h");
        assert_eq!(s.rows[1][0].symbol, "w");
    }

    #[test]
    fn join_horizontal_aligns() {
        let a = Surface::from_text("A", Style::default());
        let b = Surface::from_text("B C", Style::default());
        let joined = join_horizontal(Position::Top, &[a, b]);
        assert_eq!(joined.width(), 4);
        assert_eq!(joined.height(), 1);
    }

    #[test]
    fn join_vertical_aligns() {
        let a = Surface::from_text("A", Style::default());
        let b = Surface::from_text("B C", Style::default());
        let joined = join_vertical(Position::Left, &[a, b]);
        assert_eq!(joined.width(), 3);
        assert_eq!(joined.height(), 2);
    }

    #[test]
    fn place_centers() {
        let inner = Surface::from_text("X", Style::default());
        let outer = place(5, 3, Position::Center, Position::Center, &inner, &Cell::blank(Style::default()));
        assert_eq!(outer.width(), 5);
        assert_eq!(outer.height(), 3);
        assert_eq!(outer.rows[1][2].symbol, "X");
    }

    #[test]
    fn place_with_whitespace_cycles_chars() {
        let inner = Surface::from_text("X", Style::default());
        let outer = place_with_whitespace(4, 3, Position::Center, Position::Center, &inner, Style::default(), "ab");
        assert_eq!(outer.rows[0][0].symbol, "a");
        assert_eq!(outer.rows[0][1].symbol, "b");
        assert_eq!(outer.rows[0][2].symbol, "a");
        assert_eq!(outer.rows[0][3].symbol, "b");
        assert_eq!(outer.rows[1][1].symbol, "X"); // centered content
    }
}
