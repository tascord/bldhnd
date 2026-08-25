use {
    bobatea::{
        components::{input::Input, list::List, style::BobaStyle},
        events::SubscriptionPriority,
        theme::Theme,
    },
    crossterm::event::{KeyCode, MouseEvent},
    futures_signals::signal::Mutable,
    ratatui::prelude::*,
    std::time::Instant,
};

pub const BANNER_FONT: &str = include_str!("../../../../_assets/Pagga.tlf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Music,
    Movie,
    Series,
}

impl SearchType {
    pub fn list() -> Vec<String> { vec!["Music".to_string(), "Movie".to_string(), "Series".to_string()] }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => SearchType::Music,
            1 => SearchType::Movie,
            2 => SearchType::Series,
            _ => SearchType::Music,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SearchType::Music => "Music",
            SearchType::Movie => "Movie",
            SearchType::Series => "Series",
        }
    }

    pub fn media_type_for_download(self) -> &'static str {
        match self {
            SearchType::Music => "Music",
            SearchType::Movie => "Movie",
            SearchType::Series => "TvShow",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchStatus {
    Idle,
    Searching { since: Instant },
    Done { count: usize, elapsed_ms: u128 },
    Failed { message: String },
}

/// Keyboard focus within the search tab. Tab cycles Input → TypeList → Results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchFocus {
    Input,
    TypeList,
    Results,
}

#[derive(Clone)]
pub struct SearchPanel {
    input: Input,
    list: List,
    pub results: Mutable<Vec<crate::ipsea::SearchHit>>,
    pub status: Mutable<SearchStatus>,
    /// Index of the currently highlighted result (-1 = none).
    pub result_idx: Mutable<usize>,
    focus: Mutable<SearchFocus>,
    /// True while a backend search (soulseek/torrent/usenet) is in flight.
    searching_backend: Mutable<bool>,
    /// Media type captured at KB-search time, reused for downloads.
    media_type_buf: Mutable<String>,
    input_area: Mutable<Rect>,
    list_area: Mutable<Rect>,
}

impl SearchPanel {
    pub fn new() -> Self {
        Self {
            input: Input::new("Search").placeholder("query…"),
            list: List::new(SearchType::list()),
            results: Mutable::new(Vec::new()),
            status: Mutable::new(SearchStatus::Idle),
            result_idx: Mutable::new(0),
            focus: Mutable::new(SearchFocus::Results),
            searching_backend: Mutable::new(false),
            media_type_buf: Mutable::new("Music".to_string()),
            input_area: Mutable::new(Rect::default()),
            list_area: Mutable::new(Rect::default()),
        }
    }

    pub fn search_type(&self) -> SearchType { SearchType::from_index(self.list.selected()) }

    pub fn query(&self) -> String { self.input.value() }

    pub fn input_focused(&self) -> bool { self.input.is_focused() }

    /// True when the currently displayed results are backend results (ready to
    /// download) as opposed to KB metadata results.
    pub fn is_backend_results(&self) -> bool {
        self.results.lock_ref().first().map_or(false, |h| h.backend != "bh-server")
    }

    /// Reconcile the focus state with whichever component actually holds
    /// keyboard focus (mouse clicks / framework blurs can move it behind
    /// our back).
    fn sync_focus(&self) {
        let f = if self.input.is_focused() {
            SearchFocus::Input
        } else if self.list.is_focused() {
            SearchFocus::TypeList
        } else {
            SearchFocus::Results
        };
        self.focus.set(f);
    }

    /// Cycle keyboard focus: query input → type list → results → input.
    pub fn cycle_focus(&self) {
        self.sync_focus();
        let next = match self.focus.get() {
            SearchFocus::Input => SearchFocus::TypeList,
            SearchFocus::TypeList => SearchFocus::Results,
            SearchFocus::Results => SearchFocus::Input,
        };
        self.set_focus(next);
    }

    fn set_focus(&self, f: SearchFocus) {
        match f {
            SearchFocus::Input => {
                self.list.blur();
                self.input.focus();
            }
            SearchFocus::TypeList => {
                self.input.blur();
                self.list.focus();
            }
            SearchFocus::Results => {
                self.input.blur();
                self.list.blur();
            }
        }
        self.focus.set(f);
    }

