//! Opinionated style wrappers with layout primitives.
//!
//! ```rust
//! use bobatea::components::style::{BobaStyle, hsl};
//! let s = BobaStyle::new().rounded().fg(hsl(120.0, 0.8, 0.6));
//! ```

use {
    crate::{
        components::border::Border,
        surface::{Cell, Position, Surface},
    },
    ratatui::{
        layout::Alignment,
        style::{Color, Modifier, Style},
    },
    std::ops::Deref,
    unicode_width::UnicodeWidthStr,
};

/// Newtype around [`ratatui::style::Style`] with a fluent builder API
/// and lipgloss-like layout properties (padding, margin, width, height,
/// alignment, custom borders).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BobaStyle {
    pub inner: Style,
    pub border: Option<Border>,
    pub border_fg: Option<Color>,
    pub padding_top: u16,
    pub padding_right: u16,
    pub padding_bottom: u16,
    pub padding_left: u16,
    pub margin_top: u16,
    pub margin_right: u16,
    pub margin_bottom: u16,
    pub margin_left: u16,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub align_h: Alignment,
    pub align_v: Position,
}

impl BobaStyle {
    pub fn new() -> Self {
        Self {
            inner: Style::new(),
            border: None,
            border_fg: None,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            padding_left: 0,
            margin_top: 0,
            margin_right: 0,
            margin_bottom: 0,
            margin_left: 0,
            width: None,
            height: None,
            align_h: Alignment::Left,
            align_v: Position::Top,
        }
    }

    // ── Color / Modifier (thin wrapper around ratatui::Style) ──

    pub fn fg(self, color: impl Into<Color>) -> Self {
        let mut me = self;
        me.inner = me.inner.fg(color.into());
        me
    }

    pub fn bg(self, color: impl Into<Color>) -> Self {
        let mut me = self;
        me.inner = me.inner.bg(color.into());
        me
    }

