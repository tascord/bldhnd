use {
    crate::ui::{
        components::{rainbow, rainbow::Rainbow},
        views::vstack,
    },
    rand::RngExt,
    ratatui::{
        layout::{Constraint, HorizontalAlignment::Center},
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
};

pub struct HomeView {
    grad: Rainbow,
    banner: (Vec<String>, String),
}

pub static SPLASHES: &str = include_str!("../../../../_assets/splash.txt");
pub static BANNER_FONT: &str = include_str!("../../../../_assets/Pagga.tlf");

#[allow(clippy::new_without_default)]
impl HomeView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("bldhnd").unwrap().to_string();

        Self {
            grad: Rainbow::new(),
            banner: (text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), {
                let shs = SPLASHES.lines().collect::<Vec<_>>();
                shs[rand::rng().random_range(0..shs.len())].to_string()
            }),
        }
    }
}

impl WidgetRef for HomeView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text = self
            .grad
            .apply(&Text::from(self.banner.0.iter().map(|l| Line::from(Span::raw(l.clone()))).collect::<Vec<_>>()));

        let w = text.width();
        let h = text.height();

        let inner = area.centered(Constraint::Length(w.max(self.banner.1.len()) as u16), Constraint::Fill(1));
        let layout = vstack(&[h as u16, 1], inner);

        // Figlet Banner
        Paragraph::new(text).alignment(HorizontalAlignment::Center).render(layout[0], buf);

        // Splash Text
        Paragraph::new(self.banner.1.clone()).alignment(Center).render(layout[1], buf);
    }
}
