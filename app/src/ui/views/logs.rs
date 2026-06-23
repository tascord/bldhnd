use crate::logs;
use crate::ui::components::Focusable;
use crate::ui::components::scroll::Scroller;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, WidgetRef};
use std::sync::Arc;

pub struct LogsView {
    banner: Vec<String>,
    scroller: Arc<Scroller>,
}

impl LogsView {
    pub fn new() -> Self {
        let flet = figlet_rs::FIGlet::from_content(crate::ui::views::home::BANNER_FONT).unwrap();
        let text = flet.convert("logs").unwrap().to_string();

        let sc = logs::scroller();
        sc.focus();

        Self { banner: text.lines().map(|l| l.to_string()).collect(), scroller: sc }
    }
}

impl WidgetRef for LogsView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from_iter(self.banner.iter().map(|l| Line::from(Span::raw(l.clone()))));
        let layout =
            Layout::vertical([Constraint::Length(text.height() as u16), Constraint::Length(3), Constraint::Fill(1)])
                .split(area);

        // Title
        Paragraph::new(text).render(layout[0], buf);

        // Rule
        Paragraph::new(Text::from_iter([
            Line::raw(""),
            Line::styled(
                std::iter::repeat_n(' ', layout[1].width as usize).collect::<String>(),
                Style::new().add_modifier(Modifier::CROSSED_OUT),
            ),
            Line::raw(""),
        ]))
        .render(layout[1], buf);

        // Logs (scroller)
        self.scroller.render_ref(layout[2], buf);
    }
}
