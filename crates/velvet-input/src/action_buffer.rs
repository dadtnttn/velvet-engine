//! Fighting-game style action input history buffer.

use std::collections::VecDeque;

use crate::action::ActionId;

/// One sampled frame of action edges / holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionBufferFrame {
    /// Frame index (engine or local).
    pub frame: u64,
    /// Actions pressed this frame.
    pub pressed: Vec<ActionId>,
    /// Actions held this frame.
    pub held: Vec<ActionId>,
    /// Actions released this frame.
    pub released: Vec<ActionId>,
}

impl ActionBufferFrame {
    /// Empty frame.
    pub fn empty(frame: u64) -> Self {
        Self {
            frame,
            pressed: Vec::new(),
            held: Vec::new(),
            released: Vec::new(),
        }
    }

    /// Whether action was pressed this frame.
    pub fn was_pressed(&self, action: &ActionId) -> bool {
        self.pressed.iter().any(|a| a == action)
    }

    /// Whether action held.
    pub fn is_held(&self, action: &ActionId) -> bool {
        self.held.iter().any(|a| a == action)
    }
}

/// Ring buffer of recent action frames for motion / special move matching.
#[derive(Debug, Clone)]
pub struct ActionBuffer {
    frames: VecDeque<ActionBufferFrame>,
    capacity: usize,
}

impl Default for ActionBuffer {
    fn default() -> Self {
        Self::new(60)
    }
}

impl ActionBuffer {
    /// Create with capacity (frames).
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
        }
    }

    /// Capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Push a completed frame sample.
    pub fn push(&mut self, frame: ActionBufferFrame) {
        if self.frames.len() >= self.capacity {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    /// Clear history.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Newest frame.
    pub fn latest(&self) -> Option<&ActionBufferFrame> {
        self.frames.back()
    }

    /// Iterate oldest → newest.
    pub fn iter(&self) -> impl Iterator<Item = &ActionBufferFrame> {
        self.frames.iter()
    }

    /// Window of the last `n` frames (newest last).
    pub fn last_n(&self, n: usize) -> Vec<&ActionBufferFrame> {
        let n = n.min(self.frames.len());
        self.frames
            .iter()
            .rev()
            .take(n)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Whether `action` was pressed within the last `window` frames.
    pub fn pressed_within(&self, action: &ActionId, window: usize) -> bool {
        self.last_n(window).iter().any(|f| f.was_pressed(action))
    }

    /// Frames since `action` was last pressed (`None` if not in buffer).
    pub fn frames_since_pressed(&self, action: &ActionId) -> Option<usize> {
        for (i, f) in self.frames.iter().rev().enumerate() {
            if f.was_pressed(action) {
                return Some(i);
            }
        }
        None
    }

    /// Match a sequence of action presses in order within a max span of frames.
    ///
    /// Each step is an action that must be pressed (not necessarily consecutive frames,
    /// but order-preserving) inside the last `max_span` frames.
    pub fn match_sequence(&self, sequence: &[ActionId], max_span: usize) -> bool {
        if sequence.is_empty() {
            return true;
        }
        let window = self.last_n(max_span);
        if window.is_empty() {
            return false;
        }
        let mut seq_i = 0;
        for frame in window {
            if frame.was_pressed(&sequence[seq_i]) {
                seq_i += 1;
                if seq_i >= sequence.len() {
                    return true;
                }
            }
        }
        false
    }

    /// Match a charge: action held continuously for at least `min_frames`, then released,
    /// optionally followed by a press of `follow_up` within `follow_window`.
    pub fn match_charge(
        &self,
        charge: &ActionId,
        min_frames: usize,
        follow_up: Option<&ActionId>,
        follow_window: usize,
    ) -> bool {
        // Scan for a run of held charge ending with release.
        let frames: Vec<_> = self.frames.iter().collect();
        if frames.len() < min_frames {
            return false;
        }
        let mut hold_run = 0usize;
        let mut released_at: Option<usize> = None;
        for (i, f) in frames.iter().enumerate() {
            if f.is_held(charge) || f.was_pressed(charge) {
                hold_run += 1;
            } else {
                if hold_run >= min_frames && f.released.iter().any(|a| a == charge) {
                    released_at = Some(i);
                    break;
                }
                // Also treat missing hold after long hold as release boundary.
                if hold_run >= min_frames {
                    released_at = Some(i);
                    break;
                }
                hold_run = 0;
            }
        }
        // End of buffer still holding doesn't complete charge unless released.
        let Some(rel_i) = released_at else {
            return false;
        };
        let Some(follow) = follow_up else {
            return true;
        };
        let end = (rel_i + 1 + follow_window).min(frames.len());
        frames[rel_i + 1..end].iter().any(|f| f.was_pressed(follow))
    }

    /// Record helper: build frame from edge lists.
    pub fn record(
        &mut self,
        frame: u64,
        pressed: impl IntoIterator<Item = ActionId>,
        held: impl IntoIterator<Item = ActionId>,
        released: impl IntoIterator<Item = ActionId>,
    ) {
        self.push(ActionBufferFrame {
            frame,
            pressed: pressed.into_iter().collect(),
            held: held.into_iter().collect(),
            released: released.into_iter().collect(),
        });
    }
}

/// Motion notation step for sequence builders (down, down-forward, forward, punch…).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionStep {
    /// Action id for this step.
    pub action: ActionId,
}

impl MotionStep {
    /// Create.
    pub fn new(action: impl Into<ActionId>) -> Self {
        Self {
            action: action.into(),
        }
    }
}

/// Named special-move definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialMove {
    /// Debug name.
    pub name: String,
    /// Ordered steps.
    pub steps: Vec<ActionId>,
    /// Max frames for full sequence.
    pub max_span: usize,
}

