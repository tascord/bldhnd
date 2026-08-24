use {
    super::home::BANNER_FONT,
    crate::logs::log_store,
    bobatea::{
        components::{list::List, style::BobaStyle},
        theme::Theme,
    },
    ratatui::prelude::*,
};

pub struct LogsPanel {
    banner: Vec<String>,
}

impl LogsPanel {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("logs").unwrap().to_string();

        Self { banner: text.lines().map(|l| l.to_string()).collect() }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let banner_text = self.banner.join("\n");
        let banner_style = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();

        let surf = banner_style.render(&banner_text);
        surf.blit(buf, area.x + 2, area.y);

        let surf_h = surf.height() as u16;
        let content_area = Rect {
            x: area.x + 2,
            y: area.y + surf_h + 1,
            width: area.width.saturating_sub(4),
            height: area.height.saturating_sub(surf_h + 2),
        };

        let log_entries = log_store().entries();
        if log_entries.is_empty() {
            let list = List::new(["No logs yet".to_string()]).without_border();
            list.render_to_buf(content_area, buf, theme);
        } else {
            let list = List::new(log_entries).without_border();
            list.render_to_buf(content_area, buf, theme);
        }
    }
}

impl Default for LogsPanel {
    fn default() -> Self { Self::new() }
}
