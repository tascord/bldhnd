//! Screen-space effects that mutate a [`Buffer`] after widgets have rendered.
//!
//! Use these with [`crate::components::layer::LayerStack`] or apply them
//! manually to a [`ratatui::prelude::Buffer`] for full-screen post-processing.

use {
    rand::{Rng, rng},
    ratatui::{
        prelude::{Buffer, Rect},
        style::{Color, Style},
    },
    std::time::Instant,
};

/// Trait for buffer post-processing effects.
pub trait ScreenEffect {
    /// Apply the effect to `area` inside `buf`.
    ///
    /// `t` is a normalised time value (0..1) for transition-based effects, or
    /// elapsed seconds for continuous effects.
    fn apply(&self, area: Rect, buf: &mut Buffer, t: f64);
}

// ──────────────────────────────────────────────────────────────
//  Fade
// ──────────────────────────────────────────────────────────────

/// Fade every cell toward a solid colour by `opacity`.
pub struct Fade {
    pub color: Color,
    pub opacity: f64, // 0.0 = fully visible, 1.0 = fully replaced
}

impl Fade {
    pub fn new(color: Color, opacity: f64) -> Self { Self { color, opacity } }
}

impl ScreenEffect for Fade {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = buf[(x, y)].clone();
                let mixed = lerp_color(cell.fg, self.color, self.opacity);
                let bg_mixed = lerp_color(cell.bg, self.color, self.opacity * 0.5);
                buf[(x, y)].set_fg(mixed);
                buf[(x, y)].set_bg(bg_mixed);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Slide
// ──────────────────────────────────────────────────────────────

/// Shift the rendered content horizontally or vertically.
pub struct Slide {
    pub dx: i16,
    pub dy: i16,
}

impl Slide {
    pub fn new(dx: i16, dy: i16) -> Self { Self { dx, dy } }
}

impl ScreenEffect for Slide {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        let mut scratch = Buffer::empty(area);
        scratch.merge(buf);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].reset();
            }
        }
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let tx = x as i16 + self.dx;
                let ty = y as i16 + self.dy;
                if tx >= area.left() as i16
                    && tx < area.right() as i16
                    && ty >= area.top() as i16
                    && ty < area.bottom() as i16
                {
                    buf[(tx as u16, ty as u16)] = scratch[(x, y)].clone();
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Glitch
// ──────────────────────────────────────────────────────────────

/// Randomly swap symbols / colours inside a region.
pub struct Glitch {
    pub intensity: f64, // 0..1
}

impl Glitch {
    pub fn new(intensity: f64) -> Self { Self { intensity } }
}

const GLITCH_CHARS: &[char] = &['░', '▒', '▓', '█', '▀', '▄', '▌', '▐', '▖', '▗', '▘', '▙', '▚', '▛', '▜', '▝', '▞', '▟'];

impl ScreenEffect for Glitch {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        let mut rng = rng();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if rng.random::<f64>() < self.intensity {
                    let cell = &mut buf[(x, y)];
                    if rng.random_bool(0.5) {
                        let ch = GLITCH_CHARS[rng.random_range(0..GLITCH_CHARS.len())];
                        cell.set_char(ch);
                    }
                    if rng.random_bool(0.3) {
                        cell.set_style(Style::default().fg(Color::Green));
                    }
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Scanlines
// ──────────────────────────────────────────────────────────────

/// Dim every N-th row to simulate CRT scanlines.
pub struct Scanlines {
    pub line_height: u16,
    pub dim_color: Color,
}

impl Scanlines {
    pub fn new(line_height: u16) -> Self { Self { line_height, dim_color: Color::Black } }
}

impl ScreenEffect for Scanlines {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        for y in area.top()..area.bottom() {
            if (y % self.line_height) == 0 {
                for x in area.left()..area.right() {
                    buf[(x, y)].set_bg(self.dim_color);
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  ChromaticAberration
// ──────────────────────────────────────────────────────────────

/// Slightly offset R/G/B channels.
pub struct ChromaticAberration {
    pub offset: u16,
}

impl ChromaticAberration {
    pub fn new(offset: u16) -> Self { Self { offset } }
}

impl ScreenEffect for ChromaticAberration {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        let mut scratch = Buffer::empty(area);
        scratch.merge(buf);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                if x + self.offset < area.right() {
                    cell.set_fg(Color::Red);
                    cell.set_char(scratch[(x + self.offset, y)].symbol().chars().next().unwrap_or(' '));
                }
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Typewriter
// ──────────────────────────────────────────────────────────────

/// Reveal characters one by one based on elapsed time.
pub struct Typewriter {
    start: Instant,
    pub chars_per_sec: f64,
}

impl Typewriter {
    pub fn new(chars_per_sec: f64) -> Self { Self { start: Instant::now(), chars_per_sec } }

    pub fn reset(&mut self) { self.start = Instant::now(); }
}

impl ScreenEffect for Typewriter {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        let visible: usize = (self.start.elapsed().as_secs_f64() * self.chars_per_sec) as usize;
        let mut count = 0;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if count >= visible {
                    buf[(x, y)].reset();
                }
                count += 1;
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  MatrixRain
// ──────────────────────────────────────────────────────────────

/// Classic "Matrix" green rain effect across an area.
pub struct MatrixRain {
    columns: Vec<Column>,
    width: u16,
}

struct Column {
    x: u16,
    speed: u16,
    head: u16,
    length: u16,
    chars: Vec<char>,
}

impl MatrixRain {
    pub fn new(width: u16, height: u16, density: f64) -> Self {
        let cols = (width as f64 * density) as usize;
        let mut rng = rng();
        let columns = (0..cols)
            .map(|_| Column {
                x: rng.random_range(0..width),
                speed: rng.random_range(1..=3),
                head: 0,
                length: rng.random_range(5..height),
                chars: (0..height).map(|_| rng.random_range('!'..'~')).collect(),
            })
            .collect();
        Self { columns, width }
    }
}

impl ScreenEffect for MatrixRain {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) {
        let mut scratch = Buffer::empty(area);
        scratch.merge(buf);
        for col in &self.columns {
            let global_x = area.left() + col.x % area.width;
            for i in 0..col.length {
                let y = (col.head + i) % area.height;
                let global_y = area.top() + y;
                if global_y >= area.bottom() {
                    continue;
                }
                let cell = &mut buf[(global_x, global_y)];
                let ch = col.chars[y as usize % col.chars.len()];
                let brightness = 1.0 - (i as f64 / col.length as f64);
                let color = if i == 0 { Color::White } else { Color::Rgb(0, (255.0 * brightness) as u8, 0) };
                cell.set_char(ch);
                cell.set_fg(color);
                cell.set_bg(Color::Black);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Transition helpers
// ──────────────────────────────────────────────────────────────

/// An effect wrapper that tracks a normalised progress value (0..1).
pub struct Transition<E: ScreenEffect> {
    pub effect: E,
    pub progress: f64,
}

impl<E: ScreenEffect> Transition<E> {
    pub fn new(effect: E) -> Self { Self { effect, progress: 0.0 } }

    pub fn set_progress(&mut self, v: f64) { self.progress = v.clamp(0.0, 1.0); }
}

impl<E: ScreenEffect> ScreenEffect for Transition<E> {
    fn apply(&self, area: Rect, buf: &mut Buffer, _t: f64) { self.effect.apply(area, buf, self.progress); }
}

// ──────────────────────────────────────────────────────────────
//  Helpers
// ──────────────────────────────────────────────────────────────

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    fn rgb(c: Color) -> (u8, u8, u8) {
        match c {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Black => (0, 0, 0),
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Yellow => (255, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Magenta => (255, 0, 255),
            Color::Cyan => (0, 255, 255),
            Color::Gray => (170, 170, 170),
            Color::DarkGray => (85, 85, 85),
            Color::LightRed => (255, 85, 85),
            Color::LightGreen => (85, 255, 85),
            Color::LightYellow => (255, 255, 85),
            Color::LightBlue => (85, 85, 255),
            Color::LightMagenta => (255, 85, 255),
            Color::LightCyan => (85, 255, 255),
            Color::White => (255, 255, 255),
            _ => (128, 128, 128),
        }
    }
    let (r1, g1, b1) = rgb(a);
    let (r2, g2, b2) = rgb(b);
    let l = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    Color::Rgb(l(r1, r2), l(g1, g2), l(b1, b2))
}
