use {
    crate::{
        events::{EventTarget, SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{Focusable, InputEvent},
            views::{ModelEvent, model},
        },
    },
    crossterm::event::{KeyCode, KeyModifiers},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
    std::{
        ops::Deref,
        sync::LazyLock,
    },
};

static CURRENT_SCROLLER: LazyLock<Mutable<bool>> = LazyLock::new(|| Mutable::new(false));

static CURRENT_SCROLLER_BLUR: LazyLock<Mutable<Option<Box<dyn Fn() + Send + Sync>>>> =
    LazyLock::new(|| Mutable::new(None));

static CURRENT_SCROLLER_FOCUS: LazyLock<Mutable<Option<Box<dyn Fn() + Send + Sync>>>> =
    LazyLock::new(|| Mutable::new(None));

pub fn is_scroller_focused() -> bool {
    CURRENT_SCROLLER.get()
}

pub fn register_scroller_blur(callback: Box<dyn Fn() + Send + Sync>) {
    *CURRENT_SCROLLER_BLUR.lock_mut() = Some(callback);
}

pub fn register_scroller_focus(callback: Box<dyn Fn() + Send + Sync>) {
    *CURRENT_SCROLLER_FOCUS.lock_mut() = Some(callback);
}

pub fn blur_current_scroller() {
    CURRENT_SCROLLER.set(false);
    if let Some(cb) = CURRENT_SCROLLER_BLUR.lock_ref().as_ref() {
        cb();
    }
}

pub fn focus_current_scroller() {
    CURRENT_SCROLLER.set(true);
    if let Some(cb) = CURRENT_SCROLLER_FOCUS.lock_ref().as_ref() {
        cb();
    }
}

pub trait ScrollItem: WidgetRef + Focusable {
    fn height(&self) -> u16;
    fn width(&self) -> u16;
}

#[derive(Clone)]
pub struct ScrollText<'a>(Text<'a>, Mutable<bool>);
impl Focusable for ScrollText<'_> {
    fn focus(&self) {
        self.1.set(true);
    }

    fn blur(&self) {
        self.1.set(false);
    }
}

impl WidgetRef for ScrollText<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.0.clone())
            .style(match *self.1.lock_ref() {
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
        Self(d.into(), Mutable::new(false))
    }
}

pub struct Scroller {
    pub selected: Mutable<usize>,
    pub scroll: Mutable<usize>,
    pub items: Mutable<Vec<Box<dyn ScrollItem + Sync + Send + 'static>>>,
    pub focused: Mutable<bool>,
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
            selected: Mutable::new(0),
            scroll: Mutable::new(0),
            items: Mutable::new(Vec::new()),
            ev: EventTarget::new(),
            focused: Mutable::new(false),
            subs: None,
        };

        let sub = model().target.on(SubscriptionPriority::High, {
            let focused = this.focused.clone();
            let selected = this.selected.clone();
            let evt = this.ev.clone();
            let items = this.items.clone();

            move |ev| {
                let ModelEvent::KeyPress(key_event) = **ev;
                if !focused.get() {
                    return;
                }

                let prev = selected.get();
                let ctrl = key_event.modifiers.contains(KeyModifiers::CONTROL);
                let items = items.lock_ref();
                let len = items.len();

                match key_event.code {
                    KeyCode::Up if ctrl => {
                        selected.set(0);
                    }
                    KeyCode::Up => {
                        let cur = selected.get();
                        selected.set(cur.saturating_sub(1));
                    }
                    KeyCode::Home => {
                        selected.set(0);
                    }

                    KeyCode::Down if ctrl => {
                        if len > 0 {
                            selected.set(len - 1);
                        }
                    }
                    KeyCode::Down => {
                        let cur = selected.get();
                        if len > 0 {
                            selected.set((cur + 1).min(len - 1));
                        }
                    }
                    KeyCode::End => {
                        if len > 0 {
                            selected.set(len - 1);
                        }
                    }

                    KeyCode::Tab | KeyCode::Esc => {
                        evt.emit(InputEvent::Blur);
                        focused.set(false);
                    }

                    KeyCode::Enter => {
                        evt.emit(InputEvent::Submit(selected.get()));
                    }

                    _ => return,
                }

                ev.cancel();
                if let Some(it) = items.get(prev)
                    && focused.get()
                {
                    it.blur();
                }

                if let Some(it) = items.get(selected.get())
                    && focused.get()
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
        self.items.lock_mut().push(Box::new(i) as Box<dyn ScrollItem + Sync + Send + 'static>);
    }

    pub fn items_ref<T>(&self, i: impl IntoIterator<Item = T>)
    where
        T: ScrollItem + Sync + Send + 'static,
    {
        let i: Vec<Box<dyn ScrollItem + Send + Sync>> =
            i.into_iter().map(|i| Box::new(i) as Box<dyn ScrollItem + Sync + Send + 'static>).collect();
        self.items.lock_mut().extend(i);
    }

    fn clamp_scroll(&self, area_height: usize) {
        let items = self.items.lock_ref();
        let len = items.len();
        if len == 0 {
            return;
        }

        let selected = self.selected.get();
        let mut scroll = self.scroll.get();

        scroll = scroll.min(len.saturating_sub(1));

        let mut used_rows = 0;
        let mut start = scroll;

        if selected < scroll {
            start = selected;
        }

        if selected >= scroll {
            for i in scroll..=selected.min(len - 1) {
                let h = items[i].height().max(1);
                if used_rows + h as usize > area_height {
                    start = i;
                    break;
                }
                used_rows += h as usize;
            }
        }

        self.scroll.set(start);
    }
}

impl Focusable for Scroller {
    fn focus(&self) {
        self.focused.set(true);
        CURRENT_SCROLLER.set(true);
        self.ev.emit(InputEvent::Focus);
    }

    fn blur(&self) {
        self.focused.set(false);
        CURRENT_SCROLLER.set(false);
        self.ev.emit(InputEvent::Blur);
    }
}

impl WidgetRef for Scroller {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let items = self.items.lock_ref();
        let len = items.len();

        if len == 0 || area.height == 0 {
            return;
        }

        let height = area.height as usize;

        self.clamp_scroll(height);

        let scroll = self.scroll.get();

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