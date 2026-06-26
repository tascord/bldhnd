use {
    crate::{
        animator::Animator,
        events::{Cancellable, EventTarget, SubscriptionHandle, SubscriptionPriority},
        theme::Theme,
    },
    crossterm::{
        ExecutableCommand,
        event::{Event as CrosstermEvent, EventStream as CrosstermEventStream, KeyEvent, MouseEvent},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    futures::StreamExt,
    futures_signals::signal::Mutable,
    ratatui::{DefaultTerminal, Frame, backend::CrosstermBackend},
    std::{io::stdout, ops::Deref, sync::Arc},
};

pub mod animator;
pub mod components;
pub mod events;
pub mod surface;
pub mod theme;

// Re-export the most commonly used layout types for ergonomic access.
pub use surface::{Cell, Position, Surface, join_horizontal, join_vertical, place, place_with_whitespace};

/// Root trait for a full-screen view.
pub trait View {
    fn title(&self) -> &'static str { env!("CARGO_CRATE_NAME") }

    /// Called once before the event loop starts so the view can subscribe
    /// to `AppEvent` via `app.on(…)`.
    fn mount(&self, _app: &EventTarget<AppEvent>) {}

    fn render(&self, ctx: &mut Frame<'_>, theme: &Theme);
}

#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    Quit,
    RequestAnimationFrame,
    SetTheme(usize),
    KeyEvent(KeyEvent),
    MouseEvent(MouseEvent),
}

/// RAII guard that restores terminal state on drop.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = stdout().execute(crossterm::event::DisableMouseCapture);
        let _ = stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub struct App {
    inner: Arc<dyn View>,
    ev: EventTarget<AppEvent>,
    pub theme: Mutable<Arc<Theme>>,
    pub animator: std::sync::Mutex<Animator>,
}

impl App {
    pub fn new(v: impl View + 'static) -> Self {
        Self {
            inner: Arc::new(v),
            ev: EventTarget::new("app"),
            theme: Mutable::new(Arc::new(Theme::default())),
            animator: std::sync::Mutex::new(Animator::new()),
        }
    }

    pub fn set_theme(&self, theme: Theme) { self.theme.set(Arc::new(theme)); }

    pub async fn run(self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(crossterm::event::EnableMouseCapture)?;
        let _guard = TerminalGuard;

        let mut dt = DefaultTerminal::new(CrosstermBackend::new(stdout()))?;

        self.inner.mount(&self.ev);

        // Initial draw
        let theme = self.theme.get_cloned();
        dt.draw(|f| {
            self.inner.render(f, &theme);
        })?;

        let mut tui = self.ev.as_stream(SubscriptionPriority::Low).fuse();
        let mut io = CrosstermEventStream::new().fuse();
        let mut anim_timer = tokio::time::interval(std::time::Duration::from_millis(50));

        loop {
            tokio::select! {
                _ = anim_timer.tick() => {
                    let alive = {
                        let mut animator = self.animator.lock().unwrap();
                        animator.tick()
                    };
                    if alive {
                        let theme = self.theme.get_cloned();
                        dt.draw(|f| {
                            self.inner.render(f, &theme);
                        })?;
                    }
                }

                ev = tui.next() => {
                    if let Some(ev) = ev {
                        match *ev {
                            AppEvent::RequestAnimationFrame => {
                                let theme = self.theme.get_cloned();
                                dt.draw(|f| { self.inner.render(f, &theme); })?;
                            }
                            AppEvent::SetTheme(idx) => {
                                let preset = match idx {
                                    0 => Theme::default(),
                                    1 => Theme::light(),
                                    2 => Theme::ocean(),
                                    3 => Theme::solarized(),
                                    4 => Theme::high_contrast(),
                                    _ => Theme::default(),
                                };
                                self.theme.set(Arc::new(preset));
                                let theme = self.theme.get_cloned();
                                dt.draw(|f| { self.inner.render(f, &theme); })?;
                            }
                            AppEvent::Quit => break,
                            _ => {}
                        }
                    }
                }

                ev = io.next() => {
                    match ev {
                        Some(Ok(CrosstermEvent::Key(key))) => {
                            self.ev.emit(AppEvent::KeyEvent(key));
                            let theme = self.theme.get_cloned();
                            dt.draw(|f| { self.inner.render(f, &theme); })?;
                        }
                        Some(Ok(CrosstermEvent::Mouse(mouse))) => {
                            self.ev.emit(AppEvent::MouseEvent(mouse));
                            let theme = self.theme.get_cloned();
                            dt.draw(|f| { self.inner.render(f, &theme); })?;
                        }
                        Some(Ok(CrosstermEvent::Resize(_w, _h))) => {
                            let theme = self.theme.get_cloned();
                            dt.draw(|f| { self.inner.render(f, &theme); })?;
                        }
                        Some(Ok(_)) => {
                            let theme = self.theme.get_cloned();
                            dt.draw(|f| { self.inner.render(f, &theme); })?;
                        }
                        Some(Err(e)) => return Err(e.into()),
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }
}

impl Deref for App {
    type Target = EventTarget<AppEvent>;

    fn deref(&self) -> &Self::Target { &self.ev }
}

/// Convenience helpers for event targets that dispatch [`AppEvent`].
impl EventTarget<AppEvent> {
    /// Subscribe only to `AppEvent::KeyEvent` payloads.
    ///
    /// The handler receives the original event (so it can call [`Cancellable::cancel`])
    /// plus the extracted [`KeyEvent`].
    pub fn on_key(
        &self,
        priority: SubscriptionPriority,
        handler: impl Fn(Arc<Cancellable<AppEvent>>, KeyEvent) + Send + Sync + 'static,
    ) -> SubscriptionHandle<AppEvent> {
        self.on(priority, move |ev| {
            if let AppEvent::KeyEvent(key) = **ev {
                handler(ev, key);
            }
        })
    }
}
