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
}

#[derive(Debug, Clone)]
pub enum SearchStatus {
    Idle,
    Searching { since: Instant },
    Done { count: usize, elapsed_ms: u128 },
    Failed { message: String },
}

#[derive(Clone)]
pub struct SearchPanel {
    input: Input,
    list: List,
    search_type: Mutable<usize>,
    pub results: Mutable<Vec<crate::ipsea::SearchHit>>,
    pub status: Mutable<SearchStatus>,
    input_area: Mutable<Rect>,
    list_area: Mutable<Rect>,
}

impl SearchPanel {
    pub fn new() -> Self {
        Self {
            input: Input::new("Search").placeholder("query…"),
            list: List::new(SearchType::list()),
            search_type: Mutable::new(0),
            results: Mutable::new(Vec::new()),
            status: Mutable::new(SearchStatus::Idle),
            input_area: Mutable::new(Rect::default()),
            list_area: Mutable::new(Rect::default()),
        }
    }

    pub fn search_type(&self) -> SearchType { SearchType::from_index(self.list.selected()) }

    pub fn query(&self) -> String { self.input.value() }

    pub fn input_focused(&self) -> bool { self.input.is_focused() }

    /// Swap keyboard focus between the query input and the type list.
    pub fn cycle_focus(&self) {
        if self.input.is_focused() {
            self.input.blur();
            self.list.focus();
        } else {
            self.list.blur();
            self.input.focus();
        }
    }

    pub fn focus_input(&self) {
        self.list.blur();
        self.input.focus();
    }

    pub fn blur_input(&self) {
        self.input.blur();
        self.list.blur();
    }

    pub fn handle_key(&self, code: KeyCode) {
        match code {
            KeyCode::Up => self.list.move_selection(-1),
            KeyCode::Down => self.list.move_selection(1),
            _ => {}
        }
        if self.input.is_focused() {
            self.input.on_key(code);
        }
    }

    pub fn handle_mouse(&self, ev: &MouseEvent) {
        self.list.on_mouse(self.list_area.get_cloned(), ev);
        self.input.on_mouse(self.input_area.get_cloned(), ev);
    }

    /// Subscribe to the query input's submit event and run the search via the
    /// service IPC in the background.
    pub fn wire_submit(&self, app: bobatea::events::EventTarget<bobatea::AppEvent>) {
        let status = self.status.clone();
        let results = self.results.clone();
        let list = self.list.clone();

        self.input.clone().on(SubscriptionPriority::Low, move |ev| {
            if let bobatea::components::input::InputEvent::Submit(q) = &**ev {
                let q = q.trim().to_string();
                if q.is_empty() {
                    return;
                }
                let media_type = SearchType::from_index(list.selected()).as_str().to_string();
                status.set(SearchStatus::Searching { since: Instant::now() });
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

        // Section headers
        accent.render("Query").blit(buf, area.x, area.y);

        // Left column: query controls
        let left_w = 42.min(area.width.saturating_sub(2));
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
            SearchStatus::Searching { .. } => "Searching…".to_string(),
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

        accent.render("Results").blit(buf, area.x, area.y);

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

        // Column layout: title flexible | year 6 | size 8 | ext 6
        let year_w = 4usize;
        let size_w = 9usize;
        let ext_w = 5usize;
        let gap = 2usize;
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
        for (i, hit) in results.iter().take(shown).enumerate() {
            let y = area.y + 2 + i as u16;
            let style = if i == 0 { fg } else { fg };

            let title = match &hit.artist {
                Some(a) => format!("{a} — {}", hit.title),
                None => hit.title.clone(),
            };
            let year = hit.year.map(|y| y.to_string()).unwrap_or_default();
            let size = fmt_size(hit.size);

            let mut x = area.x;
            style.render(&trunc_pad(&title, title_w)).blit(buf, x, y);
            x += (title_w + gap) as u16;
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
