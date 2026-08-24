use {
    crate::ui::views::{home::HomePanel, logs::LogsPanel, search::SearchPanel, settings::SettingsPanel},
    bobatea::{AppEvent, View, components::tabs::Tabs, events::EventTarget, theme::Theme},
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{Frame, prelude::*},
};

pub mod home;
pub mod logs;
pub mod results;
pub mod search;
pub mod settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Home,
    Search,
    Library,
    Settings,
    Logs,
}

impl AppTab {
    fn index(self) -> usize {
        match self {
            AppTab::Home => 0,
            AppTab::Search => 1,
            AppTab::Library => 2,
            AppTab::Settings => 3,
            AppTab::Logs => 4,
        }
    }

    fn from_index(idx: usize) -> Self {
        match idx {
            0 => AppTab::Home,
            1 => AppTab::Search,
            2 => AppTab::Library,
            3 => AppTab::Settings,
            4 => AppTab::Logs,
            _ => AppTab::Home,
        }
    }
}

pub struct BldhndView {
    active_tab: Mutable<usize>,
    tabs: Tabs,
    home: HomePanel,
    search: SearchPanel,
    library: LibraryPanel,
    settings: SettingsPanel,
    logs: LogsPanel,
}

pub struct LibraryPanel;

impl LibraryPanel {
    pub fn new() -> Self { Self }
}

impl Default for LibraryPanel {
    fn default() -> Self { Self::new() }
}

impl LibraryPanel {
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let style = bobatea::components::style::BobaStyle::new().fg(theme.global_fg).bold();

        let surf = style.render("Library - Coming Soon");
        surf.blit(buf, area.x + 2, area.y + area.height / 2);
    }
}

#[allow(clippy::new_without_default)]
impl BldhndView {
    pub fn new() -> Self {
        let active_tab = Mutable::new(0);

        let tabs = Tabs::new(["Home", "Search", "Library", "Settings", "Logs"]).active(&active_tab);

        Self {
            active_tab,
            tabs,
            home: HomePanel::new(),
            search: SearchPanel::new(),
            library: LibraryPanel::new(),
            settings: SettingsPanel::new(),
            logs: LogsPanel::new(),
        }
    }
}

impl View for BldhndView {
    fn title(&self) -> &'static str { "bldhnd" }

    fn mount(&self, app: &EventTarget<AppEvent>) {
        let active_tab1 = self.active_tab.clone();
        self.tabs.clone().on_change(move |idx| {
            active_tab1.set(idx);
        });

        let active_tab2 = self.active_tab.clone();
        let app_clone = app.clone();
        app.on_key(bobatea::events::SubscriptionPriority::High, move |_ev, key| match key.code {
            KeyCode::Char('1') => active_tab2.set(0),
            KeyCode::Char('2') => active_tab2.set(1),
            KeyCode::Char('3') => active_tab2.set(2),
            KeyCode::Char('4') => active_tab2.set(3),
            KeyCode::Char('5') => active_tab2.set(4),
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app_clone.emit(AppEvent::Quit);
            }
            _ => {}
        })
        .forget();
    }

    fn render(&self, f: &mut Frame<'_>, theme: &Theme) {
        let area = f.area();

        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                f.buffer_mut()[(x, y)].set_bg(theme.global_bg);
            }
        }

        let tab_height = 3u16;
        let tabs_area = Rect { x: area.x, y: area.y, width: area.width, height: tab_height };
        self.tabs.render_to_buf(tabs_area, f.buffer_mut(), theme);

        let content_area = Rect {
            x: area.x + 1,
            y: area.y + tab_height,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(tab_height + 1),
        };

        match self.active_tab.get() {
            0 => self.home.render(content_area, f.buffer_mut(), theme),
            1 => self.search.render(content_area, f.buffer_mut(), theme),
            2 => self.library.render(content_area, f.buffer_mut(), theme),
            3 => self.settings.render(content_area, f.buffer_mut(), theme),
            4 => self.logs.render(content_area, f.buffer_mut(), theme),
            _ => {}
        }
    }
}
