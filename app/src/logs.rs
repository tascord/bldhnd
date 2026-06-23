use crate::ui::components::scroll::{ScrollText, Scroller};
use ratatui::prelude::Text;
use std::sync::{Arc, LazyLock, RwLock};
use tracing::{
    Level,
    field::{Field, Visit},
};
use tracing_subscriber::layer::{Context, Layer};

static LOG_SCROLLER: LazyLock<Arc<Scroller>> = LazyLock::new(|| Arc::new(Scroller::new()));

pub fn scroller() -> Arc<Scroller> {
    LOG_SCROLLER.clone()
}

/// Simple visitor to extract `message` field from tracing events.
#[derive(Default)]
struct MessageVisitor(String);

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" && self.0.is_empty() {
            self.0 = format!("{:?}", value);
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // CSI sequences start with '['
            if let Some('[') = chars.peek() {
                // consume '['
                chars.next();
                // consume until letter in range '@'..='~'
                while let Some(&nc) = chars.peek() {
                    let is_final = ('@'..='~').contains(&nc);
                    chars.next();
                    if is_final {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

pub struct LogsLayer {
    min_level: Level,
}

impl Default for LogsLayer {
    fn default() -> Self {
        Self { min_level: Level::TRACE }
    }
}

impl<S> Layer<S> for LogsLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let level = *event.metadata().level();
        if level > self.min_level {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut text = visitor.0;
        if text.is_empty() {
            text = event.metadata().name().to_string();
        }

        // Strip ANSI and push as a ScrollText into the scroller
        let parsed = strip_ansi(&text);
        let sc = scroller();

        // Compute a best-effort index (may race, that's acceptable for numbering)
        let idx = sc.items.read().unwrap().len();
        let numbered = format!("{:03} {}", idx + 1, parsed);

        let t: ScrollText = ScrollText::new(Text::from(numbered));
        sc.item_ref(t);

        // Trim to last ~200 entries
        let mut items_lock = sc.items.write().unwrap();
        while items_lock.len() > 200 {
            items_lock.remove(0);
        }
    }
}

pub fn install_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    // Combine sonner toasts with logs layer and initialise once
    tracing_subscriber::registry()
        .with(crate::ui::components::sonner::SonnerLayer::default())
        .with(LogsLayer::default())
        .init();
}
