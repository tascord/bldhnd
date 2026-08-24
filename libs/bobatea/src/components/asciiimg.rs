//! Simple ASCII-art image renderer.
//!
//! Renders a grid of coloured characters — useful for pixel-art, QR codes,
//! or catimg-style dithered images in the terminal.
//!
//! ```rust
//! use bobatea::components::asciiimg::AsciiImage;
//! use ratatui::style::Color;
//!
//! let img = AsciiImage::new(3, 3)
//!     .pixel(0, 0, '#', Color::Red)
//!     .pixel(1, 1, '#', Color::Green)
//!     .pixel(2, 2, '#', Color::Blue);
//! ```

use {
    crate::components::Component,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Color,
    },
    std::fmt::Display,
};

/// A single raw pixel.
#[derive(Debug, Clone, Copy)]
pub struct Pixel {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Pixel {
    pub fn new(ch: char, fg: Color) -> Self { Self { ch, fg, bg: Color::Reset } }

    pub fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }
}

/// ASCII-art image component.
pub struct AsciiImage {
    width: u16,
    height: u16,
    pixels: Vec<Pixel>,
    scale_x: u16,
    scale_y: u16,
}

impl AsciiImage {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            pixels: vec![Pixel::new(' ', Color::Reset); (width * height) as usize],
            scale_x: 1,
            scale_y: 1,
        }
    }

    pub fn from_lines(lines: &[impl Display]) -> Self {
        let v: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        let h = v.len() as u16;
        let w = v.first().map(|l| l.chars().count() as u16).unwrap_or(0);
        let mut img = Self::new(w, h);
        for (y, line) in v.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                img.pixel_raw(x as u16, y as u16, Pixel::new(ch, Color::Reset));
            }
        }
        img
    }

    pub fn scale(mut self, x: u16, y: u16) -> Self {
        self.scale_x = x.max(1);
        self.scale_y = y.max(1);
        self
    }

    pub fn pixel(mut self, x: u16, y: u16, ch: char, fg: Color) -> Self {
        self.pixel_raw(x, y, Pixel::new(ch, fg));
        self
    }

    pub fn pixel_bg(mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color) -> Self {
        self.pixel_raw(x, y, Pixel::new(ch, fg).with_bg(bg));
        self
    }

    pub fn pixel_raw(&mut self, x: u16, y: u16, p: Pixel) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = (y as usize * self.width as usize + x as usize);
        self.pixels[idx] = p;
    }

    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, p: Pixel) {
        for dy in y..(y + h).min(self.height) {
            for dx in x..(x + w).min(self.width) {
                self.pixel_raw(dx, dy, p);
            }
        }
    }

    pub fn fill(&mut self, p: Pixel) {
        for i in 0..self.pixels.len() {
            self.pixels[i] = p;
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let p = &self.pixels[(y * self.width + x) as usize];
                let screen_x = area.x + x * self.scale_x;
                let screen_y = area.y + y * self.scale_y;
                if screen_x >= area.right() || screen_y >= area.bottom() {
                    continue;
                }
                for dy in 0..self.scale_y {
                    for dx in 0..self.scale_x {
                        let cell = &mut buf[(screen_x + dx, screen_y + dy)];
                        cell.set_char(p.ch);
                        cell.set_fg(p.fg);
                        cell.set_bg(p.bg);
                    }
                }
            }
        }
    }
}

impl Component for AsciiImage {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
