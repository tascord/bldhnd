//! Mouse hitbox tracking and responsive auto-layout.
//!
//! ```rust
//! use bobatea::components::layout::{MouseArea, ResponsiveLayout};
//! let mut zones = MouseArea::new();
//! zones.track("btn", ratatui::layout::Rect::new(0, 0, 10, 3));
//! ```

use {
    crossterm::event::MouseEvent,
    ratatui::layout::{Constraint, Direction, Layout, Rect},
    std::collections::HashMap,
};

/// Tracks screen rectangles for named areas.
#[derive(Debug, Default)]
pub struct MouseArea {
    zones: HashMap<String, Rect>,
}

/// A spatial mouse-event dispatcher.
///
/// Register interactive zones during `View::render` or `View::on_mouse`, then
/// call `dispatch` to forward events to the first hit handler.
///
/// ```rust
/// use bobatea::components::layout::MouseDispatcher;
/// use crossterm::event::MouseEvent;
///
/// let mut d = MouseDispatcher::new();
/// // d.track(my_rect, |ev| button.on_mouse(my_rect, ev));
/// ```
pub struct MouseDispatcher<'a> {
    zones: Vec<(Rect, Box<dyn Fn(&MouseEvent) + 'a>)>,
}

impl<'a> MouseDispatcher<'a> {
    pub fn new() -> Self { Self { zones: Vec::new() } }

    /// Register a zone with its handler.  Order matters — first match wins.
    pub fn track<F: Fn(&MouseEvent) + 'a>(&mut self, area: Rect, handler: F) { self.zones.push((area, Box::new(handler))); }

    /// Check if a point is inside any tracked zone.
    pub fn hit(&self, col: u16, row: u16) -> bool {
        self.zones.iter().any(|(r, _)| r.left() <= col && col < r.right() && r.top() <= row && row < r.bottom())
    }

    /// Dispatch the event to every handler whose zone contains themouse cursor.
    pub fn dispatch(&self, ev: &MouseEvent) {
        for (area, handler) in &self.zones {
            if ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom() {
                handler(ev);
            }
        }
    }

    /// Dispatch to the **first** matching zone only (useful when widgets
    /// overlap and you want top-most only).
    pub fn dispatch_top(&self, ev: &MouseEvent) {
        for (area, handler) in &self.zones {
            if ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom() {
                handler(ev);
                return;
            }
        }
    }
}

impl MouseArea {
    pub fn new() -> Self { Self::default() }

    /// Register a named zone.
    pub fn track(&mut self, name: impl Into<String>, rect: Rect) { self.zones.insert(name.into(), rect); }

    /// Check if a mouse event is inside a named zone.
    pub fn hit(&self, name: &str, ev: &MouseEvent) -> bool {
        self.zones
            .get(name)
            .map(|r| {
                let x = ev.column;
                let y = ev.row;
                x >= r.left() && x < r.right() && y >= r.top() && y < r.bottom()
            })
            .unwrap_or(false)
    }

    /// Return all current zones.
    pub fn zones(&self) -> &HashMap<String, Rect> { &self.zones }

    pub fn clear(&mut self) { self.zones.clear(); }
}

/// A responsive layout that recalculates constraints on resize.
///
/// ```rust
/// use bobatea::components::layout::ResponsiveLayout;
/// let mut layout = ResponsiveLayout::new();
/// layout.push(ratatui::layout::Constraint::Length(3));
/// ```
pub struct ResponsiveLayout {
    constraints: Vec<Constraint>,
    direction: Direction,
    margin: u16,
    spacing: u16,
}

impl ResponsiveLayout {
    pub fn new() -> Self { Self { constraints: Vec::new(), direction: Direction::Vertical, margin: 0, spacing: 0 } }

    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }

    pub fn margin(mut self, m: u16) -> Self {
        self.margin = m;
        self
    }

    pub fn spacing(mut self, s: u16) -> Self {
        self.spacing = s;
        self
    }

    pub fn push(&mut self, c: Constraint) { self.constraints.push(c); }

    pub fn lengths<const N: usize>(self, constraints: [Constraint; N]) -> Self {
        let mut me = self;
        for c in &constraints {
            me.constraints.push(*c);
        }
        me
    }

    pub fn split(&self, area: Rect) -> Vec<Rect> {
        let mut l = Layout::new(self.direction, self.constraints.clone()).margin(self.margin);
        if self.spacing > 0 {
            l = l.spacing(self.spacing);
        }
        l.split(area).to_vec()
    }

    /// Quick horizontal split.
    pub fn horizontal(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
        Layout::horizontal(constraints).split(area).to_vec()
    }

    /// Quick vertical split.
    pub fn vertical(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
        Layout::vertical(constraints).split(area).to_vec()
    }

    /// Flex-wrap: split items into rows that fit the width.
    pub fn flex_row(&self, area: Rect, item_width: u16, item_height: u16) -> Vec<Rect> {
        let mut rects = vec![];
        let cols = area.width / item_width.max(1);
        let rows = area.height / item_height.max(1);
        for row in 0..rows {
            for col in 0..cols {
                let x = area.x + col * item_width;
                let y = area.y + row * item_height;
                if x + item_width <= area.right() && y + item_height <= area.bottom() {
                    rects.push(Rect { x, y, width: item_width, height: item_height });
                }
            }
        }
        rects
    }
}

use ratatui::layout::Constraint as C;

/// Shorthand constraint helpers.
pub fn fixed(n: u16) -> C { C::Length(n) }
pub fn fill(n: u16) -> C { C::Fill(n) }
pub fn min(n: u16) -> C { C::Min(n) }
pub fn max(n: u16) -> C { C::Max(n) }
pub fn percent(n: u16) -> C { C::Percentage(n) }
pub fn ratio(n: u32, d: u32) -> C { C::Ratio(n, d) }
