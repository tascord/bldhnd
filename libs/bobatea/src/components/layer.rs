//! Layer compositor — stack widgets with optional screen-space effects.
//!
//! ```rust,ignore
//! use bobatea::components::{
//!     layer::{Layer, LayerStack},
//!     button::Button,
//! };
//!
//! let mut stack = LayerStack::new();
//! stack.push(Layer::new(|_area, _buf| {
//!     Button::new("Click me");
//! }));
//! ```

use {
    crate::{
        components::{Component, effect::ScreenEffect},
        surface::Surface,
    },
    futures_signals::signal::Mutable,
    ratatui::prelude::{Buffer, Frame, Rect},
};

/// A single composited layer.
pub struct Layer {
    render: Box<dyn FnMut(Rect, &mut Buffer)>,
    effect: Option<Box<dyn ScreenEffect>>,
    visible: Mutable<bool>,
    opacity: Mutable<f64>,
    x_offset: u16,
    y_offset: u16,
}

impl Layer {
    /// Create a layer from a render closure.
    pub fn new<F: FnMut(Rect, &mut Buffer) + 'static>(f: F) -> Self {
        Self {
            render: Box::new(f),
            effect: None,
            visible: Mutable::new(true),
            opacity: Mutable::new(1.0),
            x_offset: 0,
            y_offset: 0,
        }
    }

    /// Attach a post-processing effect.
    pub fn with_effect(mut self, effect: impl ScreenEffect + 'static) -> Self {
        self.effect = Some(Box::new(effect));
        self
    }

    /// Set absolute X offset (relative to the full frame area).
    pub fn x(mut self, v: u16) -> Self {
        self.x_offset = v;
        self
    }

    /// Set absolute Y offset (relative to the full frame area).
    pub fn y(mut self, v: u16) -> Self {
        self.y_offset = v;
        self
    }

    pub fn set_visible(&self, v: bool) { self.visible.set(v); }

    pub fn set_opacity(&self, v: f64) { self.opacity.set(v.clamp(0.0, 1.0)); }
}

/// A stack of layers rendered back-to-front.
pub struct LayerStack {
    layers: Vec<Layer>,
}

impl LayerStack {
    pub fn new() -> Self { Self { layers: Vec::new() } }

    pub fn push(&mut self, layer: Layer) { self.layers.push(layer); }

    pub fn pop(&mut self) -> Option<Layer> { self.layers.pop() }

    pub fn len(&self) -> usize { self.layers.len() }

    pub fn is_empty(&self) -> bool { self.layers.is_empty() }

    /// Render all visible layers into the given area.
    pub fn render_to_buf(&mut self, area: Rect, buf: &mut Buffer, t: f64) {
        for layer in &mut self.layers {
            if !layer.visible.get() {
                continue;
            }
            let layer_area = Rect {
                x: area.x + layer.x_offset,
                y: area.y + layer.y_offset,
                width: area.width.saturating_sub(layer.x_offset),
                height: area.height.saturating_sub(layer.y_offset),
            };
            if layer_area.width == 0 || layer_area.height == 0 {
                continue;
            }
            let mut scratch = Buffer::empty(layer_area);
            scratch.merge(buf);
            (layer.render)(layer_area, &mut scratch);
            if let Some(fx) = &layer.effect {
                fx.apply(layer_area, &mut scratch, t);
            }
            buf.merge(&scratch);
        }
    }
}

impl Component for LayerStack {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        let buf = ctx.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }
        self.render_to_buf(area, buf, 0.0);
    }
}

// ── Surface-based Compositor (lipgloss-style) ──

/// A composited layer backed by a pre-rendered [`Surface`].
#[derive(Debug, Clone)]
pub struct CompositorLayer {
    pub surface: Surface,
    pub x: u16,
    pub y: u16,
    pub visible: bool,
}

impl CompositorLayer {
    pub fn new(surface: Surface) -> Self { Self { surface, x: 0, y: 0, visible: true } }

    /// Set absolute X offset (relative to the full frame area).
    pub fn x(mut self, v: u16) -> Self {
        self.x = v;
        self
    }

    /// Set absolute Y offset (relative to the full frame area).
    pub fn y(mut self, v: u16) -> Self {
        self.y = v;
        self
    }
}

/// A simple back-to-front compositor that blits [`CompositorLayer`]s onto
/// a [`ratatui::prelude::Buffer`].
///
/// ```rust,ignore
/// use bobatea::components::layer::{Compositor, CompositorLayer};
/// # use bobatea::surface::Surface;
/// let comp = Compositor::new(vec![
///     CompositorLayer::new(Surface::new(10, 10, &Default::default())),
///     CompositorLayer::new(Surface::new(5, 5, &Default::default())).x(10).y(5),
/// ]);
/// ```
pub struct Compositor {
    layers: Vec<CompositorLayer>,
}

impl Compositor {
    pub fn new(layers: Vec<CompositorLayer>) -> Self { Self { layers } }

    pub fn push(&mut self, layer: CompositorLayer) { self.layers.push(layer); }

    /// Blit all visible layers onto the destination buffer.
    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer) {
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            layer.surface.blit(buf, area.x + layer.x, area.y + layer.y);
        }
    }
}
