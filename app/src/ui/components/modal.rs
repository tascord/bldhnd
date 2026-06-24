use {
    crate::{
        events::{SubscriptionHandle, SubscriptionPriority},
        ui::{
            components::{scroll, input::Input, Focusable},
            views::{ModelEvent, model},
        },
    },
    crossterm::event::{KeyCode, KeyModifiers},
    ratatui::{
        prelude::*,
        style::Style,
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Clear, Paragraph, WidgetRef},
    },
    std::sync::{
        Arc, LazyLock, RwLock,
        atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    },
};

#[derive(Clone)]
pub enum ModalRequest {
    VolumeAdd,
    VolumeEdit { index: usize },
}

pub trait FormField: WidgetRef + Focusable + Send + Sync {
    fn height(&self) -> u16;
    fn next_suggestion(&self) {}
    fn prev_suggestion(&self) {}
    fn accept_suggestion(&self) {}
}

struct FormInput(Input, Arc<AtomicBool>);

impl Focusable for FormInput {
    fn focus(&self) {
        self.1.store(true, SeqCst);
        self.0.focus();
    }

    fn blur(&self) {
        self.1.store(false, SeqCst);
        self.0.blur();
    }
}

impl WidgetRef for FormInput {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.0.render_ref(area, buf);
    }
}

impl FormField for FormInput {
    fn height(&self) -> u16 {
        3
    }
}

impl FormInput {
    fn new(label: &str, default: &str) -> Self {
        Self(Input::new(label, default), AtomicBool::new(false).into())
    }
}

struct AutoCompleteInput {
    input: Input,
    suggestions: Arc<RwLock<Vec<String>>>,
    selected_index: Arc<AtomicUsize>,
    showing: Arc<AtomicBool>,
    focused: Arc<AtomicBool>,
    autocomplete: Arc<dyn Fn(&str) -> Vec<String> + Send + Sync>,
}

impl AutoCompleteInput {
    fn new(
        label: &str,
        default: &str,
        autocomplete: impl Fn(&str) -> Vec<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            input: Input::new(label, default),
            suggestions: Arc::new(RwLock::new(Vec::new())),
            selected_index: AtomicUsize::new(0).into(),
            showing: AtomicBool::new(false).into(),
            focused: AtomicBool::new(false).into(),
            autocomplete: Arc::new(autocomplete),
        }
    }

    fn update_suggestions(&self, text: &str) {
        let sugs = (self.autocomplete)(text);
        let has_suggestions = !sugs.is_empty();
        *self.suggestions.write().unwrap() = sugs;
        self.selected_index.store(0, SeqCst);
        self.showing.store(has_suggestions, SeqCst);
    }

    fn next_suggestion(&self) {
        let len = self.suggestions.read().unwrap().len();
        if len > 0 {
            let next = (self.selected_index.load(SeqCst) + 1) % len;
            self.selected_index.store(next, SeqCst);
        }
    }

    fn prev_suggestion(&self) {
        let len = self.suggestions.read().unwrap().len();
        if len > 0 {
            let prev = self.selected_index.load(SeqCst).saturating_sub(1);
            self.selected_index.store(prev, SeqCst);
        }
    }

    fn accept_suggestion(&self) {
        let sugs = self.suggestions.read().unwrap();
        if let Some(s) = sugs.get(self.selected_index.load(SeqCst)) {
            self.input.set_text(s);
            self.hide_suggestions();
        }
    }

    fn hide_suggestions(&self) {
        self.showing.store(false, SeqCst);
    }
}

impl Focusable for AutoCompleteInput {
    fn focus(&self) {
        self.focused.store(true, SeqCst);
        self.input.focus();
        self.update_suggestions("");
    }

    fn blur(&self) {
        self.focused.store(false, SeqCst);
        self.input.blur();
        self.hide_suggestions();
    }
}

impl WidgetRef for AutoCompleteInput {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.input.render_ref(area, buf);

        if self.showing.load(SeqCst) {
            let sugs = self.suggestions.read().unwrap();
            if !sugs.is_empty() {
                let input_height = 3;
                let suggestion_height = (sugs.len() as u16).min(5);
                let suggestion_area = Rect {
                    x: area.x,
                    y: area.y + input_height,
                    width: area.width,
                    height: suggestion_height,
                };

                let mut y = suggestion_area.y;
                for (i, s) in sugs.iter().enumerate() {
                    let is_selected = i == self.selected_index.load(SeqCst);
                    let style = if is_selected {
                        Style::new().on_white().black()
                    } else {
                        Style::new()
                    };
                    let line = Line::from_iter([Span::raw(format!("  {}", s))]);
                    Paragraph::new(line).style(style).render(
                        Rect { x: suggestion_area.x, y, width: suggestion_area.width, height: 1 },
                        buf,
                    );
                    y += 1;
                }
            }
        }
    }
}

impl FormField for AutoCompleteInput {
    fn height(&self) -> u16 {
        if self.showing.load(SeqCst) {
            let count = self.suggestions.read().unwrap().len().min(5);
            3 + count as u16
        } else {
            3
        }
    }

    fn next_suggestion(&self) {
        AutoCompleteInput::next_suggestion(self);
    }

    fn prev_suggestion(&self) {
        AutoCompleteInput::prev_suggestion(self);
    }

    fn accept_suggestion(&self) {
        AutoCompleteInput::accept_suggestion(self);
    }
}

struct ModalForm {
    fields: Vec<Box<dyn FormField + Sync + Send>>,
    focused_index: Arc<AtomicUsize>,
}

