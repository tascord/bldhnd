use {
    crate::{
        components::{Component, block::BobaBlock},
        events::EventTarget,
        theme::Theme,
    },
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Alignment,
        prelude::{Buffer, Frame, Rect},
        widgets::{Paragraph, Widget},
    },
    std::{fmt::Display, ops::Deref},
};

#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    Press,
    Focus,
    Blur,
}

/// A pretty button component.
///
/// ```rust
/// use boba::components::button::Button;
/// let btn = Button::new("Submit");
/// ```
pub struct Button {
    label: String,
    focused: Mutable<bool>,
    ev: EventTarget<ButtonEvent>,
}

impl Clone for Button {
    fn clone(&self) -> Self { Self { label: self.label.clone(), focused: self.focused.clone(), ev: self.ev.clone() } }
}

impl Deref for Button {
    type Target = EventTarget<ButtonEvent>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl Button {
    pub fn new(label: impl Display) -> Self {
        Self { label: label.to_string(), focused: Mutable::new(false), ev: EventTarget::new("component") }
    }

    pub fn focus(&self) {
        self.focused.set(true);
        self.ev.emit(ButtonEvent::Focus);
    }

    pub fn blur(&self) {
        self.focused.set(false);
        self.ev.emit(ButtonEvent::Blur);
    }

    pub fn press(&self) { self.ev.emit(ButtonEvent::Press); }

    pub fn is_focused(&self) -> bool { self.focused.get() }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        match code {
            KeyCode::Enter | KeyCode::Char(' ') => self.press(),
            _ => {}
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        if let MouseEventKind::Down(_) = ev.kind {
            if is_inside(area, ev) {
                self.focus();
                self.press();
            }
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let style = theme.button.pair.pick(self.focused.get());

        // Clear background
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let block = BobaBlock::new().rounded().border_style(style).into();
        Paragraph::new(self.label.clone()).alignment(Alignment::Center).style(style).block(block).render(area, buf);
    }
}

impl Component for Button {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &Theme) { self.render_to_buf(ctx.area(), ctx.buffer_mut(), theme); }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
