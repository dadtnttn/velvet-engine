//! Playing voices.

use crate::bus::BusId;
use crate::clip::ClipId;
use crate::engine::PlaybackId;

/// Voice lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    /// Playing.
    Playing,
    /// Paused.
    Paused,
    /// Fading out.
    FadingOut,
    /// Finished / free.
    Stopped,
}

/// One simultaneous playback instance.
#[derive(Debug, Clone)]
pub struct Voice {
    /// Playback id.
    pub id: PlaybackId,
    /// Clip.
    pub clip: ClipId,
    /// Bus.
    pub bus: BusId,
    /// State.
    pub state: VoiceState,
    /// Volume scale.
    pub volume: f32,
    /// Pan -1..=1.
    pub pan: f32,
    /// Playback cursor in seconds.
    pub cursor_secs: f32,
    /// Clip duration.
    pub duration_secs: f32,
    /// Looping.
    pub looping: bool,
    /// Priority (higher preferred when voice limit hit).
    pub priority: i32,
    /// Fade target volume.
    pub fade_to: Option<f32>,
    /// Fade speed (volume units per second).
    pub fade_speed: f32,
    /// Spatial position (2D), if enabled.
    pub position: Option<(f32, f32)>,
}

impl Voice {
    /// Whether actively consuming a voice slot.
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            VoiceState::Playing | VoiceState::Paused | VoiceState::FadingOut
        )
    }
}
