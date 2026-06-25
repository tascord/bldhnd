use {
    crate::{
        events::{EventTarget, SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent, scroll::ScrollItem},
            views::{ModelEvent, model},
        },
    },
    crossterm::event::{KeyCode, KeyModifiers},
    futures_signals::signal::Mutable,
    ratatui::{
        style::{Color::White, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Widget, WidgetRef},
    },
    std::{
        fmt::Display,
        ops::Deref,
        time::Instant,
    },
};

#[derive(Clone, Copy, Debug)]
struct Cursor {
    caret: usize,
    anchor: usize,
}

impl Cursor {
    fn point(pos: usize) -> Self {
        Self { caret: pos, anchor: pos }
    }

    fn has_sel(self) -> bool {
        self.caret != self.anchor
    }

    fn sel_lo(self) -> usize {
        self.caret.min(self.anchor)
    }

    fn sel_hi(self) -> usize {
        self.caret.max(self.anchor)
    }

    fn collapse(pos: usize) -> Self {
        Self::point(pos)
    }

    fn extend(self, pos: usize) -> Self {
        Self { caret: pos, anchor: self.anchor }
    }
}

const SPINNER_FRAMES: [&str; 4] = ["◜", "◝", "◞", "◟"];
const SPINNER_INTERVAL_MS: u128 = 30;

pub struct Input {
    focused: Mutable<bool>,
    text: Mutable<String>,
    cursor: Mutable<Cursor>,
    loading: Mutable<bool>,
    label: String,
    ts: Instant,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    ev: EventTarget<InputEvent<String>>,
}

impl ScrollItem for Input {
    fn height(&self) -> u16 {
        3
    }

    fn width(&self) -> u16 {
        0
    }
}

impl Deref for Input {
    type Target = EventTarget<InputEvent<String>>;

    fn deref(&self) -> &Self::Target {
        &self.ev
    }
}

impl Input {
    pub fn new(label: impl Display, default_value: impl Display) -> Self {
        let mut this = Self {
            focused: Mutable::new(false),
            text: Mutable::new(default_value.to_string()),
            cursor: Mutable::new(Cursor::point(0)),
            label: label.to_string(),
            ts: Instant::now(),
            subs: None,
            ev: EventTarget::new(),
            loading: Mutable::new(false),
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let text = this.text.clone();
            let cursor = this.cursor.clone();
            let focused = this.focused.clone();
            let evt = this.ev.clone();
            let loading = this.loading.clone();

            move |ev| {
                let ModelEvent::KeyPress(key_event) = **ev;
                if focused.get() && !loading.get() {
                    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
                    let shft = key_event.modifiers.contains(KeyModifiers::SHIFT);

                    let mut txt: String = text.get_cloned();
                    let mut cur: Cursor = cursor.get();

                    let prev_word = |pos: usize| -> usize {
                        let chars: Vec<char> = txt.chars().collect();
                        let mut i = pos;
                        while i > 0 && chars[i - 1].is_whitespace() {
                            i -= 1;
                        }
                        while i > 0 && !chars[i - 1].is_whitespace() {
                            i -= 1;
                        }
                        i
                    };

                    let next_word = |pos: usize| -> usize {
                        let chars: Vec<char> = txt.chars().collect();
                        let mut i = pos;
                        while i < chars.len() && !chars[i].is_whitespace() {
                            i += 1;
                        }
                        while i < chars.len() && chars[i].is_whitespace() {
                            i += 1;
                        }
                        i
                    };

                    let len = txt.chars().count();

                    match key_event.code {
                        KeyCode::Backspace if ctrl => {
                            let target = prev_word(cur.sel_lo());
                            txt.drain(target..cur.sel_hi());
                            cur = Cursor::collapse(target);
                        }
                        KeyCode::Backspace => {
                            if cur.has_sel() {
                                txt.drain(cur.sel_lo()..cur.sel_hi());
                                cur = Cursor::collapse(cur.sel_lo());
                            } else if cur.caret > 0 {
                                let new_pos = txt[..cur.caret].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                                txt.remove(new_pos);
                                cur = Cursor::collapse(new_pos);
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') if ctrl => {
                            cur = Cursor { caret: len, anchor: 0 };
                        }
                        KeyCode::Left if ctrl && shft => {
                            cur = cur.extend(prev_word(cur.caret));
                        }
                        KeyCode::Left if ctrl => {
                            cur = Cursor::collapse(prev_word(cur.caret));
                        }
                        KeyCode::Left if shft => {
                            cur = cur.extend(cur.caret.saturating_sub(1));
                        }
                        KeyCode::Left => {
                            let target = if cur.has_sel() { cur.sel_lo() } else { cur.caret.saturating_sub(1) };
                            cur = Cursor::collapse(target);
                        }
                        KeyCode::Home if shft => {
                            cur = cur.extend(0);
                        }
                        KeyCode::Home => {
                            cur = Cursor::collapse(0);
                        }
                        KeyCode::Right if ctrl && shft => {
                            cur = cur.extend(next_word(cur.caret));
                        }
                        KeyCode::Right if ctrl => {
                            cur = Cursor::collapse(next_word(cur.caret));
                        }
                        KeyCode::Right if shft => {
                            cur = cur.extend((cur.caret + 1).min(len));
                        }
                        KeyCode::Right => {
                            let target = if cur.has_sel() { cur.sel_hi() } else { (cur.caret + 1).min(len) };
                            cur = Cursor::collapse(target);
                        }
                        KeyCode::End if shft => {
                            cur = cur.extend(len);
                        }
                        KeyCode::End => {
                            cur = Cursor::collapse(len);
                        }
                        KeyCode::Char(c) => {
                            if cur.has_sel() {
                                txt.drain(cur.sel_lo()..cur.sel_hi());
                                txt.insert(cur.sel_lo(), c);
                                cur = Cursor::collapse(cur.sel_lo() + c.len_utf8());
                            } else {
                                txt.insert(cur.caret, c);
                                cur = Cursor::collapse(cur.caret + c.len_utf8());
                            }
                        }

                        KeyCode::Tab | KeyCode::Esc => {
                            evt.emit(InputEvent::Blur);
                            focused.set(false);
                        }

                        KeyCode::Enter => {
                            evt.emit(InputEvent::Submit(txt.to_string()));
                        }

                        _ => {
                            text.set(txt);
                            cursor.set(cur);
                            return;
                        }
                    }

                    text.set(txt);
                    cursor.set(cur);
                    ev.cancel();
                }
            }
        });

        this.subs = Some([sub]);
        this
    }

