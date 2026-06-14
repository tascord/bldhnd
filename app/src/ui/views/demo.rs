use std::{sync::{Arc, LazyLock, RwLock}, time::{SystemTime}};

use ratatui::{widgets::{Paragraph, Widget}};

use crate::ui::views::ModelEvent;

#[derive(Default)]
pub struct DemoData {
    last_ev: Option<String>
}

static DD: LazyLock<Arc<RwLock<DemoData>>> = LazyLock::new(Default::default);

pub fn set_last_event(s: String) {
    DD.write().unwrap().last_ev = Some(s);
}

pub struct DemoView;
impl Widget for DemoView {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized {
        Paragraph::new(format!("{:?} || {}", DD.read().unwrap().last_ev, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis())).render(area, buf);
    }
}