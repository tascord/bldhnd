use {
    futures_signals::signal::Mutable,
    std::{
        sync::{Arc, LazyLock},
        time::Instant,
    },
    tracing::{
        Level,
        field::{Field, Visit},
    },
    tracing_subscriber::layer::{Context, Layer},
};

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
            if let Some('[') = chars.peek() {
                chars.next();
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

struct LogEntry {
    idx: usize,
    message: String,
    timestamp: Instant,
}

pub struct LogStore {
    entries: Mutable<Vec<LogEntry>>,
}

impl LogStore {
    pub fn new() -> Self { Self { entries: Mutable::new(Vec::new()) } }

    pub fn push(&self, message: String, idx: usize) {
        let mut entries = self.entries.lock_mut();
        entries.push(LogEntry { idx, message, timestamp: Instant::now() });
        while entries.len() > 200 {
            entries.remove(0);
        }
    }

    pub fn entries(&self) -> Vec<String> {
        self.entries.lock_ref().iter().map(|e| format!("{:03} {}", e.idx, e.message)).collect()
    }
}

impl Default for LogStore {
    fn default() -> Self { Self::new() }
}

static LOG_STORE: LazyLock<Arc<LogStore>> = LazyLock::new(|| Arc::new(LogStore::new()));

pub fn log_store() -> Arc<LogStore> { LOG_STORE.clone() }

pub struct LogsLayer {
    min_level: Level,
}

impl Default for LogsLayer {
    fn default() -> Self { Self { min_level: Level::TRACE } }
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

        let parsed = strip_ansi(&text);
        let store = log_store();

        let idx = store.entries.lock_ref().len();
        store.push(parsed, idx + 1);
    }
}

pub fn install_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry().with(LogsLayer::default()).init();
}
