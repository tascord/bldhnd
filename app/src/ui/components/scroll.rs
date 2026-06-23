use {
    crate::{
        events::{EventTarget, SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent},
            views::{ModelEvent, model},
        },
    },
    crossterm::event::{KeyCode, KeyModifiers},
    ratatui::{
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
    std::{
        ops::Deref,
        sync::{
            Arc, RwLock,
            atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
        },
    },
};

pub trait ScrollItem: WidgetRef + Focusable {
    fn height(&self) -> u16;
    fn width(&self) -> u16;
}

#[derive(Clone)]
pub struct ScrollText<'a>(Text<'a>, Arc<AtomicBool>);
impl Focusable for ScrollText<'_> {
    fn focus(&self) {
        self.1.store(true, SeqCst);
    }

    fn blur(&self) {
        self.1.store(false, SeqCst);
    }
}

impl WidgetRef for ScrollText<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.0.clone())
            .style(match self.1.load(SeqCst) {
                true => Style::new().on_white().black(),
                false => Style::new(),
            })
            .render(area, buf);
    }
}

impl ScrollItem for ScrollText<'_> {
    fn height(&self) -> u16 {
        self.0.height() as u16
    }

    fn width(&self) -> u16 {
        self.0.width() as u16
    }
}

impl<'a> ScrollText<'a> {
    pub fn new(d: impl Into<Text<'a>>) -> Self {
        Self(d.into(), AtomicBool::new(false).into())
    }
}

pub struct Scroller {
    pub selected: Arc<AtomicUsize>,
    pub scroll: Arc<AtomicUsize>,
    pub items: Arc<RwLock<Vec<Box<dyn ScrollItem + Sync + Send + 'static>>>>,
    pub focused: Arc<AtomicBool>,
    subs: Option<[SubscriptionHandle<ModelEvent>; 1]>,
    ev: EventTarget<InputEvent<usize>>,
}

impl Deref for Scroller {
    type Target = EventTarget<InputEvent<usize>>;

    fn deref(&self) -> &Self::Target {
        &self.ev
    }
}

#[allow(clippy::new_without_default)]
impl Scroller {
    pub fn new() -> Self {
        let mut this = Self {
            selected: AtomicUsize::new(0).into(),
            scroll: AtomicUsize::new(0).into(),
            items: Default::default(),
            ev: EventTarget::new(),
            focused: AtomicBool::new(false).into(),
            subs: None,
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let focused = this.focused.clone();
            let selected = this.selected.clone();
            let evt = this.ev.clone();
            let items = this.items.clone();

            move |ev| {
                let ModelEvent::KeyPress(key_event) = **ev;
                if !focused.load(SeqCst) {
                    return;
                }

                let prev = selected.load(SeqCst);
                let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
                let items = items.read().unwrap();
                let len = items.len();

                match key_event.code {
                    KeyCode::Up if ctrl => {
                        selected.store(0, SeqCst);
                    }
                    KeyCode::Up => {
                        let cur = selected.load(SeqCst);
                        selected.store(cur.saturating_sub(1), SeqCst);
                    }
                    KeyCode::Home => {
                        selected.store(0, SeqCst);
                    }

                    KeyCode::Down if ctrl => {
                        if len > 0 {
                            selected.store(len - 1, SeqCst);
                        }
                    }
                    KeyCode::Down => {
                        let cur = selected.load(SeqCst);
                        if len > 0 {
                            selected.store((cur + 1).min(len - 1), SeqCst);
                        }
                    }
                    KeyCode::End => {
                        if len > 0 {
                            selected.store(len - 1, SeqCst);
                        }
                    }

                    KeyCode::Tab | KeyCode::Esc => {
                        evt.emit(InputEvent::Blur);
                        focused.store(false, SeqCst);
                    }

                    KeyCode::Enter => {
                        evt.emit(InputEvent::Submit(selected.load(SeqCst)));
                    }

                    _ => return,
                }

                ev.cancel();
                if let Some(it) = items.get(prev)
                    && focused.load(SeqCst)
                {
                    it.blur();
                }

                if let Some(it) = items.get(selected.load(SeqCst))
                    && focused.load(SeqCst)
                {
                    it.focus();
                }
            }
        });

        this.subs = Some([sub]);
        this
    }

    pub fn item(self, i: impl ScrollItem + Sync + Send + 'static) -> Self {
        self.item_ref(i);
        self
    }

    pub fn items<T>(self, i: impl IntoIterator<Item = T>) -> Self
    where
        T: ScrollItem + Sync + Send + 'static,
    {
        self.items_ref(i);
        self
    }

    pub fn item_ref(&self, i: impl ScrollItem + Sync + Send + 'static) {
        self.items.write().unwrap().push(Box::new(i) as Box<dyn ScrollItem + Sync + Send + 'static>);
    }

    pub fn items_ref<T>(&self, i: impl IntoIterator<Item = T>)
    where
        T: ScrollItem + Sync + Send + 'static,
    {
        let i: Vec<Box<dyn ScrollItem + Send + Sync>> =
            i.into_iter().map(|i| Box::new(i) as Box<dyn ScrollItem + Sync + Send + 'static>).collect();
        self.items.write().unwrap().extend(i);
    }

    fn clamp_scroll(&self, area_height: usize) {
        let len = self.items.read().unwrap().len();
        if len == 0 {
            return;
        }

        let selected = self.selected.load(SeqCst);
        let mut scroll = self.scroll.load(SeqCst);

        // basic bounds
        scroll = scroll.min(len.saturating_sub(1));

        // ensure selected is visible (row-aware approximation)
        let mut used_rows = 0;
        let mut start = scroll;

        // move scroll up if needed
        if selected < scroll {
            start = selected;
        }

        // move scroll down if needed
        if selected >= scroll {
            for i in scroll..=selected.min(len - 1) {
                let h = self.items.read().unwrap()[i].height().max(1);
                if used_rows + h as usize > area_height {
                    start = i;
                    break;
                }
                used_rows += h as usize;
            }
        }

        self.scroll.store(start, SeqCst);
    }
}

impl Focusable for Scroller {
    fn focus(&self) {
        self.focused.store(true, SeqCst);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.store(false, SeqCst);
        self.ev.emit(InputEvent::Blur);
    }
}

impl WidgetRef for Scroller {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let items = self.items.read().unwrap();
        let len = items.len();

        if len == 0 || area.height == 0 {
            return;
        }

        let height = area.height as usize;

        self.clamp_scroll(height);

        let scroll = self.scroll.load(SeqCst);

        let mut y = area.y;
        let mut used_rows = 0;

        for i in scroll..len {
            let item = &items[i];
            let item_h = (item.height() as usize).max(1);

            if used_rows + item_h > height {
                break;
            }

            item.render_ref(Rect { x: area.x, y, width: area.width, height: item_h as u16 }, buf);

            y += item_h as u16;
            used_rows += item_h;
        }
    }
}
