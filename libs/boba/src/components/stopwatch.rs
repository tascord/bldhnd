use {crate::components::Component, ratatui::Frame, std::time::Instant};

pub struct Stopwatch(Instant);

impl Component for Stopwatch {
    fn render(&mut self, _ctx: &mut Frame<'_>, _theme: &crate::theme::Theme) {}
}
