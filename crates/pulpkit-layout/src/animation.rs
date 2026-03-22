//! Color transition animations for hover effects.

use std::collections::HashMap;
use std::time::Instant;

use pulpkit_render::Color;

/// A single color animation that interpolates from one color to another over time.
#[derive(Debug, Clone)]
pub struct ColorAnimation {
    pub from: Color,
    pub to: Color,
    pub start: Instant,
    pub duration_ms: u32,
}

impl ColorAnimation {
    pub fn new(from: Color, to: Color, duration_ms: u32) -> Self {
        Self {
            from,
            to,
            start: Instant::now(),
            duration_ms,
        }
    }

    /// Returns the interpolated color at the current time and whether the animation is done.
    pub fn current(&self) -> (Color, bool) {
        let elapsed = self.start.elapsed().as_millis() as f32;
        let duration = self.duration_ms as f32;
        let t = (elapsed / duration).min(1.0);

        // Ease-out cubic: 1 - (1-t)^3
        let t = 1.0 - (1.0 - t).powi(3);

        let color = Color::new(
            lerp_u8(self.from.r, self.to.r, t),
            lerp_u8(self.from.g, self.to.g, t),
            lerp_u8(self.from.b, self.to.b, t),
            lerp_u8(self.from.a, self.to.a, t),
        );

        let done = elapsed >= duration;
        (color, done)
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

/// Tracks active color animations for layout nodes, keyed by node index.
#[derive(Default)]
pub struct AnimationManager {
    pub bg_animations: HashMap<usize, ColorAnimation>,
}

impl AnimationManager {
    /// Returns true if there are any active animations.
    pub fn has_active(&self) -> bool {
        !self.bg_animations.is_empty()
    }

    /// Start a background color animation for a node.
    pub fn animate_bg(&mut self, node_idx: usize, from: Color, to: Color, duration_ms: u32) {
        self.bg_animations
            .insert(node_idx, ColorAnimation::new(from, to, duration_ms));
    }

    /// Get the current animated background color for a node, if any.
    /// Removes completed animations and returns the final color.
    pub fn get_bg(&mut self, node_idx: usize) -> Option<Color> {
        if let Some(anim) = self.bg_animations.get(&node_idx) {
            let (color, done) = anim.current();
            if done {
                self.bg_animations.remove(&node_idx);
            }
            Some(color)
        } else {
            None
        }
    }
}

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
    fn color_animation_interpolates() {
        let anim = ColorAnimation::new(
            Color::new(0, 0, 0, 255),
            Color::new(255, 255, 255, 255),
            100,
        );
        let (color, done) = anim.current();
        // At t~0, should be near black
        assert!(!done);
        assert!(color.r < 50); // near start
    }

    #[test]
    fn animation_manager_tracks_active() {
        let mut mgr = AnimationManager::default();
        assert!(!mgr.has_active());

        mgr.animate_bg(0, Color::new(0, 0, 0, 255), Color::new(255, 255, 255, 255), 100);
        assert!(mgr.has_active());

        // get_bg should return a color
        let color = mgr.get_bg(0);
        assert!(color.is_some());
    }

    #[test]
    fn lerp_u8_endpoints() {
        assert_eq!(lerp_u8(0, 255, 0.0), 0);
        assert_eq!(lerp_u8(0, 255, 1.0), 255);
        assert_eq!(lerp_u8(100, 200, 0.5), 150);
    }
}
