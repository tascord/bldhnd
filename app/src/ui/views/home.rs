use {
    bobatea::{components::style::BobaStyle, theme::Theme},
    rand::RngExt,
    ratatui::{prelude::*, text::Span},
};

pub const BANNER_FONT: &str = include_str!("../../../../_assets/Pagga.tlf");
pub const SPLASHES: &str = include_str!("../../../../_assets/splash.txt");

pub struct HomePanel {
    banner: Vec<String>,
    splash: String,
}

impl HomePanel {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("bldhnd").unwrap().to_string();

        let shs = SPLASHES.lines().collect::<Vec<_>>();
        let splash = shs[rand::rng().random_range(0..shs.len())].to_string();

        Self { banner: text.lines().map(|l| l.to_string()).collect(), splash }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let banner_text = self.banner.join("\n");
        let banner_style = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();

        let surf = banner_style.render(&banner_text);
        let surf_w = surf.columns() as u16;
        let surf_h = surf.height() as u16;

        let content_width = self.splash.len().max(surf_w as usize) as u16;
        let content_area = Rect {
            x: area.x + (area.width.saturating_sub(content_width + 4)) / 2,
            y: area.y + (area.height.saturating_sub(surf_h + 3)) / 2,
            width: content_width + 4,
            height: surf_h + 3,
        };

        let banner_area = Rect { x: content_area.x + 2, y: content_area.y, width: surf_w, height: surf_h };

        surf.blit(buf, banner_area.x, banner_area.y);

        let splash_style = BobaStyle::new().fg(theme.global_fg).dim();

        let splash_surf = splash_style.render(&self.splash);
        let splash_area =
            Rect { x: content_area.x + 2, y: banner_area.y + surf_h + 1, width: self.splash.len() as u16, height: 1 };
        splash_surf.blit(buf, splash_area.x, splash_area.y);
    }
}

impl Default for HomePanel {
    fn default() -> Self { Self::new() }
}
