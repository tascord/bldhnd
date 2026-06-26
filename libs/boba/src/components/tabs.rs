use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
};

#[derive(Debug, Clone)]
pub enum TabsEvent {
    Select(usize),
}

/// A tab bar component.
///
/// ```rust
/// use boba::components::tabs::Tabs;
/// let tabs = Tabs::new(["Home", "Settings", "About"]);
/// ```
pub struct Tabs {
    labels: Vec<String>,
    current: Mutable<usize>,
    ev: crate::events::EventTarget<TabsEvent>,
}

impl Clone for Tabs {
    fn clone(&self) -> Self { Self { labels: self.labels.clone(), current: self.current.clone(), ev: self.ev.clone() } }
}

impl Tabs {
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            labels: labels.into_iter().map(|s| s.into()).collect(),
            current: Mutable::new(0),
            ev: crate::events::EventTarget::new("component"),
        }
    }

    pub fn current(&self) -> usize { self.current.get() }

    pub fn on_key(&self, code: KeyCode) {
        let len = self.labels.len();
        if len == 0 {
            return;
        }
        let mut cur = self.current.get();
        match code {
            KeyCode::Left => cur = cur.saturating_sub(1),
            KeyCode::Right => cur = (cur + 1).min(len - 1),
            _ => return,
        }
        self.current.set(cur);
        self.ev.emit(TabsEvent::Select(cur));
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        if ev.kind != MouseEventKind::Down(crossterm::event::MouseButton::Left) {
            return;
        }
        if ev.column < area.left() || ev.column >= area.right() || ev.row < area.top() || ev.row >= area.bottom() {
            return;
        }

        // evenly divide width across tabs
        let tab_w = area.width as usize / self.labels.len().max(1);
        let idx = ((ev.column - area.left()) as usize / tab_w.max(1)).min(self.labels.len().saturating_sub(1));
        self.current.set(idx);
        self.ev.emit(TabsEvent::Select(idx));
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        let cur = self.current.get();
        let fg = theme.global_fg;
        let accent = theme.palette.accent.to_rgb();

        let items: Vec<Line> = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let text = if i == cur { format!("│ {} ", label) } else { format!("│ {} ", label) };
                let style = if i == cur {
                    ratatui::style::Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    ratatui::style::Style::default().fg(fg)
                };
                Line::styled(text, style)
            })
            .collect();

        let block = BobaBlock::new().horizontal().border_style(ratatui::style::Style::default().fg(theme.border_subtle));

        Paragraph::new(items).style(ratatui::style::Style::default().fg(fg)).block(block.into()).render(area, buf);
    }
}

impl Component for Tabs {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
