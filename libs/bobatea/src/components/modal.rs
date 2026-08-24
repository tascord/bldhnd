//! Modal / Dialog component for overlays.
//!
//! ```rust
//! use bobatea::components::modal::{Modal, DialogButtons};
//! let modal = Modal::new("Confirm", "Are you sure?")
//!     .with_buttons(DialogButtons::YesNo);
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        layout::{Alignment, Constraint, Direction, Layout, Margin},
        prelude::{Buffer, Frame, Rect},
        widgets::{Clear, Paragraph, Widget},
    },
    std::fmt::Display,
};

#[derive(Clone)]
pub enum DialogButtons {
    Ok,
    YesNo,
    OkCancel,
    Custom(Vec<String>),
}

/// A centered modal dialog.
#[derive(Clone)]
pub struct Modal {
    title: String,
    body: String,
    buttons: DialogButtons,
    selected: Mutable<usize>,
    visible: Mutable<bool>,
    width: u16,
    height: u16,
}

impl Modal {
    pub fn new(title: impl Display, body: impl Display) -> Self {
        Self {
            title: title.to_string(),
            body: body.to_string(),
            buttons: DialogButtons::Ok,
            selected: Mutable::new(0),
            visible: Mutable::new(false),
            width: 50,
            height: 12,
        }
    }

    pub fn with_buttons(mut self, buttons: DialogButtons) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn size(mut self, w: u16, h: u16) -> Self {
        self.width = w;
        self.height = h;
        self
    }

    pub fn show(&self) {
        self.visible.set(true);
        self.selected.set(0);
    }

    pub fn hide(&self) { self.visible.set(false); }

    pub fn is_visible(&self) -> bool { self.visible.get() }

    pub fn on_key(&self, code: crossterm::event::KeyCode) -> Option<usize> {
        if !self.visible.get() {
            return None;
        }
        let count = self.button_count();
        let mut sel = self.selected.get();
        match code {
            crossterm::event::KeyCode::Left => {
                sel = sel.saturating_sub(1);
                self.selected.set(sel);
                None
            }
            crossterm::event::KeyCode::Right => {
                sel = (sel + 1).min(count.saturating_sub(1));
                self.selected.set(sel);
                None
            }
            crossterm::event::KeyCode::Enter => {
                self.hide();
                Some(sel)
            }
            crossterm::event::KeyCode::Esc => {
                self.hide();
                None
            }
            _ => None,
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) -> Option<usize> {
        if !self.visible.get() {
            return None;
        }
        match ev.kind {
            MouseEventKind::Down(_) => {
                let x = area.x + (area.width.saturating_sub(self.width)) / 2;
                let y = area.y + (area.height.saturating_sub(self.height)) / 2;
                let modal_area = Rect { x, y, width: self.width.min(area.width), height: self.height.min(area.height) };
                let inner = modal_area.inner(Margin { horizontal: 2, vertical: 1 });
                let btn_area = Rect { x: inner.x, y: inner.bottom().saturating_sub(3), width: inner.width, height: 3 };
                if is_inside(btn_area, ev) {
                    let labels = self.button_labels();
                    let btn_constraints: Vec<Constraint> = labels.iter().map(|_| Constraint::Length(10)).collect();
                    let btn_layout =
                        Layout::horizontal(btn_constraints).direction(Direction::Horizontal).spacing(2).split(btn_area);
                    for (i, btn_rect) in btn_layout.iter().enumerate() {
                        if is_inside(*btn_rect, ev) {
                            self.hide();
                            return Some(i);
                        }
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn button_count(&self) -> usize {
        match &self.buttons {
            DialogButtons::Ok => 1,
            DialogButtons::YesNo => 2,
            DialogButtons::OkCancel => 2,
            DialogButtons::Custom(v) => v.len(),
        }
    }

    fn button_labels(&self) -> Vec<String> {
        match &self.buttons {
            DialogButtons::Ok => vec!["OK".into()],
            DialogButtons::YesNo => vec!["Yes".into(), "No".into()],
            DialogButtons::OkCancel => vec!["OK".into(), "Cancel".into()],
            DialogButtons::Custom(v) => v.clone(),
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        if !self.visible.get() {
            return;
        }

        let dialog = &theme.dialog;

        let x = area.x + (area.width.saturating_sub(self.width)) / 2;
        let y = area.y + (area.height.saturating_sub(self.height)) / 2;
        let modal_area = Rect { x, y, width: self.width.min(area.width), height: self.height.min(area.height) };

        Clear.render(modal_area, buf);
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                cell.set_bg(dialog.dim_bg);
                cell.set_fg(theme.palette.fg_muted.to_rgb());
            }
        }

        let block: ratatui::widgets::Block<'_> =
            BobaBlock::new().rounded().border_style(dialog.border).title(self.title.clone()).padding(1, 2, 1, 2).into();
        block.render(modal_area, buf);

        let inner = modal_area.inner(Margin { horizontal: 2, vertical: 1 });
        let body_area = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height.saturating_sub(4) };
        Paragraph::new(self.body.clone()).style(dialog.view).render(body_area, buf);

        let labels = self.button_labels();
        let sel = self.selected.get();
        let btn_area = Rect { x: inner.x, y: inner.bottom().saturating_sub(3), width: inner.width, height: 3 };

        let btn_constraints: Vec<Constraint> = labels.iter().map(|_| Constraint::Length(10)).collect();
        let btn_layout = Layout::horizontal(btn_constraints).direction(Direction::Horizontal).spacing(2).split(btn_area);

        for (i, (btn_rect, label)) in btn_layout.iter().zip(labels.iter()).enumerate() {
            let style = if i == sel { dialog.selected_item } else { dialog.normal_item };
            let btn_block: ratatui::widgets::Block<'_> = BobaBlock::new().rounded().border_style(style).into();
            Paragraph::new(label.clone()).alignment(Alignment::Center).style(style).block(btn_block).render(*btn_rect, buf);
        }
    }
}

impl Component for Modal {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
