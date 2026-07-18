//! Audio engine with bus graph and voice pool.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;
use tracing::debug;

use crate::bus::{AudioBus, BusId, BusKind};
use crate::clip::{AudioClip, ClipId};
use crate::voice::{Voice, VoiceState};

static NEXT_PLAYBACK: AtomicU64 = AtomicU64::new(1);

/// Playback instance id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaybackId(u64);

impl PlaybackId {
    fn allocate() -> Self {
        Self(NEXT_PLAYBACK.fetch_add(1, Ordering::Relaxed))
    }

    /// Create from raw id (tests / serialization bridges).
    pub fn from_raw(id: u64) -> Self {
        Self(id)
    }

    /// Raw id.
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Audio errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// Unknown clip.
    #[error("unknown clip")]
    UnknownClip,
    /// Unknown bus.
    #[error("unknown bus")]
    UnknownBus,
    /// Unknown playback.
    #[error("unknown playback")]
    UnknownPlayback,
    /// Device / backend failure.
    #[error("backend: {0}")]
    Backend(String),
}

/// Parameters for starting playback.
#[derive(Debug, Clone)]
pub struct PlayParams {
    /// Target bus.
    pub bus: BusId,
    /// Volume 0..=1.
    pub volume: f32,
    /// Pan -1..=1.
    pub pan: f32,
    /// Loop.
    pub looping: bool,
    /// Fade-in seconds (0 = immediate).
    pub fade_in: f32,
    /// Start offset seconds.
    pub start_secs: f32,
    /// Priority.
    pub priority: i32,
    /// Optional spatial position.
    pub position: Option<(f32, f32)>,
}

impl Default for PlayParams {
    fn default() -> Self {
        Self {
            bus: BusId::from_kind(BusKind::Effects),
            volume: 1.0,
            pan: 0.0,
            looping: false,
            fade_in: 0.0,
            start_secs: 0.0,
            priority: 0,
            position: None,
        }
    }
}

impl PlayParams {
    /// Music defaults.
    pub fn music() -> Self {
        Self {
            bus: BusId::from_kind(BusKind::Music),
            looping: true,
            ..Default::default()
        }
    }

    /// UI defaults.
    pub fn ui() -> Self {
        Self {
            bus: BusId::from_kind(BusKind::Ui),
            ..Default::default()
        }
    }
}

/// In-process audio engine (null device: advances clocks, no OS output).
#[derive(Debug)]
pub struct AudioEngine {
    buses: HashMap<String, AudioBus>,
    clips: HashMap<ClipId, AudioClip>,
    voices: Vec<Voice>,
    /// Max simultaneous voices.
    pub max_voices: usize,
    /// Listener position for spatial panning.
    pub listener: (f32, f32),
    /// Spatial unit scale (distance for full attenuation).
    pub spatial_radius: f32,
    /// Whether backend is "healthy".
    pub device_ok: bool,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    /// Create engine with default buses.
    pub fn new() -> Self {
        let mut buses = HashMap::new();
        for kind in BusKind::all() {
            let bus = AudioBus::new(*kind);
            buses.insert(bus.id.as_str().to_string(), bus);
        }
        Self {
            buses,
            clips: HashMap::new(),
            voices: Vec::new(),
            max_voices: 64,
            listener: (0.0, 0.0),
            spatial_radius: 800.0,
            device_ok: true,
        }
    }

    /// Register a clip.
    pub fn add_clip(&mut self, clip: AudioClip) -> ClipId {
        let id = clip.id;
        self.clips.insert(id, clip);
        id
    }

    /// Bus mut.
    pub fn bus_mut(&mut self, id: &BusId) -> Option<&mut AudioBus> {
        self.buses.get_mut(id.as_str())
    }

    /// Bus ref.
    pub fn bus(&self, id: &BusId) -> Option<&AudioBus> {
        self.buses.get(id.as_str())
    }

    /// Set bus volume.
    pub fn set_bus_volume(&mut self, kind: BusKind, volume: f32) {
        if let Some(b) = self.buses.get_mut(kind.as_str()) {
            b.volume = volume.clamp(0.0, 2.0);
        }
    }

    /// Mute bus.
    pub fn set_bus_muted(&mut self, kind: BusKind, muted: bool) {
        if let Some(b) = self.buses.get_mut(kind.as_str()) {
            b.muted = muted;
        }
    }

