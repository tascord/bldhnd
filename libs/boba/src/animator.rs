//! Centralized animation system with `BobaValue<T>` for reactive animation targets.
//!
//! ```rust,ignore
//! use boba::animator::{Animator, BobaValue};
//! use futures_signals::signal::Mutable;
//!
//! let mut animator = Animator::new();
//! let opacity = Mutable::new(0.0);
//! let handle = animator.animate_f64(&opacity, 0.0, 1.0);
//!
//! let _val = BobaValue::Animated(handle);
//! ```

use {
    futures_signals::signal::Mutable,
    std::{
        collections::HashMap,
        fmt::Debug,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    },
};

static NEXT_ANIM_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 { NEXT_ANIM_ID.fetch_add(1, Ordering::Relaxed) }

// ──────────────────────────────────────────────────────────────
//  Easing
// ──────────────────────────────────────────────────────────────

pub mod ease {
    pub fn linear(t: f64) -> f64 { t.clamp(0.0, 1.0) }

    pub fn in_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        t * t
    }

    pub fn out_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t) * (1.0 - t)
    }

    pub fn in_out_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
    }

    pub fn out_cubic(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    pub fn out_elastic(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let c4 = (2.0 * std::f64::consts::PI) / 3.0;
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            2_f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
        }
    }

    pub fn out_bounce(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let n1 = 7.5625;
        let d1 = 2.75;
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  Lerp Trait
// ──────────────────────────────────────────────────────────────

pub trait Lerp: Copy + Clone + Debug + PartialEq + Send + Sync + 'static {
    fn lerp(self, other: Self, t: f64) -> Self;
}

impl Lerp for f64 {
    fn lerp(self, other: Self, t: f64) -> Self { self + (other - self) * t }
}

impl Lerp for u16 {
    fn lerp(self, other: Self, t: f64) -> Self { (self as f64 + (other as f64 - self as f64) * t).round() as u16 }
}

// ──────────────────────────────────────────────────────────────
//  BobaValue — Fixed / Mutable / Animated
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum BobaValue<T: Lerp> {
    Fixed(T),
    Mutable(Mutable<T>),
    Animated(AnimationHandle<T>),
}

impl<T: Lerp> BobaValue<T> {
    pub fn get(&self) -> T {
        match self {
            BobaValue::Fixed(v) => *v,
            BobaValue::Mutable(m) => m.get(),
            BobaValue::Animated(h) => h.value(),
        }
    }

    pub fn is_animated(&self) -> bool { matches!(self, BobaValue::Animated(_)) }
}

impl<T: Lerp> From<T> for BobaValue<T> {
    fn from(v: T) -> Self { BobaValue::Fixed(v) }
}

impl<T: Lerp> From<Mutable<T>> for BobaValue<T> {
    fn from(m: Mutable<T>) -> Self { BobaValue::Mutable(m) }
}

impl<T: Lerp> From<AnimationHandle<T>> for BobaValue<T> {
    fn from(h: AnimationHandle<T>) -> Self { BobaValue::Animated(h) }
}

// ──────────────────────────────────────────────────────────────
//  AnimationHandle
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AnimationHandle<T: Lerp> {
    id: u64,
    target: Mutable<T>,
}

impl<T: Lerp> AnimationHandle<T> {
    pub(crate) fn new(id: u64, target: Mutable<T>) -> Self { Self { id, target } }

    pub fn id(&self) -> u64 { self.id }

    pub fn value(&self) -> T { self.target.get() }
}

// ──────────────────────────────────────────────────────────────
//  RunningAnimation (internal)
// ──────────────────────────────────────────────────────────────

struct RunningAnimation<T: Lerp> {
    id: u64,
    target: Mutable<T>,
    from: T,
    to: T,
    start: Instant,
    duration: Duration,
    easing: fn(f64) -> f64,
    on_complete: Option<Box<dyn FnOnce() + Send>>,
}

impl<T: Lerp> RunningAnimation<T> {
    fn tick(&mut self) -> bool {
        let elapsed = self.start.elapsed().as_secs_f64();
        let total = self.duration.as_secs_f64();
        if total <= 0.0 || elapsed >= total {
            self.target.set(self.to);
            if let Some(cb) = self.on_complete.take() {
                cb();
            }
            true
        } else {
            let t = (self.easing)(elapsed / total);
            self.target.set(self.from.lerp(self.to, t));
            false
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  AnimationConfig
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct AnimationConfig {
    pub duration_ms: u64,
    pub easing: fn(f64) -> f64,
}

impl Default for AnimationConfig {
    fn default() -> Self { Self { duration_ms: 300, easing: ease::out_quad } }
}

impl AnimationConfig {
    pub fn duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn easing(mut self, f: fn(f64) -> f64) -> Self {
        self.easing = f;
        self
    }
}

// ──────────────────────────────────────────────────────────────
//  Animator (centralized)
// ──────────────────────────────────────────────────────────────

pub struct Animator {
    f64s: HashMap<u64, RunningAnimation<f64>>,
    u16s: HashMap<u64, RunningAnimation<u16>>,
    last_id: u64,
}

impl Default for Animator {
    fn default() -> Self { Self { f64s: HashMap::new(), u16s: HashMap::new(), last_id: 0 } }
}

impl Animator {
    pub fn new() -> Self { Self::default() }

    pub fn animate_f64(&mut self, target: &Mutable<f64>, from: f64, to: f64) -> AnimationHandle<f64> {
        self.animate_with_config(target, from, to, AnimationConfig::default())
    }

    pub fn animate_with_config(
        &mut self,
        target: &Mutable<f64>,
        from: f64,
        to: f64,
        config: AnimationConfig,
    ) -> AnimationHandle<f64> {
        let id = next_id();
        target.set(from);
        let anim = RunningAnimation {
            id,
            target: target.clone(),
            from,
            to,
            start: Instant::now(),
            duration: Duration::from_millis(config.duration_ms),
            easing: config.easing,
            on_complete: None,
        };
        self.f64s.insert(id, anim);
        self.last_id = id;
        AnimationHandle::new(id, target.clone())
    }

    /// Tick all animations. Returns true if any animations are still running.
    pub fn tick(&mut self) -> bool {
        let mut alive = false;
        self.f64s.retain(|_, a| {
            let done = a.tick();
            if !done {
                alive = true;
            }
            !done
        });
        self.u16s.retain(|_, a| {
            let done = a.tick();
            if !done {
                alive = true;
            }
            !done
        });
        alive
    }

    pub fn is_idle(&self) -> bool { self.f64s.is_empty() && self.u16s.is_empty() }

    pub fn cancel(&mut self, handle: AnimationHandle<f64>) { self.f64s.remove(&handle.id()); }

    pub fn cancel_all(&mut self) {
        self.f64s.clear();
        self.u16s.clear();
    }
}

impl Debug for Animator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Animator").field("f64_count", &self.f64s.len()).field("u16_count", &self.u16s.len()).finish()
    }
}
