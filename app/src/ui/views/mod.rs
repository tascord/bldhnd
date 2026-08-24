use {
    crate::{config, fs},
    bobatea::{
        AppEvent, View,
        components::{style::BobaStyle, tabs::Tabs},
        events::{EventTarget, SubscriptionPriority},
        theme::Theme,
    },
    crossterm::event::{KeyCode, MouseEvent},
    futures_signals::signal::Mutable,
    ratatui::{Frame, prelude::*},
    std::sync::Mutex,
};

pub mod home;
pub mod logs;
pub mod search;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Home,
    Search,
    Library,
    Settings,
    Logs,
}

pub struct BldhndView {
    active_tab: Mutable<usize>,
    tabs: Tabs,
    tabs_area: Mutex<Rect>,
    home: home::HomePanel,
    search: search::SearchPanel,
    library_panel: LibraryPanel,
    settings: SettingsPanel,
    logs: logs::LogsPanel,
}

/// Live view of configured volumes + on-disk stats.
struct LibraryPanel;

impl LibraryPanel {
    fn rows(&self) -> Vec<(String, String)> {
        let cfg = config().get_cloned();
        let stats = fs::library().volume_stats();

        cfg.volumes
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let used = match stats.get(i) {
                    Some(s) => format!("{:.1}G · {} files", s.size_gb(), s.file_count),
                    None => "not scanned".to_string(),
                };
                let cap = match v.max_size_gb {
                    Some(max) => format!("cap {:.0}G", max),
                    None => "uncapped".to_string(),
                };
                (format!("{}  {} [{}]", v.name, v.path, cap), used)
            })
            .collect()
    }

    fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());

        accent.render("Volumes").blit(buf, area.x, area.y);

        if area.height < 3 {
            return;
        }

        let rows = self.rows();
        if rows.is_empty() {
            muted.render("No volumes configured.").blit(buf, area.x, area.y + 1);
            muted.render("Add volumes via the service config file.").blit(buf, area.x, area.y + 2);
            return;
        }

        for (i, (label, usage)) in rows.iter().enumerate() {
            let y = area.y + 1 + i as u16 * 2;
            if y >= area.bottom() {
                break;
            }
            fg.render(label).blit(buf, area.x, y);
            muted.render(usage).blit(buf, area.x, y + 1);
        }
    }
}

/// Editable view of the service config.
#[derive(Clone)]
struct SettingsPanel {
    selection: Mutable<usize>,
    editor: bobatea::components::input::Input,
    editing: Mutable<bool>,
    /// Add-volume form state.
    adding: Mutable<bool>,
    add_field: Mutable<usize>,
    name_input: bobatea::components::input::Input,
    path_input: bobatea::components::input::Input,
    /// Shared with HomePanel's status line so saves refresh it.
    service_status: Mutable<String>,
}

/// Editable config fields, indexed by selection.
const SETTINGS_FIELDS: [&str; 3] = ["download_dir", "soulseek_username", "soulseek_password"];

impl SettingsPanel {
    fn new(service_status: Mutable<String>) -> Self {
        Self {
            selection: Mutable::new(0),
            editor: bobatea::components::input::Input::new("value"),
            editing: Mutable::new(false),
            adding: Mutable::new(false),
            add_field: Mutable::new(0),
            name_input: bobatea::components::input::Input::new("name"),
            path_input: bobatea::components::input::Input::new("path"),
            service_status,
        }
    }

    fn editing(&self) -> bool { self.editing.get() }

    fn adding(&self) -> bool { self.adding.get() }

