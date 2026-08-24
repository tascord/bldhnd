//! Powerline-style status bar and command palette.
//!
//! ```rust
//! use bobatea::components::powerline::{Powerline, Segment};
//! use ratatui::style::Color;
//! let bar = Powerline::new(vec![
//!     Segment::text(" normal ").fg(Color::Blue),
//!     Segment::text(" ~/projects ").fg(Color::Cyan),
//!     Segment::arrow().fg(Color::Cyan).bg(Color::Green),
//!     Segment::text(" main ").fg(Color::Green).bg(Color::Black),
//! ]);
//! ```

use {
    crate::components::{Component, style::BobaStyle},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Color,
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

/// A single powerline segment.
#[derive(Clone)]
pub struct Segment {
    text: String,
    fg: Color,
    bg: Color,
    is_arrow: bool,
}

impl Segment {
    pub fn text(t: impl Display) -> Self {
        Self { text: t.to_string(), fg: Color::White, bg: Color::Reset, is_arrow: false }
    }

    pub fn arrow() -> Self { Self { text: "".into(), fg: Color::White, bg: Color::Reset, is_arrow: true } }

    pub fn fg(mut self, c: Color) -> Self {
        self.fg = c;
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }

    fn width(&self) -> usize { if self.is_arrow { 1 } else { self.text.chars().count() } }
}

/// Powerline status bar that renders segments with arrow separators.
pub struct Powerline {
    segments: Mutable<Vec<Segment>>,
    arrow_char: char,
}

impl Powerline {
    pub fn new(segments: Vec<Segment>) -> Self { Self { segments: Mutable::new(segments), arrow_char: '▶' } }

    pub fn with_arrow(mut self, ch: char) -> Self {
        self.arrow_char = ch;
        self
    }

    pub fn set_segments(&self, segments: Vec<Segment>) { self.segments.set(segments); }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let segments = self.segments.lock_ref();
        let mut x = area.x;

        for (i, seg) in segments.iter().enumerate() {
            let w = seg.width() as u16;
            if x + w > area.right() {
                break;
            }

            let rect = Rect { x, y: area.y, width: w, height: area.height };

            if seg.is_arrow {
                // Draw arrow separator
                let prev_bg = segments.get(i.saturating_sub(1)).map(|s| s.bg).unwrap_or(Color::Reset);
                let next_bg = segments.get(i + 1).map(|s| s.bg).unwrap_or(Color::Reset);
                let style = BobaStyle::new().fg(prev_bg).bg(next_bg);
                Paragraph::new(self.arrow_char.to_string()).style(style).render(rect, buf);
            } else {
                let style = BobaStyle::new().fg(seg.fg).bg(seg.bg);
                Paragraph::new(seg.text.clone()).style(style).render(rect, buf);
                // Fill remaining height
                for y in (area.y + 1)..area.bottom() {
                    for dx in 0..w {
                        buf[(x + dx, y)].set_bg(seg.bg);
                    }
                }
            }

            x += w;
        }

        // Fill rest of area
        for y in area.top()..area.bottom() {
            for fx in x..area.right() {
                buf[(fx, y)].set_bg(theme.global_bg);
            }
        }
    }
}

impl Component for Powerline {
    fn height(&self) -> Option<usize> { Some(1) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

// ──────────────────────────────────────────────────────────────
//  Command Palette
// ──────────────────────────────────────────────────────────────

/// A searchable command palette triggered by Ctrl+P.
pub struct CommandPalette {
    items: Mutable<Vec<String>>,
    filter: Mutable<String>,
    selection: Mutable<usize>,
    visible: Mutable<bool>,
}

impl Clone for CommandPalette {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            filter: self.filter.clone(),
            selection: self.selection.clone(),
            visible: self.visible.clone(),
        }
    }
}

impl CommandPalette {
    pub fn new(items: impl IntoIterator<Item = impl Display>) -> Self {
        Self {
            items: Mutable::new(items.into_iter().map(|s| s.to_string()).collect()),
            filter: Mutable::new(String::new()),
            selection: Mutable::new(0),
            visible: Mutable::new(false),
        }
    }

    pub fn show(&self) {
        self.visible.set(true);
        self.filter.set(String::new());
        self.selection.set(0);
    }

    pub fn hide(&self) { self.visible.set(false); }

    pub fn is_visible(&self) -> bool { self.visible.get() }

    pub fn on_key(&self, code: crossterm::event::KeyCode) -> Option<String> {
        if !self.visible.get() {
            return None;
        }
        match code {
            crossterm::event::KeyCode::Esc => {
                self.hide();
                None
            }
            crossterm::event::KeyCode::Enter => {
                let items = self.filtered();
                let sel = self.selection.get();
                self.hide();
                items.get(sel).cloned()
            }
            crossterm::event::KeyCode::Up => {
                let sel = self.selection.get().saturating_sub(1);
                self.selection.set(sel);
                None
            }
            crossterm::event::KeyCode::Down => {
                let len = self.filtered().len();
                let sel = (self.selection.get() + 1).min(len.saturating_sub(1));
                self.selection.set(sel);
                None
            }
            crossterm::event::KeyCode::Backspace => {
                let mut f = self.filter.get_cloned();
                f.pop();
                self.filter.set(f);
                self.selection.set(0);
                None
            }
            crossterm::event::KeyCode::Char(c) => {
                let mut f = self.filter.get_cloned();
                f.push(c);
                self.filter.set(f);
                self.selection.set(0);
                None
            }
            _ => None,
        }
    }

    fn filtered(&self) -> Vec<String> {
        let f = self.filter.get_cloned().to_lowercase();
        self.items.get_cloned().into_iter().filter(|s| s.to_lowercase().contains(&f)).collect()
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        if !self.visible.get() {
            return;
        }

        let h = 10u16;
        let w = 40u16;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let palette_area = Rect { x, y, width: w.min(area.width), height: h.min(area.height) };

        let dialog = &theme.dialog;

        // Dim background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_bg(dialog.dim_bg);
            }
        }

        // Border
        let block = crate::components::block::BobaBlock::new()
            .rounded()
            .border_style(dialog.border)
            .title(" Command Palette ")
            .padding(1, 1, 1, 1);
        let block: ratatui::widgets::Block<'_> = block.into();
        block.render(palette_area, buf);

        let inner = Rect {
            x: palette_area.x + 1,
            y: palette_area.y + 1,
            width: palette_area.width.saturating_sub(2),
            height: palette_area.height.saturating_sub(2),
        };

        // Filter input
        let filter_text = self.filter.get_cloned();
        Paragraph::new(format!("> {}", filter_text))
            .style(dialog.title)
            .render(Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 }, buf);

        // Items
        let items = self.filtered();
        let sel = self.selection.get();
        let start_y = inner.y + 2;
        for (i, item) in items.iter().take(inner.height.saturating_sub(3) as usize).enumerate() {
            let style = if i == sel { dialog.selected_item } else { dialog.normal_item };
            Paragraph::new(item.clone())
                .style(style)
                .render(Rect { x: inner.x, y: start_y + i as u16, width: inner.width, height: 1 }, buf);
        }
    }
}

impl Component for CommandPalette {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
