//! Time-based fade with a mandatory presented-black frame before switching.
use std::time::{Duration, Instant};

pub const FADE_OUT: Duration = Duration::from_millis(180);
pub const FADE_IN: Duration = Duration::from_millis(280);

pub struct SceneTransition {
    pub started: Instant,
    pub revealed: Option<Instant>,
    pub black_presented: bool,
    reduced_motion: bool,
}

fn eased(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl SceneTransition {
    pub fn new(now: Instant, reduced_motion: bool) -> Self {
        Self {
            started: now,
            revealed: None,
            black_presented: false,
            reduced_motion,
        }
    }
    pub fn outgoing(&self, now: Instant) -> f32 {
        if self.reduced_motion {
            return 0.0;
        }
        1.0 - eased(
            now.saturating_duration_since(self.started).as_secs_f32() / FADE_OUT.as_secs_f32(),
        )
    }
    pub fn reveal_if_ready(&mut self, now: Instant, loaded: bool) {
        if loaded && self.black_presented && self.revealed.is_none() {
            self.revealed = Some(now);
        }
    }
    pub fn incoming(&self, now: Instant) -> f32 {
        self.revealed.map_or(0.0, |start| {
            if self.reduced_motion {
                1.0
            } else {
                eased(now.saturating_duration_since(start).as_secs_f32() / FADE_IN.as_secs_f32())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cannot_reveal_before_black_or_before_load() {
        let now = Instant::now();
        let mut fade = SceneTransition::new(now, false);
        assert_eq!(fade.outgoing(now), 1.0);
        assert_eq!(fade.outgoing(now + FADE_OUT), 0.0);
        fade.reveal_if_ready(now + FADE_OUT, true);
        assert!(fade.revealed.is_none());
        fade.black_presented = true;
        fade.reveal_if_ready(now + FADE_OUT, false);
        assert!(fade.revealed.is_none());
        fade.reveal_if_ready(now + FADE_OUT, true);
        assert_eq!(fade.incoming(now + FADE_OUT), 0.0);
        assert_eq!(fade.incoming(now + FADE_OUT + FADE_IN), 1.0);
    }
    #[test]
    fn reduced_motion_still_waits_for_a_ready_scene() {
        let now = Instant::now();
        let mut fade = SceneTransition::new(now, true);
        assert_eq!(fade.outgoing(now), 0.0);
        assert_eq!(fade.incoming(now), 0.0);
        fade.black_presented = true;
        fade.reveal_if_ready(now, true);
        assert_eq!(fade.incoming(now), 1.0);
    }
}