    /// Effective gain for a bus including parents.
    pub fn effective_gain(&self, bus: &BusId) -> f32 {
        let mut gain = 1.0;
        let mut current = Some(bus.clone());
        let mut guard = 0;
        while let Some(id) = current {
            if guard > 8 {
                break;
            }
            guard += 1;
            if let Some(b) = self.buses.get(id.as_str()) {
                gain *= b.local_gain();
                current = b.parent.clone();
            } else {
                break;
            }
        }
        gain
    }

    /// Play a clip.
    pub fn play(&mut self, clip: ClipId, params: PlayParams) -> Result<PlaybackId, AudioError> {
        let duration_secs = self
            .clips
            .get(&clip)
            .ok_or(AudioError::UnknownClip)?
            .duration_secs;
        if !self.buses.contains_key(params.bus.as_str()) {
            return Err(AudioError::UnknownBus);
        }
        self.ensure_voice_capacity(params.priority);

        let id = PlaybackId::allocate();
        let start_vol = if params.fade_in > 0.0 {
            0.0
        } else {
            params.volume
        };
        let voice = Voice {
            id,
            clip,
            bus: params.bus,
            state: VoiceState::Playing,
            volume: start_vol,
            pan: params.pan.clamp(-1.0, 1.0),
            cursor_secs: params.start_secs.max(0.0),
            duration_secs,
            looping: params.looping,
            priority: params.priority,
            fade_to: if params.fade_in > 0.0 {
                Some(params.volume)
            } else {
                None
            },
            fade_speed: if params.fade_in > 0.0 {
                params.volume / params.fade_in.max(1e-4)
            } else {
                0.0
            },
            position: params.position,
        };
        debug!(?id, clip = clip.raw(), "play");
        self.voices.push(voice);
        Ok(id)
    }

    /// Stop with optional fade-out seconds.
    pub fn stop(&mut self, id: PlaybackId, fade_out: f32) -> Result<(), AudioError> {
        let voice = self
            .voices
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(AudioError::UnknownPlayback)?;
        if fade_out <= 0.0 {
            voice.state = VoiceState::Stopped;
        } else {
            voice.state = VoiceState::FadingOut;
            voice.fade_to = Some(0.0);
            voice.fade_speed = voice.volume / fade_out.max(1e-4);
        }
        Ok(())
    }

    /// Pause.
    pub fn pause(&mut self, id: PlaybackId) -> Result<(), AudioError> {
        let voice = self
            .voices
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(AudioError::UnknownPlayback)?;
        if voice.state == VoiceState::Playing {
            voice.state = VoiceState::Paused;
        }
        Ok(())
    }

    /// Resume.
    pub fn resume(&mut self, id: PlaybackId) -> Result<(), AudioError> {
        let voice = self
            .voices
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or(AudioError::UnknownPlayback)?;
        if voice.state == VoiceState::Paused {
            voice.state = VoiceState::Playing;
        }
        Ok(())
    }

    /// Crossfade: fade out old music, fade in new.
    pub fn crossfade_music(
        &mut self,
        new_clip: ClipId,
        fade_secs: f32,
    ) -> Result<PlaybackId, AudioError> {
        let music = BusId::from_kind(BusKind::Music);
        for v in self.voices.iter_mut() {
            if v.bus == music && v.is_active() {
                v.state = VoiceState::FadingOut;
                v.fade_to = Some(0.0);
                v.fade_speed = v.volume / fade_secs.max(1e-4);
            }
        }
        self.play(
            new_clip,
            PlayParams {
                bus: music,
                fade_in: fade_secs,
                looping: true,
                volume: 1.0,
                ..Default::default()
            },
        )
    }

