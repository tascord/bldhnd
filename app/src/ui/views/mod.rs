use {
    crate::{
        events::{EventTarget, SubscriptionPriority},
        ui::views::demo::DemoView,
    },
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    ratatui::{DefaultTerminal, Frame, prelude::*, widgets::WidgetRef},
    std::{
        sync::{
            Arc, LazyLock, Mutex,
            atomic::{AtomicBool, Ordering::SeqCst},
        },
        time::Duration,
    },
};

pub mod demo;

static MODEL: LazyLock<Arc<Model>> = LazyLock::new(|| Arc::new(Model::new()));

pub fn model() -> Arc<Model> { MODEL.clone() }

pub struct Model {
    pub exit: AtomicBool,
    pub target: EventTarget<ModelEvent>,
    pub view: Mutex<Option<ModelView>>,
}

#[derive(Debug)]
pub enum ModelEvent {
    KeyPress(KeyEvent),
}

pub enum ModelView {
    Demo(DemoView),
}

impl Default for ModelView {
    fn default() -> Self { ModelView::Demo(DemoView::new()) }
}

impl ModelView {
    pub fn string(&self) -> String {
        match self {
            ModelView::Demo(_) => "demo",
        }
        .to_string()
    }

    pub fn char(&self) -> char {
        match self {
            ModelView::Demo(_) => 'd',
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            ModelView::Demo(v) => v.render_ref(area, buf),
        }
    }
}

#[allow(clippy::new_without_default)]
impl Model {
    pub fn new() -> Self {
        let m = Model { exit: AtomicBool::new(false), target: EventTarget::new(), view: Default::default() };

        m.target
            .on(SubscriptionPriority::Low, |v| {
                if let ModelEvent::KeyPress(key_code) = **v {
                    // quit on 'q' or 'Q'
                    if key_code.code == KeyCode::Char('q') || key_code.code == KeyCode::Char('Q') {
                        model().exit.store(true, SeqCst);
                    }
                }
            })
            .forget();

        m
    }

    pub fn draw(&self, f: &mut Frame) {
        let mut lock = self.view.lock().unwrap();
        if lock.is_none() {
            *lock = Some(ModelView::Demo(DemoView::new()));
        }

        if let Some(l) = lock.as_ref() {
            l.render(f.area(), f.buffer_mut())
        }
    }

    pub fn run(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        loop {
            // Draw while holding a short-lived read lock, then drop it before polling/input handling.
            let target_clone: EventTarget<ModelEvent>;
            let s = model();
            if s.exit.load(SeqCst) {
                break;
            }

            terminal.draw(|frame| s.draw(frame))?;
            target_clone = s.target.clone();

            // Poll for events with a short timeout so the UI can redraw periodically
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        target_clone.emit(ModelEvent::KeyPress(key_event));
                    }
                    _ => {}
                };
            }
        }

        Ok(())
    }
}
