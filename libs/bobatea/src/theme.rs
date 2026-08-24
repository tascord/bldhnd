//! Runtime theming system with HSL palette and semantic style mapping.
//!
//! Every component receives `&Theme` at render time, enabling hot-swapping.

use ratatui::style::{Color, Modifier, Style};

// ──────────────────────────────────────────────────────────────
//  HSL Color (storage + conversion)
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsl {
    pub h: f64, // 0–360
    pub s: f64, // 0–1
    pub l: f64, // 0–1
}

impl Hsl {
    pub fn new(h: f64, s: f64, l: f64) -> Self { Self { h: h % 360.0, s: s.clamp(0.0, 1.0), l: l.clamp(0.0, 1.0) } }

    pub fn to_rgb(self) -> Color {
        let rgb: colorsys::Rgb = colorsys::Hsl::new(self.h, self.s * 100.0, self.l * 100.0, None).into();
        Color::Rgb(rgb.red() as u8, rgb.green() as u8, rgb.blue() as u8)
    }

    pub fn darken(self, amount: f64) -> Self { Self { l: (self.l - amount).clamp(0.0, 1.0), ..self } }

    pub fn lighten(self, amount: f64) -> Self { Self { l: (self.l + amount).clamp(0.0, 1.0), ..self } }

    pub fn saturate(self, amount: f64) -> Self { Self { s: (self.s + amount).clamp(0.0, 1.0), ..self } }

    pub fn desaturate(self, amount: f64) -> Self { Self { s: (self.s - amount).clamp(0.0, 1.0), ..self } }
}

impl From<Hsl> for Color {
    fn from(h: Hsl) -> Self { h.to_rgb() }
}