    pub fn bold(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::BOLD);
        me
    }

    pub fn dim(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::DIM);
        me
    }

    pub fn italic(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::ITALIC);
        me
    }

    pub fn underlined(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::UNDERLINED);
        me
    }

    pub fn crossed_out(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::CROSSED_OUT);
        me
    }

    pub fn blink(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::SLOW_BLINK);
        me
    }

    pub fn reversed(self) -> Self {
        let mut me = self;
        me.inner = me.inner.add_modifier(Modifier::REVERSED);
        me
    }

    pub fn remove_modifier(self, m: Modifier) -> Self {
        let mut me = self;
        me.inner = me.inner.remove_modifier(m);
        me
    }

    // ── Shorthand semantic styles ──

    /// A "rounded" look: rounded border. No colors are set; call `.fg()` / `.bg()` after.
    pub fn rounded(self) -> Self { self.border(Border::rounded()) }

    /// A soft "accent" look: cyan text.
    pub fn accent(self) -> Self { self.fg(Color::Cyan) }

    /// Muted / disabled look.
    pub fn muted(self) -> Self { self.fg(Color::DarkGray).dim() }

    /// Error / danger look.
    pub fn danger(self) -> Self { self.fg(Color::Red).bold() }

    /// Success / ok look.
    pub fn success(self) -> Self { self.fg(Color::Green) }

    /// Warning look.
    pub fn warn(self) -> Self { self.fg(Color::Yellow) }

    /// Info look.
    pub fn info(self) -> Self { self.fg(Color::Cyan) }

    // ── Layout ──

    pub fn border(self, border: Border) -> Self {
        let mut me = self;
        me.border = Some(border);
        me
    }

    pub fn border_fg(self, color: impl Into<Color>) -> Self {
        let mut me = self;
        me.border_fg = Some(color.into());
        me
    }

    pub fn padding(self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        let mut me = self;
        me.padding_top = top;
        me.padding_right = right;
        me.padding_bottom = bottom;
        me.padding_left = left;
        me
    }

    pub fn padding_all(self, v: u16) -> Self { self.padding(v, v, v, v) }

    pub fn padding_top(self, v: u16) -> Self { self.padding(v, self.padding_right, self.padding_bottom, self.padding_left) }

    pub fn padding_right(self, v: u16) -> Self { self.padding(self.padding_top, v, self.padding_bottom, self.padding_left) }

    pub fn padding_bottom(self, v: u16) -> Self { self.padding(self.padding_top, self.padding_right, v, self.padding_left) }

    pub fn padding_left(self, v: u16) -> Self { self.padding(self.padding_top, self.padding_right, self.padding_bottom, v) }

    pub fn padding_x(self, v: u16) -> Self { self.padding(self.padding_top, v, self.padding_bottom, v) }

    pub fn padding_y(self, v: u16) -> Self { self.padding(v, self.padding_right, v, self.padding_left) }

    pub fn margin(self, top: u16, right: u16, bottom: u16, left: u16) -> Self {
        let mut me = self;
        me.margin_top = top;
        me.margin_right = right;
        me.margin_bottom = bottom;
        me.margin_left = left;
        me
    }

    pub fn margin_all(self, v: u16) -> Self { self.margin(v, v, v, v) }

    pub fn margin_top(self, v: u16) -> Self { self.margin(v, self.margin_right, self.margin_bottom, self.margin_left) }

    pub fn margin_right(self, v: u16) -> Self { self.margin(self.margin_top, v, self.margin_bottom, self.margin_left) }

    pub fn margin_bottom(self, v: u16) -> Self { self.margin(self.margin_top, self.margin_right, v, self.margin_left) }

    pub fn margin_left(self, v: u16) -> Self { self.margin(self.margin_top, self.margin_right, self.margin_bottom, v) }

    pub fn margin_x(self, v: u16) -> Self { self.margin(self.margin_top, v, self.margin_bottom, v) }

    pub fn margin_y(self, v: u16) -> Self { self.margin(v, self.margin_right, v, self.margin_left) }

    /// Inherit fg/bg/modifiers from another `BobaStyle`, keeping our own layout state.
    pub fn inherit(self, other: BobaStyle) -> Self {
        let mut me = self;
        me.inner = other.inner;
        me.border_fg = other.border_fg;
        me
    }

    pub fn width(self, w: u16) -> Self {
        let mut me = self;
        me.width = Some(w);
        me
    }

    pub fn height(self, h: u16) -> Self {
        let mut me = self;
        me.height = Some(h);
        me
    }

    pub fn align(self, h: Alignment, v: Position) -> Self {
        let mut me = self;
        me.align_h = h;
        me.align_v = v;
        me
    }

    // ── Rendering ──

    /// Simple word-wrap: split `text` into lines that fit `max_width` visual columns.
    pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
        if max_width == 0 {
            return text.lines().map(|s| s.to_string()).collect();
        }
        let mut lines = Vec::new();
        for paragraph in text.lines() {
            let mut current_line = String::new();
            let mut current_width = 0usize;
            for word in paragraph.split_whitespace() {
                let word_width = word.width();
                if current_line.is_empty() {
                    // First word on line
                    if word_width > max_width {
                        // Force-break the word
                        for ch in word.chars() {
                            let ch_w = ch.to_string().width();
                            if current_width + ch_w > max_width {
                                if !current_line.is_empty() {
                                    lines.push(current_line);
                                }
                                current_line = ch.to_string();
                                current_width = ch_w;
                            } else {
                                current_line.push(ch);
                                current_width += ch_w;
                            }
                        }
                    } else {
                        current_line.push_str(word);
                        current_width = word_width;
                    }
                } else {
                    let with_space = 1 + word_width;
                    if current_width + with_space > max_width {
                        lines.push(current_line);
                        current_line = word.to_string();
                        current_width = word_width;
                    } else {
                        current_line.push(' ');
                        current_line.push_str(word);
                        current_width += with_space;
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        lines
    }

    /// Apply layout transforms (padding, border, margin, align, clamp) to an existing surface.
    fn apply_layout(&self, mut surf: Surface, content_w: usize, content_h: usize) -> Surface {
        let blank = Cell::blank(self.inner);
        let width = surf.columns();
        let height = surf.height();

        // Clamp BEFORE padding/border expansion
        if let Some(w) = self.width {
            let target = w as usize;
            if width > target {
                for row in &mut surf.rows {
                    row.truncate(target);
                }
            }
        }
        if let Some(h) = self.height {
            let target = h as usize;
            if height > target {
                surf.rows.truncate(target);
            }
        }

        // Horizontal alignment
        let inner_w = width;
        let shift_x = match self.align_h {
            Alignment::Right => inner_w.saturating_sub(content_w),
            Alignment::Center => (inner_w.saturating_sub(content_w)) / 2,
            _ => 0,
        };
        if shift_x > 0 && content_w < width {
            let mut shifted = Surface::new(width, height, &blank);
            for y in 0..surf.height() {
                for x in 0..surf.columns() {
                    if let Some(src) = surf.cell_mut(x, y) {
                        let cell = src.clone();
                        if let Some(dst) = shifted.cell_mut(shift_x + x, y) {
                            *dst = cell;
                        }
                    }
                }
            }
            surf = shifted;
        }

        // Vertical alignment
        let shift_y = match self.align_v {
            Position::Bottom => height.saturating_sub(content_h),
            Position::Center => (height.saturating_sub(content_h)) / 2,
            _ => 0,
        };
        if shift_y > 0 && content_h < height {
            let mut shifted = Surface::new(width, height, &blank);
            for y in 0..surf.height() {
                for x in 0..surf.columns() {
                    if let Some(src) = surf.cell_mut(x, y) {
                        let cell = src.clone();
                        if let Some(dst) = shifted.cell_mut(x, shift_y + y) {
                            *dst = cell;
                        }
                    }
                }
            }
            surf = shifted;
        }

        // Padding
        if self.padding_top > 0 || self.padding_bottom > 0 || self.padding_left > 0 || self.padding_right > 0 {
            let pw = width + self.padding_left as usize + self.padding_right as usize;
            let ph = height + self.padding_top as usize + self.padding_bottom as usize;
            let mut padded = Surface::new(pw, ph, &blank);
            for y in 0..surf.height() {
                for x in 0..surf.columns() {
                    if let Some(src) = surf.cell_mut(x, y) {
                        let cell = src.clone();
                        if let Some(dst) = padded.cell_mut(self.padding_left as usize + x, self.padding_top as usize + y) {
                            *dst = cell;
                        }
                    }
                }
            }
            surf = padded;
        }

        // Border — only foreground color affects the border lines;
        // background is intentionally left unset so the terminal's default bg
        // (or whatever is already on screen) shows through.
        if let Some(border) = self.border {
            let border_fg = self.border_fg.or(self.inner.fg);
            let border_style = border_fg.map(|c| Style::new().fg(c)).unwrap_or_default();
            surf = border.draw(&surf, border_style);
        }

        // Margin
        if self.margin_top > 0 || self.margin_bottom > 0 || self.margin_left > 0 || self.margin_right > 0 {
            let mw = surf.columns() + self.margin_left as usize + self.margin_right as usize;
            let mh = surf.height() + self.margin_top as usize + self.margin_bottom as usize;
            let mut margin_style = self.inner;
            margin_style.bg = None;
            let margin_blank = Cell::blank(margin_style);
            let mut margined = Surface::new(mw, mh, &margin_blank);
            for y in 0..surf.height() {
                for x in 0..surf.columns() {
                    if let Some(src) = surf.cell_mut(x, y) {
                        let cell = src.clone();
                        if let Some(dst) = margined.cell_mut(self.margin_left as usize + x, self.margin_top as usize + y) {
                            *dst = cell;
                        }
                    }
                }
            }
            surf = margined;
        }

        surf
    }

    /// Render a string into a [`Surface`] according to this style.
    /// If a `width` is set, text will be word-wrapped to fit.
    pub fn render(&self, text: &str) -> Surface {
        let wrapped_lines = self.width.map(|w| Self::wrap_text(text, w as usize));
        let lines_ref: Vec<&str> =
            wrapped_lines.as_ref().map(|v| v.iter().map(|s| s.as_str()).collect()).unwrap_or_else(|| text.lines().collect());

        let width = self.width.map(|w| w as usize).unwrap_or_else(|| lines_ref.iter().map(|l| l.width()).max().unwrap_or(0));
        let height = self.height.map(|h| h as usize).unwrap_or_else(|| lines_ref.len());

        let blank = Cell::blank(self.inner);
        let mut surf = Surface::new(width, height, &blank);

        // Write text into the content grid
        for (y, line) in lines_ref.iter().enumerate() {
            let mut x = 0usize;
            for ch in line.chars() {
                if let Some(cell) = surf.cell_mut(x, y) {
                    *cell = Cell::new(ch.to_string(), self.inner);
                }
                x += ch.to_string().width().max(1);
            }
        }

        let content_w = lines_ref.iter().map(|l| l.width()).max().unwrap_or(0);
        let content_h = lines_ref.len();
        self.apply_layout(surf, content_w, content_h)
    }

    /// Wrap an existing [`Surface`] with this style's layout (padding,
    /// border, margin, alignment, clamp).
    pub fn render_surface(&self, surf: &Surface) -> Surface {
        let content_w = surf.columns();
        let content_h = surf.height();

        let total_extra = self.padding_left as usize
            + self.padding_right as usize
            + self.margin_left as usize
            + self.margin_right as usize;
        let border_extra = self.border.as_ref().map(|b| b.vertical_size() + b.horizontal_size()).unwrap_or(0);
        let total_extra = total_extra + border_extra;

        let content_target = self.width.map(|w| w as usize).unwrap_or(content_w);
        let target_w = content_target + total_extra;
        let target_h = self.height.map(|h| h as usize).unwrap_or(content_h)
            + self.padding_top as usize
            + self.padding_bottom as usize
            + self.margin_top as usize
            + self.margin_bottom as usize;

        let mut owned = Surface::new(target_w, target_h, &Cell::blank(self.inner));
        for (y, row) in surf.rows.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if let Some(dst) = owned.cell_mut(x, y) {
                    *dst = cell.clone();
                }
            }
        }

        if self.width.is_some() {
            crate::surface::clip(&mut owned, content_target);
        }
        // After clip, content width may be smaller than original — recalculate from owned
        let content_w = if self.width.is_some() { owned.columns() } else { content_w };
        self.apply_layout(owned, content_w, content_h)
    }
}

