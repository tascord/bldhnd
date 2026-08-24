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
pub use surface::{
    Cell, Position, Surface, clip, fit_height, fit_width, join_horizontal, join_vertical, place, place_filled, total_width,
};

/// Root trait for a full-screen view.
///
/// Implement this trait on your struct, then pass it to [`App::new`].
/// `mount` is called once before the event loop; `render` is called every frame.
///
/// # Example
///
/// ```
/// use bobatea::{App, View, theme::Theme};
/// use ratatui::Frame;
///
/// struct MyView;
/// impl View for MyView {
///     fn render(&self, ctx: &mut Frame<'_>, theme: &Theme) {
///         ctx.buffer_mut().reset();
///     }
/// }
/// ```
pub trait View {
    /// Title shown in the terminal window title bar (via OSC).
    fn title(&self) -> &'static str { env!("CARGO_CRATE_NAME") }

    /// Called once before the event loop starts.
    ///
    /// Subscribe to app-level events here via `app.on(...)`.
    fn mount(&self, _app: &EventTarget<AppEvent>) {}

    /// Called every time the view needs to be redrawn.
    fn render(&self, ctx: &mut Frame<'_>, theme: &Theme);

    /// Called for every mouse event that does not hit a focused component.
    ///
    /// Default implementation is a no-op.
    fn on_mouse(&self, _ev: &MouseEvent) {}
}

/// Application lifecycle events emitted by the boba runtime.
#[derive(Debug, Clone, Copy)]
pub enum AppEvent {
    /// Request an immediate re-render (no other data changed).
    Quit,

    /// Internal: tick the animation clock and re-render if anything is alive.
    RequestAnimationFrame,

    /// Change the active theme palette by index (0 = default, 1 = light, …).
    SetTheme(usize),

    /// Terminal was resized — new width and height in columns × rows.
    Resize(u16, u16),

    /// A key was pressed.
    KeyEvent(crossterm::event::KeyEvent),

    /// A mouse event occurred.
    MouseEvent(crossterm::event::MouseEvent),
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

/// The boba application runner.
///
/// Create an `App` with your [`View`] implementation, configure its theme,
/// then `run` it asynchronously. The terminal is managed automatically:
/// raw mode is enabled, an alternate screen is used, and state is restored on exit.
pub struct App {
    inner: Arc<dyn View>,
    ev: EventTarget<AppEvent>,
    /// The currently active [`Theme`]. Change it with [`App::set_theme`].
    pub theme: Mutable<Arc<Theme>>,
    pub animator: std::sync::Mutex<Animator>,
}

impl App {
    /// Construct an app wrapping `view`.
    pub fn new(v: impl View + 'static) -> Self {
        Self {
            inner: Arc::new(v),
            ev: EventTarget::new("app"),
            theme: Mutable::new(Arc::new(Theme::default())),
            animator: std::sync::Mutex::new(Animator::new()),
        }
    }

    /// Replace the active theme at runtime.
    pub fn set_theme(&self, theme: Theme) { self.theme.set(Arc::new(theme)); }

    /// Run the app until a [`AppEvent::Quit`] is emitted.
    ///
    /// Initializes raw mode, the alternate screen, and mouse capture;
    /// restores everything on exit.
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
                            self.inner.on_mouse(&mouse);
                            self.ev.emit(AppEvent::MouseEvent(mouse));
                            let theme = self.theme.get_cloned();
                            dt.draw(|f| { self.inner.render(f, &theme); })?;
                        }
                        Some(Ok(CrosstermEvent::Resize(w, h))) => {
                            self.ev.emit(AppEvent::Resize(w, h));
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
