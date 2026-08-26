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
pub mod downloads;

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
    downloads: downloads::DownloadsPanel,
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

/// Editable view of the service config, organised into subsections.
#[derive(Clone)]
struct SettingsPanel {
    section: Mutable<usize>,
    /// True once the user has entered the section's content pane.
    inside: Mutable<bool>,
    selection: Mutable<usize>,
    editor: bobatea::components::input::Input,
    /// What the inline editor is currently editing.
    editing: Mutable<Option<Row>>,
    /// Multi-field add form (volumes / indexers).
    adding: Mutable<bool>,
    add_field: Mutable<usize>,
    add_inputs: Vec<bobatea::components::input::Input>,
    /// Shared with HomePanel's status line so saves refresh it.
    service_status: Mutable<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sec {
    General,
    Volumes,
    Soulseek,
    Torrent,
    Usenet,
}

const SECTIONS: [Sec; 5] = [Sec::General, Sec::Volumes, Sec::Soulseek, Sec::Torrent, Sec::Usenet];
const SECTION_NAMES: [&str; 5] = ["General", "Volumes", "Soulseek", "Torrent", "Usenet"];

impl Sec {
    fn from_index(i: usize) -> Sec { SECTIONS[i.min(SECTIONS.len() - 1)] }
}

/// One selectable row inside a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    DownloadDir,
    ServerUrl,
    /// Volume #i, field 0=name 1=path 2=max-size.
    VolField(usize, usize),
    SlskUser,
    SlskPass,
    /// Indexer of kind (0=torrent, 1=usenet), #idx, field 0=name 1=url 2=api-key.
    IdxField(u8, usize, usize),
    SabUrl,
    SabKey,
}

fn mask(v: &str) -> String {
    if v.is_empty() { "(unset)".into() } else { "***".into() }
}

impl SettingsPanel {
    fn new(service_status: Mutable<String>) -> Self {
        Self {
            section: Mutable::new(0),
            inside: Mutable::new(false),
            selection: Mutable::new(0),
            editor: bobatea::components::input::Input::new("value"),
            editing: Mutable::new(None),
            adding: Mutable::new(false),
            add_field: Mutable::new(0),
            add_inputs: vec![
                bobatea::components::input::Input::new("name"),
                bobatea::components::input::Input::new("value"),
                bobatea::components::input::Input::new("api key"),
            ],
            service_status,
        }
    }

    fn editing(&self) -> bool { self.editing.get().is_some() }

    fn adding(&self) -> bool { self.adding.get() }

    /// Reset any transient UI state (used when leaving the tab).
    fn reset_transient(&self) {
        self.editing.set(None);
        self.editor.blur();
        self.adding.set(false);
        for i in &self.add_inputs {
            i.blur();
        }
    }

    fn apply_config(&self, cfg: crate::Config, app: &EventTarget<AppEvent>) {
        let committed = cfg.clone();
        config().set(cfg);
        committed.commit();
        probe_service(self.service_status.clone(), app.clone());
    }

    // ── Row model ─────────────────────────────────────────────────────────

    fn rows(&self, sec: Sec) -> Vec<Row> {
        let cfg = config().get_cloned();
        match sec {
            Sec::General => vec![Row::DownloadDir, Row::ServerUrl],
            Sec::Volumes => (0..cfg.volumes.len())
                .flat_map(|i| [Row::VolField(i, 0), Row::VolField(i, 1), Row::VolField(i, 2)])
                .collect(),
            Sec::Soulseek => vec![Row::SlskUser, Row::SlskPass],
            Sec::Torrent => {
                let mut v: Vec<Row> = (0..cfg.torrent_indexers.len())
                    .flat_map(|i| [Row::IdxField(0, i, 0), Row::IdxField(0, i, 1), Row::IdxField(0, i, 2)])
                    .collect();

                v
            }
            Sec::Usenet => {
                let mut v: Vec<Row> = (0..cfg.usenet_indexers.len())
                    .flat_map(|i| [Row::IdxField(1, i, 0), Row::IdxField(1, i, 1), Row::IdxField(1, i, 2)])
                    .collect();
                v.extend([Row::SabUrl, Row::SabKey]);
                v
            }
        }
    }

