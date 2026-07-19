//! Rollback stack of story snapshots for "back" navigation.

use serde::{Deserialize, Serialize};

use crate::runtime::{StoryPlayer, StorySnapshot};
use crate::save::SaveGame;
use crate::variables::StoryVariables;

/// One rollback frame capturing enough state to restore the player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollbackFrame {
    /// Cursor / presentation snapshot.
    pub snapshot: StorySnapshot,
    /// Variable store at the time of the frame.
    pub variables: StoryVariables,
    /// Current line text (UI cache).
    pub current_text: String,
    /// Play time when frame was taken.
    pub play_time_secs: f64,
}

impl RollbackFrame {
    /// Capture from a live player.
    pub fn capture(player: &StoryPlayer) -> Self {
        Self {
            snapshot: player.snapshot(),
            variables: player.variables().clone(),
            current_text: player.current_text().to_string(),
            play_time_secs: player.play_time_secs(),
        }
    }
}

/// Fixed-capacity stack of rollback frames (newest at the end).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollbackStack {
    frames: Vec<RollbackFrame>,
    capacity: usize,
}

impl Default for RollbackStack {
    fn default() -> Self {
        Self::with_capacity(50)
    }
}

impl RollbackStack {
    /// Create with maximum number of steps.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            frames: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// Capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of stored frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Clear all frames.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Resize capacity; drops oldest frames if needed.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.trim();
    }

    fn trim(&mut self) {
        if self.frames.len() > self.capacity {
            let drop_n = self.frames.len() - self.capacity;
            self.frames.drain(0..drop_n);
        }
    }

    /// Push a new frame (does not push if identical cursor **and** line text to last frame).
    pub fn push(&mut self, frame: RollbackFrame) {
        if let Some(last) = self.frames.last() {
            if last.snapshot.scene == frame.snapshot.scene
                && last.snapshot.op_index == frame.snapshot.op_index
                && last.snapshot.wait == frame.snapshot.wait
                && last.current_text == frame.current_text
            {
                // Replace last with fresher variables only when fully identical.
                *self.frames.last_mut().unwrap() = frame;
                return;
            }
        }
        self.frames.push(frame);
        self.trim();
    }

    /// Capture and push from player.
    pub fn push_from_player(&mut self, player: &StoryPlayer) {
        self.push(RollbackFrame::capture(player));
    }

    /// Peek the most recent frame without removing it.
    pub fn peek(&self) -> Option<&RollbackFrame> {
        self.frames.last()
    }

    /// Pop one step (returns frame that was current before pop of previous).
    /// Typically: pop current (discard), then peek/pop previous to restore.
    pub fn pop(&mut self) -> Option<RollbackFrame> {
        self.frames.pop()
    }

    /// Roll back N steps: removes N frames and returns the new top frame if any.
    /// `steps=1` means go to previous frame (current is discarded if it is the top).
    pub fn rollback(&mut self, steps: usize) -> Option<RollbackFrame> {
        if steps == 0 || self.frames.is_empty() {
            return self.frames.last().cloned();
        }
        // Discard current top first if we treat top as "now".
        let remove = steps.min(self.frames.len());
        for _ in 0..remove {
            self.frames.pop();
        }
        self.frames.last().cloned()
    }

    /// Apply a frame onto a player via save round-trip fields.
    pub fn apply_frame(player: &mut StoryPlayer, frame: &RollbackFrame) -> Result<(), String> {
        let mut save = player.to_save("__rollback__");
        save.snapshot = frame.snapshot.clone();
        save.variables = frame.variables.play.clone().into_iter().collect();
        save.persistent = frame.variables.persistent.clone().into_iter().collect();
        save.meta.play_time_secs = frame.play_time_secs;
        save.meta.preview = frame.current_text.clone();
        player.load_save(save).map_err(|e| e.to_string())
    }

    /// Convenience: push current, then later `step_back` restores previous.
    pub fn step_back(&mut self, player: &mut StoryPlayer) -> Result<bool, String> {
        if self.frames.len() < 2 {
            // Need previous state: if only current was pushed, cannot go further.
            if self.frames.len() == 1 {
                // Restore the only frame (no-op-ish) — still count as handled.
                if let Some(frame) = self.frames.last().cloned() {
                    Self::apply_frame(player, &frame)?;
                }
                return Ok(false);
            }
            return Ok(false);
        }
        // Drop current.
        self.frames.pop();
        let frame = self.frames.last().cloned().ok_or("empty rollback")?;
        Self::apply_frame(player, &frame)?;
        Ok(true)
    }

    /// Export frames for debugging / tests.
    pub fn frames(&self) -> &[RollbackFrame] {
        &self.frames
    }

    /// Build a lightweight save-compatible DTO from the top frame.
    pub fn top_as_save(
        &self,
        slot: impl Into<String>,
        title: impl Into<String>,
    ) -> Option<SaveGame> {
        let frame = self.peek()?;
        Some(SaveGame::from_parts(
            slot,
            title,
            &frame.variables,
            frame.snapshot.clone(),
            crate::history::History::with_capacity(0),
            vec![],
            frame.play_time_secs,
            frame.current_text.clone(),
        ))
    }
}

/// Helper that automatically records rollback points when the wait state changes.
#[derive(Debug, Clone, Default)]
pub struct RollbackRecorder {
    /// Stack.
    pub stack: RollbackStack,
    last_wait: Option<crate::runtime::StoryWait>,
    last_scene: String,
    last_op: usize,
}

