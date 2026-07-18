//! # velvet-audio
//!
//! Category buses, voices, fades, and a pluggable backend.
//! Default backend is an in-process mixer simulator suitable for tests and headless runs.
//! A `kira` backend can be enabled later without changing game-facing APIs.

#![deny(missing_docs)]

mod bus;
mod clip;
mod dsp;
mod engine;
mod mixer;
mod music;
mod plugin;
mod spatial;
mod voice;

pub mod prelude;

pub use bus::{AudioBus, BusId, BusKind};
pub use clip::{AudioClip, ClipId};
pub use dsp::{
    select_voices_to_steal, sort_voices_for_mix, voice_steal_order, AdsrEnvelope, AdsrParams,
    AdsrStage, DuckingBus, GainRamp, LowPass1P, VoicePriorityKey,
};
pub use engine::{AudioEngine, AudioError, PlayParams, PlaybackId};
pub use mixer::{BusMix, CategoryMixer};
pub use music::{CrossfadeState, MusicPlayer};
pub use plugin::AudioPlugin;
pub use spatial::{
    evaluate_spatial, spatialize_pan, spatialize_volume, SpatialGain, SpatialListener,
};
pub use voice::{Voice, VoiceState};
