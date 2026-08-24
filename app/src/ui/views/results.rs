use {
    crate::data::SearchResult,
    bobatea::{
        components::{list::List, style::BobaStyle},
        theme::Theme,
    },
    ratatui::prelude::*,
};

pub const BANNER_FONT: &str = include_str!("../../../../_assets/Pagga.tlf");

#[derive(Clone)]
pub struct ResultsPanel {
    banner: Vec<String>,
    items: Vec<SearchResult>,
}

impl ResultsPanel {
    pub fn new(items: Vec<SearchResult>) -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("results").unwrap().to_string();

        Self { banner: text.lines().map(|l| l.to_string()).collect(), items }
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

        let item_lines: Vec<String> =
            self.items.iter().map(|r| format!("{} - {} ({:.1}Gb)", r.name, r.ty_fmt(), r.size_gb)).collect();

        if item_lines.is_empty() {
            return;
        }

        let list = List::new(item_lines);
        list.render_to_buf(content_area, buf, theme);
    }
}
