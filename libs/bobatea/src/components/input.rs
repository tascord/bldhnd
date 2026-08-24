use {
    crate::{
        components::{Component, block::BobaBlock},
        events::EventTarget,
    },
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::{Line, Span},
        widgets::{Paragraph, Widget},
    },
    std::{fmt::Display, ops::Deref, time::Instant},
};

#[derive(Debug, Clone)]
pub enum InputEvent {
    Submit(String),
    Focus,
    Blur,
    Change(String),
}

/// A single-line text input component.
///
/// ```rust
/// use bobatea::components::input::Input;
/// let inp = Input::new("Username").placeholder("guest");
/// ```
pub struct Input {
    label: String,
    text: Mutable<String>,
    placeholder: String,
    focused: Mutable<bool>,
    cursor: Mutable<usize>,
    ev: EventTarget<InputEvent>,
    ts: Instant,
}

impl Clone for Input {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            text: self.text.clone(),
            placeholder: self.placeholder.clone(),
            focused: self.focused.clone(),
            cursor: self.cursor.clone(),
            ev: self.ev.clone(),
            ts: self.ts,
        }
    }
}

impl Deref for Input {
    type Target = EventTarget<InputEvent>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl Input {
    pub fn new(label: impl Display) -> Self {
        Self {
            label: label.to_string(),
            text: Mutable::new(String::new()),
            placeholder: String::new(),
            focused: Mutable::new(false),
            cursor: Mutable::new(0),
            ev: EventTarget::new("component"),
            ts: Instant::now(),
        }
    }

    pub fn set_label(&mut self, label: impl Display) { self.label = label.to_string(); }

    pub fn placeholder(mut self, p: impl Display) -> Self {
        self.placeholder = p.to_string();
        self
    }

    pub fn value(&self) -> String { self.text.get_cloned() }

    pub fn set_value(&self, s: impl Display) {
        let s = s.to_string();
        self.cursor.set(s.chars().count());
        self.text.set(s);
    }

    pub fn focus(&self) {
        self.focused.set(true);
        self.ev.emit(InputEvent::Focus);
    }

    pub fn blur(&self) {
        self.focused.set(false);
        self.ev.emit(InputEvent::Blur);
    }

    pub fn is_focused(&self) -> bool { self.focused.get() }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let mut txt = self.text.get_cloned();
        let mut cur = self.cursor.get();
        let len = txt.chars().count();

        match code {
            KeyCode::Backspace => {
                if cur > 0 {
                    let byte_pos = txt.char_indices().nth(cur.saturating_sub(1)).map(|(i, _)| i).unwrap_or(0);
                    txt.remove(byte_pos);
                    cur = cur.saturating_sub(1);
                }
            }
            KeyCode::Delete => {
                if cur < len {
                    let byte_pos = txt.char_indices().nth(cur).map(|(i, _)| i).unwrap_or(txt.len());
                    if byte_pos < txt.len() {
                        let ch_len = txt[byte_pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                        txt.drain(byte_pos..byte_pos + ch_len);
                    }
                }
            }
            KeyCode::Left => {
                cur = cur.saturating_sub(1);
            }
            KeyCode::Right => {
                cur = (cur + 1).min(len);
            }
            KeyCode::Home => {
                cur = 0;
            }
            KeyCode::End => {
                cur = len;
            }
            KeyCode::Char(c) => {
                let byte_pos = txt.char_indices().nth(cur).map(|(i, _)| i).unwrap_or(txt.len());
                txt.insert(byte_pos, c);
                cur += 1;
            }
            KeyCode::Enter => {
                self.ev.emit(InputEvent::Submit(txt.clone()));
                return;
            }
            _ => return,
        }

        self.text.set(txt.clone());
        self.cursor.set(cur);
        self.ev.emit(InputEvent::Change(txt));
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let offset = 1; // border
                    let rel_x = ev.column.saturating_sub(area.left() + offset);
                    let avail = area.width.saturating_sub(2) as usize;
                    let total = self.text.get_cloned().chars().count();
                    let cur = self.cursor.get();
                    let (win_lo, _win_hi) = scroll_window(total, avail, cur);
                    // place cursor at clicked position inside visible window
                    let new_cur = (win_lo + rel_x as usize).min(total);
                    self.cursor.set(new_cur);
                } else {
                    self.blur();
                }
            }
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let text = self.text.get_cloned();
        let cur = self.cursor.get();
        let focused = self.focused.get();
        let is_empty = text.is_empty();

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let avail = area.width.saturating_sub(2) as usize;
        let total = text.chars().count();
        let (win_lo, win_hi) = scroll_window(total, avail, cur);

        let byte_at = |idx: usize| text.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(text.len());
        let (b_win_lo, b_win_hi) = (byte_at(win_lo), byte_at(win_hi));
        let rel_caret = byte_at(cur.clamp(win_lo, win_hi)) - b_win_lo;

        let visible = &text[b_win_lo..b_win_hi];

        let style = theme.input.pair.pick(focused);
        let caret_bg = if focused { theme.input.cursor_bg } else { theme.global_bg };

        let blink = if focused && self.ts.elapsed().as_secs() % 2 == 0 {
            ratatui::style::Style::default().bg(caret_bg)
        } else {
            ratatui::style::Style::default()
        };

        let before = Span::styled(visible.get(..rel_caret).unwrap_or_default(), style);
        let after = Span::styled(visible.get(rel_caret..).unwrap_or_default(), style);
        let caret = Span::styled(" ", blink);

        let spans = if rel_caret == 0 && visible.is_empty() && !focused && is_empty {
            vec![Span::styled(&self.placeholder, ratatui::style::Style::default().fg(theme.input.placeholder_fg))]
        } else {
            vec![before, caret, after]
        };

        let block = BobaBlock::new().rounded().border_style(style).title(self.label.clone());

        Paragraph::new(Line::from(spans)).style(style).block(block.into()).render(area, buf);
    }
}

impl Component for Input {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn scroll_window(total: usize, avail: usize, caret: usize) -> (usize, usize) {
    if avail == 0 || total <= avail {
        return (0, total);
    }
    let start = caret.saturating_sub(avail / 2).min(total - avail);
    (start, start + avail)
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
