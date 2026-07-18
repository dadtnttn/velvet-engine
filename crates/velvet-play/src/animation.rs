//! Simple frame animation player.

use serde::{Deserialize, Serialize};

/// Animation clip (frame indices into a sprite sheet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimClip {
    /// Name.
    pub name: String,
    /// Frame indices.
    pub frames: Vec<u16>,
    /// Seconds per frame.
    pub frame_secs: f32,
    /// Loop.
    pub looping: bool,
}

impl AnimClip {
    /// Create.
    pub fn new(name: impl Into<String>, frames: Vec<u16>, frame_secs: f32, looping: bool) -> Self {
        Self {
            name: name.into(),
            frames,
            frame_secs: frame_secs.max(1e-4),
            looping,
        }
    }
}

/// Animation playback state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimState {
    /// Clip name.
    pub clip: String,
    /// Time in clip.
    pub time: f32,
    /// Playing.
    pub playing: bool,
    /// Finished (non-looping).
    pub finished: bool,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            clip: "idle".into(),
            time: 0.0,
            playing: true,
            finished: false,
        }
    }
}

/// Animation player with clip library.
#[derive(Debug, Clone, Default)]
pub struct AnimPlayer {
    /// Clips by name.
    pub clips: indexmap::IndexMap<String, AnimClip>,
    /// State.
    pub state: AnimState,
}

impl AnimPlayer {
    /// Insert clip.
    pub fn add_clip(&mut self, clip: AnimClip) {
        self.clips.insert(clip.name.clone(), clip);
    }

    /// Play named clip (restarts if different).
    pub fn play(&mut self, name: impl Into<String>) {
        let name = name.into();
        if self.state.clip != name {
            self.state.clip = name;
            self.state.time = 0.0;
            self.state.finished = false;
        }
        self.state.playing = true;
    }

    /// Advance time; returns current frame index.
    pub fn tick(&mut self, dt: f32) -> u16 {
        if !self.state.playing {
            return self.current_frame();
        }
        let Some(clip) = self.clips.get(&self.state.clip) else {
            return 0;
        };
        if clip.frames.is_empty() {
            return 0;
        }
        self.state.time += dt;
        let total = clip.frame_secs * clip.frames.len() as f32;
        if self.state.time >= total {
            if clip.looping {
                self.state.time %= total;
            } else {
                self.state.time = total - 1e-4;
                self.state.playing = false;
                self.state.finished = true;
            }
        }
        self.current_frame()
    }

    /// Current frame.
    pub fn current_frame(&self) -> u16 {
        let Some(clip) = self.clips.get(&self.state.clip) else {
            return 0;
        };
        if clip.frames.is_empty() {
            return 0;
        }
        let idx = (self.state.time / clip.frame_secs).floor() as usize;
        clip.frames[idx.min(clip.frames.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loops_frames() {
        let mut p = AnimPlayer::default();
        p.add_clip(AnimClip::new("walk", vec![0, 1, 2, 3], 0.1, true));
        p.play("walk");
        assert_eq!(p.tick(0.0), 0);
        assert_eq!(p.tick(0.15), 1);
        let _ = p.tick(1.0);
        assert!(!p.state.finished);
    }
}
