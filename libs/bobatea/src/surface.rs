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
///
/// Each cell stores a symbol (grapheme cluster) and its associated style.
/// Wide characters (e.g. CJK, emoji) occupy more than one column in the terminal
/// but are still stored as a single `Cell`; use [`Cell::width`] to query visual width.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The grapheme cluster rendered by this cell.
    pub symbol: String,
    /// The ratatui style (foreground color, background color, modifiers).
    pub style: Style,
}

impl Cell {
    /// Create a cell with the given symbol and style.
    ///
    /// # Panics
    ///
    /// Panics if `symbol` is empty. Use `" "` for a blank cell.
    pub fn new(symbol: impl Into<String>, style: Style) -> Self {
        let symbol = symbol.into();
        debug_assert!(!symbol.is_empty(), "Cell symbol must not be empty; use ' ' for blank");
        Self { symbol, style }
    }

    /// Create a blank (space) cell with the given style.
    pub fn blank(style: Style) -> Self { Self::new(" ", style) }

    /// Visual width of this cell in terminal columns.
    ///
    /// Returns `1` for regular characters and emoji, `2` for wide characters (CJK).
    pub fn width(&self) -> usize { self.symbol.width() }
}

/// An off-screen drawable pixel grid.
///
/// Think of a `Surface` as an in-memory terminal framebuffer: you compose styled text
/// and shapes into it, then blit it onto a [`ratatui`] buffer at any position.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Cell, Surface};
/// use ratatui::style::Style;
///
/// let cell = Cell::blank(Style::default());
/// let surf = Surface::new(5, 3, &cell);
/// assert_eq!(surf.columns(), 5);  // cell-count width
/// assert_eq!(surf.width(), 5);   // visual width (1 char per cell)
/// ```
#[derive(Debug, Clone)]
pub struct Surface {
    /// Row-wise grid of cells. Outer vec is rows (y), inner vec is columns (x).
    pub rows: Vec<Vec<Cell>>,
}

impl Surface {
    /// Create an empty surface of the given dimensions filled with blank cells.
    ///
    /// `width` and `height` are measured in terminal columns / rows respectively.
    /// Wide characters do not affect the cell count — use [`Surface::width`] for
    /// the visual (grapheme-based) width instead.
    pub fn new(width: usize, height: usize, fill: &Cell) -> Self {
        let rows = (0..height).map(|_| (0..width).map(|_| fill.clone()).collect()).collect();
        Self { rows }
    }

    /// Visual width of the surface in terminal columns.
    ///
    /// Unlike [`Surface::columns`], this accounts for wide characters (emoji, CJK).
    pub fn width(&self) -> usize {
        self.rows.iter().map(|r| r.iter().map(|c| c.width().max(1)).sum::<usize>()).max().unwrap_or(0)
    }

    /// Number of rows in the surface.
    pub fn height(&self) -> usize { self.rows.len() }

    /// Number of cell columns in the widest row.
    ///
    /// This is the raw cell count, **not** the visual width — wide characters
    /// still count as one cell here. For visual width use [`Surface::width`].
    pub fn columns(&self) -> usize { self.rows.iter().map(|r| r.len()).max().unwrap_or(0) }

    /// Alias for [`Surface::columns`].
    #[deprecated(since = "0.1.0", note = "renamed to `columns`")]
    pub fn cell_count_width(&self) -> usize { self.columns() }

    /// Get a mutable reference to a cell, if it exists.
    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> { self.rows.get_mut(y)?.get_mut(x) }

    /// Build a surface from pre-wrapped lines of styled text.
    ///
    /// Each [`Line`] may contain multiple [`Span`]s with different styles;
    /// they are flattened into one cell per character.
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

    /// Fill the entire surface with a background style (patching each cell's style).
    ///
    /// Only cells that don't already have an explicit background set are affected.
    pub fn fill_bg(&mut self, style: Style) {
        for row in &mut self.rows {
            for cell in row.iter_mut() {
                cell.style = cell.style.patch(style);
            }
        }
    }

