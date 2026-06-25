use std::sync::Arc;

use futures_signals::signal::Mutable;

use {
    crate::events::{EventTarget, SubscriptionPriority},
    async_trait::async_trait,
    crossterm::event::KeyEvent,
    ratatui::{buffer::Buffer, layout::Rect},
    std::{fmt::Display, ops::Deref, time::Instant},
    tokio::spawn,
};

pub mod components;
pub mod events;

pub struct Ctx(Buffer, Rect);
impl Ctx {
    pub fn new(buf: Buffer, area: Rect) -> Self { Self(buf, area) }

    pub fn buf(&mut self) -> &mut Buffer { &mut self.0 }

    pub fn area(&self) -> Rect { self.1 }
}

#[async_trait]
pub trait View {
    async fn title(&self) -> impl Display
    where
        Self: Sized,
    {
        env!("CARGO_CRATE_NAME")
    }
    async fn render(&mut self, ctx: &mut Ctx);
}

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Quit,
    RequestAnimationFrame,
    KeyEvent(KeyEvent),
}

pub struct App {
    last_draw: Mutable<Instant>,
    inner: Arc<dyn View>,
    ev: EventTarget<AppEvent>,
}

impl App {
    pub fn new(v: impl View + 'static) -> Self {
        Self { last_draw: Instant::now().into(), inner: Arc::new(v), ev: EventTarget::new() }
    }

    pub async fn run(self) {
        let ev = self.ev.clone();
        while let Some(ev) = ev.as_stream(SubscriptionPriority::Low).recv().await {
            match *ev {
                AppEvent::RequestAnimationFrame => {
                    self.last_draw.set(Instant::now());
                    self.inner.render(todo!()).await;
                }
                AppEvent::Quit => return,
                _ => {}
            }
        }
    }
}

impl Deref for App {
    type Target = EventTarget<AppEvent>;

    fn deref(&self) -> &Self::Target { &self.ev }
}