    /// (label, display value, is-secret) for a row.
    fn row_label_value(&self, row: Row) -> (String, String, bool) {
        let cfg = config().get_cloned();
        match row {
            Row::DownloadDir => ("download dir".into(), cfg.download_dir.clone().unwrap_or_else(|| "(unset)".into()), false),
            Row::ServerUrl => (
                "server url".into(),
                format!("{} (read-only)", cfg.bh_server_url.unwrap_or_else(|| "https://bldhnd.fargone.sh".into())),
                false,
            ),
            Row::VolField(i, f) => {
                let v = cfg.volumes.get(i);
                match f {
                    0 => (format!("{:>2}. name", i + 1), v.map(|v| v.name.clone()).unwrap_or_default(), false),
                    1 => ("path".into(), v.map(|v| v.path.clone()).unwrap_or_default(), false),
                    _ => (
                        "max size (gb)".into(),
                        v.and_then(|v| v.max_size_gb).map(|g| format!("{g:.0}")).unwrap_or_else(|| "(uncapped)".into()),
                        false,
                    ),
                }
            }
            Row::SlskUser => ("username".into(), cfg.soulseek_username.unwrap_or_default(), false),
            Row::SlskPass => ("password".into(), mask(&cfg.soulseek_password.unwrap_or_default()), true),
            Row::IdxField(kind, i, f) => {
                let list = if kind == 0 { &cfg.torrent_indexers } else { &cfg.usenet_indexers };
                let e = list.get(i);
                match f {
                    0 => (format!("{:>2}. name", i + 1), e.map(|e| e.name.clone()).unwrap_or_default(), false),
                    1 => ("url".into(), e.map(|e| e.url.clone()).unwrap_or_default(), false),
                    _ => ("api key".into(), e.map(|e| mask(e.api_key.as_deref().unwrap_or(""))).unwrap_or_default(), true),
                }
            }
            Row::SabUrl => ("sab url".into(), cfg.sabnzbd.as_ref().map(|s| s.url.clone()).unwrap_or_default(), false),
            Row::SabKey => ("sab key".into(), cfg.sabnzbd.as_ref().map(|s| mask(&s.api_key)).unwrap_or_default(), true),
        }
    }

    fn row_current_value(&self, row: Row) -> String {
        let (_, val, secret) = self.row_label_value(row);
        if secret && val != "(unset)" {
            // Fetch the real value for editing rather than the mask.
            return self.row_real_secret(row);
        }
        if val == "(unset)" || val == "(uncapped)" || val.ends_with("(read-only)") {
            String::new()
        } else {
            val
        }
    }

    fn row_real_secret(&self, row: Row) -> String {
        let cfg = config().get_cloned();
        match row {
            Row::SlskPass => cfg.soulseek_password.unwrap_or_default(),
            Row::IdxField(kind, i, _) => {
                let list = if kind == 0 { &cfg.torrent_indexers } else { &cfg.usenet_indexers };
                list.get(i).and_then(|e| e.api_key.clone()).unwrap_or_default()
            }
            Row::SabKey => cfg.sabnzbd.map(|s| s.api_key).unwrap_or_default(),
            _ => String::new(),
        }
    }

    fn set_row(&self, cfg: &mut crate::Config, row: Row, value: String) {
        let empty = value.trim().is_empty();
        match row {
            Row::DownloadDir => cfg.download_dir = (!empty).then_some(value),
            Row::ServerUrl | Row::VolField(_, 99) => {}
            Row::VolField(i, f) => {
                if let Some(v) = cfg.volumes.get_mut(i) {
                    match f {
                        0 if !empty => v.name = value,
                        1 => v.path = value,
                        2 => v.max_size_gb = if empty { None } else { value.trim().parse::<f32>().ok() },
                        _ => {}
                    }
                }
            }
            Row::SlskUser => cfg.soulseek_username = (!empty).then_some(value),
            Row::SlskPass => cfg.soulseek_password = (!empty).then_some(value),
            Row::IdxField(kind, i, f) => {
                let list = if kind == 0 { &mut cfg.torrent_indexers } else { &mut cfg.usenet_indexers };
                if let Some(e) = list.get_mut(i) {
                    match f {
                        0 if !empty => e.name = value,
                        1 if !empty => e.url = value,
                        2 => e.api_key = (!empty).then_some(value),
                        _ => {}
                    }
                }
            }
            Row::SabUrl | Row::SabKey => {
                let s = cfg.sabnzbd.get_or_insert_with(|| crate::ApiKeyEndpoint {
                    url: String::new(),
                    api_key: String::new(),
                });
                match row {
                    Row::SabUrl if !empty => s.url = value,
                    Row::SabKey if !empty => s.api_key = value,
                    _ => {}
                }
            }
        }
    }

