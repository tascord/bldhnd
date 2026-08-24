//! Pattern and gradient fills for foreground, background, and borders.
//!
//! Apply repeating patterns or smooth gradients to any widget surface.

use {
    crate::components::style::gradient_color,
    ratatui::{
        prelude::{Buffer, Rect},
        style::Color,
    },
};

/// A fill pattern that can be applied to a [`Buffer`] region.
pub trait FillPattern {
    /// Return the colour for cell `(x, y)` inside `area`.
    fn color_at(&self, x: u16, y: u16, area: Rect) -> Color;
}

// ──────────────────────────────────────────────────────────────
//  Linear gradient
// ──────────────────────────────────────────────────────────────

/// Smooth colour interpolation along a direction.
pub struct LinearGradient {
    stops: Vec<Color>,
    angle_deg: f64,
}

impl LinearGradient {
    pub fn new(stops: impl Into<Vec<Color>>) -> Self { Self { stops: stops.into(), angle_deg: 0.0 } }

    /// Rotate the gradient direction (0 = left→right, 90 = top→bottom).
    pub fn angle(mut self, deg: f64) -> Self {
        self.angle_deg = deg;
        self
    }
}

impl FillPattern for LinearGradient {
    fn color_at(&self, x: u16, y: u16, area: Rect) -> Color {
        let (ux, uy) = (x - area.left(), y - area.top());
        let w = area.width.max(1);
        let h = area.height.max(1);
        let t = match self.angle_deg as i32 {
            90 => (uy as f64) / (h as f64),
            180 => 1.0 - (ux as f64) / (w as f64),
            270 => 1.0 - (uy as f64) / (h as f64),
            _ => (ux as f64) / (w as f64),
        }
        .clamp(0.0, 1.0);
        gradient_color(&self.stops, t)
    }
}

// ──────────────────────────────────────────────────────────────
//  Radial gradient
// ──────────────────────────────────────────────────────────────

/// Circular gradient from center outward.
pub struct RadialGradient {
    stops: Vec<Color>,
    center: (f64, f64), // normalised 0..1
}

impl RadialGradient {
    pub fn new(stops: impl Into<Vec<Color>>) -> Self { Self { stops: stops.into(), center: (0.5, 0.5) } }

    pub fn center(mut self, x: f64, y: f64) -> Self {
        self.center = (x, y);
        self
    }
}

impl FillPattern for RadialGradient {
    fn color_at(&self, x: u16, y: u16, area: Rect) -> Color {
        let dx = (x - area.left()) as f64 / area.width.max(1) as f64 - self.center.0;
        let dy = (y - area.top()) as f64 / area.height.max(1) as f64 - self.center.1;
        let d = (dx * dx + dy * dy).sqrt().clamp(0.0, 1.0);
        gradient_color(&self.stops, d)
    }
}

// ──────────────────────────────────────────────────────────────
//  Checkerboard
// ──────────────────────────────────────────────────────────────

/// Alternating two-colour checker pattern.
pub struct Checkerboard {
    pub a: Color,
    pub b: Color,
    pub size: u16,
}

impl Checkerboard {
    pub fn new(a: Color, b: Color, size: u16) -> Self { Self { a, b, size: size.max(1) } }
}

impl FillPattern for Checkerboard {
    fn color_at(&self, x: u16, y: u16, _area: Rect) -> Color {
        if ((x / self.size) + (y / self.size)) % 2 == 0 { self.a } else { self.b }
    }
}

// ──────────────────────────────────────────────────────────────
//  Stripes
// ──────────────────────────────────────────────────────────────

/// Horizontal or vertical stripes.
pub struct Stripes {
    pub colors: Vec<Color>,
    pub width: u16,
    pub vertical: bool,
}

impl Stripes {
    pub fn new(colors: impl Into<Vec<Color>>, width: u16) -> Self {
        Self { colors: colors.into(), width: width.max(1), vertical: false }
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }
}

impl FillPattern for Stripes {
    fn color_at(&self, x: u16, y: u16, _area: Rect) -> Color {
        let idx = if self.vertical { (x / self.width) as usize } else { (y / self.width) as usize } % self.colors.len();
        self.colors[idx]
    }
}

// ──────────────────────────────────────────────────────────────
//  Diamond
// ──────────────────────────────────────────────────────────────

/// Diamond / argyle-like repeating pattern.
pub struct Diamond {
    pub a: Color,
    pub b: Color,
    pub size: u16,
}

impl Diamond {
    pub fn new(a: Color, b: Color, size: u16) -> Self { Self { a, b, size: size.max(1) } }
}

impl FillPattern for Diamond {
    fn color_at(&self, x: u16, y: u16, _area: Rect) -> Color {
        let s = self.size as i32;
        let px = (x as i32 % (2 * s)) - s;
        let py = (y as i32 % (2 * s)) - s;
        if px.abs() + py.abs() < s { self.a } else { self.b }
    }
}

// ──────────────────────────────────────────────────────────────
//  Noise
// ──────────────────────────────────────────────────────────────

/// Simple pseudo-random dither between two colours.
pub struct Noise {
    pub a: Color,
    pub b: Color,
    pub threshold: f64,
}

impl Noise {
    pub fn new(a: Color, b: Color, threshold: f64) -> Self { Self { a, b, threshold } }
}

impl FillPattern for Noise {
    fn color_at(&self, x: u16, y: u16, _area: Rect) -> Color {
        // simple hash
        let h = ((x as u64 * 6364136223846793005 + y as u64 * 1442695040888963407) >> 32) as f64 / u32::MAX as f64;
        if h < self.threshold { self.a } else { self.b }
    }
}

// ──────────────────────────────────────────────────────────────
//  Apply helpers
// ──────────────────────────────────────────────────────────────

/// Apply a pattern to the **foreground** of every cell in `area`.
pub fn apply_fg(area: Rect, buf: &mut Buffer, pattern: &impl FillPattern) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let c = pattern.color_at(x, y, area);
            buf[(x, y)].set_fg(c);
        }
    }
}

/// Apply a pattern to the **background** of every cell in `area`.
pub fn apply_bg(area: Rect, buf: &mut Buffer, pattern: &impl FillPattern) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let c = pattern.color_at(x, y, area);
            buf[(x, y)].set_bg(c);
        }
    }
}

/// Apply a pattern **only** to the border cells of `area`.
pub fn apply_border(area: Rect, buf: &mut Buffer, pattern: &impl FillPattern) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    // top & bottom
    for x in area.left()..area.right() {
        buf[(x, area.top())].set_fg(pattern.color_at(x, area.top(), area));
        buf[(x, area.bottom().saturating_sub(1))].set_fg(pattern.color_at(x, area.bottom().saturating_sub(1), area));
    }
    // left & right
    for y in area.top()..area.bottom() {
        buf[(area.left(), y)].set_fg(pattern.color_at(area.left(), y, area));
        buf[(area.right().saturating_sub(1), y)].set_fg(pattern.color_at(area.right().saturating_sub(1), y, area));
    }
}

/// Linear gradient factory — shorthand.
pub fn gradient(stops: &[Color]) -> LinearGradient { LinearGradient::new(stops.to_vec()) }

/// Checkerboard factory.
pub fn checker(a: Color, b: Color, size: u16) -> Checkerboard { Checkerboard::new(a, b, size) }

/// Stripes factory.
pub fn stripes(colors: &[Color], width: u16) -> Stripes { Stripes::new(colors.to_vec(), width) }
