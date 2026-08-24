//! Styled button component with variants, callbacks, and event handling.
//!
//! ```rust
//! use bobatea::components::button::{Button, ButtonVariant};
//! let btn = Button::new("Submit").variant(ButtonVariant::Primary);
//! ```

use {
    crate::{
        components::{Component, style::BobaStyle},
        events::SubscriptionPriority,
        surface::Position,
        theme::Theme,
    },
    crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Alignment,
        prelude::{Buffer, Frame, Rect},
    },
    std::{fmt::Display, ops::Deref, sync::Arc},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Default,
    Hovered,
    Pressed,
    Focused,
    Disabled,
}

impl ButtonState {
    pub fn is_hovered(&self) -> bool { matches!(self, ButtonState::Hovered) }

    pub fn is_pressed(&self) -> bool { matches!(self, ButtonState::Pressed) }

    pub fn is_focused(&self) -> bool { matches!(self, ButtonState::Focused) }

    pub fn is_disabled(&self) -> bool { matches!(self, ButtonState::Disabled) }
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    Press,
    Release,
    Focus,
    Blur,
    HoverStart,
    HoverEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

impl Default for ButtonVariant {
    fn default() -> Self { ButtonVariant::Primary }
}

/// A button component with variants, callbacks, and event handling.
///
/// ```rust
/// use bobatea::components::button::{Button, ButtonVariant};
///
/// let btn = Button::new("Click Me")
///     .variant(ButtonVariant::Primary)
///     .on_click(|| println!("Clicked!"));
/// ```
pub struct Button {
    label: String,
    variant: ButtonVariant,
    state: Mutable<ButtonState>,
    focused: Mutable<bool>,
    default_style: BobaStyle,
    hovered_style: BobaStyle,
    pressed_style: BobaStyle,
    focused_style: BobaStyle,
    disabled_style: BobaStyle,
    on_press: Option<Arc<dyn Fn() + Send + Sync>>,
    ev: crate::events::EventTarget<ButtonEvent>,
    id: String,
}

impl Clone for Button {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            variant: self.variant,
            state: self.state.clone(),
            focused: self.focused.clone(),
            default_style: self.default_style.clone(),
            hovered_style: self.hovered_style.clone(),
            pressed_style: self.pressed_style.clone(),
            focused_style: self.focused_style.clone(),
            disabled_style: self.disabled_style.clone(),
            on_press: self.on_press.clone(),
            ev: self.ev.clone(),
            id: self.id.clone(),
        }
    }
}