    /// Advance simulation by `dt` seconds (call each frame).
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for voice in &mut self.voices {
            if voice.state == VoiceState::Paused || voice.state == VoiceState::Stopped {
                continue;
            }
            // Fade
            if let Some(target) = voice.fade_to {
                let dir = (target - voice.volume).signum();
                voice.volume += dir * voice.fade_speed * dt;
                if (voice.volume - target).abs() <= voice.fade_speed * dt + 1e-4 {
                    voice.volume = target;
                    voice.fade_to = None;
                    if voice.state == VoiceState::FadingOut && target <= 0.0 {
                        voice.state = VoiceState::Stopped;
                    }
                }
            }
            if voice.state == VoiceState::Playing || voice.state == VoiceState::FadingOut {
                voice.cursor_secs += dt;
                if voice.duration_secs > 0.0 && voice.cursor_secs >= voice.duration_secs {
                    if voice.looping {
                        voice.cursor_secs %= voice.duration_secs;
                    } else {
                        voice.state = VoiceState::Stopped;
                    }
                }
            }
        }
        self.voices.retain(|v| v.state != VoiceState::Stopped);
    }

    /// Active voice count.
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// Voices slice.
    pub fn voices(&self) -> &[Voice] {
        &self.voices
    }

    /// Mixed gain for a voice including bus and simple spatial.
    pub fn voice_output_gain(&self, voice: &Voice) -> f32 {
        let mut g = voice.volume * self.effective_gain(&voice.bus);
        if let Some((x, y)) = voice.position {
            let dx = x - self.listener.0;
            let dy = y - self.listener.1;
            let dist = (dx * dx + dy * dy).sqrt();
            let atten = (1.0 - (dist / self.spatial_radius).clamp(0.0, 1.0)).max(0.0);
            g *= atten;
        }
        g
    }

    fn ensure_voice_capacity(&mut self, new_priority: i32) {
        while self.active_voices() >= self.max_voices {
            // Steal lowest priority playing voice.
            let victim = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_active())
                .min_by_key(|(_, v)| v.priority)
                .map(|(i, v)| (i, v.priority));
            if let Some((i, prio)) = victim {
                if prio > new_priority {
                    break; // cannot steal higher
                }
                self.voices[i].state = VoiceState::Stopped;
            } else {
                break;
            }
        }
        self.voices.retain(|v| v.state != VoiceState::Stopped);
    }

    /// Simulate device recovery.
    pub fn recover_device(&mut self) {
        self.device_ok = true;
    }

    /// Mark device failed (tests / runtime).
    pub fn mark_device_failed(&mut self, reason: impl Into<String>) {
        self.device_ok = false;
        debug!(reason = %reason.into(), "audio device failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_and_tick_finishes() {
        let mut eng = AudioEngine::new();
        let clip = eng.add_clip(AudioClip::silent("blip", 0.05, 44100));
        let id = eng
            .play(
                clip,
                PlayParams {
                    bus: BusId::from_kind(BusKind::Effects),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(eng.active_voices(), 1);
        eng.tick(0.1);
        assert!(eng.voices().iter().all(|v| v.id != id) || eng.active_voices() == 0);
    }

    #[test]
    fn bus_mute_zeros_gain() {
        let mut eng = AudioEngine::new();
        eng.set_bus_muted(BusKind::Music, true);
        let g = eng.effective_gain(&BusId::from_kind(BusKind::Music));
        assert_eq!(g, 0.0);
    }

    #[test]
    fn fade_in_reaches_volume() {
        let mut eng = AudioEngine::new();
        let clip = eng.add_clip(AudioClip::silent("m", 2.0, 22050));
        let id = eng
            .play(
                clip,
                PlayParams {
                    bus: BusId::from_kind(BusKind::Music),
                    volume: 1.0,
                    fade_in: 0.5,
                    looping: true,
                    ..Default::default()
                },
            )
            .unwrap();
        eng.tick(0.6);
        let v = eng.voices().iter().find(|v| v.id == id).unwrap();
        assert!((v.volume - 1.0).abs() < 0.05);
    }

    #[test]
    fn voice_limit_steals_low_priority() {
        let mut eng = AudioEngine::new();
        eng.max_voices = 2;
        let clip = eng.add_clip(AudioClip::silent("x", 10.0, 8000));
        eng.play(
            clip,
            PlayParams {
                priority: 0,
                bus: BusId::from_kind(BusKind::Effects),
                ..Default::default()
            },
        )
        .unwrap();
        eng.play(
            clip,
            PlayParams {
                priority: 0,
                bus: BusId::from_kind(BusKind::Effects),
                ..Default::default()
            },
        )
        .unwrap();
        eng.play(
            clip,
            PlayParams {
                priority: 10,
                bus: BusId::from_kind(BusKind::Effects),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(eng.active_voices() <= 2);
    }

    #[test]
    fn spatial_attenuates() {
        let mut eng = AudioEngine::new();
        eng.listener = (0.0, 0.0);
        eng.spatial_radius = 100.0;
        let clip = eng.add_clip(AudioClip::silent("s", 1.0, 8000));
        let id = eng
            .play(
                clip,
                PlayParams {
                    position: Some((1000.0, 0.0)),
                    volume: 1.0,
                    bus: BusId::from_kind(BusKind::Effects),
                    ..Default::default()
                },
            )
            .unwrap();
        let v = eng.voices().iter().find(|v| v.id == id).unwrap();
        assert_eq!(eng.voice_output_gain(v), 0.0);
    }
}
