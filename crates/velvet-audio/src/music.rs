//! Music player with crossfade state machine.

use crate::bus::BusKind;
use crate::clip::ClipId;
use crate::engine::{AudioEngine, AudioError, PlayParams, PlaybackId};

/// Crossfade state.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CrossfadeState {
    /// No crossfade in progress.
    #[default]
    Idle,
    /// Fading from one track to another.
    Fading {
        /// Outgoing playback.
        from: PlaybackId,
        /// Incoming playback.
        to: PlaybackId,
        /// Elapsed seconds.
        t: f32,
        /// Total duration seconds.
        duration: f32,
    },
}

impl CrossfadeState {
    /// Progress `0..=1` if fading.
    pub fn progress(&self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Fading { t, duration, .. } => (*t / duration.max(1e-4)).clamp(0.0, 1.0),
        }
    }

    /// Whether a crossfade is active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Fading { .. })
    }
}

/// High-level music controller over [`AudioEngine`].
#[derive(Debug, Default)]
pub struct MusicPlayer {
    /// Current primary music playback.
    pub current: Option<PlaybackId>,
    /// Crossfade machine.
    pub crossfade: CrossfadeState,
    /// Default crossfade length seconds.
    pub default_fade: f32,
    /// Music volume scale (applied via play params).
    pub volume: f32,
}

impl MusicPlayer {
    /// Create with default 1s crossfade.
    pub fn new() -> Self {
        Self {
            current: None,
            crossfade: CrossfadeState::Idle,
            default_fade: 1.0,
            volume: 1.0,
        }
    }

    /// Play immediately, stopping previous music without crossfade.
    pub fn play(
        &mut self,
        engine: &mut AudioEngine,
        clip: ClipId,
    ) -> Result<PlaybackId, AudioError> {
        self.stop(engine, 0.0);
        let id = engine.play(
            clip,
            PlayParams {
                bus: BusKind::Music.into(),
                volume: self.volume,
                looping: true,
                ..Default::default()
            },
        )?;
        self.current = Some(id);
        self.crossfade = CrossfadeState::Idle;
        Ok(id)
    }

    /// Start clip with crossfade from current track.
    pub fn crossfade_to(
        &mut self,
        engine: &mut AudioEngine,
        clip: ClipId,
        duration: Option<f32>,
    ) -> Result<PlaybackId, AudioError> {
        let duration = duration.unwrap_or(self.default_fade).max(1e-3);
        let from = self.current;
        // If nothing playing, just play with fade-in.
        if from.is_none() {
            let id = engine.play(
                clip,
                PlayParams {
                    bus: BusKind::Music.into(),
                    volume: self.volume,
                    fade_in: duration,
                    looping: true,
                    ..Default::default()
                },
            )?;
            self.current = Some(id);
            self.crossfade = CrossfadeState::Idle;
            return Ok(id);
        }
        let from = from.unwrap();
        let _ = engine.stop(from, duration);
        let to = engine.play(
            clip,
            PlayParams {
                bus: BusKind::Music.into(),
                volume: self.volume,
                fade_in: duration,
                looping: true,
                ..Default::default()
            },
        )?;
        self.current = Some(to);
        self.crossfade = CrossfadeState::Fading {
            from,
            to,
            t: 0.0,
            duration,
        };
        Ok(to)
    }

    /// Stop music with optional fade-out.
    pub fn stop(&mut self, engine: &mut AudioEngine, fade_out: f32) {
        if let Some(id) = self.current.take() {
            let _ = engine.stop(id, fade_out);
        }
        if let CrossfadeState::Fading { from, to, .. } = self.crossfade {
            let _ = engine.stop(from, fade_out);
            let _ = engine.stop(to, fade_out);
        }
        self.crossfade = CrossfadeState::Idle;
    }

    /// Advance crossfade clock (engine.tick should also be called by game).
    pub fn tick(&mut self, dt: f32) {
        if let CrossfadeState::Fading { t, duration, .. } = &mut self.crossfade {
            *t += dt.max(0.0);
            if *t >= *duration {
                self.crossfade = CrossfadeState::Idle;
            }
        }
    }

    /// Whether music is considered playing (has current id).
    pub fn is_playing(&self) -> bool {
        self.current.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::AudioClip;

    #[test]
    fn play_sets_current() {
        let mut eng = AudioEngine::new();
        let clip = eng.add_clip(AudioClip::silent("m", 5.0, 22050));
        let mut player = MusicPlayer::new();
        player.play(&mut eng, clip).unwrap();
        assert!(player.is_playing());
        assert!(!player.crossfade.is_active());
    }

    #[test]
    fn crossfade_state_machine() {
        let mut eng = AudioEngine::new();
        let a = eng.add_clip(AudioClip::silent("a", 10.0, 22050));
        let b = eng.add_clip(AudioClip::silent("b", 10.0, 22050));
        let mut player = MusicPlayer::new();
        player.play(&mut eng, a).unwrap();
        player.crossfade_to(&mut eng, b, Some(0.5)).unwrap();
        assert!(player.crossfade.is_active());
        assert!((player.crossfade.progress() - 0.0).abs() < 1e-5);
        player.tick(0.25);
        assert!((player.crossfade.progress() - 0.5).abs() < 1e-4);
        player.tick(0.3);
        assert!(!player.crossfade.is_active());
    }
}