    /// Blit this surface into a `ratatui::Buffer` at `(x, y)`.
    ///
    /// Handles wide characters safely by skipping cells that would overflow
    /// the buffer boundary.
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
    ///
    /// Makes `Surface` usable as a `ratatui::widgets::Widget`.
    pub fn render_to_area(&self, area: Rect, buf: &mut Buffer) { self.blit(buf, area.x, area.y); }
}

impl Widget for &Surface {
    fn render(self, area: Rect, buf: &mut Buffer) { self.render_to_area(area, buf); }
}

/// Horizontal or vertical alignment when placing or joining surfaces.
///
/// Used by [`place`], [`place_filled`], [`join_horizontal`], and [`join_vertical`]
/// to resolve the offset of content within a larger container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Align to the left (horizontal) or top (vertical).
    Top,
    /// Align to the right (horizontal) or bottom (vertical).
    Bottom,
    /// Align to the left edge.
    Left,
    /// Align to the right edge.
    Right,
    /// Center align along this axis.
    Center,
}

/// Join surfaces side-by-side, aligning them vertically.
///
/// All input surfaces are placed in a single row, aligned according to `align`.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Surface, join_horizontal, Position};
/// // "A" is 1 cell, "B\nC" spans 2 rows (row const "B", row 1 "C")
/// let left  = Surface::from_text("A", Default::default());
/// let right = Surface::from_text("B\nC", Default::default());
/// let combined = join_horizontal(Position::Top, &[left, right]);
/// // Top row: "AB" = 2 visual columns; bottom row: "C" = 1 column
/// assert_eq!(combined.width(), 2);
/// ```
pub fn join_horizontal(align: Position, surfaces: &[Surface]) -> Surface {
    if surfaces.is_empty() {
        return Surface { rows: Vec::new() };
    }

    let total_width: usize = surfaces.iter().map(|s| s.width()).sum();
    let max_height = surfaces.iter().map(|s| s.height()).max().unwrap_or(0);
    let blank = Cell::blank(Style::default());
    let mut out = Surface::new(total_width, max_height, &blank);

    let mut x_offset = 0usize;
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

/// Join surfaces stacked vertically, aligning them horizontally.
///
/// All input surfaces are placed in a single column, aligned according to `align`.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Surface, join_vertical, Position};
/// let top    = Surface::from_text("ABC", Default::default());
/// let bottom = Surface::from_text("X", Default::default());
/// let combined = join_vertical(Position::Center, &[top, bottom]);
/// assert_eq!(combined.height(), 2);
/// ```
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

/// Place a surface inside a fixed-size box with the given alignment.
///
/// The output surface is exactly `width × height`. Content is aligned within it
/// using `h_align` (horizontal) and `v_align` (vertical), then blitted on top
/// of a background filled with `fill`.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Surface, Cell, place, Position};
/// let inner = Surface::from_text("Hi", Default::default());
/// let outer = place(5, 3, Position::Center, Position::Center, &inner, &Cell::blank(Default::default()));
/// assert_eq!(outer.width(), 5);
/// assert_eq!(outer.height(), 3);
/// ```
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

/// Adjust a surface to exactly `width` columns, truncating or padding with `fill`.
///
/// Truncation cuts cells from the end of each row at the visual boundary.
/// Padding appends `fill` cells to the right of each row.
///
/// See also [`clip`] for truncation-without-padding, and [`fit_height`] for the vertical counterpart.
pub fn fit_width(surf: &mut Surface, width: usize, fill: &Cell) {
    for row in &mut surf.rows {
        let current_width: usize = row.iter().map(|c| c.width().max(1)).sum();
        if current_width > width {
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

/// Alias for [`fit_width`].
#[deprecated(since = "0.1.0", note = "renamed to `fit_width`")]
pub fn set_width(surf: &mut Surface, width: usize, fill: &Cell) { fit_width(surf, width, fill); }

/// Clip a surface to `max_width` visual columns, cutting mid-word if needed.
///
/// Unlike [`fit_width`] this does **not** pad short rows — it only truncates
/// content that overflows `max_width`.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Surface, clip, Cell};
/// let mut surf = Surface::from_text("Hello, World!", Default::default());
/// clip(&mut surf, 5);
/// // Surface is now at most 5 visual columns wide
/// ```
pub fn clip(surf: &mut Surface, max_width: usize) {
    for row in &mut surf.rows {
        let mut w = 0usize;
        for (i, cell) in row.iter().enumerate() {
            let cw = cell.width().max(1);
            if w + cw > max_width {
                row.truncate(i);
                break;
            }
            w += cw;
        }
    }
}

/// Alias for [`clip`].
#[deprecated(since = "0.1.0", note = "renamed to `clip`")]
pub fn flex_truncate(surf: &mut Surface, max_width: usize) { clip(surf, max_width); }

/// Adjust a surface to exactly `height` rows, truncating or padding with `fill`.
pub fn fit_height(surf: &mut Surface, height: usize, fill: &Cell) {
    let current_height = surf.height();
    if current_height > height {
        surf.rows.truncate(height);
    } else if current_height < height {
        let width = surf.columns();
        for _ in 0..(height - current_height) {
            surf.rows.push((0..width).map(|_| fill.clone()).collect());
        }
    }
}

/// Alias for [`fit_height`].
#[deprecated(since = "0.1.0", note = "renamed to `fit_height`")]
pub fn set_height(surf: &mut Surface, height: usize, fill: &Cell) { fit_height(surf, height, fill); }

/// Sum the visual widths of a slice of surfaces.
///
/// Useful for computing how much space a row of segments will occupy.
pub fn total_width(surfaces: &[Surface]) -> usize { surfaces.iter().map(|s| s.width()).sum() }

/// Alias for [`total_width`].
#[deprecated(since = "0.1.0", note = "renamed to `total_width`")]
pub fn width(surfaces: &[Surface]) -> usize { total_width(surfaces) }

/// Like [`place`], but fills empty space with cycling placeholder characters.
///
/// Equivalent to lipgloss's `WithWhitespaceChars` / `Fill` option.
/// The output surface is exactly `width × height`; content is centered or aligned
/// within it, and all remaining cells are filled with repeating `chars`.
///
/// # Example
///
/// ```
/// use bobatea::surface::{Surface, Cell, place_filled, Position};
/// let inner = Surface::from_text("Hi", Default::default());
/// let outer = place_filled(5, 3, Position::Center, Position::Center, &inner, Default::default(), "░");
/// // outer is 5×3, with "░" filling the gaps around "Hi"
/// ```
pub fn place_filled(
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

    let mut char_idx = 0;
    for y in 0..height {
        let mut x = 0usize;
        while x < width {
            let ch = chars_vec[char_idx % chars_vec.len()];
            if let Some(dst) = out.cell_mut(x, y) {
                *dst = Cell::new(ch.to_string(), fill_style);
            }
            x += 1;
            char_idx += 1;
        }
    }

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

/// Alias for [`place_filled`].
#[deprecated(since = "0.1.0", note = "renamed to `place_filled`")]
#[doc(hidden)]
pub fn place_with_whitespace(
    width: usize,
    height: usize,
    h_align: Position,
    v_align: Position,
    surf: &Surface,
    fill_style: Style,
    chars: &str,
) -> Surface {
    place_filled(width, height, h_align, v_align, surf, fill_style, chars)
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
        let outer = place_filled(4, 3, Position::Center, Position::Center, &inner, Style::default(), "ab");
        assert_eq!(outer.rows[0][0].symbol, "a");
        assert_eq!(outer.rows[0][1].symbol, "b");
        assert_eq!(outer.rows[0][2].symbol, "a");
        assert_eq!(outer.rows[0][3].symbol, "b");
        assert_eq!(outer.rows[1][1].symbol, "X"); // centered content
    }
}
