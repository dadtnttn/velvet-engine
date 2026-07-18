//! Voice line playback queue for dialogue.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Voice queue errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VoiceError {
    /// Queue is empty when an operation expected a clip.
    #[error("voice queue is empty")]
    Empty,
}

/// A queued voice clip request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceClip {
    /// Clip asset id / path.
    pub clip_id: String,
    /// Optional character / speaker id for routing.
    #[serde(default)]
    pub speaker: Option<String>,
    /// Estimated or known duration in seconds (0 = unknown; host fills later).
    #[serde(default)]
    pub duration_secs: f32,
    /// Volume scale 0..=1 (multiplied by prefs voice volume).
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_volume() -> f32 {
    1.0
}

impl VoiceClip {
    /// Create a clip request.
    pub fn new(clip_id: impl Into<String>) -> Self {
        Self {
            clip_id: clip_id.into(),
            speaker: None,
            duration_secs: 0.0,
            volume: 1.0,
        }
    }

    /// Builder: speaker.
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }

    /// Builder: duration.
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.duration_secs = secs.max(0.0);
        self
    }

    /// Builder: volume.
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// Playback state of the front clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VoicePlayState {
    /// Nothing playing.
    #[default]
    Idle,
    /// Currently playing.
    Playing,
    /// Finished naturally.
    Finished,
    /// Skipped by user / system.
    Skipped,
}

/// FIFO voice line queue with wait-for-voice support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceQueue {
    /// Pending + current clips (front is current when playing).
    queue: VecDeque<VoiceClip>,
    /// Playback state of the front clip.
    state: VoicePlayState,
    /// Elapsed playback time of the front clip.
    elapsed: f32,
    /// When true, dialogue auto-advance should wait for voice.
    pub wait_for_voice: bool,
    /// Master volume scale from preferences.
    pub master_volume: f32,
}

impl Default for VoiceQueue {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            state: VoicePlayState::Idle,
            elapsed: 0.0,
            wait_for_voice: false,
            master_volume: 1.0,
        }
    }
}

impl VoiceQueue {
    /// Create empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with wait-for-voice and master volume from prefs-like values.
    pub fn with_prefs(wait_for_voice: bool, master_volume: f32) -> Self {
        Self {
            wait_for_voice,
            master_volume: master_volume.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Enqueue a clip. Starts playing immediately if idle.
    pub fn enqueue(&mut self, clip: VoiceClip) {
        self.queue.push_back(clip);
        if self.state == VoicePlayState::Idle || self.state == VoicePlayState::Finished {
            self.start_front();
        }
    }

    /// Enqueue by clip id convenience.
    pub fn enqueue_id(&mut self, clip_id: impl Into<String>) {
        self.enqueue(VoiceClip::new(clip_id));
    }

    fn start_front(&mut self) {
        if self.queue.is_empty() {
            self.state = VoicePlayState::Idle;
            self.elapsed = 0.0;
            return;
        }
        self.state = VoicePlayState::Playing;
        self.elapsed = 0.0;
    }

    /// Currently playing clip.
    pub fn current(&self) -> Option<&VoiceClip> {
        if matches!(self.state, VoicePlayState::Playing) {
            self.queue.front()
        } else {
            None
        }
    }

    /// Playback state.
    pub fn state(&self) -> VoicePlayState {
        self.state
    }

    /// Number of clips waiting including current.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Empty and idle.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Whether voice is actively playing.
    pub fn is_playing(&self) -> bool {
        self.state == VoicePlayState::Playing
    }

    /// Whether dialogue should block advance for voice.
    pub fn should_wait(&self) -> bool {
        self.wait_for_voice && self.is_playing()
    }

    /// Effective volume for the current clip.
    pub fn effective_volume(&self) -> f32 {
        let clip_vol = self.queue.front().map(|c| c.volume).unwrap_or(1.0);
        (clip_vol * self.master_volume).clamp(0.0, 1.0)
    }

    /// Elapsed seconds on current clip.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Tick playback. Returns `true` if a clip finished this frame.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.state != VoicePlayState::Playing {
            return false;
        }
        let Some(front) = self.queue.front() else {
            self.state = VoicePlayState::Idle;
            return false;
        };
        // Only auto-finish when duration is known (> 0).
        if front.duration_secs > 0.0 {
            self.elapsed += dt.max(0.0);
            if self.elapsed >= front.duration_secs {
                return self.finish_current(false);
            }
        } else {
            // Still accumulate elapsed for UI even without known duration.
            self.elapsed += dt.max(0.0);
        }
        false
    }

