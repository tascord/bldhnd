use {
    crate::{
        events::{EventTarget, SubscriptionPriority},
        ui::components::{Focusable, InputEvent, scroll::Scroller},
    },
    futures_signals::signal::Mutable,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Block, BorderType, Borders, Clear, WidgetRef},
    },
    std::{
        collections::VecDeque,
        fmt::Display,
        ops::Deref,
        sync::{Arc, LazyLock},
    },
    uuid::Uuid,
};

pub struct Modal {
    title: String,
    inner: Scroller,
    ev: EventTarget<InputEvent<()>>,
}

impl Modal {
    pub fn new(t: impl Display, i: Scroller) -> Self { Self { title: t.to_string(), inner: i, ev: EventTarget::new() } }
}

impl Deref for Modal {
    type Target = EventTarget<InputEvent<()>>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

impl WidgetRef for Modal {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let box_width = area.width.min(40);
        let box_height = area.height.min(10);
        let rect = Rect {
            x: area.x + (area.width.saturating_sub(box_width) / 2),
            y: area.y + (area.height.saturating_sub(box_height) / 2),
            width: box_width,
            height: box_height,
        };

        Clear.render(rect, buf);

        let layout = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(rect);

        Block::new().title(self.title.as_str()).borders(Borders::ALL).border_type(BorderType::Rounded).render(rect, buf);

        self.inner.render_ref(layout[1], buf);
    }
}

#[derive(Clone)]
pub struct ModalManager {
    queue: Mutable<VecDeque<(Modal, Uuid)>>,
    last: Mutable<Uuid>,
}

static MODAL: LazyLock<Arc<ModalManager>> = LazyLock::new(|| Arc::new(ModalManager::new()));

pub fn modal() -> Arc<ModalManager> { MODAL.clone() }

impl ModalManager {
    fn new() -> Self { Self { queue: Default::default(), last: Default::default() } }

    pub fn push(&self, m: Modal) {
        let inner = &m.inner;
        let manager = self.clone();
        inner.on(SubscriptionPriority::Low, move |ev| {
            if let InputEvent::Blur = (**ev).clone() {
                manager.pop();
            }
        });

        self.queue.lock_mut().push_back((m, Uuid::new_v4()));
    }

    pub fn pop(&self) { self.queue.lock_mut().pop_front(); }

    pub fn is_empty(&self) -> bool { self.queue.lock_ref().is_empty() }
}

impl WidgetRef for ModalManager {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if let Some((m, u)) = self.queue.lock_ref().front() {
            if self.last.get() != *u {
                self.last.set(*u);
                m.inner.focus();
            }

            m.render_ref(area, buf);
        }
    }
}
