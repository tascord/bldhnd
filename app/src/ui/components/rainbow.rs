use {
    ratatui::{
        style::Color,
        text::{Line, Span, Text},
    },
    std::time::Instant,
};

const HUE_STEP: f64 = 1.5;
const ANIM_SPEED: f64 = 40.0;

pub struct Rainbow {
    start: Instant,
}

impl Rainbow {
    pub fn new() -> Self {
        Self { start: Instant::now() }
    }

    pub fn apply(&self, text: &Text<'_>) -> Text<'static> {
        let time_offset = self.start.elapsed().as_secs_f64() * ANIM_SPEED;

        let lines: Vec<Line<'static>> = text
            .lines
            .iter()
            .enumerate()
            .map(|(row, line)| {
                let mut col: usize = 0;
                let spans: Vec<Span<'static>> = line
                    .spans
                    .iter()
                    .flat_map(|span| {
                        span.content
                            .chars()
                            .map(|ch| {
                                let hue = ((col + row) as f64 * HUE_STEP + time_offset) % 360.0;
                                col += 1;
                                let style = span.style.fg(hsl_to_rgb(hue, 1.0, 0.5));
                                Span::styled(ch.to_string(), style)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();
                Line::from(spans)
            })
            .collect();

        Text::from(lines)
    }
}

impl Default for Rainbow {
    fn default() -> Self {
        Self::new()
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Color::Rgb(((r1 + m) * 255.0).round() as u8, ((g1 + m) * 255.0).round() as u8, ((b1 + m) * 255.0).round() as u8)
}
