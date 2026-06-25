use {
    crate::{
        events::{EventTarget, SubscriptionPriority},
        ui::views::{home::HomeView, library::LibraryView, logs::LogsView, search::SearchView, settings::SettingsView},
    },
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    futures_signals::signal::Mutable,
    ratatui::{DefaultTerminal, Frame, prelude::*, widgets::WidgetRef},
    std::{
        sync::{Arc, LazyLock},
        time::Duration,
    },
    unicode_width::UnicodeWidthStr,
};

pub mod home;
pub mod library;
pub mod logs;
pub mod results;
pub mod search;
pub mod settings;

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
    pub exit: Mutable<bool>,
    pub target: EventTarget<ModelEvent>,
    pub view: Mutable<Option<ModelView>>,
}

#[derive(Debug)]
pub enum ModelEvent {
    KeyPress(KeyEvent),
}

pub enum ModelView {
    Home(HomeView),
    Search(SearchView),
    Library(LibraryView),
    Settings(SettingsView),
    Logs(LogsView),
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
            ModelView::Logs(v) => v.render_ref(area, buf),
        }
    }

    pub fn list() -> Vec<(String, usize)> {
        vec![
            ("Home".to_string(), 1),
            ("Search".to_string(), 2),
            ("Library".to_string(), 3),
            ("Settings".to_string(), 4),
            ("Logs".to_string(), 5),
        ]
    }

    pub fn key(k: KeyCode) {
        let ex = {
            let m = model();
            m.view.lock_ref().as_ref().map(|v| v.n()).unwrap_or_default()
        };

        match k {
            KeyCode::Char('1') if ex != 1 => {
                model().view.set(Some(ModelView::Home(HomeView::new())));
            }
            KeyCode::Char('2') if ex != 2 => {
                model().view.set(Some(ModelView::Search(SearchView::new())));
            }
            KeyCode::Char('3') if ex != 3 => {
                model().view.set(Some(ModelView::Library(LibraryView::new())));
            }
            KeyCode::Char('4') if ex != 4 => {
                model().view.set(Some(ModelView::Settings(SettingsView::new())));
            }
            KeyCode::Char('5') if ex != 5 => {
                model().view.set(Some(ModelView::Logs(LogsView::new())));
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
            ModelView::Logs(_) => 5,
        }
    }
}

#[allow(clippy::new_without_default)]
impl Model {
    pub fn new() -> Self {
        let m = Model { exit: Mutable::new(false), target: EventTarget::new(), view: Mutable::new(None) };

        m.target
            .on(SubscriptionPriority::Low, |v| {
                let ModelEvent::KeyPress(ev) = **v;
                if ev.code == KeyCode::Char('q') || ev.code == KeyCode::Char('Q') {
                    model().exit.set(true);
                }

                ModelView::key(ev.code);
            })
            .forget();

        m
    }

    pub fn draw(&self, f: &mut Frame) {
        let area = f.area();

        let content_area = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        for x in (area.x + 1)..area.right().saturating_sub(1) {
            f.buffer_mut()[(x, area.y + 1)].set_char('─');
        }

        let sel = self.view.lock_ref().as_ref().map(|v| v.n()).unwrap_or_default();

        let tabs = ModelView::list();
        let max_tab_width = (area.width.saturating_sub(3)) / 2;
        let mut x = area.x + 2;

        for (i, tab) in tabs.iter().enumerate() {
            let is_selected = sel == tab.1;
            let raw_text = format!("{} ", tab.0);
            let text = if raw_text.width() > max_tab_width as usize {
                raw_text[..raw_text.char_indices().nth(max_tab_width as usize - 1).map(|(i, _)| i).unwrap_or(raw_text.len())]
                    .to_string()
            } else {
                raw_text.clone()
            };

            let left_char = if i == 0 { "├" } else { "┼" };
            let right_char = if i == tabs.len() - 1 { "┤" } else { "┼" };
            let separator = if is_selected { "┬" } else { "┼" };

            let line = if is_selected {
                Line::from(vec![
                    Span::raw(left_char),
                    Span::styled(format!(" {} ", text), Style::new().fg(Color::Cyan)),
                    Span::raw(separator),
                ])
            } else {
                Line::from(vec![
                    Span::raw(left_char),
                    Span::styled(format!(" {} ", text), Style::new()),
                    Span::raw(right_char),
                ])
            };

            let width = text.width() + 4;
            if x + (width as u16) < area.right().saturating_sub(1) {
                line.render(Rect { x, y: area.y + 1, width: width as u16, height: 1 }, f.buffer_mut());
            }
            x += width as u16;
        }

        let view_lock = self.view.lock_ref();
        if view_lock.is_none() {
            self.view.set(Some(ModelView::Home(HomeView::new())));
        }

        if let Some(l) = view_lock.as_ref() {
            l.render(content_area, f.buffer_mut())
        }

        crate::ui::components::sonner::sonner().render_ref(area, f.buffer_mut());
        crate::ui::components::modal::modal().render_ref(area, f.buffer_mut());
    }

    pub fn run(terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        crate::logs::install_tracing();

        loop {
            let target_clone: EventTarget<ModelEvent>;
            let s = model();
            if s.exit.get() {
                break;
            }

            terminal.draw(|frame| s.draw(frame))?;
            target_clone = s.target.clone();

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
