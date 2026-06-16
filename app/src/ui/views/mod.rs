use crate::ui::views::settings::SettingsView;

use {
    crate::{
        events::{EventTarget, SubscriptionPriority},
        ui::views::{home::HomeView, library::LibraryView, search::SearchView},
    },
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    ratatui::{
        DefaultTerminal, Frame,
        prelude::*,
        style::Color::{Black, White},
        widgets::{Block, BorderType, Borders, Paragraph, WidgetRef},
    },
    std::{
        sync::{
            Arc, LazyLock, Mutex,
            atomic::{AtomicBool, Ordering::SeqCst},
        },
        time::Duration,
    },
};

pub mod home;
pub mod library;
pub mod search;
pub mod settings;
pub mod results;

static MODEL: LazyLock<Arc<Model>> = LazyLock::new(|| Arc::new(Model::new()));

pub fn model() -> Arc<Model> { MODEL.clone() }

pub fn vstack(c: &[u16], a: Rect) -> Vec<Rect> {
    let mut cons = Vec::new();

    cons.push(Constraint::Fill(1));
    for ele in c.iter().zip(std::iter::repeat(Constraint::Length(1))).flat_map(|(a, b)| [Constraint::Length(*a), b]) {
        cons.push(ele);
    }
    cons.push(Constraint::Fill(1));

    let l = Layout::new(Direction::Vertical, cons).split(a);
    l.iter()
        .skip(1)
        .enumerate()
        .take_while(|(i, _)| i <= &(c.len() + 1))
        .filter(|(i, _)| i.is_multiple_of(2))
        .map(|v| v.1)
        .cloned()
        .collect::<Vec<_>>()
}

pub fn hstack(c: &[u16], a: Rect) -> Vec<Rect> {
    let mut cons = Vec::new();

    cons.push(Constraint::Fill(1));
    for ele in c.iter().zip(std::iter::repeat(Constraint::Length(1))).flat_map(|(a, b)| [Constraint::Length(*a), b]) {
        cons.push(ele);
    }
    cons.push(Constraint::Fill(1));

    let l = Layout::new(Direction::Horizontal, cons).split(a);
    l.iter()
        .skip(1)
        .enumerate()
        .take_while(|(i, _)| i <= &(c.len() + 1))
        .filter(|(i, _)| i.is_multiple_of(2))
        .map(|v| v.1)
        .cloned()
        .collect::<Vec<_>>()
}

pub fn vcenter(l: u16, a: Rect) -> Rect {
    Layout::vertical([Constraint::Fill(1), Constraint::Length(l), Constraint::Fill(1)]).split(a)[1]
}

pub fn hcenter(l: u16, a: Rect) -> Rect {
    Layout::horizontal([Constraint::Fill(1), Constraint::Length(l), Constraint::Fill(1)]).split(a)[1]
}

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
    Home(HomeView),
    Search(SearchView),
    Library(LibraryView),
    Settings(SettingsView)
}

impl Default for ModelView {
    fn default() -> Self { ModelView::Home(HomeView::new()) }
}

impl ModelView {
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            ModelView::Home(v) => v.render_ref(area, buf),
            ModelView::Search(v) => v.render_ref(area, buf),
            ModelView::Library(v) => v.render_ref(area, buf),
            ModelView::Settings(v) => v.render_ref(area, buf),
        }
    }

    pub fn list() -> Vec<(String, usize)> {
        vec![("Home".to_string(), 1), ("Search".to_string(), 2), ("Library".to_string(), 3), ("Settings".to_string(), 4)]
    }

    pub fn key(k: KeyCode) {
        let ex = {
            let m = model();
            m.view.lock().unwrap().as_ref().map(|v| v.n()).unwrap_or_default()
        };

        match k {
            KeyCode::Char('1') if ex != 1 => {
                *model().view.lock().unwrap() = Some(ModelView::Home(HomeView::new()));
            }
            KeyCode::Char('2') if ex != 2 => {
                *model().view.lock().unwrap() = Some(ModelView::Search(SearchView::new()));
            }
            KeyCode::Char('3') if ex != 3 => {
                *model().view.lock().unwrap() = Some(ModelView::Library(LibraryView::new()));
            }
            KeyCode::Char('4') if ex != 4 => {
                *model().view.lock().unwrap() = Some(ModelView::Settings(SettingsView::new()));
            }
            _ => {}
        }
    }

    pub fn n(&self) -> usize {
        match self {
            ModelView::Home(_) => 1,
            ModelView::Search(_) => 2,
            ModelView::Library(_) => 3,
            ModelView::Settings(_) => 4,
        }
    }
}

#[allow(clippy::new_without_default)]
impl Model {
    pub fn new() -> Self {
        let m = Model { exit: AtomicBool::new(false), target: EventTarget::new(), view: Default::default() };

        m.target
            .on(SubscriptionPriority::Low, |v| {
                if let ModelEvent::KeyPress(ev) = **v {
                    if ev.code == KeyCode::Char('q') || ev.code == KeyCode::Char('Q') {
                        return model().exit.store(true, SeqCst);
                    }

                    ModelView::key(ev.code);
                }
            })
            .forget();

        m
    }

    pub fn draw(&self, f: &mut Frame) {
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .render(f.area().inner(Margin::new(1, 1)), f.buffer_mut());

        let sel = self.view.lock().unwrap().as_ref().map(|v| v.n()).unwrap_or_default();

        let mut x = 0;
        for tab in ModelView::list() {
            let text = format!("{} ({})", tab.0, tab.1);
            let text = Line::from_iter([
                Span::raw("| "),
                Span::styled(text, match sel == tab.1 {
                    true => Style::new().bg(White).fg(Black),
                    false => Style::new(),
                }),
                Span::raw(" |"),
            ]);

            Paragraph::new(text.to_string())
                .render(Rect { x: x + 3, y: 1, width: text.width() as u16, height: 1 }, f.buffer_mut());
            x += (text.width() as u16) + 1;
        }

        let mut lock = self.view.lock().unwrap();
        if lock.is_none() {
            *lock = Some(ModelView::Home(HomeView::new()));
        }

        if let Some(l) = lock.as_ref() {
            l.render(f.area().inner(Margin::new(4, 3)), f.buffer_mut())
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