    fn field_value(key: &str) -> String {
        let cfg = config().get_cloned();
        match key {
            "download_dir" => cfg.download_dir.unwrap_or_default(),
            "soulseek_username" => cfg.soulseek_username.unwrap_or_default(),
            "soulseek_password" => cfg.soulseek_password.unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn display_value(key: &str) -> String {
        let v = Self::field_value(key);
        if key == "soulseek_password" && !v.is_empty() {
            "***".to_string()
        } else if v.is_empty() {
            "(unset)".to_string()
        } else {
            v
        }
    }

    fn apply_config(cfg: crate::Config) {
        let committed = cfg.clone();
        config().set(cfg);
        committed.commit();
    }

    /// Returns true when the key was consumed.
    fn handle_key(&self, app: EventTarget<AppEvent>, code: KeyCode) -> bool {
        if self.adding.get() {
            // Keep whichever field is active focused so its input receives keys.
            let f = self.add_field.get();
            (if f == 0 { &self.name_input } else { &self.path_input }).focus();
            (if f == 0 { &self.path_input } else { &self.name_input }).blur();

            match code {
                KeyCode::Esc => self.cancel_add(),
                KeyCode::Enter => return self.commit_add(app),
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    self.add_field.set(1 - f);
                }
                _ => {
                    (if f == 0 { &self.name_input } else { &self.path_input }).on_key(code);
                }
            }
            return true;
        }

        if self.editing.get() {
            match code {
                KeyCode::Esc => {
                    tracing::info!("Config edit cancelled");
                    self.editing.set(false);
                    self.editor.blur();
                }
                KeyCode::Enter => {
                    let key = SETTINGS_FIELDS[self.selection.get()];
                    let value = self.editor.value();

                    let mut cfg = config().get_cloned();
                    let empty = value.trim().is_empty();
                    match key {
                        "download_dir" => cfg.download_dir = (!empty).then_some(value),
                        "soulseek_username" => cfg.soulseek_username = (!empty).then_some(value),
                        "soulseek_password" => cfg.soulseek_password = (!empty).then_some(value),
                        _ => {}
                    }
                    Self::apply_config(cfg);

                    tracing::info!("Config saved");
                    self.editing.set(false);
                    self.editor.blur();
                    probe_service(self.service_status.clone(), app);
                }
                _ => self.editor.on_key(code),
            }
            return true;
        }

        let len = SETTINGS_FIELDS.len();
        match code {
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.name_input.set_value("");
                self.path_input.set_value("");
                self.add_field.set(0);
                self.adding.set(true);
                true
            }
            KeyCode::Up => {
                self.selection.set(self.selection.get().saturating_sub(1));
                true
            }
            KeyCode::Down => {
                self.selection.set((self.selection.get() + 1).min(len - 1));
                true
            }
            KeyCode::Enter => {
                let key = SETTINGS_FIELDS[self.selection.get()];
                self.editor.set_value(Self::field_value(key));
                self.editor.focus();
                self.editing.set(true);
                true
            }
            _ => false,
        }
    }

    fn cancel_add(&self) {
        tracing::info!("Add volume cancelled");
        self.adding.set(false);
        self.name_input.blur();
        self.path_input.blur();
    }

    fn commit_add(&self, app: EventTarget<AppEvent>) -> bool {
        let name = self.name_input.value().trim().to_string();
        let path = self.path_input.value().trim().to_string();
        if name.is_empty() || path.is_empty() {
            tracing::warn!("Volume needs both a name and a path");
            return true;
        }

        let mut cfg = config().get_cloned();
        cfg.volumes.push(crate::Volume::new(name, path, cfg.volumes.len() as u8));
        Self::apply_config(cfg);

        tracing::info!("Volume added");
        self.adding.set(false);
        self.name_input.blur();
        self.path_input.blur();
        probe_service(self.service_status.clone(), app);
        true
    }

    fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());

        accent.render("Service Config").blit(buf, area.x, area.y);

        let labels = ["download dir", "soulseek user", "soulseek pass"];
        let glyph_w = 2u16;
        let label_w = 16u16;
        let editing_row = self.editing.get().then_some(self.selection.get()).unwrap_or(usize::MAX);

        for (i, label) in labels.iter().enumerate() {
            let y = area.y + 2 + i as u16 * 2;
            if y >= area.bottom() {
                break;
            }

            if i == self.selection.get() && !self.editing.get() {
                sel_style(theme).render("▸").blit(buf, area.x, y);
            }

            muted.render(label).blit(buf, area.x + glyph_w, y);

            if i == editing_row {
                fg.render(&format!("{} ", self.editor.value())).blit(buf, area.x + glyph_w + label_w, y);
            } else {
                fg.render(&Self::display_value(SETTINGS_FIELDS[i])).blit(buf, area.x + glyph_w + label_w, y);
            }
        }

        let mut y = area.y + 2 + labels.len() as u16 * 2;

        if y < area.bottom() {
            muted.render("server url").blit(buf, area.x + glyph_w, y);
            fg.render(&format!(
                "{} (read-only)",
                config().get_cloned().bh_server_url.unwrap_or_else(|| "https://bldhnd.fargone.sh".into())
            ))
            .blit(buf, area.x + glyph_w + label_w, y);
            y += 2;
        }

        // Volumes list
        let volumes = config().get_cloned().volumes;
        if y < area.bottom() {
            accent.render("Volumes").blit(buf, area.x, y);
            y += 1;
            if volumes.is_empty() {
                muted.render("(none — press a to add)").blit(buf, area.x + glyph_w, y);
                y += 2;
            } else {
                for v in &volumes {
                    if y >= area.bottom() {
                        break;
                    }
                    fg.render(&format!("{} → {}", v.name, v.path)).blit(buf, area.x + glyph_w, y);
                    y += 1;
                }
                y += 1;
            }
        }

        // Add-volume form
        if self.adding.get() && y < area.bottom().saturating_sub(4) {
            accent.render("add volume").blit(buf, area.x, y);

            let rows: [(&str, &bobatea::components::input::Input); 2] =
                [("name", &self.name_input), ("path", &self.path_input)];
            for (i, (label, input)) in rows.iter().enumerate() {
                let ry = y + 1 + i as u16 * 2;
                if ry >= area.bottom() - 1 {
                    break;
                }
                if self.add_field.get() == i {
                    sel_style(theme).render("▸").blit(buf, area.x, ry);
                }
                muted.render(label).blit(buf, area.x + glyph_w, ry);
                fg.render(&format!("{} ", input.value())).blit(buf, area.x + glyph_w + label_w, ry);
            }
        }
    }
}

fn sel_style(theme: &Theme) -> BobaStyle { BobaStyle::new().fg(theme.palette.accent.to_rgb()) }

fn hint_for(tab: usize) -> &'static str {
    match tab {
        1 => "tab swap focus · enter focus/submit · esc blur · ↑/↓ type",
        2 => "s scan volumes",
        3 => "↑/↓ select · enter edit/save · a add volume",
        _ => "",
    }
}

/// Probe the service asynchronously and update the Home status line.
fn probe_service(status: Mutable<String>, app: EventTarget<AppEvent>) {
    tokio::spawn(async move {
        let res = tokio::task::spawn_blocking(|| {
            crate::ipsea::Client::connect().get_config().map(|cfg| (cfg.volumes.len(), cfg.download_dir))
        })
        .await;
        let line = match res {
            Ok(Ok((vols, dir))) => {
                format!("connected · {vols} volumes · download dir {}", dir.unwrap_or_else(|| "(unset)".into()))
            }
            Ok(Err(e)) => format!("service error — {e}"),
            Err(e) => format!("service unreachable — {e}"),
        };
        status.set(line);
        app.emit(AppEvent::RequestAnimationFrame);
    });
}

#[allow(clippy::new_without_default)]
impl BldhndView {
    pub fn new() -> Self {
        let active_tab = Mutable::new(0);

        let tabs = Tabs::new(["Home", "Search", "Library", "Settings", "Logs"]).active(&active_tab);
        let home = home::HomePanel::new();
        let settings = SettingsPanel::new(home.service_status.clone());

        Self {
            active_tab,
            tabs,
            tabs_area: Mutex::new(Rect::default()),
            home,
            search: search::SearchPanel::new(),
            library_panel: LibraryPanel,
            settings,
            logs: logs::LogsPanel::new(),
        }
    }
}