// ──────────────────────────────────────────────────────────────
//  Palette
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Palette {
    pub primary: Hsl,
    pub secondary: Hsl,
    pub accent: Hsl,
    pub destructive: Hsl,
    pub success: Hsl,
    pub warning: Hsl,
    pub info: Hsl,
    pub fg_base: Hsl,
    pub fg_subtle: Hsl,
    pub fg_muted: Hsl,
    pub bg_base: Hsl,
    pub bg_elevated: Hsl,
    pub bg_overlay: Hsl,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            primary: Hsl::new(200.0, 0.85, 0.55),
            secondary: Hsl::new(260.0, 0.70, 0.60),
            accent: Hsl::new(180.0, 0.90, 0.55),
            destructive: Hsl::new(0.0, 0.80, 0.55),
            success: Hsl::new(140.0, 0.70, 0.50),
            warning: Hsl::new(45.0, 0.90, 0.55),
            info: Hsl::new(200.0, 0.85, 0.60),
            fg_base: Hsl::new(0.0, 0.0, 0.95),
            fg_subtle: Hsl::new(0.0, 0.0, 0.70),
            fg_muted: Hsl::new(0.0, 0.0, 0.45),
            bg_base: Hsl::new(220.0, 0.15, 0.08),
            bg_elevated: Hsl::new(220.0, 0.12, 0.12),
            bg_overlay: Hsl::new(220.0, 0.10, 0.15),
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Semantic Style Pairs (Focused / Blurred)
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FocusPair {
    pub focused: Style,
    pub blurred: Style,
}

impl FocusPair {
    pub fn new(focused: Style, blurred: Style) -> Self { Self { focused, blurred } }

    pub fn pick(&self, is_focused: bool) -> Style { if is_focused { self.focused } else { self.blurred } }
}

// ──────────────────────────────────────────────────────────────
//  Component-Specific Themes
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ButtonStyles {
    pub default: Style,
    pub hovered: Style,
    pub pressed: Style,
    pub focused: Style,
    pub disabled: Style,
}

impl ButtonStyles {
    pub fn style_for_state(&self, state: crate::components::button::ButtonState) -> Style {
        use crate::components::button::ButtonState;
        match state {
            ButtonState::Default => self.default,
            ButtonState::Hovered => self.hovered,
            ButtonState::Pressed => self.pressed,
            ButtonState::Focused => self.focused,
            ButtonState::Disabled => self.disabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonTheme {
    pub styles: ButtonStyles,
    pub padding_x: u16,
    pub padding_y: u16,
}

impl ButtonTheme {
    pub fn style_for_state(&self, state: crate::components::button::ButtonState) -> Style {
        self.styles.style_for_state(state)
    }
}

#[derive(Debug, Clone)]
pub struct InputTheme {
    pub pair: FocusPair,
    pub placeholder_fg: Color,
    pub cursor_bg: Color,
}

#[derive(Debug, Clone)]
pub struct ListTheme {
    pub pair: FocusPair,
    pub selected_glyph: String,
    pub unselected_glyph: String,
}

#[derive(Debug, Clone)]
pub struct DialogTheme {
    pub title: Style,
    pub view: Style,
    pub border: Style,
    pub dim_bg: Color,
    pub normal_item: Style,
    pub selected_item: Style,
    pub title_grad_from: Hsl,
    pub title_grad_to: Hsl,
}

#[derive(Debug, Clone)]
pub struct ToastTheme {
    pub info: Style,
    pub warn: Style,
    pub error: Style,
    pub info_border: Color,
    pub warn_border: Color,
    pub error_border: Color,
}

#[derive(Debug, Clone)]
pub struct SpinnerTheme {
    pub fg: Color,
    pub label_fg: Color,
}

#[derive(Debug, Clone)]
pub struct ProgressTheme {
    pub filled: Color,
    pub empty: Color,
    pub label_fg: Color,
}

#[derive(Debug, Clone)]
pub struct HelpTheme {
    pub key_bg: Color,
    pub key_fg: Color,
    pub desc_fg: Color,
    pub separator_fg: Color,
}

// ──────────────────────────────────────────────────────────────
//  The Big Theme Struct
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Theme {
    pub palette: Palette,
    pub global_bg: Color,
    pub global_fg: Color,
    pub button: ButtonTheme,
    pub input: InputTheme,
    pub list: ListTheme,
    pub dialog: DialogTheme,
    pub toast: ToastTheme,
    pub spinner: SpinnerTheme,
    pub progress: ProgressTheme,
    pub help: HelpTheme,
    pub badge_info: Style,
    pub badge_success: Style,
    pub badge_warn: Style,
    pub badge_error: Style,
    pub badge_primary: Style,
    pub border_accent: Color,
    pub border_subtle: Color,
    pub gradient_working_from: Hsl,
    pub gradient_working_to: Hsl,
}

impl Theme {
    pub fn from_palette(p: Palette) -> Self {
        let bg = p.bg_base.to_rgb();
        let fg = p.fg_base.to_rgb();
        let subtle = p.fg_subtle.to_rgb();
        let muted = p.fg_muted.to_rgb();
        let elevated = p.bg_elevated.to_rgb();
        let overlay = p.bg_overlay.to_rgb();

        let primary = p.primary.to_rgb();
        let accent = p.accent.to_rgb();
        let destructive = p.destructive.to_rgb();
        let success = p.success.to_rgb();
        let warning = p.warning.to_rgb();
        let info = p.info.to_rgb();

        Self {
            palette: p.clone(),
            global_bg: bg,
            global_fg: fg,
            button: ButtonTheme {
                styles: ButtonStyles {
                    default: Style::default().fg(subtle).bg(elevated),
                    hovered: Style::default().fg(fg).bg(elevated).add_modifier(Modifier::BOLD),
                    pressed: Style::default().fg(bg).bg(primary),
                    focused: Style::default().fg(fg).bg(primary).add_modifier(Modifier::BOLD),
                    disabled: Style::default().fg(muted).bg(elevated),
                },
                padding_x: 2,
                padding_y: 0,
            },
            input: InputTheme {
                pair: FocusPair::new(
                    Style::default().fg(fg).bg(overlay).add_modifier(Modifier::BOLD),
                    Style::default().fg(subtle).bg(bg),
                ),
                placeholder_fg: muted,
                cursor_bg: Color::White,
            },
            list: ListTheme {
                pair: FocusPair::new(
                    Style::default().fg(fg).bg(elevated).add_modifier(Modifier::BOLD),
                    Style::default().fg(subtle).bg(bg),
                ),
                selected_glyph: "▸".into(),
                unselected_glyph: " ".into(),
            },
            dialog: DialogTheme {
                title: Style::default().fg(fg).add_modifier(Modifier::BOLD),
                view: Style::default().fg(fg).bg(overlay),
                border: Style::default().fg(accent),
                dim_bg: Color::Rgb(0, 0, 0),
                normal_item: Style::default().fg(subtle),
                selected_item: Style::default().fg(fg).bg(primary),
                title_grad_from: p.accent,
                title_grad_to: p.primary,
            },
            toast: ToastTheme {
                info: Style::default().fg(info),
                warn: Style::default().fg(warning),
                error: Style::default().fg(destructive),
                info_border: info,
                warn_border: warning,
                error_border: destructive,
            },
            spinner: SpinnerTheme { fg: accent, label_fg: subtle },
            progress: ProgressTheme { filled: primary, empty: elevated, label_fg: subtle },
            help: HelpTheme { key_bg: muted, key_fg: bg, desc_fg: subtle, separator_fg: muted },
            badge_info: Style::default().fg(bg).bg(info),
            badge_success: Style::default().fg(bg).bg(success),
            badge_warn: Style::default().fg(bg).bg(warning),
            badge_error: Style::default().fg(bg).bg(destructive),
            badge_primary: Style::default().fg(bg).bg(primary),
            border_accent: accent,
            border_subtle: muted,
            gradient_working_from: p.accent,
            gradient_working_to: p.primary,
        }
    }

    pub fn new() -> Self { Self::default() }

    pub fn light() -> Self {
        Self::from_palette(Palette {
            primary: Hsl::new(210.0, 0.90, 0.50),
            secondary: Hsl::new(260.0, 0.70, 0.60),
            accent: Hsl::new(180.0, 0.90, 0.45),
            destructive: Hsl::new(0.0, 0.80, 0.55),
            success: Hsl::new(140.0, 0.70, 0.45),
            warning: Hsl::new(45.0, 0.90, 0.55),
            info: Hsl::new(200.0, 0.85, 0.55),
            fg_base: Hsl::new(0.0, 0.0, 0.10),
            fg_subtle: Hsl::new(0.0, 0.0, 0.40),
            fg_muted: Hsl::new(0.0, 0.0, 0.55),
            bg_base: Hsl::new(0.0, 0.0, 0.98),
            bg_elevated: Hsl::new(0.0, 0.0, 0.94),
            bg_overlay: Hsl::new(0.0, 0.0, 0.90),
        })
    }

    pub fn ocean() -> Self {
        Self::from_palette(Palette {
            primary: Hsl::new(170.0, 0.90, 0.60),
            secondary: Hsl::new(220.0, 0.80, 0.60),
            accent: Hsl::new(160.0, 0.95, 0.65),
            destructive: Hsl::new(0.0, 0.80, 0.60),
            success: Hsl::new(140.0, 0.70, 0.60),
            warning: Hsl::new(45.0, 0.90, 0.65),
            info: Hsl::new(200.0, 0.85, 0.70),
            fg_base: Hsl::new(190.0, 0.10, 0.95),
            fg_subtle: Hsl::new(190.0, 0.10, 0.75),
            fg_muted: Hsl::new(190.0, 0.10, 0.55),
            bg_base: Hsl::new(200.0, 0.30, 0.12),
            bg_elevated: Hsl::new(200.0, 0.25, 0.16),
            bg_overlay: Hsl::new(200.0, 0.20, 0.20),
        })
    }

    pub fn solarized() -> Self {
        Self::from_palette(Palette {
            primary: Hsl::new(18.0, 0.80, 0.44),      // orange
            secondary: Hsl::new(68.0, 1.0, 0.35),     // yellow-green
            accent: Hsl::new(175.0, 0.74, 0.45),      // cyan
            destructive: Hsl::new(1.0, 0.79, 0.53),   // red
            success: Hsl::new(68.0, 1.0, 0.35),       // green
            warning: Hsl::new(45.0, 1.0, 0.55),       // yellow
            info: Hsl::new(205.0, 0.69, 0.49),        // blue
            fg_base: Hsl::new(44.0, 0.21, 0.46),      // base0
            fg_subtle: Hsl::new(44.0, 0.18, 0.40),    // base00
            fg_muted: Hsl::new(44.0, 0.14, 0.33),     // base01
            bg_base: Hsl::new(192.0, 1.0, 0.97),      // base3
            bg_elevated: Hsl::new(192.0, 0.90, 0.92), // base2
            bg_overlay: Hsl::new(192.0, 0.80, 0.87),  // base2 darker
        })
    }

    pub fn high_contrast() -> Self {
        Self::from_palette(Palette {
            primary: Hsl::new(220.0, 1.0, 0.55),   // bright blue
            secondary: Hsl::new(280.0, 1.0, 0.60), // bright purple
            accent: Hsl::new(50.0, 1.0, 0.60),     // bright yellow
            destructive: Hsl::new(0.0, 1.0, 0.60), // bright red
            success: Hsl::new(120.0, 1.0, 0.50),   // bright green
            warning: Hsl::new(30.0, 1.0, 0.60),    // bright orange
            info: Hsl::new(200.0, 1.0, 0.60),      // bright cyan
            fg_base: Hsl::new(0.0, 0.0, 1.0),      // white
            fg_subtle: Hsl::new(0.0, 0.0, 0.90),   // near-white
            fg_muted: Hsl::new(0.0, 0.0, 0.70),    // light gray
            bg_base: Hsl::new(0.0, 0.0, 0.0),      // black
            bg_elevated: Hsl::new(0.0, 0.0, 0.15), // dark gray
            bg_overlay: Hsl::new(0.0, 0.0, 0.25),  // medium gray
        })
    }
}

impl Default for Theme {
    fn default() -> Self { Self::from_palette(Palette::default()) }
}

impl Theme {
    pub fn with_palette(mut self, f: impl FnOnce(&mut Palette)) -> Self {
        (f)(&mut self.palette);
        Self::from_palette(self.palette)
    }
}

// ──────────────────────────────────────────────────────────────
//  ThemeBuilder (user-specified themes)
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ThemeBuilder {
    palette: Palette,
}

impl ThemeBuilder {
    pub fn new() -> Self { Self { palette: Palette::default() } }

    pub fn from_palette(p: Palette) -> Self { Self { palette: p } }

    pub fn primary(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.primary = Hsl::new(h, s, l);
        self
    }

    pub fn secondary(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.secondary = Hsl::new(h, s, l);
        self
    }

    pub fn accent(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.accent = Hsl::new(h, s, l);
        self
    }

    pub fn destructive(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.destructive = Hsl::new(h, s, l);
        self
    }

    pub fn success(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.success = Hsl::new(h, s, l);
        self
    }

    pub fn warning(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.warning = Hsl::new(h, s, l);
        self
    }

    pub fn info(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.info = Hsl::new(h, s, l);
        self
    }

    pub fn fg_base(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.fg_base = Hsl::new(h, s, l);
        self
    }

    pub fn fg_subtle(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.fg_subtle = Hsl::new(h, s, l);
        self
    }

    pub fn fg_muted(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.fg_muted = Hsl::new(h, s, l);
        self
    }

    pub fn bg_base(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.bg_base = Hsl::new(h, s, l);
        self
    }

    pub fn bg_elevated(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.bg_elevated = Hsl::new(h, s, l);
        self
    }

    pub fn bg_overlay(mut self, h: f64, s: f64, l: f64) -> Self {
        self.palette.bg_overlay = Hsl::new(h, s, l);
        self
    }

    pub fn primary_color(mut self, c: Color) -> Self {
        self.palette.primary = Hsl::from_rgb(&c);
        self
    }

    pub fn secondary_color(mut self, c: Color) -> Self {
        self.palette.secondary = Hsl::from_rgb(&c);
        self
    }

    pub fn accent_color(mut self, c: Color) -> Self {
        self.palette.accent = Hsl::from_rgb(&c);
        self
    }

    pub fn destructive_color(mut self, c: Color) -> Self {
        self.palette.destructive = Hsl::from_rgb(&c);
        self
    }

    pub fn success_color(mut self, c: Color) -> Self {
        self.palette.success = Hsl::from_rgb(&c);
        self
    }

    pub fn warning_color(mut self, c: Color) -> Self {
        self.palette.warning = Hsl::from_rgb(&c);
        self
    }

    pub fn info_color(mut self, c: Color) -> Self {
        self.palette.info = Hsl::from_rgb(&c);
        self
    }

    pub fn fg_base_color(mut self, c: Color) -> Self {
        self.palette.fg_base = Hsl::from_rgb(&c);
        self
    }

    pub fn fg_subtle_color(mut self, c: Color) -> Self {
        self.palette.fg_subtle = Hsl::from_rgb(&c);
        self
    }

    pub fn fg_muted_color(mut self, c: Color) -> Self {
        self.palette.fg_muted = Hsl::from_rgb(&c);
        self
    }

    pub fn bg_base_color(mut self, c: Color) -> Self {
        self.palette.bg_base = Hsl::from_rgb(&c);
        self
    }

    pub fn bg_elevated_color(mut self, c: Color) -> Self {
        self.palette.bg_elevated = Hsl::from_rgb(&c);
        self
    }

    pub fn bg_overlay_color(mut self, c: Color) -> Self {
        self.palette.bg_overlay = Hsl::from_rgb(&c);
        self
    }

    pub fn build(self) -> Theme { Theme::from_palette(self.palette) }
}

impl Default for ThemeBuilder {
    fn default() -> Self { Self::new() }
}

impl Hsl {
    fn from_rgb(c: &Color) -> Self {
        match c {
            Color::Rgb(r, g, b) => {
                let rgb: colorsys::Rgb = colorsys::Rgb::new(*r as f64, *g as f64, *b as f64, None);
                let hsl: colorsys::Hsl = rgb.into();
                Hsl::new(hsl.hue(), hsl.saturation() / 100.0, hsl.lightness() / 100.0)
            }
            Color::Indexed(i) => {
                let ansi = match i {
                    0 => [46.0, 46.0, 46.0],
                    1 => [187.0, 0.0, 0.0],
                    2 => [0.0, 187.0, 0.0],
                    3 => [187.0, 187.0, 0.0],
                    4 => [0.0, 0.0, 187.0],
                    5 => [187.0, 0.0, 187.0],
                    6 => [0.0, 187.0, 187.0],
                    7 => [255.0, 255.0, 255.0],
                    8 => [128.0, 128.0, 128.0],
                    9 => [255.0, 0.0, 0.0],
                    10 => [0.0, 255.0, 0.0],
                    11 => [255.0, 255.0, 0.0],
                    12 => [0.0, 0.0, 255.0],
                    13 => [255.0, 0.0, 255.0],
                    14 => [0.0, 255.0, 255.0],
                    15 => [255.0, 255.0, 255.0],
                    _ => [128.0, 128.0, 128.0],
                };
                let rgb: colorsys::Rgb = colorsys::Rgb::new(ansi[0], ansi[1], ansi[2], None);
                let hsl: colorsys::Hsl = rgb.into();
                Hsl::new(hsl.hue(), hsl.saturation() / 100.0, hsl.lightness() / 100.0)
            }
            _ => Hsl::new(0.0, 0.0, 0.5),
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Image Palette Extraction
// ──────────────────────────────────────────────────────────────

pub struct ImagePaletteExtractor {
    max_colors: usize,
    quality: u32,
}

impl ImagePaletteExtractor {
    pub fn new(max_colors: usize) -> Self { Self { max_colors, quality: 5 } }

    pub fn quality(mut self, q: u32) -> Self {
        self.quality = q.max(1);
        self
    }

    pub fn extract(&self, img_path: &str) -> anyhow::Result<Palette> {
        use image::{GenericImageView, Pixel};
        let img = image::open(img_path)?;
        let (w, h) = img.dimensions();
        let total = (w * h) as usize;

        let mut pixels: Vec<[f64; 3]> = Vec::with_capacity(total);
        for y in 0..h {
            for x in 0..w {
                if (y * w + x) as u32 % self.quality != 0 {
                    continue;
                }
                let px = img.get_pixel(x, y);
                let [r, g, b] = px.to_rgb().0;
                let r = r as f64 / 255.0;
                let g = g as f64 / 255.0;
                let b = b as f64 / 255.0;
                let brightness = (r + g + b) / 3.0;
                if brightness > 0.05 && brightness < 0.95 {
                    pixels.push([r, g, b]);
                }
            }
        }

        if pixels.is_empty() {
            return Ok(Palette::default());
        }

        let palette = self.kmeans(&pixels, self.max_colors);

        let bg_colors: Vec<_> = palette
            .iter()
            .filter(|c| {
                let lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                lum < 0.4
            })
            .cloned()
            .collect();

        let fg_colors: Vec<_> = palette
            .iter()
            .filter(|c| {
                let lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                lum > 0.6
            })
            .cloned()
            .collect();

        let accent_colors: Vec<_> = palette
            .iter()
            .filter(|c| {
                let lum = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
                lum > 0.4 && lum < 0.6
            })
            .cloned()
            .collect();

        let avg_bg = if !bg_colors.is_empty() {
            let sum = bg_colors.iter().fold([0.0; 3], |acc: [f64; 3], c| [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]]);
            [sum[0] / bg_colors.len() as f64, sum[1] / bg_colors.len() as f64, sum[2] / bg_colors.len() as f64]
        } else {
            palette[0]
        };

        let avg_fg = if !fg_colors.is_empty() {
            let sum = fg_colors.iter().fold([0.0; 3], |acc: [f64; 3], c| [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]]);
            [sum[0] / fg_colors.len() as f64, sum[1] / fg_colors.len() as f64, sum[2] / fg_colors.len() as f64]
        } else {
            palette[1]
        };

        let avg_accent = if !accent_colors.is_empty() {
            let sum = accent_colors.iter().fold([0.0; 3], |acc: [f64; 3], c| [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]]);
            [sum[0] / accent_colors.len() as f64, sum[1] / accent_colors.len() as f64, sum[2] / accent_colors.len() as f64]
        } else {
            palette[2]
        };

        Ok(Palette {
            primary: Self::rgb_to_hsl(avg_accent[0], avg_accent[1], avg_accent[2]),
            secondary: Self::rgb_to_hsl(
                palette[3 % self.max_colors][0],
                palette[3 % self.max_colors][1],
                palette[3 % self.max_colors][2],
            ),
            accent: Self::rgb_to_hsl(avg_accent[0], avg_accent[1], avg_accent[2]),
            destructive: Hsl::new(0.0, 0.8, 0.55),
            success: Hsl::new(140.0, 0.7, 0.5),
            warning: Hsl::new(45.0, 0.9, 0.55),
            info: Hsl::new(200.0, 0.85, 0.6),
            fg_base: Self::rgb_to_hsl(avg_fg[0], avg_fg[1], avg_fg[2]),
            fg_subtle: Self::rgb_to_hsl(avg_fg[0] * 0.7 + 0.3, avg_fg[1] * 0.7 + 0.3, avg_fg[2] * 0.7 + 0.3),
            fg_muted: Self::rgb_to_hsl(avg_fg[0] * 0.5 + 0.5, avg_fg[1] * 0.5 + 0.5, avg_fg[2] * 0.5 + 0.5),
            bg_base: Self::rgb_to_hsl(avg_bg[0], avg_bg[1], avg_bg[2]),
            bg_elevated: Self::rgb_to_hsl(avg_bg[0] * 0.9 + 0.1, avg_bg[1] * 0.9 + 0.1, avg_bg[2] * 0.9 + 0.1),
            bg_overlay: Self::rgb_to_hsl(avg_bg[0] * 0.8 + 0.2, avg_bg[1] * 0.8 + 0.2, avg_bg[2] * 0.8 + 0.2),
        })
    }

    fn kmeans(&self, pixels: &[[f64; 3]], k: usize) -> Vec<[f64; 3]> {
        if pixels.is_empty() || k == 0 {
            return vec![];
        }

        let k = k.min(pixels.len());
        let mut centroids: Vec<[f64; 3]> = pixels.iter().step_by(pixels.len() / k).take(k).cloned().collect();
        while centroids.len() < k {
            centroids.push(pixels[centroids.len() % pixels.len()]);
        }

        for _ in 0..20 {
            let mut clusters: Vec<Vec<[f64; 3]>> = vec![vec![]; k];

            for px in pixels {
                let mut min_dist = f64::MAX;
                let mut closest = 0;
                for (i, c) in centroids.iter().enumerate() {
                    let dist = (px[0] - c[0]).powi(2) + (px[1] - c[1]).powi(2) + (px[2] - c[2]).powi(2);
                    if dist < min_dist {
                        min_dist = dist;
                        closest = i;
                    }
                }
                clusters[closest].push(*px);
            }

            let mut changed = false;
            for (i, cluster) in clusters.iter().enumerate() {
                if cluster.is_empty() {
                    continue;
                }
                let new_centroid = [
                    cluster.iter().map(|p| p[0]).sum::<f64>() / cluster.len() as f64,
                    cluster.iter().map(|p| p[1]).sum::<f64>() / cluster.len() as f64,
                    cluster.iter().map(|p| p[2]).sum::<f64>() / cluster.len() as f64,
                ];
                if (new_centroid[0] - centroids[i][0]).abs() > 0.001
                    || (new_centroid[1] - centroids[i][1]).abs() > 0.001
                    || (new_centroid[2] - centroids[i][2]).abs() > 0.001
                {
                    centroids[i] = new_centroid;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        centroids
    }

    fn rgb_to_hsl(r: f64, g: f64, b: f64) -> Hsl {
        let rgb: colorsys::Rgb = colorsys::Rgb::new(r * 255.0, g * 255.0, b * 255.0, None);
        let hsl: colorsys::Hsl = rgb.into();
        Hsl::new(hsl.hue(), hsl.saturation() / 100.0, hsl.lightness() / 100.0)
    }
}

// ──────────────────────────────────────────────────────────────
//  Helpers
// ──────────────────────────────────────────────────────────────

pub fn lerp_hsl(a: Hsl, b: Hsl, t: f64) -> Hsl {
    Hsl::new(
        (a.h + (b.h - a.h) * t) % 360.0,
        (a.s + (b.s - a.s) * t).clamp(0.0, 1.0),
        (a.l + (b.l - a.l) * t).clamp(0.0, 1.0),
    )
}

pub fn gradient_hsl(stops: &[Hsl], t: f64) -> Hsl {
    if stops.len() < 2 {
        return stops.first().copied().unwrap_or(Hsl::new(0.0, 0.0, 1.0));
    }
    let t = t.clamp(0.0, 1.0);
    let scaled = t * (stops.len() - 1) as f64;
    let i = scaled as usize;
    let frac = scaled.fract();
    if i >= stops.len() - 1 { stops[stops.len() - 1] } else { lerp_hsl(stops[i], stops[i + 1], frac) }
}
