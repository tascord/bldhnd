//! File picker / directory tree browser.
//!
//! ```rust
//! use bobatea::components::filepicker::FilePicker;
//! let fp = FilePicker::new(".");
//! ```

use {
    crate::components::{Component, block::BobaBlock},
    crossterm::event::{KeyCode, MouseEvent, MouseEventKind},
    futures_signals::signal::Mutable,
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        text::Line,
        widgets::{Paragraph, Widget},
    },
    std::path::PathBuf,
};

pub enum FilePickerEvent {
    Select(PathBuf),
    Focus,
    Blur,
}

/// A file/directory browser.
pub struct FilePicker {
    root: PathBuf,
    items: Mutable<Vec<(String, bool)>>, // (name, is_dir)
    selection: Mutable<usize>,
    focused: Mutable<bool>,
    current_dir: Mutable<PathBuf>,
}

impl FilePicker {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let items = Self::scan(&root);
        Self {
            root: root.clone(),
            items: Mutable::new(items),
            selection: Mutable::new(0),
            focused: Mutable::new(false),
            current_dir: Mutable::new(root),
        }
    }

    fn scan(dir: &PathBuf) -> Vec<(String, bool)> {
        let mut items = vec![];
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut v: Vec<_> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    (name, is_dir)
                })
                .collect();
            v.sort_by(|a, b| match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            });
            items.extend(v);
        }
        items.insert(0, ("..".into(), true));
        items
    }

    pub fn focus(&self) { self.focused.set(true); }

    pub fn blur(&self) { self.focused.set(false); }

    pub fn on_key(&self, code: KeyCode) {
        if !self.focused.get() {
            return;
        }
        let len = self.items.lock_ref().len();
        if len == 0 {
            return;
        }
        let mut sel = self.selection.get();

        match code {
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Down => sel = (sel + 1).min(len - 1),
            KeyCode::Home => sel = 0,
            KeyCode::End => sel = len - 1,
            KeyCode::Enter | KeyCode::Right => {
                if let Some((name, is_dir)) = self.items.lock_ref().get(sel).cloned() {
                    if is_dir {
                        let mut current = self.current_dir.get_cloned();
                        if name == ".." {
                            if let Some(parent) = current.parent() {
                                current = parent.to_path_buf();
                            }
                        } else {
                            current.push(&name);
                        }
                        self.current_dir.set(current.clone());
                        self.items.set(Self::scan(&current));
                        self.selection.set(0);
                    }
                }
                return;
            }
            KeyCode::Left => {
                let current = self.current_dir.get_cloned();
                if let Some(parent) = current.parent() {
                    self.current_dir.set(parent.to_path_buf());
                    self.items.set(Self::scan(&parent.to_path_buf()));
                    self.selection.set(0);
                }
                return;
            }
            _ => return,
        }
        self.selection.set(sel);
    }

    pub fn on_mouse(&self, area: Rect, ev: &MouseEvent) {
        match ev.kind {
            MouseEventKind::Down(_) => {
                if is_inside(area, ev) {
                    self.focus();
                    let offset = 1; // border
                    let inner_y = ev.row.saturating_sub(area.top() + offset);
                    let sel = (inner_y as usize).min(self.items.lock_ref().len().saturating_sub(1));
                    self.selection.set(sel);
                } else {
                    self.blur();
                }
            }
            MouseEventKind::ScrollUp => {
                self.focus();
                let sel = self.selection.get().saturating_sub(1);
                self.selection.set(sel);
            }
            MouseEventKind::ScrollDown => {
                self.focus();
                let len = self.items.lock_ref().len();
                let sel = (self.selection.get() + 1).min(len.saturating_sub(1));
                self.selection.set(sel);
            }
            _ => {}
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        let sel = self.selection.get();
        self.items.lock_ref().get(sel).map(|(name, _)| {
            let mut path = self.current_dir.get_cloned();
            path.push(name);
            path
        })
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let sel = self.selection.get();
        let focused = self.focused.get();
        let items = self.items.lock_ref();
        let current = self.current_dir.get_cloned();
        let pair = &theme.list.pair;

        let lines: Vec<Line> = items
            .iter()
            .enumerate()
            .map(|(i, (name, is_dir))| {
                let glyph = if i == sel { "▸" } else { " " };
                let icon = if *is_dir { "📁" } else { "📄" };
                let style = if i == sel && focused { pair.focused } else { pair.blurred };
                Line::styled(format!("{} {} {}", glyph, icon, name), style)
            })
            .collect();

        let border_style = pair.pick(focused);

        let block = BobaBlock::new().rounded().border_style(border_style).title(format!(" {} ", current.display()));
        let block: ratatui::widgets::Block<'_> = block.into();

        Paragraph::new(lines).style(border_style).block(block).render(area, buf);
    }
}

impl Component for FilePicker {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}

fn is_inside(area: Rect, ev: &MouseEvent) -> bool {
    ev.column >= area.left() && ev.column < area.right() && ev.row >= area.top() && ev.row < area.bottom()
}
