//! Touch/mouse gesture helpers (swipe detection).

use velvet_math::Vec2;

/// Finger / pointer id for multi-touch (0 = primary mouse).
pub type PointerId = u32;

/// Raw pointer sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerSample {
    /// Pointer id.
    pub id: PointerId,
    /// Position in screen/logical pixels.
    pub position: Vec2,
    /// Time seconds (monotonic).
    pub time: f64,
}

/// Detected swipe direction (4-way + diagonals optional).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwipeDirection {
    /// Up.
    Up,
    /// Down.
    Down,
    /// Left.
    Left,
    /// Right.
    Right,
    /// Up-left.
    UpLeft,
    /// Up-right.
    UpRight,
    /// Down-left.
    DownLeft,
    /// Down-right.
    DownRight,
}

impl SwipeDirection {
    /// Unit vector for the cardinal/diagonal.
    pub fn as_vec2(self) -> Vec2 {
        match self {
            Self::Up => Vec2::Y,
            Self::Down => Vec2::NEG_Y,
            Self::Left => Vec2::NEG_X,
            Self::Right => Vec2::X,
            Self::UpLeft => Vec2::new(-1.0, 1.0).normalize_or_zero(),
            Self::UpRight => Vec2::new(1.0, 1.0).normalize_or_zero(),
            Self::DownLeft => Vec2::new(-1.0, -1.0).normalize_or_zero(),
            Self::DownRight => Vec2::new(1.0, -1.0).normalize_or_zero(),
        }
    }

    /// From a delta vector; `allow_diagonal` enables 8-way.
    pub fn from_delta(delta: Vec2, allow_diagonal: bool) -> Option<Self> {
        if delta.length_squared() < 1e-12 {
            return None;
        }
        let angle = delta.y.atan2(delta.x); // -pi..pi, 0 = right
        let deg = angle.to_degrees();
        if allow_diagonal {
            // 8 sectors of 45°
            let d = ((deg + 360.0) % 360.0 + 22.5) % 360.0;
            let sector = (d / 45.0) as i32;
            Some(match sector {
                0 => Self::Right,
                1 => Self::UpRight,
                2 => Self::Up,
                3 => Self::UpLeft,
                4 => Self::Left,
                5 => Self::DownLeft,
                6 => Self::Down,
                _ => Self::DownRight,
            })
        } else {
            // 4-way
            if delta.x.abs() > delta.y.abs() {
                if delta.x > 0.0 {
                    Some(Self::Right)
                } else {
                    Some(Self::Left)
                }
            } else if delta.y > 0.0 {
                Some(Self::Up)
            } else {
                Some(Self::Down)
            }
        }
    }
}

/// Configuration for swipe detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwipeConfig {
    /// Minimum distance in pixels.
    pub min_distance: f32,
    /// Maximum duration seconds.
    pub max_duration: f64,
    /// Minimum average speed (px/s), 0 to disable.
    pub min_speed: f32,
    /// Allow 8-way diagonals.
    pub allow_diagonal: bool,
}

impl Default for SwipeConfig {
    fn default() -> Self {
        Self {
            min_distance: 40.0,
            max_duration: 0.5,
            min_speed: 0.0,
            allow_diagonal: false,
        }
    }
}

/// A completed swipe gesture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwipeGesture {
    /// Pointer id.
    pub pointer: PointerId,
    /// Direction.
    pub direction: SwipeDirection,
    /// Start position.
    pub start: Vec2,
    /// End position.
    pub end: Vec2,
    /// Delta end - start.
    pub delta: Vec2,
    /// Duration seconds.
    pub duration: f64,
    /// Average speed px/s.
    pub speed: f32,
}

/// Tracks active pointers and emits swipes on release.
#[derive(Debug, Clone, Default)]
pub struct SwipeDetector {
    /// Config.
    pub config: SwipeConfig,
    active: Vec<ActiveStroke>,
    pending: Vec<SwipeGesture>,
}

#[derive(Debug, Clone)]
struct ActiveStroke {
    id: PointerId,
    start: PointerSample,
    last: PointerSample,
}

