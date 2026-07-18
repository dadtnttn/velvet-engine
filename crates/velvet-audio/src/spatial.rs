//! 2D spatial attenuation and stereo pan from a listener.

use velvet_math::Vec2;

/// Spatial audio parameters for 2D games.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialListener {
    /// Listener world position.
    pub position: Vec2,
    /// Distance at which attenuation reaches zero (full silence beyond).
    pub max_distance: f32,
    /// Distance with full volume (no attenuation inside).
    pub min_distance: f32,
    /// Rolloff exponent (1 = linear between min/max).
    pub rolloff: f32,
}

impl Default for SpatialListener {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            max_distance: 800.0,
            min_distance: 50.0,
            rolloff: 1.0,
        }
    }
}

impl SpatialListener {
    /// Create at position with radius.
    pub fn new(position: Vec2, max_distance: f32) -> Self {
        Self {
            position,
            max_distance: max_distance.max(1.0),
            ..Default::default()
        }
    }
}

/// Result of a spatial evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialGain {
    /// Linear attenuation `0..=1`.
    pub attenuation: f32,
    /// Stereo pan `-1..=1` (left to right).
    pub pan: f32,
    /// Distance from listener.
    pub distance: f32,
}

impl SpatialGain {
    /// Combined left/right channel scales from pan (constant power-ish).
    pub fn channel_gains(self) -> (f32, f32) {
        // Equal-power pan: L = cos((pan+1)*pi/4), R = sin((pan+1)*pi/4)
        let t = ((self.pan + 1.0) * 0.5).clamp(0.0, 1.0);
        let angle = t * std::f32::consts::FRAC_PI_2;
        let left = angle.cos() * self.attenuation;
        let right = angle.sin() * self.attenuation;
        (left, right)
    }
}

/// Evaluate distance attenuation and pan for a source relative to listener.
pub fn evaluate_spatial(listener: &SpatialListener, source: Vec2) -> SpatialGain {
    let delta = source - listener.position;
    let distance = delta.length();
    let min_d = listener.min_distance.max(0.0);
    let max_d = listener.max_distance.max(min_d + 1e-3);
    let attenuation = if distance <= min_d {
        1.0
    } else if distance >= max_d {
        0.0
    } else {
        let t = (distance - min_d) / (max_d - min_d);
        (1.0 - t).powf(listener.rolloff.max(0.01)).clamp(0.0, 1.0)
    };
    // Pan from X delta: clamp by max_distance so far sources don't hard-pan only by angle.
    let pan = if distance < 1e-5 {
        0.0
    } else {
        (delta.x / max_d).clamp(-1.0, 1.0)
    };
    SpatialGain {
        attenuation,
        pan,
        distance,
    }
}

/// Convenience: effective voice volume with spatial attenuation applied.
pub fn spatialize_volume(base_volume: f32, listener: &SpatialListener, source: Vec2) -> f32 {
    base_volume * evaluate_spatial(listener, source).attenuation
}

/// Convenience: pan for a source.
pub fn spatialize_pan(listener: &SpatialListener, source: Vec2) -> f32 {
    evaluate_spatial(listener, source).pan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_volume_inside_min() {
        let l = SpatialListener::new(Vec2::ZERO, 100.0);
        let g = evaluate_spatial(&l, Vec2::new(10.0, 0.0));
        assert!((g.attenuation - 1.0).abs() < 1e-5);
    }

    #[test]
    fn silent_beyond_max() {
        let l = SpatialListener {
            min_distance: 0.0,
            max_distance: 100.0,
            ..Default::default()
        };
        let g = evaluate_spatial(&l, Vec2::new(500.0, 0.0));
        assert_eq!(g.attenuation, 0.0);
    }

    #[test]
    fn pan_follows_x() {
        let l = SpatialListener::new(Vec2::ZERO, 100.0);
        let right = evaluate_spatial(&l, Vec2::new(100.0, 0.0));
        let left = evaluate_spatial(&l, Vec2::new(-100.0, 0.0));
        assert!(right.pan > 0.5);
        assert!(left.pan < -0.5);
    }

    #[test]
    fn channel_gains_sum_positive() {
        let g = SpatialGain {
            attenuation: 1.0,
            pan: 0.0,
            distance: 0.0,
        };
        let (l, r) = g.channel_gains();
        assert!((l - r).abs() < 1e-4);
        assert!(l > 0.5);
    }
}