    pub fn focus_input(&self) { self.set_focus(SearchFocus::Input) }

    /// Leave any text-entry state; focus falls back to the results list.
    pub fn blur_input(&self) { self.set_focus(SearchFocus::Results) }

    pub fn handle_mouse(&self, ev: &MouseEvent) {
        // Clicks move keyboard focus to the clicked region.
        if matches!(ev.kind, crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)) {
            let in_rect = |r: Rect| {
                r.width > 0
                    && ev.column >= r.x
                    && ev.column < r.x + r.width
                    && ev.row >= r.y
                    && ev.row < r.y + r.height
            };
            if in_rect(self.input_area.get_cloned()) {
                self.set_focus(SearchFocus::Input);
            } else if in_rect(self.list_area.get_cloned()) {
                self.set_focus(SearchFocus::TypeList);
            } else {
                self.set_focus(SearchFocus::Results);
            }
        }
        self.list.on_mouse(self.list_area.get_cloned(), ev);
        self.input.on_mouse(self.input_area.get_cloned(), ev);
        self.sync_focus();
    }

    pub fn handle_key(&self, code: KeyCode) {
        self.sync_focus();
        match self.focus.get() {
            SearchFocus::Input => self.input.on_key(code),
            SearchFocus::TypeList => match code {
                KeyCode::Up | KeyCode::Left => self.list.move_selection(-1),
                KeyCode::Down | KeyCode::Right => self.list.move_selection(1),
                _ => {}
            },
            SearchFocus::Results => match code {
                KeyCode::Up => {
                    let idx = self.result_idx.get();
                    if idx > 0 {
                        self.result_idx.set(idx - 1);
                    }
                }
                KeyCode::Down => {
                    let idx = self.result_idx.get();
                    let total = self.results.lock_ref().len();
                    if total > 0 && idx + 1 < total {
                        self.result_idx.set(idx + 1);
                    }
                }
                KeyCode::Enter => self.fire_action(),
                _ => {}
            },
        }
    }

    /// Enter pressed on a result: either trigger a backend search (KB result)
    /// or queue a download (backend result). With nothing selected/empty
    /// results, Enter jumps back to the query input.
    fn fire_action(&self) {
        let idx = self.result_idx.get();
        let results = self.results.lock_ref().clone();
        let hit = match results.get(idx) {
            Some(h) => h.clone(),
            None => {
                self.focus_input();
                return;
            }
        };

        if hit.backend == "bh-server" {
            // KB result → search download backend
            self.trigger_backend_search(&hit.title);
        } else {
            // Backend result → queue download
            self.queue_download(&hit);
        }
    }

    fn trigger_backend_search(&self, query: &str) {
        let query = query.to_string();
        let results = self.results.clone();
        let status = self.status.clone();
        let result_idx = self.result_idx.clone();
        let searching_backend = self.searching_backend.clone();
        let media_type = self.media_type_buf.get_cloned();

        searching_backend.set(true);
        status.set(SearchStatus::Searching { since: Instant::now() });
        result_idx.set(0);
        tracing::info!("Backend search for '{query}'…");

        tokio::spawn(async move {
            let started = Instant::now();
            let res = tokio::task::spawn_blocking(move || {
                crate::ipsea::Client::connect().search_backend(&query, &media_type)
            })
            .await;

            searching_backend.set(false);
            match res {
                Ok(Ok(hits)) => {
                    let count = hits.len();
                    results.set(hits);
                    status.set(SearchStatus::Done { count, elapsed_ms: started.elapsed().as_millis() });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Backend search failed: {e:#}");
                    status.set(SearchStatus::Failed { message: format!("{e:#}") });
                }
                Err(e) => {
                    tracing::warn!("Backend search task failed: {e}");
                    status.set(SearchStatus::Failed { message: e.to_string() });
                }
            }
        });
    }

    fn queue_download(&self, hit: &crate::ipsea::SearchHit) {
        let hit = hit.clone();
        let media_type = self.media_type_buf.get_cloned();
        let status = self.status.clone();

        tracing::info!("Queuing download: {} ({})", hit.title, hit.backend);

        tokio::spawn(async move {
            let res = tokio::task::spawn_blocking(move || {
                crate::ipsea::Client::connect().start_download(
                    &hit.backend,
                    &hit.title,
                    &hit.title,      // filename = title for soulseek/torrent
                    &hit.url,        // magnet/.torrent link (torrent hits)
                    hit.size,
                    hit.year.map(|y| y as u32),
                    &media_type,
                )
            })
            .await;

            match res {
                Ok(Ok(id)) => {
                    tracing::info!("Download queued: id={id}");
                    status.set(SearchStatus::Done { count: 1, elapsed_ms: 0 });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Queue download failed: {e:#}");
                    status.set(SearchStatus::Failed { message: format!("{e:#}") });
                }
                Err(e) => {
                    tracing::warn!("Queue download task failed: {e}");
                    status.set(SearchStatus::Failed { message: e.to_string() });
                }
            }
        });
    }

    /// Subscribe to the query input's submit event and run the search via the
    /// service IPC in the background.
    pub fn wire_submit(&self, app: bobatea::events::EventTarget<bobatea::AppEvent>) {
        let status = self.status.clone();
        let results = self.results.clone();
        let list = self.list.clone();
        let result_idx = self.result_idx.clone();
        let media_type_buf = self.media_type_buf.clone();
        let panel = self.clone();

        self.input.clone().on(SubscriptionPriority::Low, move |ev| {
            if let bobatea::components::input::InputEvent::Submit(q) = &**ev {
                let q = q.trim().to_string();
                if q.is_empty() {
                    return;
                }
                let media_type = SearchType::from_index(list.selected()).as_str().to_string();
                media_type_buf.set(media_type.clone());
                status.set(SearchStatus::Searching { since: Instant::now() });
                result_idx.set(0);
                // Move focus to the results pane so Enter/arrows act on hits
                // instead of re-submitting the query.
                panel.set_focus(SearchFocus::Results);
                tracing::info!("Searching {media_type} for '{q}'…");

                let (status, results, app) = (status.clone(), results.clone(), app.clone());
                tokio::spawn(async move {
                    let started = std::time::Instant::now();
                    let res = tokio::task::spawn_blocking(move || {
                        crate::ipsea::Client::connect().search(&q, &media_type)
                    })
                    .await;

                    match res {
                        Ok(Ok(hits)) => {
                            let count = hits.len();
                            results.set(hits);
                            status.set(SearchStatus::Done {
                                count,
                                elapsed_ms: started.elapsed().as_millis(),
                            });
                        }
                        Ok(Err(e)) => {
                            tracing::warn!("Search failed: {e:#}");
                            status.set(SearchStatus::Failed { message: format!("{e:#}") });
                        }
                        Err(e) => {
                            tracing::warn!("Search task failed: {e}");
                            status.set(SearchStatus::Failed { message: e.to_string() });
                        }
                    }

                    app.emit(bobatea::AppEvent::RequestAnimationFrame);
                });
            }
        })
        .forget();
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());

        // Left column: query controls
        let left_w = 42.min(area.width.saturating_sub(2));

        // Section headers with rule lines
        accent.render("Query").blit(buf, area.x, area.y);
        muted.render(&rule(6, left_w as usize)).blit(buf, area.x + 6, area.y);
        let input_area = Rect { x: area.x, y: area.y + 1, width: left_w, height: 3 };
        let type_header = Rect { x: area.x, y: area.y + 4, width: left_w, height: 1 };
        let radio_area = Rect { x: area.x, y: area.y + 5, width: left_w, height: 5 };
        let status_line = Rect { x: area.x, y: area.y + 10, width: left_w.max(area.width / 2), height: 1 };

        self.input_area.set(input_area);
        self.list_area.set(radio_area);

        self.input.render_to_buf(input_area, buf, theme);

        muted.render("type").blit(buf, type_header.x, type_header.y);
        self.list.render_to_buf(radio_area, buf, theme);

        // Status line
        let status_text = match &*self.status.lock_ref() {
            SearchStatus::Idle => String::new(),
            SearchStatus::Searching { .. } => {
                if *self.searching_backend.lock_ref() {
                    "Searching download backend…".to_string()
                } else {
                    "Searching…".to_string()
                }
            }
            SearchStatus::Done { count, elapsed_ms } => format!("{count} results in {elapsed_ms}ms"),
            SearchStatus::Failed { message } => format!("Error: {message}"),
        };
        let status_style = match &*self.status.lock_ref() {
            SearchStatus::Failed { .. } => BobaStyle::new().fg(theme.palette.destructive.to_rgb()),
            _ => muted,
        };
        if !status_text.is_empty() {
            status_style.render(&status_text).blit(buf, status_line.x, status_line.y);
        }

        // Right column: results
        let rx = area.x + left_w + 2;
        if rx < area.right() {
            let results_area = Rect { x: rx, y: area.y, width: area.right() - rx, height: area.height };
            self.render_results(results_area, buf, theme);
        }
    }

    fn render_results(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());
        let warn = BobaStyle::new().fg(theme.palette.warning.to_rgb());
        let highlight = BobaStyle::new().fg(theme.palette.accent.to_rgb());

        let is_backend = self.is_backend_results();
        let header_label = if is_backend { "Download Results" } else { "Results" };
        accent.render(header_label).blit(buf, area.x, area.y);
        muted.render(&rule(header_label.len(), area.width as usize)).blit(buf, area.x + header_label.len() as u16, area.y);

        if area.height < 3 || area.width < 20 {
            return;
        }

        let searching = matches!(&*self.status.lock_ref(), SearchStatus::Searching { .. });
        let results = self.results.lock_ref();

        if searching && results.is_empty() {
            warn.render("Searching…").blit(buf, area.x, area.y + 1);
            return;
        }

        if results.is_empty() {
            muted.render("No results yet — enter a query and press Enter.").blit(buf, area.x, area.y + 1);
            return;
        }

        // Column layout
        let (year_w, size_w, ext_w, gap) = (4usize, 9usize, 5usize, 2usize);
        let title_w =
            (area.width as usize).saturating_sub(year_w + size_w + ext_w + gap * 3).clamp(12, 64);

        // Header row
        let mut hx = area.x;
        for (label, w) in [("TITLE", title_w), ("YEAR", year_w), ("SIZE", size_w), ("EXT", ext_w)] {
            muted.render(&pad(&label.to_string(), w)).blit(buf, hx, area.y + 1);
            hx += (w + gap) as u16;
        }

        let rows = area.height as usize - 2;
        let total = results.len();
        let shown = rows.min(total);
        let sel = self.result_idx.get();

        for (i, hit) in results.iter().take(shown).enumerate() {
            let y = area.y + 2 + i as u16;
            let style = if i == sel { highlight } else { fg };
            let marker = if i == sel { "▸ " } else { "  " };

            let title = match &hit.artist {
                Some(a) => format!("{a} — {}", hit.title),
                None => hit.title.clone(),
            };
            let year = hit.year.map(|y| y.to_string()).unwrap_or_default();
            let size = fmt_size(hit.size);

            let mut x = area.x;
            // Marker + title
            let marker_w = 2;
            style.render(marker).blit(buf, x, y);
            x += marker_w as u16;
            style.render(&trunc_pad(&title, title_w.saturating_sub(marker_w))).blit(buf, x, y);
            x += (title_w + gap - marker_w) as u16;
            muted.render(&trunc_pad(&year, year_w)).blit(buf, x, y);
            x += (year_w + gap) as u16;
            muted.render(&trunc_pad(&size, size_w)).blit(buf, x, y);
            x += (size_w + gap) as u16;
            muted.render(&trunc_pad(&hit.ext, ext_w)).blit(buf, x, y);
        }

        if total > shown {
            let note = format!("+ {} more", total - shown);
            muted.render(&note).blit(buf, area.x, area.bottom() - 1);
        }
    }
}

impl Default for SearchPanel {
    fn default() -> Self { Self::new() }
}

fn pad(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w { s.chars().take(w).collect() } else { format!("{s}{}", " ".repeat(w - len)) }
}

/// Dim horizontal rule starting after a header label.
fn rule(from: usize, to: usize) -> String { "─".repeat(to.saturating_sub(from + 1)) }

fn trunc_pad(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len > w {
        let cut: String = s.chars().take(w.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        pad(s, w)
    }
}

fn fmt_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB { format!("{:.1}G", bytes as f64 / GB as f64) } else { format!("{:.0}M", bytes as f64 / MB as f64) }
}
