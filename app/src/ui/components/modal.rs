use {
    ratatui::{
        prelude::*,
        widgets::WidgetRef,
    },
    std::
        sync::{
            Arc, LazyLock, RwLock,
        }
};


pub struct Modal {
    queue: RwLock<Vec<()>>,
}

static MODAL: LazyLock<Arc<Modal>> = LazyLock::new(|| Arc::new(Modal::new()));

pub fn modal() -> Arc<Modal> { MODAL.clone() }

impl Modal {
    fn new() -> Self {
        Self { queue: RwLock::new(Vec::new()),  }
    }
}

impl WidgetRef for Modal {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
    }
}
