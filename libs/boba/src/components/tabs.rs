//! Tab bar component with reactive binding and callbacks.
//!
//! ```rust
//! use boba::components::tabs::Tabs;
//! use futures_signals::signal::Mutable;
//!
//! let active = Mutable::new(0);
//! let tabs = Tabs::new(["Home", "Settings", "About"])
//!     .active(&active)
//!     .on_change(|idx| println!("Tab {} selected", idx));
//! ```

use {
    crate::{
        components::{block::BobaBlock, Component},
        events::SubscriptionPriority,
        surface::{Cell, Surface},
        theme::Theme,
    },
    crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Style,
        text::Line,
        widgets::{Paragraph, Widget},
    },
    std::sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Default,
    Hovered,
    Selected,
    Disabled,
}

#[derive(Debug, Clone)]
pub enum TabsEvent {
    Select(usize),
    TabHover(usize),
    TabLeave,
}

/// A tab bar component with reactive binding and callbacks.
///
/// ```rust
/// use boba::components::tabs::Tabs;
/// use futures_signals::signal::Mutable;
///
/// let active = Mutable::new(0);
/// let tabs = Tabs::new(["Home", "Settings", "About"])
///     .active(&active);
/// ```
pub struct TabStates {
    states: Vec<Mutable<TabState>>,
    disabled: Vec<bool>,
}

impl TabStates {
    fn new(count: usize) -> Self {
        Self {
            states: (0..count).map(|_| Mutable::new(TabState::Default)).collect(),
            disabled: vec![false; count],
        }
    }

    fn get(&self, idx: usize) -> TabState {
        self.states.get(idx).map(|s| *s.lock_ref()).unwrap_or(TabState::Default)
    }

    fn set(&self, idx: usize, state: TabState) {
        if let Some(s) = self.states.get(idx) {
            s.set(state);
        }
    }
}

/// A tab bar component.
///
/// ```rust
/// use boba::components::tabs::Tabs;
/// let mut tabs = Tabs::new(["Home", "Settings", "About"]);
/// ```
pub struct Tabs {
    labels: Vec<String>,
    active: Mutable<usize>,
    tab_states: TabStates,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    ev: crate::events::EventTarget<TabsEvent>,
    id: String,
}

impl Clone for Tabs {
    fn clone(&self) -> Self {
        Self {
            labels: self.labels.clone(),
            active: self.active.clone(),
            tab_states: TabStates::new(self.labels.len()),
            on_change: self.on_change.clone(),
            ev: self.ev.clone(),
            id: format!("{}-clone", self.id),
        }
    }
}

impl Tabs {
    pub fn new(labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let labels: Vec<String> = labels.into_iter().map(|s| s.into()).collect();
        Self {
            labels: labels.clone(),
            active: Mutable::new(0),
            tab_states: TabStates::new(labels.len()),
            on_change: None,
            ev: crate::events::EventTarget::new("tabs"),
            id: "tabs".to_string(),
        }
    }

    pub fn id(&self) -> &str { &self.id }

