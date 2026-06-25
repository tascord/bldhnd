use {
    futures::Stream,
    futures_signals::signal_map::MutableBTreeMap,
    std::{
        fmt::Debug,
        ops::Deref,
        pin::Pin,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering::SeqCst},
        },
        task::{Context, Poll},
    },
    tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel},
    tracing::instrument,
    uuid::Uuid,
};

#[derive(Debug)]
pub struct Cancellable<T> {
    inner: Arc<T>,
    cancelled: AtomicBool,
}

impl<T> std::ops::Deref for Cancellable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T> Cancellable<T> {
    pub fn cancel(&self) { self.cancelled.store(true, SeqCst); }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SubscriptionPriority {
    High,
    Low,
}

type Handler<T> = Box<dyn Fn(Arc<Cancellable<T>>) + Send + Sync>;

pub struct Subscription<T: Debug> {
    id: Uuid,
    handler: Handler<T>,
    priority: SubscriptionPriority,
}

impl<T: Debug> Debug for Subscription<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription").field("id", &self.id).field("handler", &"<function>").finish()
    }
}

impl<T: Debug> Subscription<T> {
    #[instrument(level = "trace")]
    pub(crate) fn update(&self, v: Arc<Cancellable<T>>) { (self.handler)(v) }
}

#[derive(Debug)]
pub struct EventTarget<T: Debug> {
    // MutableBTreeMap gives us a safe, lock-ordered collection with no
    // raw pointers and no separate channel to drain.
    listeners: MutableBTreeMap<Uuid, Arc<Subscription<T>>>,
}

impl<T: Debug> EventTarget<T> {
    pub fn new() -> Self { Self { listeners: MutableBTreeMap::new() } }

    #[instrument(level = "trace")]
    pub fn emit(&self, v: impl Into<Arc<T>> + Debug) {
        let v = Arc::new(Cancellable { inner: v.into(), cancelled: AtomicBool::new(false) });

        // Single short-lived lock; snapshot then drop before invoking handlers,
        // so a handler that calls `off`/drops a Subscription can't deadlock
        // re-entering the same map.
        let high: Vec<_> = {
            let lock = self.listeners.lock_ref();
            lock.values().filter(|s| s.priority == SubscriptionPriority::High).cloned().collect()
        };
        for s in &high {
            s.update(v.clone());
        }

        if !v.cancelled.load(SeqCst) {
            let low: Vec<_> = {
                let lock = self.listeners.lock_ref();
                lock.values().filter(|s| s.priority == SubscriptionPriority::Low).cloned().collect()
            };
            for s in &low {
                s.update(v.clone());
            }
        }
    }

    pub fn on(
        &self,
        priority: SubscriptionPriority,
        handler: impl Fn(Arc<Cancellable<T>>) + Send + Sync + 'static,
    ) -> SubscriptionHandle<T> {
        let id = Uuid::new_v4();
        let sub = Arc::new(Subscription { id, handler: Box::new(handler), priority });
        self.listeners.lock_mut().insert_cloned(id, sub.clone());
        SubscriptionHandle { id, sub, target: self.listeners.clone() }
    }

    pub fn as_stream(&self, p: SubscriptionPriority) -> EventStream<T>
    where
        T: Send + Sync + 'static,
    {
        EventStream::new(p, self)
    }
}

impl<T: Debug> Default for EventTarget<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Debug> Clone for EventTarget<T> {
    fn clone(&self) -> Self { Self { listeners: self.listeners.clone() } }
}

/// Owns removal-on-drop instead of a raw pointer back to the target.
/// MutableBTreeMap is itself Arc-backed internally, so cloning it here
/// is cheap and avoids any lifetime/unsafe tricks.
#[allow(dead_code)]
pub struct SubscriptionHandle<T: Debug> {
    id: Uuid,
    sub: Arc<Subscription<T>>,
    target: MutableBTreeMap<Uuid, Arc<Subscription<T>>>,
}

impl<T: Debug> SubscriptionHandle<T> {
    pub fn off(&self) { self.target.lock_mut().remove(&self.id); }

    /// Keep subscription alive past handle drop.
    pub fn forget(self) { std::mem::forget(self); }
}

impl<T: Debug> Drop for SubscriptionHandle<T> {
    fn drop(&mut self) { self.target.lock_mut().remove(&self.id); }
}

#[allow(dead_code)]
pub struct EventStream<T: Debug> {
    sub: SubscriptionHandle<T>,
    ch: UnboundedReceiver<Arc<T>>,
}

impl<T: Debug> EventStream<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(p: SubscriptionPriority, et: &EventTarget<T>) -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            ch: rx,
            sub: et.on(p, move |v| {
                let _ = tx.send(v.inner.clone());
            }),
        }
    }
}

impl<T: Debug> Deref for EventStream<T> {
    type Target = UnboundedReceiver<Arc<T>>;

    fn deref(&self) -> &Self::Target { &self.ch }
}

impl<T: Debug> Stream for EventStream<T> {
    type Item = Arc<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> { self.ch.poll_recv(cx) }
}
