//! Simple stopwatch / timer component.
//!
//! ```rust
//! use bobatea::components::stopwatch::Stopwatch;
//! let mut sw = Stopwatch::new();
//! sw.start();
//! ```

use {
    crate::components::Component,
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
    std::time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopwatchState {
    Idle,
    Running,
    Paused,
}

/// A stopwatch component that displays elapsed time.
pub struct Stopwatch {
    state: Mutable<StopwatchState>,
    elapsed: Mutable<Duration>,
    last_tick: Mutable<Option<Instant>>,
    label: Mutable<String>,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            state: Mutable::new(StopwatchState::Idle),
            elapsed: Mutable::new(Duration::ZERO),
            last_tick: Mutable::new(None),
            label: Mutable::new(String::new()),
        }
    }

    pub fn with_label(self, label: impl Into<String>) -> Self {
        self.label.set(label.into());
        self
    }

    pub fn start(&self) {
        if *self.state.lock_ref() != StopwatchState::Running {
            self.state.set(StopwatchState::Running);
            self.last_tick.set(Some(Instant::now()));
        }
    }

    pub fn stop(&self) {
        if *self.state.lock_ref() == StopwatchState::Running {
            if let Some(last) = self.last_tick.get() {
                let additional = Instant::now().saturating_duration_since(last);
                let total = self.elapsed.get() + additional;
                self.elapsed.set(total);
            }
            self.state.set(StopwatchState::Paused);
            self.last_tick.set(None);
        }
    }

    pub fn reset(&self) {
        self.state.set(StopwatchState::Idle);
        self.elapsed.set(Duration::ZERO);
        self.last_tick.set(None);
    }

    pub fn toggle(&self) {
        match *self.state.lock_ref() {
            StopwatchState::Idle | StopwatchState::Paused => self.start(),
            StopwatchState::Running => self.stop(),
        }
    }

    fn current_elapsed(&self) -> Duration {
        let base = self.elapsed.get();
        if *self.state.lock_ref() == StopwatchState::Running {
            if let Some(last) = self.last_tick.get() { base + Instant::now().saturating_duration_since(last) } else { base }
        } else {
            base
        }
    }

    pub fn on_key(&self, code: KeyCode) {
        match code {
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle(),
            KeyCode::Char('r') => self.reset(),
            _ => {}
        }
    }

    pub fn tick(&self) {
        if *self.state.lock_ref() == StopwatchState::Running && self.last_tick.get().is_none() {
            self.last_tick.set(Some(Instant::now()));
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let elapsed = self.current_elapsed();
        let total_secs = elapsed.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        let millis = elapsed.subsec_millis();

        let text = format!("{:02}:{:02}.{:03}", mins, secs, millis);
        let label = self.label.get_cloned();
        let display = if label.is_empty() { text } else { format!("{} {}", text, label) };

        let style = match *self.state.lock_ref() {
            StopwatchState::Running => ratatui::style::Style::default()
                .fg(theme.palette.success.to_rgb())
                .add_modifier(ratatui::style::Modifier::BOLD),
            StopwatchState::Paused => ratatui::style::Style::default().fg(theme.palette.warning.to_rgb()),
            StopwatchState::Idle => ratatui::style::Style::default().fg(theme.palette.fg_subtle.to_rgb()),
        };

        Paragraph::new(Line::raw(display)).style(style).render(area, buf);
    }
}

impl Default for Stopwatch {
    fn default() -> Self { Self::new() }
}

impl Component for Stopwatch {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
