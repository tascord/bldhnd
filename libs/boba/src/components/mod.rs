use {
    crossterm::event::Event,
    ratatui::{ prelude::Rect, Frame },
};

pub mod anim;
pub mod asciiimg;
pub mod badge;
pub mod bigtext;
pub mod block;
pub mod border;
pub mod button;
pub mod canvas;
pub mod effect;
pub mod filepicker;
pub mod form;
pub mod help;
pub mod input;
pub mod layer;
pub mod layout;
pub mod list;
pub mod modal;
pub mod paginator;
pub mod pattern;
pub mod powerline;
pub mod progress;
pub mod reactive;
pub mod spinner;
pub mod stopwatch;
pub mod style;
pub mod syntax;
pub mod table;
pub mod tabs;
pub mod textarea;
pub mod toast;
pub mod tree;
pub mod viewport;

use crate::theme::Theme;

pub trait Component {
    fn width(&self) -> Option<usize> { None }
    fn height(&self) -> Option<usize> { None }

    fn render(&mut self, ctx: &mut Frame<'_>, theme: &Theme);

    fn handle_event(&mut self, _area: Rect, _ev: &Event) {}

    fn wants_focus(&self) -> bool { false }

    fn id(&self) -> &str { "anonymous" }
}
