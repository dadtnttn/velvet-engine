//! Screen / scene transition kinds with duration and progress.

use serde::{Deserialize, Serialize};

/// Visual transition style between backgrounds or scenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransitionKind {
    /// No visual transition (instant cut).
    #[default]
    None,
    /// Fade through black (or solid color).
    Fade,
    /// Cross-dissolve between images.
    Dissolve,
    /// Directional wipe.
    Wipe,
    /// Sprite / camera move (position lerp).
    Move,
}

/// Wipe direction when using [`TransitionKind::Wipe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum WipeDirection {
    /// Left to right.
    #[default]
    LeftToRight,
    /// Right to left.
    RightToLeft,
    /// Top to bottom.
    TopToBottom,
    /// Bottom to top.
    BottomToTop,
}

/// Active or completed transition state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    /// Kind of transition.
    pub kind: TransitionKind,
    /// Total duration in seconds (ignored for None).
    pub duration: f32,
    /// Elapsed seconds.
    pub elapsed: f32,
    /// Wipe direction (if applicable).
    pub wipe: WipeDirection,
    /// Optional solid color for fade (RGBA 0..=1 components packed as [r,g,b,a]).
    pub fade_color: [f32; 4],
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            kind: TransitionKind::None,
            duration: 0.0,
            elapsed: 0.0,
            wipe: WipeDirection::default(),
            fade_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl Transition {
    /// Instant cut (already complete).
    pub fn none() -> Self {
        Self {
            kind: TransitionKind::None,
            duration: 0.0,
            elapsed: 0.0,
            ..Default::default()
        }
    }

    /// Fade of the given duration.
    pub fn fade(duration: f32) -> Self {
        Self {
            kind: TransitionKind::Fade,
            duration: duration.max(0.0),
            elapsed: 0.0,
            ..Default::default()
        }
    }

    /// Dissolve of the given duration.
    pub fn dissolve(duration: f32) -> Self {
        Self {
            kind: TransitionKind::Dissolve,
            duration: duration.max(0.0),
            elapsed: 0.0,
            ..Default::default()
        }
    }

    /// Wipe of the given duration and direction.
    pub fn wipe(duration: f32, direction: WipeDirection) -> Self {
        Self {
            kind: TransitionKind::Wipe,
            duration: duration.max(0.0),
            elapsed: 0.0,
            wipe: direction,
            ..Default::default()
        }
    }

    /// Move / slide of the given duration (direction encodes axis bias).
    pub fn r#move(duration: f32, direction: WipeDirection) -> Self {
        Self {
            kind: TransitionKind::Move,
            duration: duration.max(0.0),
            elapsed: 0.0,
            wipe: direction,
            ..Default::default()
        }
    }

    /// Builder: set fade color.
    pub fn with_fade_color(mut self, rgba: [f32; 4]) -> Self {
        self.fade_color = rgba;
        self
    }

    /// Linear progress in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        if self.kind == TransitionKind::None || self.duration <= 0.0 {
            return 1.0;
        }
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Smoothstep eased progress for nicer fades.
    pub fn eased_progress(&self) -> f32 {
        let t = self.progress();
        t * t * (3.0 - 2.0 * t)
    }

    /// Whether the transition has finished.
    pub fn is_finished(&self) -> bool {
        self.progress() >= 1.0
    }

    /// Whether still running.
    pub fn is_active(&self) -> bool {
        !self.is_finished()
    }

    /// Advance by `dt` seconds. Returns progress after the tick.
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.kind == TransitionKind::None || self.duration <= 0.0 {
            self.elapsed = self.duration;
            return 1.0;
        }
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.duration);
        self.progress()
    }

    /// Skip to the end.
    pub fn skip(&mut self) {
        self.elapsed = self.duration.max(0.0);
    }

    /// Reset elapsed to zero (replay).
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    /// Opacity of the outgoing image during dissolve/fade (1 → 0).
    pub fn outgoing_alpha(&self) -> f32 {
        match self.kind {
            TransitionKind::None => 0.0,
            TransitionKind::Fade => {
                // First half fade out to color, second half fade in.
                let t = self.eased_progress();
                if t < 0.5 {
                    1.0 - t * 2.0
                } else {
                    0.0
                }
            }
            TransitionKind::Dissolve => 1.0 - self.eased_progress(),
            TransitionKind::Wipe | TransitionKind::Move => 1.0,
        }
    }

    /// Opacity of the incoming image during dissolve/fade (0 → 1).
    pub fn incoming_alpha(&self) -> f32 {
        match self.kind {
            TransitionKind::None => 1.0,
            TransitionKind::Fade => {
                let t = self.eased_progress();
                if t < 0.5 {
                    0.0
                } else {
                    (t - 0.5) * 2.0
                }
            }
            TransitionKind::Dissolve => self.eased_progress(),
            TransitionKind::Wipe | TransitionKind::Move => 1.0,
        }
    }

    /// Move progress (0 = start pose, 1 = end pose).
    pub fn move_amount(&self) -> f32 {
        if self.kind != TransitionKind::Move {
            return if self.is_finished() { 1.0 } else { 0.0 };
        }
        self.eased_progress()
    }

    /// Wipe coverage (0 = fully old, 1 = fully new). Direction encodes edge.
    pub fn wipe_amount(&self) -> f32 {
        if self.kind != TransitionKind::Wipe {
            return if self.is_finished() { 1.0 } else { 0.0 };
        }
        self.eased_progress()
    }
}