    fn begin_edit(&self, row: Row) {
        if matches!(row, Row::ServerUrl) {
            tracing::info!("server url is read-only");
            return;
        }
        self.editor.set_value(self.row_current_value(row));
        self.editor.focus();
        self.editing.set(Some(row));
    }

    fn commit_edit(&self, app: EventTarget<AppEvent>) -> bool {
        let Some(row) = self.editing.get() else { return false };
        let value = self.editor.value();

        let mut cfg = config().get_cloned();
        self.set_row(&mut cfg, row, value);

        tracing::info!("Config saved");
        self.editing.set(None);
        self.editor.blur();
        self.apply_config(cfg, &app);
        true
    }

    /// Delete the item (volume / indexer) the given row belongs to.
    fn delete_item(&self, row: Row, app: EventTarget<AppEvent>) {
        let mut cfg = config().get_cloned();
        match row {
            Row::VolField(i, _) => {
                if i < cfg.volumes.len() {
                    let removed = cfg.volumes.remove(i);
                    tracing::info!("Removed volume {}", removed.name);
                }
            }
            Row::IdxField(0, i, _) => {
                if i < cfg.torrent_indexers.len() {
                    let removed = cfg.torrent_indexers.remove(i);
                    tracing::info!("Removed torrent indexer {}", removed.name);
                }
            }
            Row::IdxField(_, i, _) => {
                if i < cfg.usenet_indexers.len() {
                    let removed = cfg.usenet_indexers.remove(i);
                    tracing::info!("Removed usenet indexer {}", removed.name);
                }
            }
            _ => return,
        }
        let max = self.rows(Sec::from_index(self.section.get())).len().saturating_sub(1);
        self.selection.set(self.selection.get().min(max));
        self.apply_config(cfg, &app);
    }