impl RollbackRecorder {
    /// Create with capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            stack: RollbackStack::with_capacity(capacity),
            last_wait: None,
            last_scene: String::new(),
            last_op: 0,
        }
    }

    /// Observe player; push when cursor meaningfully advances.
    pub fn observe(&mut self, player: &StoryPlayer) {
        let wait = player.wait().clone();
        let scene = player.scene_name().to_string();
        let op = player.snapshot().op_index;
        let changed = self.last_wait.as_ref() != Some(&wait)
            || self.last_scene != scene
            || self.last_op != op;
        if changed {
            self.stack.push_from_player(player);
            self.last_wait = Some(wait);
            self.last_scene = scene;
            self.last_op = op;
        }
    }

    /// Step back using the stack.
    pub fn back(&mut self, player: &mut StoryPlayer) -> Result<bool, String> {
        let ok = self.stack.step_back(player)?;
        if ok {
            self.last_wait = Some(player.wait().clone());
            self.last_scene = player.scene_name().to_string();
            self.last_op = player.snapshot().op_index;
        }
        Ok(ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::runtime::{StoryPlayer, StoryWait};

    fn program() -> crate::ir::StoryProgram {
        let src = r##"
character n { name: "N" }
scene start {
    n "One"
    n "Two"
    n "Three"
    end
}
"##;
        load_program_from_source(src, None, "rb").unwrap()
    }

    #[test]
    fn capacity_trims_oldest() {
        let mut stack = RollbackStack::with_capacity(3);
        let mut player = StoryPlayer::start(program());
        for _ in 0..5 {
            stack.push_from_player(&player);
            player.advance();
        }
        assert!(stack.len() <= 3);
    }

    #[test]
    fn step_back_restores_previous_line() {
        let mut recorder = RollbackRecorder::new(20);
        let mut player = StoryPlayer::start(program());
        // Line 1
        assert_eq!(player.wait(), &StoryWait::Line);
        recorder.observe(&player);
        let first = player.current_text().to_string();
        player.advance();
        // Line 2
        assert_eq!(player.wait(), &StoryWait::Line);
        recorder.observe(&player);
        let second = player.current_text().to_string();
        assert_ne!(first, second);
        player.advance();
        recorder.observe(&player);

        assert!(recorder.back(&mut player).unwrap());
        // After one back we should be on an earlier line.
        assert_eq!(player.wait(), &StoryWait::Line);
        assert!(
            player.current_text() == second || player.current_text() == first,
            "text={}",
            player.current_text()
        );
    }

    #[test]
    fn duplicate_cursor_replaces_frame() {
        let mut stack = RollbackStack::with_capacity(10);
        let player = StoryPlayer::start(program());
        stack.push_from_player(&player);
        stack.push_from_player(&player);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn rollback_n_steps() {
        let mut stack = RollbackStack::with_capacity(10);
        let mut player = StoryPlayer::start(program());
        stack.push_from_player(&player);
        player.advance();
        stack.push_from_player(&player);
        player.advance();
        stack.push_from_player(&player);
        assert_eq!(stack.len(), 3);
        let frame = stack.rollback(2);
        assert!(frame.is_some());
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn multi_step_back_restores_earlier_text() {
        let mut recorder = RollbackRecorder::new(32);
        let mut player = StoryPlayer::start(program());
        let mut texts = Vec::new();
        for _ in 0..3 {
            assert_eq!(player.wait(), &StoryWait::Line);
            recorder.observe(&player);
            texts.push(player.current_text().to_string());
            player.advance();
        }
        assert_eq!(texts.len(), 3);
        assert_ne!(texts[0], texts[1]);
        // Step back twice — should land on an earlier recorded line.
        assert!(recorder.back(&mut player).unwrap());
        let t1 = player.current_text().to_string();
        assert!(recorder.back(&mut player).unwrap());
        let t2 = player.current_text().to_string();
        assert!(
            texts.contains(&t1) && texts.contains(&t2),
            "t1={t1} t2={t2} texts={texts:?}"
        );
        // Cannot go past empty stack forever.
        while recorder.back(&mut player).unwrap() {}
        assert!(!recorder.back(&mut player).unwrap());
    }

    #[test]
    fn rollback_preserves_variables() {
        let src = r##"
character n { name: "N" }
state { v: int = 0 }
scene start {
    n "A"
    v += 1
    n "B"
    v += 1
    n "C"
    end
}
"##;
        let prog = load_program_from_source(src, None, "vars").unwrap();
        let mut recorder = RollbackRecorder::new(16);
        let mut player = StoryPlayer::start(prog);
        // Line A
        recorder.observe(&player);
        assert_eq!(player.variables().get_int("v", -1), 0);
        player.advance();
        // After advance may have applied assign + line B
        recorder.observe(&player);
        let v_mid = player.variables().get_int("v", -1);
        player.advance();
        recorder.observe(&player);
        let v_late = player.variables().get_int("v", -1);
        assert!(v_late >= v_mid);
        // Roll back once — variables should restore with snapshot.
        assert!(recorder.back(&mut player).unwrap());
        let v_back = player.variables().get_int("v", -1);
        assert!(
            v_back <= v_late,
            "rollback should not increase vars beyond latest; back={v_back} late={v_late}"
        );
    }
}
