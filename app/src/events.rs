use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use uuid::Uuid;

use {
    futures::Stream,
    std::{
        collections::HashMap,
        fmt::Debug,
        ops::Deref,
        pin::Pin,
        sync::{Arc, RwLock},
        task::{Context, Poll},
    },
    tokio::sync::{
        Mutex,
        mpsc::{self, UnboundedReceiver, unbounded_channel},
    },
    tracing::instrument,
};

#[derive(Debug)]
pub struct Cancellable<T> {
    inner: Arc<T>,
    cancelled: AtomicBool
}

impl<T> Deref for Cancellable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> Cancellable<T> {
    pub fn cancel(&self) {
        self.cancelled.store(true, SeqCst);
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct EventTarget<T: Debug> {
    listeners: Arc<RwLock<HashMap<Uuid, Arc<Subscription<T>>>>>,
    sender: Arc<mpsc::UnboundedSender<Arc<Cancellable<T>>>>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<Arc<Cancellable<T>>>>>,
}

impl<T: Debug> EventTarget<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            listeners: Arc::new(RwLock::new(HashMap::new())),
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver)),
        } 
    }

    #[instrument(level = "trace")]
    pub fn emit(&self, v: impl Into<Arc<T>> + Debug) {
        let v = Arc::new(Cancellable {
            inner: v.into(),
            cancelled: AtomicBool::new(false)
        });

        // Notify all listeners

        if let Ok(listeners) = self.listeners.read() {
            listeners.values().for_each(|s| {
                if s.priority == SubscriptionPriority::High {
                    s.update(v.clone());
                }  
            });

            if !v.cancelled.load(SeqCst) {
                listeners.values().for_each(|s| {
                    if s.priority == SubscriptionPriority::Low {
                        s.update(v.clone());
                    }  
                });
            }
        }

        // Send to stream (ignore error if receiver is dropped)
        let _ = self.sender.send(v);
    }

    pub fn on(&self, priority: SubscriptionPriority, handler: impl Fn(Arc<Cancellable<T>>) + Send + Sync + 'static) -> Arc<Subscription<T>> {
        let sub = Arc::new(Subscription::new(self, priority, handler));
        if let Ok(mut listeners) = self.listeners.write() {
            listeners.insert(sub.id, sub.clone());
        }
        sub
    }

    pub fn off(&self, sub: &Subscription<T>) {
        if let Ok(mut listeners) = self.listeners.write() {
            listeners.remove(&sub.id);
        }
    }

   
}

impl<T: Debug> Default for EventTarget<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Debug> Clone for EventTarget<T> {
    fn clone(&self) -> Self {
        Self {
            listeners: self.listeners.clone(),
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SubscriptionPriority {
    High,
    Low
}

pub struct Subscription<T: Debug> {
    id: Uuid,
    handler: Box<dyn Fn(Arc<Cancellable<T>>) + Send + Sync>,
    to: *const EventTarget<T>, // Using raw pointer to avoid lifetime issues
    priority: SubscriptionPriority,
}

impl<T: Debug> Debug for Subscription<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscription").field("id", &self.id).field("handler", &"<function>").field("to", &self.to).finish()
    }
}

unsafe impl<T: Debug> Send for Subscription<T> {}
unsafe impl<T: Debug> Sync for Subscription<T> {}

impl<T: Debug> Subscription<T> {
    pub fn new(to: &EventTarget<T>, priority: SubscriptionPriority, handler: impl Fn(Arc<Cancellable<T>>) + Send + Sync + 'static) -> Self {
        Self { id: Uuid::new_v4(), handler: Box::new(handler), to: to as *const _, priority }
    }

    pub fn off(&self) {
        unsafe {
            if let Some(target) = self.to.as_ref() {
                target.off(self);
            }
        }
    }

    /// Keep after drop
    pub fn forget(self: Arc<Self>) {
        std::mem::forget(self);
    }

    #[instrument(level = "trace")]
    pub(crate) fn update(&self, v: Arc<Cancellable<T>>) { (self.handler)(v) }
}

impl<T: Debug> Drop for Subscription<T> {
    fn drop(&mut self) {
        unsafe {
            self.to.read().off(self);
        }
    }
}