    /// Fields shown by the add-form for the active section.
    fn add_form_labels(&self, sec: Sec) -> &'static [&'static str] {
        match sec {
            Sec::Volumes => &["name", "path"],
            Sec::Torrent | Sec::Usenet => &["name", "url", "api key"],
            _ => &[],
        }
    }

    fn start_add(&self) {
        let sec = Sec::from_index(self.section.get());
        let labels = self.add_form_labels(sec);
        if labels.is_empty() {
            return;
        }
        for inp in &self.add_inputs {
            inp.set_value(String::new());
            inp.blur();
        }
        self.add_field.set(0);
        self.add_inputs[0].focus();
        self.adding.set(true);
    }

    fn cancel_add(&self) {
        tracing::info!("Add cancelled");
        self.adding.set(false);
        for i in &self.add_inputs {
            i.blur();
        }
    }

    fn commit_add(&self, app: EventTarget<AppEvent>) -> bool {
        let sec = Sec::from_index(self.section.get());
        let vals: Vec<String> = self.add_inputs.iter().take(3).map(|i| i.value().trim().to_string()).collect();

        let mut cfg = config().get_cloned();
        match sec {
            Sec::Volumes => {
                let (name, path) = (vals[0].clone(), vals[1].clone());
                if name.is_empty() || path.is_empty() {
                    tracing::warn!("Volume needs both a name and a path");
                    return true;
                }
                cfg.volumes.push(crate::Volume::new(name, path, cfg.volumes.len() as u8));
                tracing::info!("Volume added");
            }
            Sec::Torrent => {
                let (name, url) = (vals[0].clone(), vals[1].clone());
                if name.is_empty() || url.is_empty() {
                    tracing::warn!("Indexer needs a name and a url");
                    return true;
                }
                let mut e = crate::Indexer::new(name, url);
                e.api_key = (!vals[2].is_empty()).then_some(vals[2].clone());
                cfg.torrent_indexers.push(e);
                tracing::info!("Torrent indexer added");
            }
            Sec::Usenet => {
                let (name, url) = (vals[0].clone(), vals[1].clone());
                if name.is_empty() || url.is_empty() {
                    tracing::warn!("Indexer needs a name and a url");
                    return true;
                }
                let mut e = crate::Indexer::new(name, url);
                e.api_key = (!vals[2].is_empty()).then_some(vals[2].clone());
                cfg.usenet_indexers.push(e);
                tracing::info!("Usenet indexer added");
            }
            _ => return true,
        }

        self.apply_config(cfg, &app);
        self.cancel_add();
        true
    }

    /// Returns true when the key was consumed.
    fn handle_key(&self, app: EventTarget<AppEvent>, code: KeyCode) -> bool {
        // ── Add form ───────────────────────────────────────────────────────
        if self.adding.get() {
            let n = self.add_form_labels(Sec::from_index(self.section.get())).len();
            let f = self.add_field.get().min(n - 1);
            for (i, inp) in self.add_inputs.iter().enumerate().take(n) {
                if i == f { inp.focus(); } else { inp.blur(); }
            }
            match code {
                KeyCode::Esc => self.cancel_add(),
                KeyCode::Enter => return self.commit_add(app),
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    self.add_field.set((f + 1) % n);
                }
                _ => self.add_inputs[f].on_key(code),
            }
            return true;
        }

        // ── Inline editor ──────────────────────────────────────────────────
        if self.editing.get().is_some() {
            match code {
                KeyCode::Esc => {
                    tracing::info!("Edit cancelled");
                    self.editing.set(None);
                    self.editor.blur();
                }
                KeyCode::Enter => return self.commit_edit(app),
                _ => self.editor.on_key(code),
            }
            return true;
        }

        // ── Section / row navigation ───────────────────────────────────────
        // Up/Down move between sections until the user enters one with →;
        // ← leaves back to the section menu.
        if !self.inside.get() {
            return match code {
                KeyCode::Up => {
                    let s = self.section.get();
                    self.section.set(s.saturating_sub(1));
                    true
                }
                KeyCode::Down => {
                    let s = self.section.get();
                    self.section.set((s + 1).min(SECTIONS.len() - 1));
                    true
                }
                KeyCode::Right | KeyCode::Enter | KeyCode::Char('l') => {
                    self.selection.set(0);
                    self.inside.set(true);
                    true
                }
                _ => false,
            };
        }

        let sec = Sec::from_index(self.section.get());
        let rows = self.rows(sec);
        let sel = self.selection.get();

        match code {
            KeyCode::Left | KeyCode::Char('h') => {
                if self.editing.get().is_none() && !self.adding.get() {
                    self.inside.set(false);
                }
                true
            }
            KeyCode::Up => {
                self.selection.set(sel.saturating_sub(1));
                true
            }
            KeyCode::Down => {
                self.selection.set((sel + 1).min(rows.len().saturating_sub(1)));
                true
            }
            KeyCode::Enter => {
                if let Some(row) = rows.get(sel) {
                    self.begin_edit(*row);
                }
                true
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.start_add();
                true
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(row) = rows.get(sel) {
                    self.delete_item(*row, app);
                }
                true
            }
            _ => false,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());

        accent.render("Settings").blit(buf, area.x, area.y);

        // ── Sidebar ────────────────────────────────────────────────────────
        let side_w = 14u16;
        for (i, name) in SECTION_NAMES.iter().enumerate() {
            let y = area.y + 2 + i as u16;
            if y >= area.bottom() {
                break;
            }
            if i == self.section.get() {
                if self.inside.get() {
                    BobaStyle::new().fg(theme.palette.accent.to_rgb()).render("· ").blit(buf, area.x, y);
                } else {
                    sel_style(theme).render("▸ ").blit(buf, area.x, y);
                }
                accent.render(name).blit(buf, area.x + 2, y);
            } else {
                muted.render(name).blit(buf, area.x + 2, y);
            }
        }

        // ── Content pane ───────────────────────────────────────────────────
        let cx = area.x + side_w + 2;
        if cx >= area.right() {
            return;
        }
        let carea = Rect { x: cx, y: area.y + 2, width: area.right() - cx, height: area.bottom().saturating_sub(area.y + 2) };

        let sec = Sec::from_index(self.section.get());
        muted.render(&rule(0, carea.width as usize)).blit(buf, cx, area.y + 1);

        let rows = self.rows(sec);
        let sel = self.selection.get();
        let editing_target = self.editing.get();

        // Section headers between groups.
        let mut last_group = String::new();
        let mut visual_y = 0u16;

        for (ri, row) in rows.iter().enumerate() {
            if area.y + 2 + visual_y >= area.bottom() {
                break;
            }

            // Group headers.
            let group = match row {
                Row::SabUrl | Row::SabKey => "SABnzbd",
                _ => "",
            };
            if !group.is_empty() && group != last_group {
                accent.render(group).blit(buf, carea.x, carea.y + visual_y);
                visual_y += 1;
                last_group = group.to_string();
            }

            let y = carea.y + visual_y;
            visual_y += 1;

            if self.inside.get() && ri == sel && editing_target.is_none() && !self.adding.get() {
                sel_style(theme).render("▸").blit(buf, carea.x, y);
            }

            let (label, value, _secret) = self.row_label_value(*row);
            let label_x = carea.x + 2;

            // Inline editor takes over the value cell.
            let shown = if editing_target == Some(*row) {
                format!("{} ", self.editor.value())
            } else {
                value
            };

            muted.render(&label).blit(buf, label_x, y);
            let value_x = label_x + 16;
            if value_x < area.right() {
                fg.render(&shown).blit(buf, value_x, y);
            }
        }

        // ── Add form overlay ───────────────────────────────────────────────
        if self.adding.get() {
            let fy = carea.y + visual_y + 1;
            if fy < area.bottom() - 4 {
                accent.render("+ add").blit(buf, carea.x, fy);
                let labels = self.add_form_labels(sec);
                for (i, label) in labels.iter().enumerate() {
                    let ry = fy + 1 + i as u16 * 2;
                    if ry >= area.bottom() - 1 {
                        break;
                    }
                    if self.add_field.get() == i {
                        sel_style(theme).render("▸").blit(buf, carea.x, ry);
                    }
                    muted.render(label).blit(buf, carea.x + 2, ry);
                    let vx = carea.x + 2 + 10;
                    if vx < area.right() {
                        fg.render(&format!("{} ", self.add_inputs[i].value())).blit(buf, vx, ry);
                    }
                }
            }
        }
    }
}