    fn scroll_window(total: usize, avail: usize, sel_lo: usize, sel_hi: usize, caret: usize) -> (usize, usize) {
        if avail == 0 || total <= avail {
            return (0, total);
        }

        let sel_span = sel_hi - sel_lo;
        let center = if sel_span <= avail { (sel_lo + sel_hi) / 2 } else { caret };

        let start = center.saturating_sub(avail / 2).min(total - avail);
        (start, start + avail)
    }

    fn spinner_frame(&self) -> &'static str {
        let idx = (self.ts.elapsed().as_millis() / SPINNER_INTERVAL_MS) as usize % SPINNER_FRAMES.len();
        SPINNER_FRAMES[idx]
    }

    pub fn load(&self, v: bool) {
        self.loading.set(v);
    }

    pub fn set_text(&self, text: &str) {
        self.text.set(text.to_string());
        self.cursor.set(Cursor::collapse(text.len()));
    }

    pub fn get_text(&self) -> String {
        self.text.get_cloned()
    }
}

impl Focusable for Input {
    fn focus(&self) {
        self.focused.set(true);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.set(false);
        self.ev.emit(InputEvent::Blur);
    }
}

impl WidgetRef for Input {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let text = self.text.get_cloned();
        let cur = self.cursor.get();

        let sel_lo = cur.sel_lo();
        let sel_hi = cur.sel_hi();
        let is_loading = self.loading.get();

        let border_avail = area.width.saturating_sub(2) as usize;
        let avail = border_avail.saturating_sub(if is_loading { 2 } else { 0 });
        let total = text.chars().count();
        let (win_lo, win_hi) = Self::scroll_window(total, avail, sel_lo, sel_hi, cur.caret);

        let byte_at = |idx: usize| text.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(text.len());

        let (b_win_lo, b_win_hi) = (byte_at(win_lo), byte_at(win_hi));
        let (b_sel_lo, b_sel_hi) = (byte_at(sel_lo.clamp(win_lo, win_hi)), byte_at(sel_hi.clamp(win_lo, win_hi)));
        let rel_caret = byte_at(cur.caret.clamp(win_lo, win_hi)) - b_win_lo;

        let visible = &text[b_win_lo..b_win_hi];
        let rel_sel_lo = b_sel_lo - b_win_lo;
        let rel_sel_hi = b_sel_hi - b_win_lo;

        let blink = match self.ts.elapsed().as_secs().is_multiple_of(2) && self.focused.get() {
            true => Style::new().on_white(),
            false => Style::new(),
        };

        let before = Span::styled(visible.get(0..rel_sel_lo).unwrap_or_default(), Style::new());
        let selected = Span::styled(visible.get(rel_sel_lo..rel_sel_hi).unwrap_or_default(), Style::new().bg(White));
        let after = Span::styled(visible.get(rel_sel_hi..).unwrap_or_default(), Style::new());
        let caret = Span::styled(" ", blink);

        let mut spans = if rel_caret == rel_sel_lo {
            vec![before, caret, selected, after]
        } else {
            vec![before, selected, caret, after]
        };

        if is_loading {
            let visible_len = win_hi - win_lo;
            let pad = avail.saturating_sub(visible_len + 2);
            if pad > 0 {
                spans.push(Span::raw(" ".repeat(pad)));
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled(self.spinner_frame(), Style::new().white()));
        }

        let line = Line::from_iter(spans);

        Paragraph::new(line)
            .block(
                Block::new()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .border_style(match self.focused.get() {
                        true => Style::new().white(),
                        false => Style::new().gray(),
                    })
                    .title(self.label.as_str()),
            )
            .render(area, buf);
    }
}