//! Data table component with sortable columns and keyboard navigation.
//!
//! ```rust
//! use bobatea::components::table::{Table, Column};
//! let cols = vec![
//!     Column::new("Name").width(20),
//!     Column::new("Size").width(10).right(),
//! ];
//! let table = Table::new(cols)
//!     .row(["boba.rs", "4.2 KB"])
//!     .row(["Cargo.toml", "2.1 KB"]);
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        layout::{Alignment, Constraint},
        prelude::{Buffer, Frame, Rect},
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

/// Column definition.
pub struct Column {
    label: String,
    width: Constraint,
    align: Alignment,
    sortable: bool,
}

impl Column {
    pub fn new(label: impl Display) -> Self {
        Self { label: label.to_string(), width: Constraint::Length(10), align: Alignment::Left, sortable: false }
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = Constraint::Length(w);
        self
    }

    pub fn flex(mut self) -> Self {
        self.width = Constraint::Fill(1);
        self
    }

    pub fn min_width(mut self, w: u16) -> Self {
        self.width = Constraint::Min(w);
        self
    }

    pub fn right(mut self) -> Self {
        self.align = Alignment::Right;
        self
    }

    pub fn center(mut self) -> Self {
        self.align = Alignment::Center;
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

/// A single row of data.
pub type Row = Vec<String>;

/// Data table widget.
pub struct Table {
    columns: Vec<Column>,
    rows: Mutable<Vec<Row>>,
    selection: Mutable<usize>,
    scroll: Mutable<usize>,
    focused: Mutable<bool>,
}

impl Table {
    pub fn new(columns: impl Into<Vec<Column>>) -> Self {
        Self {
            columns: columns.into(),
            rows: Mutable::new(Vec::new()),
            selection: Mutable::new(0),
            scroll: Mutable::new(0),
            focused: Mutable::new(false),
        }
    }

    pub fn row(self, cells: impl IntoIterator<Item = impl Display>) -> Self {
        let r: Vec<String> = cells.into_iter().map(|c| c.to_string()).collect();
        self.rows.lock_mut().push(r);
        self
    }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let row_count = self.rows.lock_ref().len();
        if row_count == 0 {
            return;
        }
        let mut sel = self.selection.get();
        let mut scroll = self.scroll.get();
        match code {
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Down => sel = (sel + 1).min(row_count - 1),
            KeyCode::Home => sel = 0,
            KeyCode::End => sel = row_count - 1,
            KeyCode::PageUp => sel = sel.saturating_sub(10),
            KeyCode::PageDown => sel = (sel + 10).min(row_count - 1),
            _ => {}
        }
        self.selection.set(sel);
        // keep selection in view
        let visible = 10;
        if sel < scroll {
            scroll = sel;
        } else if sel >= scroll + visible {
            scroll = sel.saturating_sub(visible - 1);
        }
        self.scroll.set(scroll);
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let header_height = 1u16;
                    let offset = 1; // border
                    let inner_y = ev.row.saturating_sub(area.top() + header_height + offset);
                    let visible_idx = inner_y as usize;
                    let scroll = self.scroll.get();
                    let sel = scroll + visible_idx;
                    let row_count = self.rows.lock_ref().len();
                    if sel < row_count {
                        self.selection.set(sel);
                    }
                } else {
                    self.blur();
                }
            }
            MouseEventKind::ScrollUp => {
                let scroll = self.scroll.get().saturating_sub(1);
                self.scroll.set(scroll);
            }
            MouseEventKind::ScrollDown => {
                let row_count = self.rows.lock_ref().len();
                let visible = 10;
                let scroll = (self.scroll.get() + 1).min(row_count.saturating_sub(visible));
                self.scroll.set(scroll);
            }
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let rows = self.rows.lock_ref();
        let sel = self.selection.get();
        let scroll = self.scroll.get();
        let focused = self.focused.get();

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let header_height = 1u16;
        let inner_height = area.height.saturating_sub(header_height + 2);
        let inner = Rect { x: area.x + 1, y: area.y + 1, width: area.width.saturating_sub(2), height: inner_height };

        let accent = theme.palette.accent.to_rgb();
        let pair = &theme.list.pair;

        // ── header ──
        let mut x = inner.x;
        for col in &self.columns {
            let w = resolve_width(col.width, inner.width, x, inner.right());
            if w == 0 {
                continue;
            }
            let style = ratatui::style::Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD);
            Paragraph::new(col.label.clone())
                .alignment(col.align)
                .style(style)
                .render(Rect { x, y: inner.y - 1, width: w, height: 1 }, buf);
            x += w + 1; // +1 gutter
        }

        // ── rows ──
        let mut y = inner.y;
        for (i, row) in rows.iter().enumerate().skip(scroll) {
            if y >= inner.bottom() {
                break;
            }
            let is_sel = i == sel;
            let style = if is_sel && focused { pair.focused } else { pair.blurred };
            let mut x = inner.x;
            for (col, cell) in self.columns.iter().zip(row.iter()) {
                let w = resolve_width(col.width, inner.width, x, inner.right());
                if w == 0 {
                    continue;
                }
                Paragraph::new(cell.clone())
                    .alignment(col.align)
                    .style(style)
                    .render(Rect { x, y, width: w, height: 1 }, buf);
                x += w + 1;
            }
            y += 1;
        }

        // ── border ──
        let border_style = if focused {
            ratatui::style::Style::default().fg(accent)
        } else {
            ratatui::style::Style::default().fg(theme.border_subtle)
        };
        let block: ratatui::widgets::Block<'_> = BobaBlock::new().rounded().border_style(border_style).into();
        block.render(area, buf);
    }
}

impl Component for Table {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn resolve_width(c: Constraint, total: u16, used: u16, right: u16) -> u16 {
    match c {
        Constraint::Length(v) => v.min(total.saturating_sub(used)),
        Constraint::Fill(_) => right.saturating_sub(used).max(1),
        Constraint::Min(v) => v.min(total.saturating_sub(used)),
        _ => 10,
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
