//! Pagination controls.
//!
//! ```rust
//! use bobatea::components::paginator::Paginator;
//! let pager = Paginator::new(100, 10); // 100 items, 10 per page
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Alignment,
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
};

/// Pagination controller and display.
pub struct Paginator {
    total: usize,
    per_page: usize,
    page: Mutable<usize>,
    focused: Mutable<bool>,
}

impl Paginator {
    pub fn new(total: usize, per_page: usize) -> Self {
        Self { total, per_page: per_page.max(1), page: Mutable::new(0), focused: Mutable::new(false) }
    }

    pub fn set_total(&mut self, total: usize) {
        self.total = total;
        let max_page = self.max_page();
        if self.page.get() > max_page {
            self.page.set(max_page);
        }
    }

    pub fn page(&self) -> usize { self.page.get() }

    pub fn per_page(&self) -> usize { self.per_page }

    pub fn offset(&self) -> usize { self.page.get() * self.per_page }

    pub fn len_on_page(&self) -> usize {
        let start = self.offset();
        (self.per_page).min(self.total.saturating_sub(start))
    }

    fn max_page(&self) -> usize { if self.total == 0 { 0 } else { (self.total - 1) / self.per_page } }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let max = self.max_page();
        let mut p = self.page.get();
        match code {
            KeyCode::Left => p = p.saturating_sub(1),
            KeyCode::Right => p = (p + 1).min(max),
            KeyCode::Home => p = 0,
            KeyCode::End => p = max,
            _ => {}
        }
        self.page.set(p);
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let max = self.max_page();
                    // Bullets are centered with " " spacing. Estimate width per bullet.
                    let bullet_str =
                        (0..=max).map(|i| if i == self.page.get() { "●" } else { "○" }).collect::<Vec<_>>().join(" ");
                    let total_w = bullet_str.chars().count() as u16;
                    let start_x = area.x + (area.width.saturating_sub(total_w)) / 2;
                    let rel_x = ev.column.saturating_sub(start_x);
                    // Each bullet is 1 char + 1 space, except last
                    let mut x = 0u16;
                    for i in 0..=max {
                        let w = if i == max { 1 } else { 2 };
                        if rel_x >= x && rel_x < x + w {
                            self.page.set(i);
                            return;
                        }
                        x += w;
                    }
                } else {
                    self.blur();
                }
            }
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let max = self.max_page();
        let p = self.page.get();
        let focused = self.focused.get();

        let bullets: String = (0..=max).map(|i| if i == p { "●" } else { "○" }).collect::<Vec<_>>().join(" ");

        let fg = theme.global_fg;
        let accent = theme.palette.accent.to_rgb();
        let style = if focused {
            ratatui::style::Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            ratatui::style::Style::default().fg(fg)
        };
        let block = BobaBlock::new().horizontal().border_style(ratatui::style::Style::default().fg(theme.border_subtle));

        Paragraph::new(Line::styled(bullets, style)).alignment(Alignment::Center).block(block.into()).render(area, buf);
    }
}

impl Component for Paginator {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
