//! Re-exports of all UI building blocks.
//!
//! ## Component overview
//!
//! | Component | Description |
//! |---|---|
//! | [`asciiimg`] | Pixel-art image renderer |
//! | [`badge`] | Small inline status badge |
//! | [`bigtext`] | FIGlet-style large text |
//! | [`block`] | Titled bordered container (BobaBlock) |
//! | [`border`] | Customizable box-drawing border |
//! | [`button`] | Styled button with variants and callbacks |
//! | [`canvas`] | Low-level pixel drawing surface |
//! | [`effect`] | Post-processing effects (scanlines, glow, etc.) |
//! | [`filepicker`] | File/directory browser |
//! | [`form`] | Tab-navigable form with labeled fields |
//! | [`help`] | Keyboard shortcut legend |
//! | [`input`] | Single-line text input |
//! | [`layer`] | Layer stack compositor |
//! | [`layout`] | Flex-like layout helpers + mouse dispatcher |
//! | [`list`] | Scrollable selectable list |
//! | [`modal`] | Modal dialog overlay |
//! | [`paginator`] | Dot-indicator page selector |
//! | [`progress`] | Horizontal progress bar |
//! | [`spinner`] | Animated loading spinner |
//! | [`stopwatch`] | Elapsed-time display |
//! | [`syntax`] | Syntax-highlighted code block |
//! | [`table`] | Column-aligned table with selection |
//! | [`tabs`] | Horizontal tab bar |
//! | [`textarea`] | Multi-line text editor |
//! | [`toast`] | Toast notification pop-up |
//! | [`tree`] | Collapsible tree navigator |
//! | [`viewport`] | Scrollable viewport into a larger canvas |
//!
//! See the [`Component`] trait for the interface all interactive widgets implement.

use {
    crate::theme::Theme,
    crossterm::event::{Event, MouseEvent},
    ratatui::{Frame, prelude::Rect},
};

/// The core interface for an interactive UI component.
///
/// Implement this trait to create custom widgets that can be mounted into
/// a boba [`App`][crate::App] and receive keyboard/mouse events.
///
/// # Example
///
/// ```
/// use bobatea::components::Component;
/// use bobatea::theme::Theme;
/// use ratatui::{Frame, prelude::Rect};
///
/// struct MyWidget { value: i32 }
/// impl Component for MyWidget {
///     fn render(&mut self, ctx: &mut Frame<'_>, _theme: &Theme) {
///         ctx.buffer_mut().reset();
///     }
/// }
/// ```
pub trait Component {
    /// Preferred content width in columns, used for layout hints.
    ///
    /// Returning `None` means "let the parent decide."
    fn width(&self) -> Option<usize> { None }

    /// Preferred content height in rows, used for layout hints.
    ///
    /// Returning `None` means "let the parent decide."
    fn height(&self) -> Option<usize> { None }

    /// Render the component into the current [`ratatui::Frame`] area.
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &Theme);

    /// Handle a terminal event directed at this component's area.
    ///
    /// Components that are keyboard-interactive override this to dispatch
    /// to their internal handlers.
    fn handle_event(&mut self, _area: Rect, _ev: &Event) {}

    /// Handle a mouse event within the component's area.
    ///
    /// Default implementation is a no-op (component is not mouse-interactive).
    fn on_mouse(&self, _area: Rect, _ev: &MouseEvent) {}

    /// Whether this component should receive keyboard focus automatically.
    fn wants_focus(&self) -> bool { false }

    /// Stable identifier for this component instance.
    ///
    /// Used for focus management and event routing. Default is `"anonymous"`.
    fn id(&self) -> &str { "anonymous" }
}

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