impl SwipeDetector {
    /// Create with config.
    pub fn new(config: SwipeConfig) -> Self {
        Self {
            config,
            active: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// Pointer down.
    pub fn pointer_down(&mut self, sample: PointerSample) {
        self.active.retain(|s| s.id != sample.id);
        self.active.push(ActiveStroke {
            id: sample.id,
            start: sample,
            last: sample,
        });
    }

    /// Pointer move.
    pub fn pointer_move(&mut self, sample: PointerSample) {
        if let Some(s) = self.active.iter_mut().find(|s| s.id == sample.id) {
            s.last = sample;
        }
    }

    /// Pointer up — may produce a swipe.
    pub fn pointer_up(&mut self, sample: PointerSample) -> Option<SwipeGesture> {
        let idx = self.active.iter().position(|s| s.id == sample.id)?;
        let stroke = self.active.swap_remove(idx);
        let start = stroke.start;
        let end = sample;
        let delta = end.position - start.position;
        let duration = (end.time - start.time).max(0.0);
        let dist = delta.length();
        let speed = if duration > 1e-6 {
            dist / duration as f32
        } else {
            f32::MAX
        };
        if dist < self.config.min_distance {
            return None;
        }
        if duration > self.config.max_duration {
            return None;
        }
        if self.config.min_speed > 0.0 && speed < self.config.min_speed {
            return None;
        }
        let direction = SwipeDirection::from_delta(delta, self.config.allow_diagonal)?;
        let g = SwipeGesture {
            pointer: sample.id,
            direction,
            start: start.position,
            end: end.position,
            delta,
            duration,
            speed,
        };
        self.pending.push(g);
        Some(g)
    }

    /// Cancel a pointer without emitting.
    pub fn pointer_cancel(&mut self, id: PointerId) {
        self.active.retain(|s| s.id != id);
    }

    /// Drain detected swipes since last drain.
    pub fn drain(&mut self) -> Vec<SwipeGesture> {
        std::mem::take(&mut self.pending)
    }

    /// Active pointer count.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Clear all.
    pub fn clear(&mut self) {
        self.active.clear();
        self.pending.clear();
    }
}

/// Simple tap detector (down+up with small movement and short time).
#[derive(Debug, Clone)]
pub struct TapDetector {
    /// Max movement.
    pub max_distance: f32,
    /// Max duration seconds.
    pub max_duration: f64,
    down: Option<PointerSample>,
    taps: Vec<PointerSample>,
}

impl Default for TapDetector {
    fn default() -> Self {
        Self {
            max_distance: 15.0,
            max_duration: 0.35,
            down: None,
            taps: Vec::new(),
        }
    }
}

impl TapDetector {
    /// Down.
    pub fn pointer_down(&mut self, sample: PointerSample) {
        self.down = Some(sample);
    }

    /// Up; returns tap position if recognized.
    pub fn pointer_up(&mut self, sample: PointerSample) -> Option<Vec2> {
        let start = self.down.take()?;
        if start.id != sample.id {
            return None;
        }
        let delta = sample.position - start.position;
        let duration = sample.time - start.time;
        if delta.length() <= self.max_distance && duration <= self.max_duration {
            self.taps.push(sample);
            Some(sample.position)
        } else {
            None
        }
    }

    /// Drain taps.
    pub fn drain(&mut self) -> Vec<PointerSample> {
        std::mem::take(&mut self.taps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swipe_right() {
        let mut d = SwipeDetector::new(SwipeConfig::default());
        d.pointer_down(PointerSample {
            id: 0,
            position: Vec2::ZERO,
            time: 0.0,
        });
        d.pointer_move(PointerSample {
            id: 0,
            position: Vec2::new(50.0, 2.0),
            time: 0.1,
        });
        let g = d
            .pointer_up(PointerSample {
                id: 0,
                position: Vec2::new(80.0, 0.0),
                time: 0.2,
            })
            .unwrap();
        assert_eq!(g.direction, SwipeDirection::Right);
        assert_eq!(d.drain().len(), 1);
    }

    #[test]
    fn swipe_too_short() {
        let mut d = SwipeDetector::new(SwipeConfig::default());
        d.pointer_down(PointerSample {
            id: 0,
            position: Vec2::ZERO,
            time: 0.0,
        });
        assert!(d
            .pointer_up(PointerSample {
                id: 0,
                position: Vec2::new(5.0, 0.0),
                time: 0.1,
            })
            .is_none());
    }

    #[test]
    fn direction_from_delta() {
        assert_eq!(
            SwipeDirection::from_delta(Vec2::new(0.0, 1.0), false),
            Some(SwipeDirection::Up)
        );
        assert_eq!(
            SwipeDirection::from_delta(Vec2::new(-1.0, 0.0), false),
            Some(SwipeDirection::Left)
        );
    }

    #[test]
    fn tap_detect() {
        let mut t = TapDetector::default();
        t.pointer_down(PointerSample {
            id: 0,
            position: Vec2::new(10.0, 10.0),
            time: 1.0,
        });
        let p = t
            .pointer_up(PointerSample {
                id: 0,
                position: Vec2::new(12.0, 11.0),
                time: 1.1,
            })
            .unwrap();
        assert!((p.x - 12.0).abs() < 1e-5);
    }
}
