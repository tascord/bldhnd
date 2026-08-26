use {
    bobatea::{components::style::BobaStyle, events::EventTarget, theme::Theme, AppEvent},
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::prelude::*,
};

/// Live view of the download queue: list, select, cancel.
#[derive(Clone)]
pub struct DownloadsPanel {
    /// The app event target, wired during mount(); used to request redraws
    /// after async refreshes complete.
    app: Mutable<Option<EventTarget<AppEvent>>>,
    pub items: Mutable<Vec<crate::ipsea::DownloadInfo>>,
    /// Byte-level progress for in-flight downloads, keyed by id.
    progress: Mutable<std::collections::HashMap<u64, crate::ipsea::ProgressInfo>>,
    selection: Mutable<usize>,
    status: Mutable<String>,
}

impl DownloadsPanel {
    pub fn new() -> Self {
        Self {
            app: Mutable::new(None),
            items: Mutable::new(Vec::new()),
            progress: Mutable::new(std::collections::HashMap::new()),
            selection: Mutable::new(0),
            status: Mutable::new(String::new()),
        }
    }

    /// Attach the app event loop so async completions can request a redraw.
    pub fn wire(&self, app: EventTarget<AppEvent>) { self.app.set(Some(app)); }

    fn frame(&self) {
        if let Some(target) = &*self.app.lock_ref() {
            target.emit(AppEvent::RequestAnimationFrame);
        }
    }

    /// Ids of downloads that can still move (worth polling).
    fn active_ids(&self) -> Vec<u64> {
        self.items
            .lock_ref()
            .iter()
            .filter(|d| matches!(d.state.as_str(), "queued" | "connecting" | "downloading"))
            .map(|d| d.id)
            .collect()
    }

    /// Poll live byte progress for all in-flight items. Called ~1/s while the
    /// tab is visible.
    pub fn poll_progress(&self) {
        let ids = self.active_ids();
        if ids.is_empty() {
            return;
        }
        let panel = self.clone();
        tokio::spawn(async move {
            let mut updates = Vec::new();
            for id in ids {
                if let Ok(Ok(p)) =
                    tokio::task::spawn_blocking(move || crate::ipsea::Client::connect().download_progress(id)).await
                {
                    updates.push((id, p));
                }
            }
            if !updates.is_empty() {
                let mut map = panel.progress.lock_mut();
                for (id, p) in updates {
                    map.insert(id, p);
                }
                panel.frame();
            }
        });
    }

    /// Fetch the current queue from the service.
    pub fn refresh(&self) {
        let status = self.status.clone();
        status.set("refreshing…".into());
        self.refresh_async();
    }

    fn cancel(&self, idx: usize) -> bool {
        let Some(d) = self.items.lock_ref().get(idx).cloned() else { return false };
        // Only in-flight downloads can sensibly be cancelled.
        if matches!(d.state.as_str(), "complete" | "cancelled" | "failed") {
            return false;
        }
        let panel = self.clone();
        tracing::info!("Cancelling download {} ({})", d.id, d.title);
        tokio::spawn(async move {
            let id = d.id;
            match tokio::task::spawn_blocking(move || crate::ipsea::Client::connect().cancel_download(id)).await {
                Ok(Ok(())) => {
                    panel.status.set(format!("cancelled #{id}"));
                    panel.refresh_async().await;
                }
                Ok(Err(e)) => panel.status.set(format!("error: {e}")),
                Err(e) => panel.status.set(format!("error: {e}")),
            }
            panel.frame();
        });
        true
    }

    /// Re-drive a failed/cancelled download.
    fn retry(&self, idx: usize) -> bool {
        let Some(d) = self.items.lock_ref().get(idx).cloned() else { return false };
        if !matches!(d.state.as_str(), "failed" | "cancelled") {
            return false;
        }
        let panel = self.clone();
        tracing::info!("Retrying download {} ({})", d.id, d.title);
        tokio::spawn(async move {
            let id = d.id;
            match tokio::task::spawn_blocking(move || crate::ipsea::Client::connect().retry_download(id)).await {
                Ok(Ok(())) => {
                    panel.status.set(format!("retrying #{id}"));
                    panel.refresh_async().await;
                }
                Ok(Err(e)) => panel.status.set(format!("error: {e}")),
                Err(e) => panel.status.set(format!("error: {e}")),
            }
            panel.frame();
        });
        true
    }

    /// Fetch the queue from the service; awaitable so actions can chain it.
    fn refresh_async(&self) -> tokio::task::JoinHandle<()> {
        let items = self.items.clone();
        let status = self.status.clone();
        let panel = self.clone();
        tokio::spawn(async move {
            match tokio::task::spawn_blocking(|| crate::ipsea::Client::connect().list_downloads()).await {
                Ok(Ok(list)) => {
                    status.set(String::new());
                    items.set(list);
                }
                Ok(Err(e)) => status.set(format!("error: {e}")),
                Err(e) => status.set(format!("error: {e}")),
            }
            panel.frame();
        })
    }

