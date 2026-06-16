use {
    crate::ui::components::input::Input,
    ratatui::{
        layout::{
            Constraint,
            Direction::{Horizontal, Vertical},
            Layout,
        },
        prelude::*,
        widgets::{Block, Widget, WidgetRef},
    },
};

pub struct DemoView {
    i: Input,
}

impl DemoView {
    pub fn new() -> Self {
        let this = Self { i: Input::new("Demo Input", "") };
        this.i.focus();
        this
    }
}

impl WidgetRef for DemoView {
    fn render_ref(&self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // Block::new().borders(ratatui::widgets::Borders::ALL).bg(Color::LightGreen).render(area, buf);

        let y = Layout::new(Vertical, [Constraint::Fill(1), Constraint::Length(3), Constraint::Fill(1)]).split(area);
        let x = Layout::new(Horizontal, [Constraint::Fill(1), Constraint::Length(16), Constraint::Fill(1)]).split(y[1]);

        let l = x[1];
        self.i.render_ref(l, buf);
    }
}