impl SpecialMove {
    /// Create.
    pub fn new(name: impl Into<String>, steps: Vec<ActionId>, max_span: usize) -> Self {
        Self {
            name: name.into(),
            steps,
            max_span,
        }
    }

    /// Try match against buffer.
    pub fn matches(&self, buffer: &ActionBuffer) -> bool {
        buffer.match_sequence(&self.steps, self.max_span)
    }
}

/// Check multiple specials; returns first match name.
pub fn match_first_special<'a>(buffer: &ActionBuffer, moves: &'a [SpecialMove]) -> Option<&'a str> {
    moves
        .iter()
        .find(|m| m.matches(buffer))
        .map(|m| m.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aid(s: &str) -> ActionId {
        ActionId::new(s)
    }

    #[test]
    fn sequence_qcf() {
        let mut buf = ActionBuffer::new(30);
        // quarter-circle forward: down, down-forward, forward, punch
        buf.record(1, [aid("down")], [aid("down")], []);
        buf.record(
            2,
            [aid("down_forward")],
            [aid("down_forward")],
            [aid("down")],
        );
        buf.record(3, [aid("forward")], [aid("forward")], [aid("down_forward")]);
        buf.record(4, [aid("punch")], [], [aid("forward")]);
        let seq = [
            aid("down"),
            aid("down_forward"),
            aid("forward"),
            aid("punch"),
        ];
        assert!(buf.match_sequence(&seq, 10));
        assert!(!buf.match_sequence(&[aid("kick")], 10));
    }

    #[test]
    fn pressed_within() {
        let mut buf = ActionBuffer::new(10);
        buf.record(1, [aid("a")], [], []);
        buf.record(2, [], [], []);
        buf.record(3, [], [], []);
        assert!(buf.pressed_within(&aid("a"), 3));
        assert_eq!(buf.frames_since_pressed(&aid("a")), Some(2));
    }

    #[test]
    fn charge_move() {
        let mut buf = ActionBuffer::new(40);
        for i in 0..10 {
            buf.record(i, [], [aid("back")], []);
        }
        buf.record(10, [], [], [aid("back")]);
        buf.record(11, [aid("forward")], [], []);
        assert!(buf.match_charge(&aid("back"), 8, Some(&aid("forward")), 5));
    }

    #[test]
    fn special_move_list() {
        let mut buf = ActionBuffer::new(20);
        buf.record(1, [aid("down")], [], []);
        buf.record(2, [aid("forward")], [], []);
        buf.record(3, [aid("punch")], [], []);
        let moves = vec![
            SpecialMove::new(
                "fireball",
                vec![aid("down"), aid("forward"), aid("punch")],
                12,
            ),
            SpecialMove::new(
                "flash",
                vec![aid("forward"), aid("forward"), aid("kick")],
                12,
            ),
        ];
        assert_eq!(match_first_special(&buf, &moves), Some("fireball"));
    }
}
