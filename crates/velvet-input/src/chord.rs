//! Multi-key chord detection.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::binding::KeyCode;
use crate::state::InputState;

/// A chord: all keys must be held; optional "just activated" when last key lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyChord {
    /// Keys that must all be down.
    pub keys: Vec<KeyCode>,
    /// Optional name for debugging / mapping.
    pub name: String,
}

impl KeyChord {
    /// Create from keys.
    pub fn new(name: impl Into<String>, keys: impl IntoIterator<Item = KeyCode>) -> Self {
        let mut keys: Vec<KeyCode> = keys.into_iter().collect();
        keys.sort_by_key(|k| format!("{k:?}"));
        keys.dedup();
        Self {
            keys,
            name: name.into(),
        }
    }

    /// Whether all keys are currently held.
    pub fn is_held(&self, input: &InputState) -> bool {
        !self.keys.is_empty() && self.keys.iter().all(|k| input.key_held(*k))
    }

    /// Whether the chord activated this frame (all held, and at least one key just pressed).
    pub fn just_activated(&self, input: &InputState) -> bool {
        if !self.is_held(input) {
            return false;
        }
        self.keys.iter().any(|k| input.key_just_pressed(*k))
    }
}

/// Tracks multiple chords and their edge state.
#[derive(Debug, Default, Clone)]
pub struct ChordDetector {
    chords: Vec<KeyChord>,
    /// Names held last frame.
    was_held: HashSet<String>,
    /// Names held this frame.
    held: HashSet<String>,
    /// Names activated this frame.
    just: HashSet<String>,
}

impl ChordDetector {
    /// Empty detector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a chord.
    pub fn add(&mut self, chord: KeyChord) {
        self.chords.push(chord);
    }

    /// Builder-style add.
    pub fn with(mut self, chord: KeyChord) -> Self {
        self.add(chord);
        self
    }

    /// Evaluate against input (call once per frame after key events, before or after end_frame).
    pub fn update(&mut self, input: &InputState) {
        self.was_held = std::mem::take(&mut self.held);
        self.just.clear();
        for c in &self.chords {
            if c.is_held(input) {
                self.held.insert(c.name.clone());
                if !self.was_held.contains(&c.name) {
                    self.just.insert(c.name.clone());
                }
            }
        }
    }

    /// Chord currently held by name.
    pub fn held(&self, name: &str) -> bool {
        self.held.contains(name)
    }

    /// Chord just activated by name.
    pub fn just_activated(&self, name: &str) -> bool {
        self.just.contains(name)
    }

    /// All held chord names.
    pub fn held_names(&self) -> impl Iterator<Item = &str> + '_ {
        self.held.iter().map(String::as_str)
    }

    /// Registered chords.
    pub fn chords(&self) -> &[KeyChord] {
        &self.chords
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_s_chord() {
        let mut input = InputState::with_defaults();
        let chord = KeyChord::new("save", [KeyCode::ControlLeft, KeyCode::S]);
        let mut det = ChordDetector::new().with(chord);

        input.begin_frame();
        input.key_down(KeyCode::ControlLeft);
        input.end_frame();
        det.update(&input);
        assert!(!det.held("save"));

        input.begin_frame();
        input.key_down(KeyCode::S);
        input.end_frame();
        det.update(&input);
        assert!(det.held("save"));
        assert!(det.just_activated("save"));

        input.begin_frame();
        input.end_frame();
        det.update(&input);
        assert!(det.held("save"));
        assert!(!det.just_activated("save"));
    }

    #[test]
    fn just_activated_on_key() {
        let mut input = InputState::with_defaults();
        input.begin_frame();
        input.key_down(KeyCode::ControlLeft);
        input.key_down(KeyCode::C);
        input.end_frame();
        let chord = KeyChord::new("copy", [KeyCode::ControlLeft, KeyCode::C]);
        assert!(chord.is_held(&input));
        assert!(chord.just_activated(&input));
    }
}