impl Default for BobaStyle {
    fn default() -> Self { Self::new() }
}

impl Deref for BobaStyle {
    type Target = Style;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl From<Style> for BobaStyle {
    fn from(s: Style) -> Self { Self { inner: s, ..Self::new() } }
}

impl From<BobaStyle> for Style {
    fn from(s: BobaStyle) -> Self { s.inner }
}

// ── Color helpers ──

/// Helpers for constructing [`Color`]s from HSL values.
pub fn hsl(h: f64, s: f64, l: f64) -> Color {
    let rgb: colorsys::Rgb = colorsys::Hsl::new(h, s * 100.0, l * 100.0, None).into();
    Color::Rgb(rgb.red() as u8, rgb.green() as u8, rgb.blue() as u8)
}

/// Parse a hex color string like `"#FFF"`, `"#FFFFFF"`, `"#FF0055"`.
pub fn hex_color(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1].repeat(2), 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[1..2].repeat(2), 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[2..3].repeat(2), 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    } else if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
        Color::Rgb(r, g, b)
    } else {
        Color::Black
    }
}

/// Linearly interpolate between two ratatui [`Color`]s.
pub fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    fn to_rgb(c: Color) -> (u8, u8, u8) {
        match c {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (128, 128, 128),
        }
    }
    let (r1, g1, b1) = to_rgb(a);
    let (r2, g2, b2) = to_rgb(b);
    let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t) as u8;
    Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
}

