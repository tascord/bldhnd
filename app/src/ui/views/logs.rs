use {
    crate::logs::log_store,
    bobatea::{components::style::BobaStyle, theme::Theme},
    ratatui::prelude::*,
};

pub struct LogsPanel;

impl LogsPanel {
    pub fn new() -> Self { Self }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let accent = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();
        let fg = BobaStyle::new().fg(theme.global_fg);
        let muted = BobaStyle::new().fg(theme.palette.fg_muted.to_rgb());

        accent.render("Logs").blit(buf, area.x, area.y);

        if area.height < 3 {
            return;
        }

        let entries = log_store().entries();

        if entries.is_empty() {
            muted.render("No logs yet.").blit(buf, area.x, area.y + 1);
            return;
        }

        // Show the most recent entries at the bottom.
        let rows = (area.height - 1) as usize;
        let start = entries.len().saturating_sub(rows);
        for (i, entry) in entries[start..].iter().enumerate() {
            let y = area.y + 1 + i as u16;
            fg.render(entry).blit(buf, area.x, y);
        }
    }
}

impl Default for LogsPanel {
    fn default() -> Self { Self::new() }
}
