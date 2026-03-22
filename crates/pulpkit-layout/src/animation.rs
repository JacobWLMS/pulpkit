//! Animation primitives for layout effects.

use std::time::Instant;

/// A simple float animation for popup fade in/out effects.
///
/// Interpolates a float value from `from` to `to` over `duration_ms`
/// using ease-out cubic easing.
#[derive(Debug, Clone)]
pub struct FadeAnimation {
    pub from: f32,
    pub to: f32,
    pub start: Instant,
    pub duration_ms: u32,
}

impl FadeAnimation {
    pub fn new(from: f32, to: f32, duration_ms: u32) -> Self {
        Self {
            from,
            to,
            start: Instant::now(),
            duration_ms,
        }
    }

    /// Returns the interpolated value at the current time and whether the animation is done.
    pub fn current(&self) -> (f32, bool) {
        let elapsed = self.start.elapsed().as_millis() as f32;
        let duration = self.duration_ms as f32;
        let t = (elapsed / duration).min(1.0);
        // Ease-out cubic: 1 - (1-t)^3
        let t = 1.0 - (1.0 - t).powi(3);
        let value = self.from + (self.to - self.from) * t;
        (value, elapsed >= duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_animation_starts_near_from() {
        let anim = FadeAnimation::new(0.0, 1.0, 200);
        let (val, done) = anim.current();
        assert!(!done);
        assert!(val < 0.2, "expected near start, got {}", val);
    }
}