impl View for BldhndView {
    fn title(&self) -> &'static str { "bldhnd" }

    fn on_mouse(&self, ev: &MouseEvent) {
        let area = *self.tabs_area.lock().unwrap();
        self.tabs.on_mouse(area, ev);
        if self.active_tab.get() == 1 {
            self.search.handle_mouse(ev);
        }
    }

    fn mount(&self, app: &EventTarget<AppEvent>) {
        let active_tab1 = self.active_tab.clone();
        self.tabs.clone().on_change(move |idx| {
            active_tab1.set(idx);
        });

        // Search submit -> query the service in the background.
        self.search.wire_submit(app.clone());

        // Probe service connectivity for the Home tab.
        probe_service(self.home.service_status.clone(), app.clone());

        let active_tab2 = self.active_tab.clone();
        let app_clone = app.clone();
        let search = self.search.clone();
        let settings = self.settings.clone();
        let lib_panel_app = app.clone();
        let lib_events = fs::library().clone();
        lib_events
            .on(SubscriptionPriority::Low, move |_ev| {
                lib_panel_app.emit(AppEvent::RequestAnimationFrame);
            })
            .forget();

        app.on_key(bobatea::events::SubscriptionPriority::High, move |_ev, key| {
            let tab = active_tab2.get();
            let search_typing = tab == 1 && search.input_focused();
            let settings_typing = tab == 3 && (settings.editing() || settings.adding());
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') if !search_typing && !settings_typing => {
                    app_clone.emit(AppEvent::Quit);
                }
                _ if search_typing => search.handle_key(key.code),
                _ if settings_typing => {
                    settings.handle_key(app_clone.clone(), key.code);
                }
                KeyCode::Esc if tab == 1 => search.blur_input(),
                KeyCode::Tab if tab == 1 => search.cycle_focus(),
                KeyCode::Char(c @ '1'..='5') => {
                    active_tab2.set((c as u8 - b'1') as usize);
                    search.blur_input();
                    settings.editing.set(false);
                    settings.editor.blur();
                }
                KeyCode::Char('s') | KeyCode::Char('S') if tab == 2 => {
                    tracing::info!("Scanning volumes…");
                    fs::library().scan();
                }
                KeyCode::Enter if tab == 1 && !search.input_focused() => search.focus_input(),
                _ if tab == 1 => search.handle_key(key.code),
                _ if tab == 3 => {
                    settings.handle_key(app_clone.clone(), key.code);
                }
                _ => {}
            }
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
        *self.tabs_area.lock().unwrap() = tabs_area;
        self.tabs.render_to_buf(tabs_area, f.buffer_mut(), theme);

        // Footer hint bar
        let footer_y = area.bottom().saturating_sub(1);
        let hint = hint_for(self.active_tab.get());
        let footer = format!("  1-5 tabs · q quit{}", if hint.is_empty() { String::new() } else { format!(" · {hint}") });
        BobaStyle::new()
            .fg(theme.palette.fg_muted.to_rgb())
            .render(&footer)
            .blit(f.buffer_mut(), area.x, footer_y);

        let content_area = Rect {
            x: area.x + 2,
            y: area.y + tab_height + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(tab_height + 3),
        };

        match self.active_tab.get() {
            0 => self.home.render(content_area, f.buffer_mut(), theme),
            1 => self.search.render(content_area, f.buffer_mut(), theme),
            2 => self.library_panel.render(content_area, f.buffer_mut(), theme),
            3 => self.settings.render(content_area, f.buffer_mut(), theme),
            4 => self.logs.render(content_area, f.buffer_mut(), theme),
            _ => {}
        }
    }
}
