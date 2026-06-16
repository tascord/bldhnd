use {
    crate::ui::{
        components::{input::Input, radio::Radio},
        views::{hcenter, home::BANNER_FONT, hstack, vcenter, vstack},
    },
    crossterm::terminal,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
};

pub struct SearchView {
    banner: Vec<String>,
    input: Input,
    radio: Radio,
}

#[allow(clippy::new_without_default)]
impl SearchView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("search").unwrap().to_string();

        let this = Self {
            banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(),
            input: Input::new("", ""),
            radio: Radio::new(["Music", "Movie", "Series"]),
        };

        this.input.focus();
        this.radio.focus();

        this
    }
}

impl WidgetRef for SearchView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));

        let w = text.width();
        let h = text.height();

        let inner = area.centered(Constraint::Length((w as u16).max(area.width.min(32))), Constraint::Fill(1));
        let layout = vstack(&[h as u16, 3, 5], inner);

        // Figlet Banner
        Paragraph::new(text).alignment(HorizontalAlignment::Center).render(layout[0], buf);

        // Search Bar
        self.input.render_ref(layout[1], buf);

        // Radio
        self.radio.render_ref(hcenter(w as u16, layout[2]), buf);
    }
}
