use {
    crate::config,
    bobatea::{
        components::{list::List, style::BobaStyle},
        theme::Theme,
    },
    ratatui::prelude::*,
};

pub const BANNER_FONT: &str = include_str!("../../../../_assets/Pagga.tlf");

pub struct SettingsPanel {
    banner: Vec<String>,
    volumes: Vec<String>,
}

impl SettingsPanel {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("settings").unwrap().to_string();

        let cfg = config().get_cloned();
        let volumes: Vec<String> = cfg
            .volumes
            .iter()
            .enumerate()
            .map(|(i, v)| format!("[{}] {} ({}) - {:?}", i + 1, v.name, v.path, v.max_size_gb))
            .collect();

        Self { banner: text.lines().map(|l| l.to_string()).collect(), volumes }
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

        let mut all_items = vec!["Volumes:".to_string()];
        all_items.extend(self.volumes.clone());
        if self.volumes.is_empty() {
            all_items.push("(No volumes configured)".to_string());
        }

        let list = List::new(all_items).without_border();
        list.render_to_buf(content_area, buf, theme);
    }
}

impl Default for SettingsPanel {
    fn default() -> Self { Self::new() }
}