    /// Returns true when the key was consumed.
    pub fn handle_key(&self, code: KeyCode) -> bool {
        let sel = self.selection.get();
        let total = self.items.lock_ref().len();
        match code {
            KeyCode::Up => {
                self.selection.set(sel.saturating_sub(1));
                true
            }
            KeyCode::Down => {
                self.selection.set((sel + 1).min(total.saturating_sub(1)));
                true
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh();
                true
            }
            KeyCode::Enter | KeyCode::Char('c') | KeyCode::Char('C') => {
                // Contextual: failed/cancelled rows retry, active rows cancel.
                let state = self.items.lock_ref().get(sel).map(|d| d.state.clone()).unwrap_or_default();
                match state.as_str() {
                    "failed" | "cancelled" => self.retry(sel),
                    _ => self.cancel(sel),
                }
            }
            _ => false,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());
        let warn = BobaStyle::new().fg(theme.palette.warning.to_rgb());
        let ok = BobaStyle::new().fg(theme.palette.success.to_rgb());

        accent.render("Downloads").blit(buf, area.x, area.y);
        muted.render(&"─".repeat((area.width as usize).saturating_sub(10))).blit(buf, area.x + 10, area.y);

        let status = self.status.get_cloned();
        if !status.is_empty() {
            muted.render(&status).blit(buf, area.x, area.y + 1);
        }

        let items: Vec<_> = self.items.lock_ref().iter().cloned().collect();
        if items.is_empty() {
            muted
                .render("Queue empty — queue something from the Search tab (enter on a result).")
                .blit(buf, area.x, area.y + 2);
            return;
        }

        let sel = self.selection.get();
        let progress = self.progress.lock_ref().clone();
        let rows = (area.height as usize).saturating_sub(3);

        for (i, d) in items.iter().take(rows).enumerate() {
            let y = area.y + 2 + i as u16;
            let live = progress.get(&d.id);
            let style = if i == sel { accent } else { fg };
            if i == sel && !matches!(d.state.as_str(), "complete" | "failed" | "cancelled") {
                sel_marker(theme).render("▸").blit(buf, area.x, y);
            }

            let glyph = match d.state.as_str() {
                "downloading" => "▼",
                "connecting" => "◆",
                "queued" => "…",
                "complete" => "✓",
                "failed" => "✗",
                "cancelled" => "⨯",
                _ => "?",
            };
            let glyph_style = match d.state.as_str() {
                "complete" => ok,
                "failed" | "cancelled" => warn,
                _ => warn,
            };
            glyph_style.render(glyph).blit(buf, area.x + 2, y);

            style.render(&trunc(&d.title, 40)).blit(buf, area.x + 5, y);
            muted.render(&format!("{:<9}", d.backend)).blit(buf, area.x + 46, y);

            match live.filter(|p| p.total_bytes > 0) {
                Some(p) => {
                    // In-flight: percent bar in the size column, speed in the
                    // state column.
                    let pct = (p.bytes_done.min(p.total_bytes) as f64 / p.total_bytes as f64 * 100.0) as u16;
                    let bar_w = 10;
                    let filled = (bar_w * pct as usize / 100).min(bar_w);
                    let bar = format!("[{}{}] {:>3}%", "█".repeat(filled), " ".repeat(bar_w - filled), pct);
                    warn.render(&bar).blit(buf, area.x + 56, y);
                    muted.render(&format!("{:>9}/s", fmt_size(p.speed_bps))).blit(buf, area.x + 72, y);
                }
                None => {
                    let size = fmt_size(d.size);
                    muted.render(&format!("{:>7}", size)).blit(buf, area.x + 58, y);
                    muted.render(&format!("{:<11}", d.state)).blit(buf, area.x + 68, y);
                }
            }
        }

        if items.len() > rows {
            muted.render(&format!("+ {} more", items.len() - rows)).blit(buf, area.x, area.bottom() - 1);
        }
    }
}

impl Default for DownloadsPanel {
    fn default() -> Self { Self::new() }
}

fn sel_marker(theme: &Theme) -> BobaStyle { BobaStyle::new().fg(theme.palette.accent.to_rgb()) }

fn trunc(s: &str, w: usize) -> String {
    if s.chars().count() > w {
        format!("{}…", s.chars().take(w.saturating_sub(1)).collect::<String>())
    } else {
        s.to_string()
    }
}

fn fmt_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else {
        format!("{:.0}M", bytes as f64 / MB as f64)
    }
}