impl ModalForm {
    fn new() -> Self {
        let path_autocomplete = |_text: &str| -> Vec<String> {
            if let Some(home) = std::env::home_dir() {
                let path = home.join(".config");
                if path.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(path) {
                        return entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().is_dir())
                            .map(|e| e.path().display().to_string())
                            .collect();
                    }
                }
            }
            Vec::new()
        };

        let fields: Vec<Box<dyn FormField + Sync + Send>> = vec![
            Box::new(AutoCompleteInput::new("Path", "", path_autocomplete)),
            Box::new(FormInput::new("Name", "")),
            Box::new(FormInput::new("Priority", "100")),
            Box::new(FormInput::new("Max Size (GB)", "")),
        ];

        Self { fields, focused_index: AtomicUsize::new(0).into() }
    }

    fn is_last_field(&self) -> bool {
        self.focused_index.load(SeqCst) == self.fields.len() - 1
    }

    fn focus_next(&self) {
        let next = (self.focused_index.load(SeqCst) + 1) % self.fields.len();
        self.focus_index(next);
    }

    fn focus_prev(&self) {
        let prev = self.focused_index.load(SeqCst).saturating_sub(1);
        self.focus_index(prev);
    }

    fn focus_index(&self, idx: usize) {
        let idx = idx.min(self.fields.len().saturating_sub(1));
        self.fields.iter().for_each(|f| f.blur());
        if let Some(f) = self.fields.get(idx) {
            f.focus();
        }
        self.focused_index.store(idx, SeqCst);
    }

    fn blur_all(&self) {
        self.fields.iter().for_each(|f| f.blur());
    }

    fn navigate_suggestion_up(&self) {
        if let Some(f) = self.fields.get(self.focused_index.load(SeqCst)) {
            f.prev_suggestion();
        }
    }

    fn navigate_suggestion_down(&self) {
        if let Some(f) = self.fields.get(self.focused_index.load(SeqCst)) {
            f.next_suggestion();
        }
    }

    fn accept_current_suggestion(&self) {
        if let Some(f) = self.fields.get(self.focused_index.load(SeqCst)) {
            f.accept_suggestion();
        }
    }
}

pub struct Modal {
    queue: RwLock<Vec<(ModalRequest, ModalForm)>>,
    was_scroller_focused: RwLock<bool>,
    subs: Option<SubscriptionHandle<ModelEvent>>,
}

static MODAL: LazyLock<Arc<Modal>> = LazyLock::new(|| Arc::new(Modal::new()));

pub fn modal() -> Arc<Modal> {
    MODAL.clone()
}

impl Modal {
    fn new() -> Self {
        let mut this = Self { queue: RwLock::new(Vec::new()), was_scroller_focused: RwLock::new(false), subs: None };

        let subs = model().target.on(SubscriptionPriority::High, {
            move |ev| {
                let ModelEvent::KeyPress(key_event) = **ev;

                if key_event.code == KeyCode::Esc {
                    modal().pop();
                    ev.cancel();
                }

                if !modal().is_empty() {
                    let m = modal();
                    let queue = m.queue.read().unwrap();
                    if let Some((_, form)) = queue.last() {
                        match key_event.code {
                            KeyCode::Tab => {
                                if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                                    form.focus_prev();
                                } else {
                                    form.accept_current_suggestion();
                                    form.focus_next();
                                }
                                ev.cancel();
                            }
                            KeyCode::Enter => {
                                if form.is_last_field() {
                                    modal().pop();
                                } else {
                                    form.accept_current_suggestion();
                                    form.focus_next();
                                }
                                ev.cancel();
                            }
                            KeyCode::Up => {
                                form.navigate_suggestion_up();
                                ev.cancel();
                            }
                            KeyCode::Down => {
                                form.navigate_suggestion_down();
                                ev.cancel();
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        this.subs = Some(subs);
        this
    }

    pub fn push(&self, content: ModalRequest) {
        let was_focused = scroll::is_scroller_focused();
        *self.was_scroller_focused.write().unwrap() = was_focused;

        if was_focused {
            scroll::blur_current_scroller();
        }

        let form = ModalForm::new();
        form.focus_index(0);

        self.queue.write().unwrap().push((content, form));
    }

    pub fn pop(&self) {
        let was_focused = *self.was_scroller_focused.read().unwrap();

        if let Some((_, form)) = self.queue.write().unwrap().pop() {
            form.blur_all();
        }

        if was_focused {
            scroll::focus_current_scroller();
        }
    }

    pub fn close_all(&self) {
        self.queue.write().unwrap().clear();
    }

    pub fn is_empty(&self) -> bool {
        self.queue.read().unwrap().is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.read().unwrap().len()
    }
}

impl WidgetRef for Modal {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if self.is_empty() {
            return;
        }

        let queue = self.queue.read().unwrap();
        if let Some((_, form)) = queue.last() {
            let width = 60;
            let height = 20;

            let rect = {
                let w = width.min(area.width.saturating_sub(4)).max(30);
                let x = (area.width - w) / 2;
                let h = height.min(area.height.saturating_sub(4)).max(10);
                let y = (area.height - h) / 2;
                Rect { x, y, width: w, height: h }
            };

            let title = match &queue.last().unwrap().0 {
                ModalRequest::VolumeAdd { .. } => "Add Volume",
                ModalRequest::VolumeEdit { .. } => "Edit Volume",
            };

            Clear.render(rect, buf);

            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title)
                .render(rect, buf);

            let inner = Rect { x: rect.x + 2, y: rect.y + 2, width: rect.width - 4, height: rect.height - 4 };

            let mut y = inner.y;
            for field in &form.fields {
                field.render_ref(Rect { x: inner.x, y, width: inner.width, height: field.height() }, buf);
                y += field.height();
            }
        }
    }
}