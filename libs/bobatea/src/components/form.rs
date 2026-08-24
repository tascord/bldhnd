//! Form container — groups inputs, buttons, and lists with automatic focus/tab management.
//!
//! ```rust
//! use bobatea::components::{
//!     form::Form,
//!     input::Input,
//!     button::Button,
//! };
//!
//! let form = Form::new()
//!     .field("Username", Input::new("Username"))
//!     .field("Password", Input::new("Password"))
//!     .action(Button::new("Submit"));
//! ```

use {
    crate::components::{Component, button::Button, input::Input},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
    std::fmt::Display,
};

/// Describes a form field.
pub enum FormField {
    Input(Input, String),
    Button(Button),
    Static(String),
}

impl FormField {
    pub fn height(&self) -> u16 {
        match self {
            FormField::Input(..) => 3,
            FormField::Button(_) => 3,
            FormField::Static(_) => 1,
        }
    }

    pub fn on_key(&self, code: KeyCode) {
        match self {
            FormField::Input(i, _) => i.on_key(code),
            FormField::Button(b) => b.on_key(code),
            FormField::Static(_) => {}
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match self {
            FormField::Input(i, _) => i.on_mouse(area, ev),
            FormField::Button(b) => b.on_mouse(area, ev),
            FormField::Static(_) => {}
        }
    }

    pub fn focus(&self) {
        match self {
            FormField::Input(i, _) => i.focus(),
            FormField::Button(b) => b.focus(),
            FormField::Static(_) => {}
        }
    }

    pub fn blur(&self) {
        match self {
            FormField::Input(i, _) => i.blur(),
            FormField::Button(b) => b.blur(),
            FormField::Static(_) => {}
        }
    }
}

/// A vertical form that manages focus between fields via Tab / Shift-Tab.
pub struct Form {
    fields: Vec<FormField>,
    focus_index: Mutable<usize>,
    focused: Mutable<bool>,
    spacing: u16,
}

impl Form {
    pub fn new() -> Self {
        Self { fields: Vec::new(), focus_index: Mutable::new(0), focused: Mutable::new(false), spacing: 1 }
    }

    pub fn spacing(mut self, n: u16) -> Self {
        self.spacing = n;
        self
    }

    pub fn field(mut self, label: impl Display, mut input: Input) -> Self {
        input.set_label(label.to_string());
        self.fields.push(FormField::Input(input, label.to_string()));
        self
    }

    pub fn action(mut self, button: Button) -> Self {
        self.fields.push(FormField::Button(button));
        self
    }

    pub fn text(mut self, text: impl Display) -> Self {
        self.fields.push(FormField::Static(text.to_string()));
        self
    }

    fn focus_current(&self) {
        if let Some(field) = self.fields.get(self.focus_index.get()) {
            field.focus();
        }
    }

    fn blur_current(&self) {
        if let Some(field) = self.fields.get(self.focus_index.get()) {
            field.blur();
        }
    }

    pub fn focus(&self) {
        self.focused.set(true);
        self.focus_current();
    }

    pub fn blur(&self) {
        self.focused.set(false);
        for f in &self.fields {
            f.blur();
        }
    }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        match code {
            KeyCode::Tab => {
                self.blur_current();
                let next = (self.focus_index.get() + 1) % self.fields.len().max(1);
                self.focus_index.set(next);
                self.focus_current();
            }
            KeyCode::BackTab => {
                self.blur_current();
                let prev = if self.focus_index.get() == 0 {
                    self.fields.len().saturating_sub(1)
                } else {
                    self.focus_index.get() - 1
                };
                self.focus_index.set(prev);
                self.focus_current();
            }
            code => {
                if let Some(field) = self.fields.get(self.focus_index.get()) {
                    field.on_key(code);
                }
            }
        }
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        let mut y = area.y;
        for (i, field) in self.fields.iter().enumerate() {
            let h = field.height();
            let rect = Rect { x: area.x, y, width: area.width, height: h };
            if rect.left() <= ev.column && ev.column < rect.right() && rect.top() <= ev.row && ev.row < rect.bottom() {
                if matches!(ev.kind, MouseEventKind::Down(_)) {
                    self.blur_current();
                    self.focus_index.set(i);
                    self.focus_current();
                }
                field.on_mouse(rect, ev);
            }
            y += h + self.spacing;
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let mut y = area.y;
        let focus_idx = self.focus_index.get();

        for (i, field) in self.fields.iter().enumerate() {
            let is_focused = i == focus_idx && self.focused.get();
            let h = field.height();
            let rect = Rect { x: area.x, y, width: area.width, height: h };
            match field {
                FormField::Input(input, label) => {
                    // Update input label before rendering if it changed
                    let _ = label;
                    input.render_to_buf(rect, buf, theme)
                }
                FormField::Button(button) => button.render_to_buf(rect, buf, theme),
                FormField::Static(text) => {
                    let style = if is_focused {
                        theme.input.pair.focused
                    } else {
                        ratatui::style::Style::default().fg(theme.palette.fg_muted.to_rgb())
                    };
                    Paragraph::new(Line::raw(text.clone())).style(style).render(rect, buf);
                }
            }
            y += h + self.spacing;
        }
    }
}

impl Component for Form {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
