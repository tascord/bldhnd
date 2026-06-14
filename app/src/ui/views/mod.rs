use std::{sync::{Arc, LazyLock, RwLock}, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

use crate::{events::{EventTarget, SubscriptionPriority}, ui::views::demo::DemoView};

pub mod demo;

static MODEL: LazyLock<Arc<RwLock<Model>>> = LazyLock::new(|| Arc::new(RwLock::new(Model::new())));

pub fn model() -> Arc<RwLock<Model>> {
    MODEL.clone()
}

pub struct Model {
    pub exit: bool,
    pub target: EventTarget<ModelEvent>
}

#[derive(Debug)]
pub enum ModelEvent {
    KeyPress(KeyCode)
}

#[allow(clippy::new_without_default)]
impl Model {
    pub fn new() -> Self {
        let m = Model { exit: false, target: EventTarget::new() };

        // Add keypress handler: update demo view and handle quit
        m.target.on(SubscriptionPriority::Low, |v| {
            if let ModelEvent::KeyPress(key_code) = **v {
                // update demo view's last event
                crate::ui::views::demo::set_last_event(format!("{:?}", key_code));

                // quit on 'q' or 'Q'
                if key_code == KeyCode::Char('q') || key_code == KeyCode::Char('Q') {
                    model().write().unwrap().exit = true
                }
            }
        }).forget();

        m
    }

    pub fn handle_events(&self) -> anyhow::Result<()> {
        // Poll for events with a short timeout so the UI can redraw periodically
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.target.emit(ModelEvent::KeyPress(key_event.code));
                }
                _ => {}
            };
        }
        Ok(())
    }

    pub fn draw(&self, f: &mut Frame) {
        f.render_widget(DemoView, f.area());
    }

    pub fn run(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        loop {
            // Draw while holding a short-lived read lock, then drop it before polling/input handling.
            let target_clone: EventTarget<ModelEvent>;
            {
                let s = model();
                let s = s.read().unwrap();

                if s.exit {
                    break
                }

                terminal.draw(|frame| s.draw(frame))?;
                target_clone = s.target.clone();
            }

            // Poll for events with a short timeout so the UI can redraw periodically
            if event::poll(Duration::from_millis(200))? {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        target_clone.emit(ModelEvent::KeyPress(key_event.code));
                    }
                    _ => {}
                };
            }
        }

        Ok(())
    }
}