//! Tree view — collapsible hierarchical list.
//!
//! ```rust
//! use boba::components::tree::{Tree, TreeNode};
//! let tree = Tree::new("root", vec![
//!     TreeNode::leaf("a.txt"),
//!     TreeNode::branch("src", vec![TreeNode::leaf("main.rs")]),
//! ]);
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::KeyCode,
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
};

pub struct TreeNode {
    label: String,
    children: Vec<TreeNode>,
    expanded: Mutable<bool>,
    is_leaf: bool,
    depth: usize,
}

impl TreeNode {
    pub fn leaf(label: impl Into<String>) -> Self {
        Self { label: label.into(), children: vec![], expanded: Mutable::new(false), is_leaf: true, depth: 0 }
    }

    pub fn branch(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        let mut me = Self { label: label.into(), children, expanded: Mutable::new(true), is_leaf: false, depth: 0 };
        for child in &mut me.children {
            child.depth = me.depth + 1;
        }
        me
    }

    fn compute_depths(&mut self, depth: usize) {
        self.depth = depth;
        for child in &mut self.children {
            child.compute_depths(depth + 1);
        }
    }

    fn toggle(&self) {
        if !self.is_leaf {
            self.expanded.set(!self.expanded.get());
        }
    }

    fn flatten(&self, lines: &mut Vec<(usize, String, bool, bool)>) {
        lines.push((self.depth, self.label.clone(), self.is_leaf, self.expanded.get()));
        if self.expanded.get() {
            for child in &self.children {
                child.flatten(lines);
            }
        }
    }
}

pub struct Tree {
    root: TreeNode,
    selection: Mutable<usize>,
    focused: Mutable<bool>,
    scroll: Mutable<usize>,
}

impl Tree {
    pub fn new(label: impl Into<String>, children: Vec<TreeNode>) -> Self {
        let mut root = TreeNode::branch(label, children);
        root.compute_depths(0);
        Self { root, selection: Mutable::new(0), focused: Mutable::new(false), scroll: Mutable::new(0) }
    }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let flat = self.flatten();
        let len = flat.len();
        if len == 0 {
            return;
        }
        let mut sel = self.selection.get();

        match code {
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Down => sel = (sel + 1).min(len - 1),
            KeyCode::Home => sel = 0,
            KeyCode::End => sel = len - 1,
            KeyCode::Enter | KeyCode::Right | KeyCode::Left => {
                // toggle expansion
                if let Some((_depth, _, is_leaf, _)) = flat.get(sel) {
                    if !is_leaf {
                        // Need to find the actual TreeNode to toggle
                        // This is simplified; in a real impl you'd keep node refs
                    }
                }
                return;
            }
            _ => {}
        }
        self.selection.set(sel);
    }

    fn flatten(&self) -> Vec<(usize, String, bool, bool)> {
        let mut lines = vec![];
        self.root.flatten(&mut lines);
        lines
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let flat = self.flatten();
        let sel = self.selection.get();
        let focused = self.focused.get();
        let scroll = self.scroll.get();
        let visible = area.height.saturating_sub(2) as usize;

        let pair = &theme.list.pair;

        let lines: Vec<Line> = flat
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(i, (depth, label, is_leaf, expanded))| {
                let indent = "  ".repeat(*depth);
                let icon = if *is_leaf {
                    "📄"
                } else if *expanded {
                    "▼"
                } else {
                    "▶"
                };
                let style = if i == sel && focused { pair.focused } else { pair.blurred };
                Line::styled(format!("{}{} {}", indent, icon, label), style)
            })
            .collect();

        let border_style = pair.pick(focused);
        let block = BobaBlock::new().rounded().border_style(border_style);
        let block: ratatui::widgets::Block<'_> = block.into();
        Paragraph::new(lines).style(border_style).block(block).render(area, buf);
    }
}

impl Component for Tree {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
