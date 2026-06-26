//! Minimal animation primitives that integrate with [`futures_signals::signal::Mutable`].
//!
//! Animations mutate a [`Mutable`] over time and are fed by the event loop's
//! `RequestAnimationFrame` tick.

use {
    futures_signals::signal::Mutable,
    std::time::{Duration, Instant},
};

/// A standard set of easing functions.
pub mod ease {
    /// Linear interpolation.
    pub fn linear(t: f64) -> f64 { t.clamp(0.0, 1.0) }

    /// Ease-in: slow start, fast end.
    pub fn in_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        t * t
    }

    /// Ease-out: fast start, slow end.
    pub fn out_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t) * (1.0 - t)
    }

    /// Ease-in-out: symmetric acceleration.
    pub fn in_out_quad(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
    }

    /// Ease-out cubic.
    pub fn out_cubic(t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// Ease-out elastic (bouncy).
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

    /// Ease-out bounce.
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

/// Trait for anything that can be linearly interpolated.
pub trait Lerp: Copy + Clone {
    fn lerp(self, other: Self, t: f64) -> Self;
}

impl Lerp for f64 {
    fn lerp(self, other: Self, t: f64) -> Self { self + (other - self) * t }
}

impl Lerp for u16 {
    fn lerp(self, other: Self, t: f64) -> Self { (self as f64 + (other as f64 - self as f64) * t).round() as u16 }
}

/// An active animation that mutates a [`Mutable<T>`] over time.
pub struct Animation<T: Lerp> {
    target: Mutable<T>,
    from: T,
    to: T,
    start: Instant,
    duration: Duration,
    easing: fn(f64) -> f64,
}

impl<T: Lerp> Animation<T> {
    pub fn new(target: Mutable<T>, from: T, to: T, duration: Duration) -> Self {
        Self { target, from, to, start: Instant::now(), duration, easing: ease::out_quad }
    }

    /// Set a custom easing function.
    pub fn easing(mut self, f: fn(f64) -> f64) -> Self {
        self.easing = f;
        self
    }

    /// Tick the animation. Returns `true` when finished.
    pub fn tick(&mut self) -> bool {
        let elapsed = self.start.elapsed().as_secs_f64();
        let total = self.duration.as_secs_f64();
        if total <= 0.0 || elapsed >= total {
            self.target.set(self.to);
            true
        } else {
            let t = (self.easing)(elapsed / total);
            self.target.set(self.from.lerp(self.to, t));
            false
        }
    }
}

/// A manager for many running animations. Tick this every frame.
#[derive(Default)]
pub struct Animator {
    f64s: Vec<Animation<f64>>,
    u16s: Vec<Animation<u16>>,
}

impl Animator {
    pub fn new() -> Self { Self::default() }

    pub fn animate_f64(&mut self, target: Mutable<f64>, from: f64, to: f64, dur: Duration) {
        self.f64s.push(Animation::new(target, from, to, dur));
    }

    pub fn animate_u16(&mut self, target: Mutable<u16>, from: u16, to: u16, dur: Duration) {
        self.u16s.push(Animation::new(target, from, to, dur));
    }

    /// Advance all animations by one step. Call this every `RequestAnimationFrame`.
    pub fn tick(&mut self) {
        self.f64s.retain_mut(|a| !a.tick());
        self.u16s.retain_mut(|a| !a.tick());
    }

    pub fn is_idle(&self) -> bool { self.f64s.is_empty() && self.u16s.is_empty() }
}

/// A simple spinner that cycles through frames based on elapsed time.
pub struct Spinner {
    frames: &'static [&'static str],
    interval_ms: u128,
    start: Instant,
}

impl Spinner {
    pub fn dots() -> Self {
        Self {
            frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], interval_ms: 80, start: Instant::now()
        }
    }

    pub fn line() -> Self { Self { frames: &["-", "\\", "|", "/"], interval_ms: 120, start: Instant::now() } }

    pub fn mini() -> Self {
        Self {
            frames: &["⠋", "⠙", "⠚", "⠞", "⠖", "⠦", "⠴", "⠲", "⠐"], interval_ms: 100, start: Instant::now()
        }
    }

    pub fn frame(&self) -> &'static str {
        let idx = (self.start.elapsed().as_millis() / self.interval_ms) as usize % self.frames.len();
        self.frames[idx]
    }
}
