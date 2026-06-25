use ratatui::style::Style;

use crate::ui::components::Focusable;

use {
    crate::{
        events::{EventTarget, SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::InputEvent,
            views::{ModelEvent, model},
        },
    },
    crossterm::event::{KeyCode, KeyModifiers},
    futures_signals::signal::Mutable,
    ratatui::{
        text::Line,
        widgets::{Paragraph, Widget, WidgetRef},
    },
    std::{
        fmt::Display,
        ops::Deref,
    },
};

pub struct Radio {
    options: Vec<String>,
    selection: Mutable<usize>,
    focused: Mutable<bool>,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    ev: EventTarget<InputEvent<usize>>,
}

impl Deref for Radio {
    type Target = EventTarget<InputEvent<usize>>;

    fn deref(&self) -> &Self::Target {
        &self.ev
    }
}

impl Radio {
    pub fn new<D: Display>(options: impl IntoIterator<Item = D>) -> Self {
        let mut this = Self {
            options: options.into_iter().map(|v| v.to_string()).collect(),
            selection: Mutable::new(0),
            focused: Mutable::new(false),
            subs: None,
            ev: EventTarget::new(),
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let focused = this.focused.clone();
            let len = this.options.len();
            let selected = this.selection.clone();
            let evt = this.ev.clone();

            move |ev| {
                let ModelEvent::KeyPress(key_event) = **ev;
                if focused.get() {
                    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);

                    match key_event.code {
                        KeyCode::Up if ctrl => selected.set(0),
                        KeyCode::Up => selected.set(selected.get().saturating_sub(1)),
                        KeyCode::Home => selected.set(0),

                        KeyCode::Down if ctrl => selected.set(len - 1),
                        KeyCode::Down => selected.set((selected.get() + 1).min(len - 1)),
                        KeyCode::End => selected.set(len - 1),

                        KeyCode::Tab | KeyCode::Esc => {
                            evt.emit(InputEvent::Blur);
                            focused.set(false);
                            ev.cancel();
                            return;
                        }

                        _ => return,
                    }

                    evt.emit(InputEvent::Submit(selected.get()));
                    ev.cancel();
                }
            }
        });

        this.subs = Some([sub]);
        this
    }
}

impl Focusable for Radio {
    fn focus(&self) {
        self.focused.set(true);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.set(false);
        self.ev.emit(InputEvent::Blur);
    }
}

impl WidgetRef for Radio {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let s = self.selection.get();
        let lines = self
            .options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                Line::styled(
                    format!(
                        "{} {}",
                        match s == i {
                            true => '●',
                            false => '○',
                        },
                        o
                    ),
                    match self.focused.get() {
                        true => Style::new().white(),
                        false => Style::new().gray(),
                    },
                )
            })
            .collect::<Vec<_>>();

        Paragraph::new(lines).render(area, buf);
    }
}