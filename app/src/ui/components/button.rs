use {
    crate::{
        events::{EventTarget, SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent, scroll::ScrollItem},
            views::{ModelEvent, model},
        },
    },
    crossterm::event::KeyCode,
    ratatui::{
        layout::Alignment,
        style::Style,
        widgets::{Block, BorderType, Borders, Paragraph, Widget, WidgetRef},
    },
    std::{
        fmt::Display,
        ops::Deref,
        sync::{
            Arc, atomic::{AtomicBool, Ordering::SeqCst},
        },
    },
};

pub struct Button {
    focused: Arc<AtomicBool>,
    label: String,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    ev: EventTarget<InputEvent<()>>,
}

impl ScrollItem for Button {
    fn height(&self) -> u16 { 3 }

    fn width(&self) -> u16 { 0 }
}

impl Deref for Button {
    type Target = EventTarget<InputEvent<()>>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl Button {
    pub fn new(label: impl Display) -> Self {
        let mut this = Self {
            focused: Arc::new(AtomicBool::new(false)),
            label: label.to_string(),
            ev: EventTarget::new(),
            subs: Option::None,
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let focused = this.focused.clone();
            let evt = this.ev.clone();

            move |ev| {
                if let ModelEvent::KeyPress(key_event) = **ev
                    && focused.load(SeqCst)
                {
                    match key_event.code {
                        KeyCode::Tab | KeyCode::Esc => {
                            evt.emit(InputEvent::Blur);
                            focused.store(false, SeqCst);
                        }

                        KeyCode::Enter => {
                            evt.emit(InputEvent::Submit(()));
                        }

                        _ => {
                            return;
                        }
                    }

                    ev.cancel();
                }
            }
        });

        this.subs = Some([sub]);
        this
    }
 
}

impl Focusable for Button {
    fn focus(&self) {
        self.focused.store(true, SeqCst);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.store(false, SeqCst);
        self.ev.emit(InputEvent::Blur);
    }
}

impl WidgetRef for Button {
    fn render_ref(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        Paragraph::new(self.label.clone())
            .alignment(Alignment::Center)
            .block(
                Block::new()
                    .border_type(BorderType::Rounded)
                    .borders(Borders::ALL)
                    .border_style(match self.focused.load(SeqCst) {
                        true => Style::new().white(),
                        false => Style::new().gray(),
                    })
            )
            .render(area, buf);
    }
}