/// Pick a color from a gradient at position `t` in `[0..1]`.
pub fn gradient_color(stops: &[Color], t: f64) -> Color {
    if stops.len() < 2 {
        return stops.first().copied().unwrap_or(Color::White);
    }
    let t = t.clamp(0.0, 1.0);
    let scaled = t * (stops.len() - 1) as f64;
    let i = scaled as usize;
    let frac = scaled.fract();
    if i >= stops.len() - 1 { stops[stops.len() - 1] } else { lerp_color(stops[i], stops[i + 1], frac) }
}

/// Apply a rainbow gradient to a string, using column index and an optional time offset.
pub fn rainbow_text(text: &str, hue_offset: f64) -> Vec<(char, Color)> {
    text.chars()
        .enumerate()
        .map(|(col, ch)| {
            let hue = ((col as f64 * 1.5) + hue_offset) % 360.0;
            (ch, hsl(hue, 1.0, 0.5))
        })
        .collect()
}

/// Apply a two-color gradient to a string (character-level).
pub fn gradient_text(text: &str, from: Color, to: Color) -> Vec<(char, Color)> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len().max(2);
    chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let t = i as f64 / (n - 1) as f64;
            (ch, gradient_color(&[from, to], t))
        })
        .collect()
}

/// Linearly interpolate between two colors across `n` steps.
pub fn blend_1d(n: usize, from: Color, to: Color) -> Vec<Color> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![from];
    }
    (0..n)
        .map(|i| {
            let t = i as f64 / (n - 1) as f64;
            lerp_color(from, to, t)
        })
        .collect()
}