    /// Host notifies that the audio backend finished the clip.
    pub fn notify_finished(&mut self) -> bool {
        if self.state == VoicePlayState::Playing {
            self.finish_current(false)
        } else {
            false
        }
    }

    /// Skip the current voice line. Returns the skipped clip.
    pub fn skip(&mut self) -> Result<VoiceClip, VoiceError> {
        self.skip_current().ok_or(VoiceError::Empty)
    }

    /// Skip current line. Returns skipped clip if any was playing or queued.
    pub fn skip_current(&mut self) -> Option<VoiceClip> {
        if self.queue.is_empty() {
            return None;
        }
        let clip = self.queue.pop_front().unwrap();
        self.elapsed = 0.0;
        if !self.queue.is_empty() {
            self.start_front();
        } else {
            self.state = VoicePlayState::Idle;
        }
        Some(clip)
    }

    /// Clear the entire queue and stop playback.
    pub fn clear(&mut self) {
        self.queue.clear();
        self.state = VoicePlayState::Idle;
        self.elapsed = 0.0;
    }

    fn finish_current(&mut self, _skipped: bool) -> bool {
        if self.queue.is_empty() {
            self.state = VoicePlayState::Idle;
            return false;
        }
        self.queue.pop_front();
        self.elapsed = 0.0;
        if !self.queue.is_empty() {
            self.start_front();
        } else {
            self.state = VoicePlayState::Idle;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_plays_and_finishes_by_duration() {
        let mut q = VoiceQueue::new();
        q.enqueue(VoiceClip::new("line1").with_duration(0.5));
        assert!(q.is_playing());
        assert_eq!(q.current().unwrap().clip_id, "line1");
        assert!(!q.tick(0.2));
        assert!(q.tick(0.4));
        assert!(q.is_empty());
        assert!(!q.is_playing());
    }

    #[test]
    fn queue_chains_clips() {
        let mut q = VoiceQueue::new();
        q.enqueue(VoiceClip::new("a").with_duration(0.3));
        q.enqueue(VoiceClip::new("b").with_duration(0.3));
        assert_eq!(q.len(), 2);
        assert!(q.tick(0.5));
        assert_eq!(q.current().unwrap().clip_id, "b");
        assert!(q.tick(0.5));
        assert!(q.is_empty());
    }

    #[test]
    fn skip_current_advances() {
        let mut q = VoiceQueue::new();
        q.enqueue_id("a");
        q.enqueue_id("b");
        let skipped = q.skip_current().unwrap();
        assert_eq!(skipped.clip_id, "a");
        assert_eq!(q.current().unwrap().clip_id, "b");
        q.skip_current();
        assert!(q.is_empty());
        assert!(q.skip_current().is_none());
    }

    #[test]
    fn wait_for_voice_flag() {
        let mut q = VoiceQueue::with_prefs(true, 0.8);
        assert!(!q.should_wait());
        q.enqueue(VoiceClip::new("v").with_duration(1.0).with_volume(0.5));
        assert!(q.should_wait());
        assert!((q.effective_volume() - 0.4).abs() < 1e-5);
        q.notify_finished();
        assert!(!q.should_wait());
    }

    #[test]
    fn unknown_duration_needs_notify() {
        let mut q = VoiceQueue::new();
        q.enqueue(VoiceClip::new("live"));
        assert!(!q.tick(10.0));
        assert!(q.is_playing());
        assert!(q.notify_finished());
        assert!(!q.is_playing());
    }

    #[test]
    fn clear_stops_all() {
        let mut q = VoiceQueue::new();
        q.enqueue_id("a");
        q.enqueue_id("b");
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.state(), VoicePlayState::Idle);
    }

    #[test]
    fn serde_roundtrip() {
        let mut q = VoiceQueue::with_prefs(true, 1.0);
        q.enqueue(VoiceClip::new("x").with_speaker("hero").with_duration(1.2));
        let json = serde_json::to_string(&q).unwrap();
        let back: VoiceQueue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 1);
        assert!(back.wait_for_voice);
    }
}
