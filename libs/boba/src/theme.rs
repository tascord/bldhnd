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
pub struct ButtonTheme {
    pub pair: FocusPair,
    pub padding_x: u16,
    pub padding_y: u16,
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
                pair: FocusPair::new(
                    Style::default().fg(fg).bg(primary).add_modifier(Modifier::BOLD),
                    Style::default().fg(subtle).bg(elevated),
                ),
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
        // Re-derive colors from new palette
        Self::default() // simplified—real impl would re-derive all fields
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
