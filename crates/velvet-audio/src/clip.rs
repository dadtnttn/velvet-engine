//! Audio clip handles (decoded or streamed metadata).

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

static NEXT_CLIP: AtomicU64 = AtomicU64::new(1);

/// Clip identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClipId(u64);

impl ClipId {
    /// Allocate new id.
    pub fn allocate() -> Self {
        Self(NEXT_CLIP.fetch_add(1, Ordering::Relaxed))
    }

    /// Raw.
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// CPU-side clip description. Samples may be empty for streaming placeholders.
#[derive(Debug, Clone)]
pub struct AudioClip {
    /// Id.
    pub id: ClipId,
    /// Debug name / path.
    pub name: String,
    /// Sample rate.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u16,
    /// Duration seconds (0 if unknown).
    pub duration_secs: f32,
    /// Interleaved f32 samples (mono or stereo) for the null mixer.
    pub samples: Vec<f32>,
}

impl AudioClip {
    /// Create a silent clip of given duration (test utility).
    pub fn silent(name: impl Into<String>, duration_secs: f32, sample_rate: u32) -> Self {
        let frames = (duration_secs.max(0.0) * sample_rate as f32) as usize;
        Self {
            id: ClipId::allocate(),
            name: name.into(),
            sample_rate,
            channels: 1,
            duration_secs,
            samples: vec![0.0; frames],
        }
    }

    /// Create a short sine tone for tests / placeholders.
    pub fn sine(
        name: impl Into<String>,
        frequency: f32,
        duration_secs: f32,
        sample_rate: u32,
    ) -> Self {
        let frames = (duration_secs.max(0.0) * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(frames);
        for i in 0..frames {
            let t = i as f32 / sample_rate as f32;
            samples.push((t * frequency * std::f32::consts::TAU).sin() * 0.2);
        }
        Self {
            id: ClipId::allocate(),
            name: name.into(),
            sample_rate,
            channels: 1,
            duration_secs,
            samples,
        }
    }
}