    pub fn id_as(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn active(mut self, idx: &Mutable<usize>) -> Self {
        self.active = idx.clone();
        self
    }

    pub fn active_signal(&self) -> &Mutable<usize> { &self.active }

    pub fn on_change(self, f: impl Fn(usize) + Send + Sync + 'static) -> Self {
        let f = Arc::new(f);
        let mut me = self;
        me.on_change = Some(f.clone());
        me.ev.on(SubscriptionPriority::Low, move |ev| {
            if let TabsEvent::Select(idx) = **ev {
                f(idx);
            }
        }).forget();
        me
    }

    pub fn current(&self) -> usize { *self.active.lock_ref() }

    pub fn select(&self, idx: usize) {
        let idx = idx.min(self.labels.len().saturating_sub(1));
        self.active.set(idx);
        for i in 0..self.labels.len() {
            self.tab_states.set(i, if i == idx { TabState::Selected } else { TabState::Default });
        }
        self.ev.emit(TabsEvent::Select(idx));
    }

    pub fn disable_tab(&mut self, idx: usize) {
        self.tab_states.disable(idx);
    }

    pub fn enable_tab(&mut self, idx: usize) {
        self.tab_states.enable(idx);
    }

    pub fn on_key(&self, code: KeyCode) {
        let len = self.labels.len();
        if len == 0 {
            return;
        }
        let mut cur = self.active.get();
        match code {
            KeyCode::Left => cur = cur.saturating_sub(1),
            KeyCode::Right => cur = (cur + 1).min(len - 1),
            _ => return,
        }
        self.select(cur);
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        if ev.column < area.left() || ev.column >= area.right() || ev.row < area.top() || ev.row >= area.bottom() {
            if matches!(ev.kind, MouseEventKind::Moved) {
                let any_hovered = (0..self.labels.len()).any(|i| self.tab_states.get(i) == TabState::Hovered);
                if any_hovered {
                    for i in 0..self.labels.len() {
                        if self.tab_states.get(i) == TabState::Hovered {
                            self.tab_states.set(i, TabState::Default);
                        }
                    }
                    self.ev.emit(TabsEvent::TabLeave);
                }
            }
            return;
        }

        let tab_w = area.width as usize / self.labels.len().max(1);
        let hovered_idx = ((ev.column - area.left()) as usize / tab_w.max(1)).min(self.labels.len().saturating_sub(1));

        match ev.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                if !self.tab_states.is_disabled(hovered_idx) {
                    self.select(hovered_idx);
                }
            }
            MouseEventKind::Moved => {
                for i in 0..self.labels.len() {
                    let new_state = if i == hovered_idx && !self.tab_states.is_disabled(i) {
                        TabState::Hovered
                    } else if i == self.active.get() {
                        TabState::Selected
                    } else {
                        TabState::Default
                    };
                    let old_state = self.tab_states.get(i);
                    if old_state != new_state {
                        self.tab_states.set(i, new_state.clone());
                        if new_state == TabState::Hovered {
                            self.ev.emit(TabsEvent::TabHover(i));
                        } else if old_state == TabState::Hovered {
                            self.ev.emit(TabsEvent::TabLeave);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_event(&mut self, area: Rect, ev: &Event) {
        match ev {
            Event::Key(KeyEvent { code, .. }) => self.on_key(*code),
            Event::Mouse(mouse) => self.on_mouse(area, mouse),
            _ => {}
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        let fg = theme.global_fg;
        let accent = theme.palette.accent.to_rgb();
        let muted = theme.palette.fg_muted.to_rgb();

        let items: Vec<Line> = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let state = self.tab_states.get(i);
                let (text, style) = match state {
                    TabState::Disabled => (
                        format!("│ {} ", label),
                        ratatui::style::Style::default().fg(muted),
                    ),
                    TabState::Hovered => (
                        format!("│ {} ", label),
                        ratatui::style::Style::default().fg(fg).add_modifier(ratatui::style::Modifier::REVERSED),
                    ),
                    TabState::Selected => (
                        format!("│ {} ", label),
                        ratatui::style::Style::default().fg(accent).add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    TabState::Default => (
                        format!("│ {} ", label),
                        ratatui::style::Style::default().fg(fg),
                    ),
                };
                Line::styled(text, style)
            })
            .collect();

        let block = BobaBlock::new().horizontal().border_style(ratatui::style::Style::default().fg(theme.border_subtle));

        Paragraph::new(items).style(ratatui::style::Style::default().fg(fg)).block(block.into()).render(area, buf);
    }

    pub fn render_to_surface(&self) -> Surface {
        let label_padding = 1;

        let mut line1 = Vec::new();
        let mut line2 = Vec::new();
        let mut line3 = Vec::new();
        let mut rows: Vec<Vec<Cell>> = Vec::new();

        for (i, label) in self.labels.iter().enumerate() {
            let state = self.tab_states.get(i);
            let (tab_style, active_style) = match state {
                TabState::Disabled => (
                    Style::default().fg(ratatui::style::Color::Gray),
                    Style::default().fg(ratatui::style::Color::Gray),
                ),
                TabState::Hovered => (
                    Style::default().fg(ratatui::style::Color::White),
                    Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::REVERSED),
                ),
                TabState::Selected => (
                    Style::default().fg(ratatui::style::Color::White),
                    Style::default().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD),
                ),
                TabState::Default => (
                    Style::default().fg(ratatui::style::Color::White),
                    Style::default().fg(ratatui::style::Color::White),
                ),
            };

            let is_active = *self.active.lock_ref() == i;
            let label_w = label.len() + label_padding * 2;
            let top_edge = '─';
            let bottom_edge = if is_active { ' ' } else { '─' };
            let bottom_left = if is_active { '┘' } else { '┴' };
            let bottom_right = if is_active { '└' } else { '┴' };

            line1.push(Cell::new("╭".to_string(), tab_style));
            for _ in 0..label_w {
                line1.push(Cell::new(top_edge.to_string(), tab_style));
            }
            line1.push(Cell::new("╮".to_string(), tab_style));

            line2.push(Cell::new("│".to_string(), if is_active { active_style } else { tab_style }));
            for _ in 0..label_padding {
                line2.push(Cell::new(" ".to_string(), if is_active { active_style } else { tab_style }));
            }
            for ch in label.chars() {
                line2.push(Cell::new(ch.to_string(), if is_active { active_style } else { tab_style }));
            }
            for _ in 0..label_padding {
                line2.push(Cell::new(" ".to_string(), if is_active { active_style } else { tab_style }));
            }
            line2.push(Cell::new("│".to_string(), if is_active { active_style } else { tab_style }));

            line3.push(Cell::new(bottom_left.to_string(), tab_style));
            for _ in 0..label_w {
                line3.push(Cell::new(bottom_edge.to_string(), tab_style));
            }
            line3.push(Cell::new(bottom_right.to_string(), tab_style));
        }

        rows.push(line1);
        rows.push(line2);
        rows.push(line3);

        let mut surface = Surface { rows };

        let current_width = surface.cell_count_width();
        let gap = 96usize.saturating_sub(current_width);
        if gap > 0 {
            let gap_line1 = (0..gap).map(|_| Cell::new("─", Style::default())).collect::<Vec<_>>();
            let gap_line2 = (0..gap).map(|_| Cell::new(" ", Style::default())).collect::<Vec<_>>();
            surface.rows[0].extend_from_slice(&gap_line1);
            surface.rows[1].extend_from_slice(&gap_line2);
            surface.rows[2].extend_from_slice(&gap_line2);
        }

        surface
    }
}

impl Component for Tabs {
    fn height(&self) -> Option<usize> { Some(3) }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }

    fn handle_event(&mut self, area: Rect, ev: &Event) {
        self.handle_event(area, ev);
    }

    fn wants_focus(&self) -> bool { true }

    fn id(&self) -> &str { &self.id }
}

impl TabStates {
    fn disable(&mut self, idx: usize) {
        if let Some(d) = self.disabled.get_mut(idx) {
            *d = true;
        }
        self.set(idx, TabState::Disabled);
    }

    fn enable(&mut self, idx: usize) {
        if let Some(d) = self.disabled.get_mut(idx) {
            *d = false;
        }
        self.set(idx, TabState::Default);
    }

    fn is_disabled(&self, idx: usize) -> bool {
        self.disabled.get(idx).copied().unwrap_or(false)
    }
}