/// Queue of transitions (play one after another).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransitionQueue {
    /// Pending transitions; index 0 is active.
    queue: Vec<Transition>,
}

impl TransitionQueue {
    /// Empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a transition.
    pub fn push(&mut self, transition: Transition) {
        self.queue.push(transition);
    }

    /// Active transition, if any.
    pub fn current(&self) -> Option<&Transition> {
        self.queue.first()
    }

    /// Mutable active transition.
    pub fn current_mut(&mut self) -> Option<&mut Transition> {
        self.queue.first_mut()
    }

    /// Whether a transition is playing.
    pub fn is_busy(&self) -> bool {
        self.queue.first().map(|t| t.is_active()).unwrap_or(false)
    }

    /// Tick active transition; pops finished ones. Returns true if something finished this frame.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut finished = false;
        if let Some(t) = self.queue.first_mut() {
            t.tick(dt);
            if t.is_finished() {
                self.queue.remove(0);
                finished = true;
            }
        }
        finished
    }

    /// Skip current transition.
    pub fn skip_current(&mut self) {
        if let Some(t) = self.queue.first_mut() {
            t.skip();
            self.queue.remove(0);
        }
    }

    /// Clear all.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Number of queued (including active).
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_instant() {
        let t = Transition::none();
        assert!(t.is_finished());
        assert_eq!(t.progress(), 1.0);
        assert_eq!(t.incoming_alpha(), 1.0);
    }

    #[test]
    fn fade_progress_and_alphas() {
        let mut t = Transition::fade(1.0);
        assert_eq!(t.progress(), 0.0);
        t.tick(0.25);
        assert!((t.progress() - 0.25).abs() < 1e-5);
        assert!(t.outgoing_alpha() > 0.0);
        assert_eq!(t.incoming_alpha(), 0.0);
        t.tick(0.5);
        // elapsed 0.75
        assert!(t.incoming_alpha() > 0.0);
        t.tick(1.0);
        assert!(t.is_finished());
        assert!((t.incoming_alpha() - 1.0).abs() < 1e-5);
        assert!((t.outgoing_alpha() - 0.0).abs() < 1e-5);
    }

    #[test]
    fn dissolve_crossfade() {
        let mut t = Transition::dissolve(2.0);
        t.tick(1.0);
        let out = t.outgoing_alpha();
        let inc = t.incoming_alpha();
        assert!((out + inc - 1.0).abs() < 1e-4);
    }

    #[test]
    fn wipe_amount() {
        let mut t = Transition::wipe(1.0, WipeDirection::LeftToRight);
        assert_eq!(t.wipe_amount(), 0.0);
        t.tick(0.5);
        assert!(t.wipe_amount() > 0.0 && t.wipe_amount() < 1.0);
        t.skip();
        assert_eq!(t.wipe_amount(), 1.0);
    }

    #[test]
    fn queue_sequences() {
        let mut q = TransitionQueue::new();
        q.push(Transition::fade(0.2));
        q.push(Transition::dissolve(0.2));
        assert!(q.is_busy());
        assert_eq!(q.len(), 2);
        // Advance past first
        for _ in 0..10 {
            let _ = q.tick(0.05);
        }
        // Should be on second or done
        assert!(q.len() <= 1);
        q.skip_current();
        assert!(q.is_empty());
    }

    #[test]
    fn restart_and_eased() {
        let mut t = Transition::fade(1.0);
        t.tick(0.5);
        let e = t.eased_progress();
        assert!(e > 0.0 && e < 1.0);
        t.restart();
        assert_eq!(t.progress(), 0.0);
    }

    #[test]
    fn serde_roundtrip() {
        let t =
            Transition::wipe(0.4, WipeDirection::TopToBottom).with_fade_color([1.0, 0.0, 0.0, 1.0]);
        let json = serde_json::to_string(&t).unwrap();
        let back: Transition = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, TransitionKind::Wipe);
        assert_eq!(back.wipe, WipeDirection::TopToBottom);
    }
}
