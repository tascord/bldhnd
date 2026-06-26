use {
    crate::{
        components::{Component, block::BobaBlock},
        events::EventTarget,
    },
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

#[derive(Debug, Clone, Copy)]
pub enum ListEvent {
    Select(usize),
    Focus,
    Blur,
}

/// A selectable list of items.
///
/// ```rust
/// use boba::components::list::List;
/// let list = List::new(["Apple", "Banana", "Cherry"]);
/// ```
pub struct List {
    items: Mutable<Vec<String>>,
    selection: Mutable<usize>,
    focused: Mutable<bool>,
    ev: EventTarget<ListEvent>,
}

impl Clone for List {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            selection: self.selection.clone(),
            focused: self.focused.clone(),
            ev: self.ev.clone(),
        }
    }
}

impl List {
    pub fn new(items: impl IntoIterator<Item = impl Display>) -> Self {
        let v: Vec<String> = items.into_iter().map(|x| x.to_string()).collect();
        Self {
            items: Mutable::new(v),
            selection: Mutable::new(0),
            focused: Mutable::new(false),
            ev: EventTarget::new("component"),
        }
    }

    pub fn focus(&self) {
        self.focused.set(true);
        self.ev.emit(ListEvent::Focus);
    }

    pub fn blur(&self) {
        self.focused.set(false);
        self.ev.emit(ListEvent::Blur);
    }

    pub fn selected(&self) -> usize { self.selection.get() }

    pub fn is_focused(&self) -> bool { self.focused.get() }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let len = self.items.lock_ref().len();
        if len == 0 {
            return;
        }
        let mut sel = self.selection.get();
        match code {
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Down => sel = (sel + 1).min(len - 1),
            KeyCode::Home => sel = 0,
            KeyCode::End => sel = len - 1,
            KeyCode::Enter => {
                self.ev.emit(ListEvent::Select(sel));
                return;
            }
            _ => return,
        }
        self.selection.set(sel);
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let inner_y = ev.row.saturating_sub(area.top() + 1);
                    let sel = (inner_y as usize).min(self.items.lock_ref().len().saturating_sub(1));
                    self.selection.set(sel);
                    self.ev.emit(ListEvent::Select(sel));
                }
            }
            MouseEventKind::ScrollUp => {
                self.focus();
                self.selection.set(self.selection.get().saturating_sub(1));
            }
            MouseEventKind::ScrollDown => {
                let len = self.items.lock_ref().len();
                self.focus();
                self.selection.set((self.selection.get() + 1).min(len.saturating_sub(1)));
            }
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let sel = self.selection.get();
        let focused = self.focused.get();
        let items = self.items.lock_ref();
        let pair = &theme.list.pair;
        let selected_glyph = &theme.list.selected_glyph;
        let unselected_glyph = &theme.list.unselected_glyph;

        let lines: Vec<Line> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let glyph = if i == sel { selected_glyph } else { unselected_glyph };
                let item_style = if i == sel { pair.focused } else { pair.blurred };
                Line::styled(format!("{} {}", glyph, item), item_style)
            })
            .collect();

        let border_style = pair.pick(focused);
        let block = BobaBlock::new().rounded().border_style(border_style);

        Paragraph::new(lines).style(border_style).block(block.into()).render(area, buf);
    }
}

impl Component for List {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
