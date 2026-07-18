//! Record and playback action frames for tests and demos.

use serde::{Deserialize, Serialize};

use crate::action::{ActionId, ActionState, ActionValue};
use crate::state::InputState;

/// One recorded frame of action values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionFrame {
    /// Frame index (monotonic).
    pub frame: u64,
    /// Delta time seconds (optional for pure action replay).
    pub dt: f32,
    /// Action id → value snapshots.
    pub actions: Vec<(ActionId, ActionValue)>,
}

impl ActionFrame {
    /// Empty frame.
    pub fn new(frame: u64, dt: f32) -> Self {
        Self {
            frame,
            dt,
            actions: Vec::new(),
        }
    }

    /// Push action sample.
    pub fn push(&mut self, id: impl Into<ActionId>, value: ActionValue) {
        self.actions.push((id.into(), value));
    }

    /// Lookup value.
    pub fn get(&self, id: &ActionId) -> Option<ActionValue> {
        self.actions.iter().find(|(a, _)| a == id).map(|(_, v)| *v)
    }
}

/// Records resolved actions from [`InputState`] each frame.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InputRecorder {
    frames: Vec<ActionFrame>,
    frame_counter: u64,
    /// When false, `capture` is a no-op.
    pub enabled: bool,
}

impl InputRecorder {
    /// Create enabled recorder.
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Capture currently resolved actions after `end_frame`.
    pub fn capture(&mut self, input: &InputState, dt: f32) {
        if !self.enabled {
            return;
        }
        let mut frame = ActionFrame::new(self.frame_counter, dt);
        self.frame_counter += 1;
        for (id, state) in input.actions_iter() {
            if let Some(v) = state.value {
                frame.push(id.clone(), v);
            }
        }
        self.frames.push(frame);
    }

    /// Recorded frames.
    pub fn frames(&self) -> &[ActionFrame] {
        &self.frames
    }

    /// Clear recording.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.frame_counter = 0;
    }

    /// Frame count.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    /// Parse JSON.
    pub fn from_json(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

/// Plays back recorded frames into synthetic action states.
#[derive(Debug, Clone, Default)]
pub struct InputPlayback {
    frames: Vec<ActionFrame>,
    cursor: usize,
    /// Current applied actions.
    current: Vec<(ActionId, ActionState)>,
    /// Finished.
    finished: bool,
}

impl InputPlayback {
    /// From recorder frames.
    pub fn new(frames: Vec<ActionFrame>) -> Self {
        Self {
            frames,
            cursor: 0,
            current: Vec::new(),
            finished: false,
        }
    }

    /// From recorder.
    pub fn from_recorder(rec: &InputRecorder) -> Self {
        Self::new(rec.frames().to_vec())
    }

    /// Advance one frame; returns false when finished.
    pub fn advance(&mut self) -> bool {
        if self.cursor >= self.frames.len() {
            self.finished = true;
            self.current.clear();
            return false;
        }
        let frame = &self.frames[self.cursor];
        self.cursor += 1;
        let mut next = Vec::new();
        for (id, value) in &frame.actions {
            let mut state = ActionState::default();
            // Seed prior active so `update` computes correct edges.
            if let Some((_, prev)) = self.current.iter().find(|(a, _)| a == id) {
                state.active = prev.active;
            }
            state.update(*value, 0.5);
            next.push((id.clone(), state));
        }
        // Mark released for ids that disappeared.
        for (id, prev) in &self.current {
            if !next.iter().any(|(a, _)| a == id) && prev.active {
                let mut state = *prev;
                state.update(ActionValue::Bool(false), 0.5);
                next.push((id.clone(), state));
            }
        }
        self.current = next;
        true
    }

    /// Action state for id.
    pub fn action(&self, id: impl Into<ActionId>) -> ActionState {
        let id = id.into();
        self.current
            .iter()
            .find(|(a, _)| *a == id)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Just pressed convenience.
    pub fn just_pressed(&self, id: impl Into<ActionId>) -> bool {
        self.action(id).just_pressed
    }

    /// Finished playback.
    pub fn is_finished(&self) -> bool {
        self.finished || self.cursor >= self.frames.len() && self.current.is_empty()
    }

    /// Cursor index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Total frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Restart.
    pub fn restart(&mut self) {
        self.cursor = 0;
        self.current.clear();
        self.finished = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::KeyCode;
    use crate::builtin;

    #[test]
    fn record_and_playback_confirm() {
        let mut input = InputState::with_defaults();
        let mut rec = InputRecorder::new();

        input.begin_frame();
        input.key_down(KeyCode::Enter);
        input.end_frame();
        rec.capture(&input, 1.0 / 60.0);

        input.begin_frame();
        input.end_frame();
        rec.capture(&input, 1.0 / 60.0);

        assert_eq!(rec.len(), 2);
        let mut play = InputPlayback::from_recorder(&rec);
        assert!(play.advance());
        assert!(play.just_pressed(builtin::CONFIRM) || play.action(builtin::CONFIRM).active);
        assert!(play.advance());
        // Second frame still held — just_pressed false
        assert!(play.action(builtin::CONFIRM).active);
        assert!(!play.just_pressed(builtin::CONFIRM));
    }

    #[test]
    fn json_roundtrip() {
        let mut rec = InputRecorder::new();
        let mut f = ActionFrame::new(0, 0.016);
        f.push("jump", ActionValue::Bool(true));
        rec.frames.push(f);
        let json = rec.to_json().unwrap();
        let rec2 = InputRecorder::from_json(&json).unwrap();
        assert_eq!(rec2.len(), 1);
    }
}
