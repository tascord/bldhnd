//! Multiline text area — like `<textarea>` but for the terminal.
//!
//! ```rust
//! use bobatea::components::textarea::TextArea;
//! let ta = TextArea::new().placeholder("Write something...");
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::{Line, Span},
        widgets::{Paragraph, Widget},
    },
    std::time::Instant,
};

#[derive(Debug, Clone)]
pub enum TextAreaEvent {
    Submit(String),
    Change(String),
    Focus,
    Blur,
}

pub struct TextArea {
    lines: Mutable<Vec<String>>,
    cursor: (Mutable<usize>, Mutable<usize>), // (row, col)
    focused: Mutable<bool>,
    placeholder: String,
    max_lines: usize,
    ts: Instant,
}

impl TextArea {
    pub fn new() -> Self {
        Self {
            lines: Mutable::new(vec![String::new()]),
            cursor: (Mutable::new(0), Mutable::new(0)),
            focused: Mutable::new(false),
            placeholder: String::new(),
            max_lines: 100,
            ts: Instant::now(),
        }
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn max_lines(mut self, n: usize) -> Self {
        self.max_lines = n;
        self
    }

    pub fn value(&self) -> String { self.lines.get_cloned().join("\n") }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let (row, col) = (self.cursor.0.get(), self.cursor.1.get());
        let mut lines = self.lines.lock_mut();

        match code {
            KeyCode::Char(c) => {
                if lines[row].chars().count() < self.max_lines {
                    let byte_pos = lines[row].char_indices().nth(col).map(|(i, _)| i).unwrap_or(lines[row].len());
                    lines[row].insert(byte_pos, c);
                    self.cursor.1.set(col + 1);
                }
            }
            KeyCode::Backspace => {
                if col > 0 {
                    let byte_pos = lines[row].char_indices().nth(col.saturating_sub(1)).map(|(i, _)| i).unwrap_or(0);
                    lines[row].remove(byte_pos);
                    self.cursor.1.set(col.saturating_sub(1));
                } else if row > 0 {
                    let prev_len = lines[row - 1].chars().count();
                    let removed = lines.remove(row);
                    lines[row - 1].push_str(&removed);
                    self.cursor.0.set(row - 1);
                    self.cursor.1.set(prev_len);
                }
            }
            KeyCode::Enter => {
                if row + 1 < self.max_lines {
                    let byte_pos = lines[row].char_indices().nth(col).map(|(i, _)| i).unwrap_or(lines[row].len());
                    let rest: String = lines[row].split_off(byte_pos);
                    lines.insert(row + 1, rest);
                    self.cursor.0.set(row + 1);
                    self.cursor.1.set(0);
                }
            }
            KeyCode::Up => {
                if row > 0 {
                    let prev_len = lines[row - 1].chars().count();
                    self.cursor.0.set(row - 1);
                    self.cursor.1.set(col.min(prev_len));
                }
            }
            KeyCode::Down => {
                if row + 1 < lines.len() {
                    let next_len = lines[row + 1].chars().count();
                    self.cursor.0.set(row + 1);
                    self.cursor.1.set(col.min(next_len));
                }
            }
            KeyCode::Left => {
                if col > 0 {
                    self.cursor.1.set(col - 1);
                } else if row > 0 {
                    self.cursor.0.set(row - 1);
                    self.cursor.1.set(lines[row - 1].chars().count());
                }
            }
            KeyCode::Right => {
                let len = lines[row].chars().count();
                if col < len {
                    self.cursor.1.set(col + 1);
                } else if row + 1 < lines.len() {
                    self.cursor.0.set(row + 1);
                    self.cursor.1.set(0);
                }
            }
            KeyCode::Home => self.cursor.1.set(0),
            KeyCode::End => self.cursor.1.set(lines[row].chars().count()),
            _ => {}
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let offset = 1; // border
                    let inner_x = ev.column.saturating_sub(area.left() + offset);
                    let inner_y = ev.row.saturating_sub(area.top() + offset);
                    let lines = self.lines.lock_ref();
                    let row = (inner_y as usize).min(lines.len().saturating_sub(1));
                    let col = (inner_x as usize).min(lines[row].chars().count());
                    self.cursor.0.set(row);
                    self.cursor.1.set(col);
                } else {
                    self.blur();
                }
            }
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let cursor_row = self.cursor.0.get();
        let cursor_col = self.cursor.1.get();
        let focused = self.focused.get();
        let lines = self.lines.lock_ref();
        let visible_lines = area.height.saturating_sub(2) as usize;

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let style = theme.input.pair.pick(focused);
        let block = BobaBlock::new().rounded().border_style(style);
        let block: ratatui::widgets::Block<'_> = block.into();
        block.render(area, buf);

        let inner = Rect { x: area.x + 1, y: area.y + 1, width: area.width - 2, height: area.height - 2 };

        for (i, line_text) in lines.iter().take(visible_lines).enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.bottom() {
                break;
            }

            let mut spans = vec![];
            let caret_bg = if focused { theme.input.cursor_bg } else { theme.global_bg };
            if i == cursor_row && focused && (self.ts.elapsed().as_millis() / 500) % 2 == 0 {
                let byte_at = |idx: usize| line_text.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(line_text.len());
                let (b_lo, b_hi) = (byte_at(0), byte_at(cursor_col.min(line_text.chars().count())));
                spans.push(Span::raw(&line_text[..b_lo]));
                spans.push(Span::styled(" ", ratatui::style::Style::default().bg(caret_bg)));
                spans.push(Span::raw(&line_text[b_hi..]));
            } else {
                spans.push(Span::raw(line_text.as_str()));
            }

            Paragraph::new(Line::from(spans))
                .style(style)
                .render(Rect { x: inner.x, y, width: inner.width, height: 1 }, buf);
        }

        // Show placeholder if empty and not focused
        if lines.len() == 1 && lines[0].is_empty() && !focused {
            Paragraph::new(self.placeholder.as_str())
                .style(ratatui::style::Style::default().fg(theme.input.placeholder_fg))
                .render(Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 }, buf);
        }
    }
}

impl Component for TextArea {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