/// Create a 2D color grid by blending four corner colors.
pub fn blend_2d(x_steps: usize, y_steps: usize, tl: Color, tr: Color, bl: Color, br: Color) -> Vec<Vec<Color>> {
    let left_col = blend_1d(y_steps, tl, bl);
    let right_col = blend_1d(y_steps, tr, br);
    (0..y_steps)
        .map(|y| {
            let left = left_col[y];
            let right = right_col[y];
            blend_1d(x_steps, left, right)
        })
        .collect()
}

/// Pick `light` or `dark` based on a boolean flag (useful for adaptive palettes).
pub fn light_dark<T>(is_dark: bool, light: T, dark: T) -> T { if is_dark { dark } else { light } }

/// Mock background detection. Returns `true` (dark) for the MVP.
/// A future implementation can query the terminal via OSC 10/11.
pub fn has_dark_background() -> bool { true }

#[cfg(test)]
mod tests {
    use {super::*, ratatui::style::Color};

    #[test]
    fn padding_has_background() {
        let style = BobaStyle::new().bg(Color::Red).padding_all(1);
        let surf = style.render("X");
        // Content area is 1x1, padding adds 1 on all sides -> 3x3
        // Center cell (1,1) should have content with Red bg
        assert_eq!(surf.rows[1][1].style.bg, Some(Color::Red));
        // Padding cells should also have Red bg
        assert_eq!(surf.rows[0][1].style.bg, Some(Color::Red));
        assert_eq!(surf.rows[2][1].style.bg, Some(Color::Red));
        assert_eq!(surf.rows[1][0].style.bg, Some(Color::Red));
        assert_eq!(surf.rows[1][2].style.bg, Some(Color::Red));
    }

    #[test]
    fn margin_has_no_background() {
        let style = BobaStyle::new().bg(Color::Red).margin_all(1);
        let surf = style.render("X");
        // Content area is 1x1, margin adds 1 on all sides -> 3x3
        // Center cell (1,1) should have content with Red bg
        assert_eq!(surf.rows[1][1].style.bg, Some(Color::Red));
        // Margin cells should have no bg
        assert_eq!(surf.rows[0][1].style.bg, None);
        assert_eq!(surf.rows[2][1].style.bg, None);
        assert_eq!(surf.rows[1][0].style.bg, None);
        assert_eq!(surf.rows[1][2].style.bg, None);
    }
}
