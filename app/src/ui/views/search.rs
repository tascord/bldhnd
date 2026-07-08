use {
    bobatea::{
        components::{input::Input, list::List, style::BobaStyle},
        theme::Theme,
    },
    futures_signals::signal::Mutable,
    ratatui::{prelude::*, text::Span},
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
}

pub struct SearchPanel {
    banner: Vec<String>,
    input: Input,
    search_type: Mutable<usize>,
}

impl SearchPanel {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("search").unwrap().to_string();

        Self {
            banner: text.lines().map(|l| l.to_string()).collect(),
            input: Input::new("Search"),
            search_type: Mutable::new(0),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let banner_text = self.banner.join("\n");
        let banner_style = BobaStyle::new().fg(theme.palette.accent.to_rgb()).bold();

        let surf = banner_style.render(&banner_text);
        let surf_h = surf.height() as u16;

        let input_area =
            Rect { x: area.x + 2, y: area.y + surf_h + 2, width: area.width.saturating_sub(4).min(40), height: 3 };

        let radio_area = Rect { x: area.x + 2, y: input_area.y + 5, width: area.width.saturating_sub(4).min(40), height: 3 };

        surf.blit(buf, area.x + 2, area.y);

        self.input.render_to_buf(input_area, buf, theme);

        let search_type_list = List::new(SearchType::list());
        search_type_list.render_to_buf(radio_area, buf, theme);
    }
}

impl Default for SearchPanel {
    fn default() -> Self { Self::new() }
}
