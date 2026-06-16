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
    ratatui::{
        text::Line,
        widgets::{Paragraph, Widget, WidgetRef},
    },
    std::{
        fmt::Display,
        ops::Deref,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
        },
    },
};

pub struct Radio {
    options: Vec<String>,
    selection: Arc<AtomicUsize>,
    focused: Arc<AtomicBool>,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    ev: EventTarget<InputEvent<usize>>,
}

impl Deref for Radio {
    type Target = EventTarget<InputEvent<usize>>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl Radio {
    pub fn new<D: Display>(options: impl IntoIterator<Item = D>) -> Self {
        let mut this = Self {
            options: options.into_iter().map(|v| v.to_string()).collect(),
            selection: AtomicUsize::new(0).into(),
            focused: AtomicBool::new(false).into(),
            subs: None,
            ev: EventTarget::new(),
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let focused = this.focused.clone();
            let len = this.options.len();
            let selected = this.selection.clone();
            let evt = this.ev.clone();

            move |ev| {
                if let ModelEvent::KeyPress(key_event) = **ev
                    && focused.load(SeqCst)
                {
                    let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);

                    match key_event.code {
                        KeyCode::Up if ctrl => selected.store(0, SeqCst),
                        KeyCode::Up => selected.store(selected.load(SeqCst).saturating_sub(1), SeqCst),
                        KeyCode::Home => selected.store(0, SeqCst),

                        KeyCode::Down if ctrl => selected.store(len - 1, SeqCst),
                        KeyCode::Down => selected.store(selected.load(SeqCst).saturating_add(1).min(len - 1), SeqCst),
                        KeyCode::End => selected.store(len - 1, SeqCst),

                        KeyCode::Tab | KeyCode::Esc => {
                            evt.emit(InputEvent::Blur);
                            focused.store(false, SeqCst);
                            ev.cancel();
                            return;
                        }


                        _ => return,
                    }

                    evt.emit(InputEvent::Submit(selected.load(SeqCst)));
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
        self.focused.store(true, SeqCst);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.store(false, SeqCst);
        self.ev.emit(InputEvent::Blur);
    }
}


impl WidgetRef for Radio {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let s = self.selection.load(SeqCst);
        let lines = self
            .options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                Line::styled(format!(
                    "{} {}",
                    match s == i as usize {
                        true => '●',
                        false => '○',
                    },
                    o
                ), match self.focused.load(SeqCst) {
                    true => Style::new().white(),
                    false => Style::new().gray(),
                })
            })
            .collect::<Vec<_>>();

        Paragraph::new(lines).render(area, buf);
    }
}