fn sel_style(theme: &Theme) -> BobaStyle { BobaStyle::new().fg(theme.palette.accent.to_rgb()) }

/// Dim horizontal rule.
fn rule(from: usize, to: usize) -> String { "─".repeat(to.saturating_sub(from + 1)) }


fn hint_for(tab: usize) -> &'static str {
    match tab {
        1 => "tab focus · enter submit/action · esc blur · ↑/↓ navigate",
        2 => "s scan volumes",
        3 => "↑/↓ section · → enter · ← back · enter edit · a add · d delete",
        5 => "r refresh · enter/c cancel · ↑/↓ select",
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

        let tabs = Tabs::new(["Home", "Search", "Library", "Settings", "Logs", "Downloads"])
            .active(&active_tab);
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
            downloads: downloads::DownloadsPanel::new(),
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

        // Downloads tab redraws after async refreshes.
        self.downloads.wire(app.clone());

        // Poll live download progress ~1/s while the Downloads tab is visible.
        let dl_panel = self.downloads.clone();
        let dl_tab = self.active_tab.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if dl_tab.get() == 5 {
                    dl_panel.poll_progress();
                }
            }
        });

        // Probe service connectivity for the Home tab.
        probe_service(self.home.service_status.clone(), app.clone());

        let active_tab2 = self.active_tab.clone();
        let app_clone = app.clone();
        let search = self.search.clone();
        let settings = self.settings.clone();
        let downloads_panel = self.downloads.clone();
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
                // 'q' quits everywhere except search — typing a query starting
                // with q must not kill the app.
                KeyCode::Char('q') | KeyCode::Char('Q') if tab != 1 && !settings_typing => {
                    app_clone.emit(AppEvent::Quit);
                }
                // Esc toggles between the query input and results; it must be
                // handled before the typing gate or the input swallows it.
                KeyCode::Esc if tab == 1 => {
                    if search.input_focused() {
                        search.blur_input();
                    } else {
                        search.focus_input();
                    }
                }
                _ if search_typing => search.handle_key(key.code),
                _ if settings_typing => {
                    settings.handle_key(app_clone.clone(), key.code);
                }
                KeyCode::Tab if tab == 1 => search.cycle_focus(),
                KeyCode::Char(c @ '1'..='6') => {
                    let new_tab = (c as u8 - b'1') as usize;
                    active_tab2.set(new_tab);
                    search.blur_input();
                    settings.reset_transient();
                    if new_tab == 5 {
                        downloads_panel.refresh();
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') if tab == 2 => {
                    tracing::info!("Scanning volumes…");
                    fs::library().scan();
                }
                KeyCode::Enter if tab == 1 && !search.input_focused() => {
                    search.handle_key(KeyCode::Enter);
                }
                _ if tab == 1 => search.handle_key(key.code),
                _ if tab == 3 => {
                    settings.handle_key(app_clone.clone(), key.code);
                }
                _ if tab == 5 => {
                    downloads_panel.handle_key(key.code);
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
        let footer = format!("  1-6 tabs · q quit{}", if hint.is_empty() { String::new() } else { format!(" · {hint}") });
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
            5 => self.downloads.render(content_area, f.buffer_mut(), theme),
            _ => {}
        }
    }
}