impl Deref for Button {
    type Target = crate::events::EventTarget<ButtonEvent>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl Button {
    pub fn new(label: impl Display) -> Self {
        let base = BobaStyle::new().padding_y(0).padding_x(2);
        Self {
            label: label.to_string(),
            variant: ButtonVariant::Primary,
            state: Mutable::new(ButtonState::Default),
            focused: Mutable::new(false),
            default_style: base,
            hovered_style: base,
            pressed_style: base,
            focused_style: base,
            disabled_style: base,
            on_press: None,
            ev: crate::events::EventTarget::new("button"),
            id: format!("button-{}", label),
        }
    }

    pub fn id(&self) -> &str { &self.id }

    pub fn id_as(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.variant = v;
        self.apply_variant_styles(v);
        self
    }

    fn apply_variant_styles(&mut self, variant: ButtonVariant) {
        match variant {
            ButtonVariant::Primary => {
                self.default_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.hovered_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.pressed_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.focused_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.disabled_style = BobaStyle::new().padding_y(0).padding_x(2);
            }
            ButtonVariant::Secondary => {
                self.default_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.hovered_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.pressed_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.focused_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.disabled_style = BobaStyle::new().padding_y(0).padding_x(2);
            }
            ButtonVariant::Ghost => {
                self.default_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.hovered_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.pressed_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.focused_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.disabled_style = BobaStyle::new().padding_y(0).padding_x(2);
            }
            ButtonVariant::Danger => {
                self.default_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.hovered_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.pressed_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.focused_style = BobaStyle::new().padding_y(0).padding_x(2);
                self.disabled_style = BobaStyle::new().padding_y(0).padding_x(2);
            }
        }
    }

    pub fn on_click(self, f: impl Fn() + Send + Sync + 'static) -> Self {
        let f = Arc::new(f);
        let mut me = self;
        me.on_press = Some(f.clone());
        me.ev
            .on(SubscriptionPriority::Low, move |ev| {
                if let ButtonEvent::Press = **ev {
                    f();
                }
            })
            .forget();
        me
    }

    pub fn on_press(self, f: impl Fn() + Send + Sync + 'static) -> Self {
        let f = Arc::new(f);
        let mut me = self;
        me.on_press = Some(f.clone());
        me.ev
            .on(SubscriptionPriority::Low, move |ev| {
                if let ButtonEvent::Press = **ev {
                    f();
                }
            })
            .forget();
        me
    }

    pub fn set_style<F>(self, f: F) -> Self
    where
        F: FnOnce(BobaStyle) -> BobaStyle,
    {
        let styled = f(BobaStyle::new());
        self.with_styles(styled.clone(), styled.clone(), styled.clone(), styled.clone(), styled)
    }

    pub fn with_styles(
        self,
        default: BobaStyle,
        hovered: BobaStyle,
        pressed: BobaStyle,
        focused: BobaStyle,
        disabled: BobaStyle,
    ) -> Self {
        Self {
            default_style: default,
            hovered_style: hovered,
            pressed_style: pressed,
            focused_style: focused,
            disabled_style: disabled,
            ..self
        }
    }

    pub fn default_style(mut self, style: BobaStyle) -> Self {
        self.default_style = style;
        self
    }

    pub fn hovered_style(mut self, style: BobaStyle) -> Self {
        self.hovered_style = style;
        self
    }

    pub fn pressed_style(mut self, style: BobaStyle) -> Self {
        self.pressed_style = style;
        self
    }

    pub fn focused_style(mut self, style: BobaStyle) -> Self {
        self.focused_style = style;
        self
    }

    pub fn disabled_style(mut self, style: BobaStyle) -> Self {
        self.disabled_style = style;
        self
    }

    pub fn focus(&self) {
        let current = *self.state.lock_ref();
        if current != ButtonState::Disabled {
            self.focused.set(true);
            self.state.set(ButtonState::Focused);
            self.ev.emit(ButtonEvent::Focus);
        }
    }

    pub fn blur(&self) {
        self.focused.set(false);
        let current = *self.state.lock_ref();
        if current != ButtonState::Disabled {
            self.state.set(ButtonState::Default);
        }
        self.ev.emit(ButtonEvent::Blur);
    }

    pub fn press(&self) {
        if *self.state.lock_ref() == ButtonState::Disabled {
            return;
        }
        self.state.set(ButtonState::Pressed);
        self.ev.emit(ButtonEvent::Press);
    }

    pub fn release(&self) {
        if *self.state.lock_ref() == ButtonState::Disabled {
            return;
        }
        self.ev.emit(ButtonEvent::Release);
        let was_pressed = matches!(*self.state.lock_ref(), ButtonState::Pressed);
        if was_pressed {
            self.state.set(ButtonState::Hovered);
        } else if matches!(*self.state.lock_ref(), ButtonState::Hovered) {
            // already hovered — stay there
        } else {
            self.state.set(ButtonState::Default);
        }
    }

    pub fn disable(&self) {
        self.state.set(ButtonState::Disabled);
        self.focused.set(false);
    }

    pub fn enable(&self) {
        if matches!(*self.state.lock_ref(), ButtonState::Disabled) {
            self.state.set(ButtonState::Default);
        }
    }

    pub fn is_focused(&self) -> bool { self.focused.get() }

    pub fn state(&self) -> ButtonState { *self.state.lock_ref() }

    fn style_for_state(&self) -> BobaStyle {
        match *self.state.lock_ref() {
            ButtonState::Default => self.default_style,
            ButtonState::Hovered => self.hovered_style,
            ButtonState::Pressed => self.pressed_style,
            ButtonState::Focused => self.focused_style,
            ButtonState::Disabled => self.disabled_style,
        }
    }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() || *self.state.lock_ref() == ButtonState::Disabled {
            return;
        }
        match code {
            KeyCode::Enter | KeyCode::Char(' ') => self.press(),
            _ => {}
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        let st = *self.state.lock_ref();
        if st == ButtonState::Disabled {
            return;
        }

        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.state.set(ButtonState::Pressed);
                    self.focus();
                }
            }
            MouseEventKind::Up(_) => {
                if is_inside(area, ev) && *self.state.lock_ref() == ButtonState::Pressed {
                    self.ev.emit(ButtonEvent::Release);
                }
                if is_inside(area, ev) {
                    self.state.set(ButtonState::Hovered);
                } else {
                    self.state.set(ButtonState::Default);
                }
            }
            MouseEventKind::Moved => {
                if is_inside(area, ev) {
                    if *self.state.lock_ref() != ButtonState::Pressed {
                        let was_hovered = *self.state.lock_ref() == ButtonState::Hovered;
                        self.state.set(ButtonState::Hovered);
                        if !was_hovered {
                            self.ev.emit(ButtonEvent::HoverStart);
                        }
                    }
                } else {
                    let was_hovered = *self.state.lock_ref() == ButtonState::Hovered;
                    if *self.state.lock_ref() != ButtonState::Pressed {
                        self.state.set(ButtonState::Default);
                    }
                    if was_hovered {
                        self.ev.emit(ButtonEvent::HoverEnd);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_event(&mut self, area: Rect, ev: &Event) {
        match ev {
            Event::Key(KeyEvent { code, .. }) => self.on_key(*code),
            Event::Mouse(mouse) => self.on_mouse(area, mouse),
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, _theme: &Theme) {
        let style = self.style_for_state();
        let surf = style.render(&self.label);

        let surf_w = surf.columns() as u16;
        let surf_h = surf.height() as u16;
        let x = match style.align_h {
            Alignment::Right => area.width.saturating_sub(surf_w),
            Alignment::Center => (area.width.saturating_sub(surf_w)) / 2,
            _ => 0,
        };
        let y = match style.align_v {
            Position::Bottom => area.height.saturating_sub(surf_h),
            Position::Center => (area.height.saturating_sub(surf_h)) / 2,
            _ => 0,
        };

        surf.blit(buf, area.x + x, area.y + y);
    }

    pub fn render_to_surface(&self, _theme: &Theme) -> crate::surface::Surface {
        let style = self.style_for_state();
        style.render(&self.label)
    }
}

impl Component for Button {
    fn height(&self) -> Option<usize> { Some(1) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &Theme) { self.render_to_buf(ctx.area(), ctx.buffer_mut(), theme); }

    fn handle_event(&mut self, area: Rect, ev: &Event) { self.handle_event(area, ev); }

    fn wants_focus(&self) -> bool { true }

    fn id(&self) -> &str { &self.id }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
