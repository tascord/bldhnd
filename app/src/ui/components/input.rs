use {
    crate::{
        events::{Subscription, SubscriptionHandle, SubscriptionPriority},
        ui::views::{ModelEvent, model},
    },
    crossterm::{
        cursor,
        event::{KeyCode, KeyModifiers},
    },
    ratatui::{
        layout::Margin,
        style::{Color::White, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Widget, WidgetRef},
    },
    std::{
        fmt::Display,
        pin::Pin,
        sync::{
            Arc, RwLock,
            atomic::{AtomicBool, Ordering::SeqCst},
        },
        time::Instant,
    },
};

#[derive(Clone, Copy, Debug)]
struct Cursor {
    caret: usize,
    anchor: usize,
}

impl Cursor {
    fn point(pos: usize) -> Self { Self { caret: pos, anchor: pos } }

    fn has_sel(self) -> bool { self.caret != self.anchor }

    fn sel_lo(self) -> usize { self.caret.min(self.anchor) }

    fn sel_hi(self) -> usize { self.caret.max(self.anchor) }

    /// Collapse selection: caret and anchor both land on `pos`.
    fn collapse(pos: usize) -> Self { Self::point(pos) }

    /// Extend selection: caret moves to `pos`, anchor stays.
    fn extend(self, pos: usize) -> Self { Self { caret: pos, anchor: self.anchor } }
}

pub struct Input {
    focused: Arc<AtomicBool>,
    text: Arc<RwLock<String>>,
    cursor: Arc<RwLock<Cursor>>,
    label: String,
    ts: Instant,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    on_blur: Arc<RwLock<Box<dyn Fn() + Sync + Send + 'static>>>,
}

impl Input {
    pub fn new(label: impl Display, default_value: impl Display) -> Self {
        let mut this = Self {
            focused: Arc::new(AtomicBool::new(false)),
            text: Arc::new(default_value.to_string().into()),
            cursor: Arc::new(Cursor::point(0).into()),
            label: label.to_string(),
            ts: Instant::now(),
            subs: None,
            on_blur: Arc::new(RwLock::new(Box::new(|| {}))),
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let text = this.text.clone();
            let cursor = this.cursor.clone();
            let focused = this.focused.clone();
            let on_blur = this.on_blur.clone();

            move |ev| {
                if let ModelEvent::KeyPress(key_event) = **ev
                    && focused.load(SeqCst)
                {
                    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
                    let shft = key_event.modifiers.contains(KeyModifiers::SHIFT);

                    // Clone out immediately and drop the read guards before any writes.
                    let mut txt: String = text.read().unwrap().clone();
                    let mut cur: Cursor = *cursor.read().unwrap();
                    // Both read guards are now dropped — temporaries are gone.

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

                    let len = txt.chars().count(); // ← char count, not byte count

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
                        KeyCode::Tab => {
                            (on_blur.read().unwrap())();
                            focused.store(false, SeqCst);
                        }

                        // Unhandled, skip before we cancel event
                        _ => {
                            *text.write().unwrap() = txt;
                            *cursor.write().unwrap() = cur;
                            return;
                        }
                    }

                    *text.write().unwrap() = txt;
                    *cursor.write().unwrap() = cur;
                    ev.cancel();
                }

                // Write back — no read locks are held at this point.
            }
        });

        this.subs = Some([sub]);
        this
    }

    pub fn focus(&self) { self.focused.store(true, SeqCst); }

    pub fn blur(self, f: impl Fn() + Sync + Send + 'static) -> Self {
        *self.on_blur.write().unwrap() = Box::new(f);
        self
    }
}

impl WidgetRef for Input {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let text = self.text.read().map(|v| v.to_string()).unwrap_or_default();
        let cur = self.cursor.read().map(|v| *v).unwrap_or(Cursor::point(0));

        let sel_lo = cur.sel_lo();
        let sel_hi = cur.sel_hi();

        let blink = match self.ts.elapsed().as_secs().is_multiple_of(2) && self.focused.load(SeqCst) {
            true => Style::new().on_white(),
            false => Style::new(),
        };

        let before = Span::styled(text.get(0..sel_lo).unwrap_or_default(), Style::new());
        let selected = Span::styled(text.get(sel_lo..sel_hi).unwrap_or_default(), Style::new().bg(White));
        let after = Span::styled(text.get(sel_hi..).unwrap_or_default(), Style::new());
        let caret = Span::styled(" ", blink);

        let line = if cur.caret == sel_lo {
            Line::from_iter([before, caret, selected, after])
        } else {
            Line::from_iter([before, selected, caret, after])
        };

        Paragraph::new(line)
            .block(Block::new().border_type(BorderType::Rounded).borders(Borders::ALL).title(self.label.as_str()))
            .render(area, buf);
    }
}
