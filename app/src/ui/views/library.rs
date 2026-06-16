use std::sync::Arc;

use crate::{events::SubscriptionPriority, ui::components::{Focusable, InputEvent, scroll::{ScrollText, Scroller}}};

use {
    crate::ui::views::home::BANNER_FONT,
    ratatui::{
        layout::Constraint,
        prelude::*,
        widgets::{Paragraph, WidgetRef},
    },
};

pub struct LibraryView {
    banner: Vec<String>,
    scroller: Arc<Scroller>
}

#[allow(clippy::new_without_default)]
impl LibraryView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(BANNER_FONT).unwrap();
        let text = flet.convert("library").unwrap().to_string();

        let placeholder_text = std::iter::repeat_n(ScrollText::new(Text::from_iter([Line::raw("foo"), Line::raw("bar")])), 10).collect::<Vec<_>>();
        let this = Self { banner: text.lines().map(|l| l.to_string()).collect::<Vec<_>>(), scroller: Scroller::new(placeholder_text).into() };

        this.scroller.focus();
        this
    }
}

impl WidgetRef for LibraryView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));

        let layout =
            Layout::vertical([Constraint::Length(text.height() as u16), Constraint::Length(3), Constraint::Fill(1)])
                .split(area);

        // Title
        Paragraph::new(text).render(layout[0], buf);

        // Rule
        Paragraph::new(Text::from_iter([
            Line::raw(""),
            Line::styled(std::iter::repeat_n(' ', layout[1].width as usize).collect::<String>(), Style::new().add_modifier(Modifier::CROSSED_OUT)),
            Line::raw(""),
        ]))
        .render(layout[1], buf);


        // Library
        self.scroller.render_ref(layout[2], buf);
    }
}
