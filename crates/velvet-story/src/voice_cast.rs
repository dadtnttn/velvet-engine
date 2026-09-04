//! Voice per character: clip routing + auto-advance waits for audio.
//!
//! Maps `speaker -> voice config` so `luna "Hola"` auto-enqueues
//! `luna_0042.ogg` (or id) with her volume/pitch, and auto-mode blocks
//! until the clip finishes (or its estimated duration elapses).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

fn default_volume() -> f32 {
    1.0
}

fn default_pitch() -> f32 {
    1.0
}

fn default_rate() -> f32 {
    1.0
}

/// Voice config for one speaker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterVoice {
    /// Speaker id (matches `character` id / dialogue speaker).
    pub speaker: String,
    /// Clip id prefix, e.g. `luna` -> `luna_0042`.
    pub clip_prefix: String,
    /// Volume 0..=1 (multiplied by prefs voice volume).
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Pitch scale (host hint; 1.0 = natural).
    #[serde(default = "default_pitch")]
    pub pitch: f32,
    /// Words-per-minute-ish rate factor for duration estimate.
    #[serde(default = "default_rate")]
    pub rate: f32,
}

impl CharacterVoice {
    /// New with sane defaults.
    pub fn new(speaker: impl Into<String>, clip_prefix: impl Into<String>) -> Self {
        Self {
            speaker: speaker.into(),
            clip_prefix: clip_prefix.into(),
            volume: 1.0,
            pitch: 1.0,
            rate: 1.0,
        }
    }

    /// Builder volume.
    pub fn with_volume(mut self, v: f32) -> Self {
        self.volume = v.clamp(0.0, 1.0);
        self
    }

    /// Builder pitch.
    pub fn with_pitch(mut self, p: f32) -> Self {
        self.pitch = p.clamp(0.5, 2.0);
        self
    }
}

/// Cast table: speaker -> voice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceCast {
    /// Entries by speaker id.
    #[serde(default)]
    pub voices: BTreeMap<String, CharacterVoice>,
    /// Default volume for unlisted speakers.
    #[serde(default = "default_one")]
    pub default_volume: f32,
}

impl Default for VoiceCast {
    fn default() -> Self {
        Self {
            voices: BTreeMap::new(),
            default_volume: 1.0,
        }
    }
}

fn default_one() -> f32 {
    1.0
}

/// File format for `voice_cast.json` packs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VoiceCastFile {
    /// Voices list.
    #[serde(default)]
    pub voices: Vec<CharacterVoice>,
    /// Default volume.
    #[serde(default = "default_one")]
    pub default_volume: f32,
}

impl VoiceCast {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from a `voice_cast.json` document.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let file: VoiceCastFile =
            serde_json::from_str(text).map_err(|e| format!("voice_cast json: {e}"))?;
        let mut cast = Self {
            default_volume: if file.default_volume <= 0.0 {
                1.0
            } else {
                file.default_volume.clamp(0.0, 1.0)
            },
            ..Default::default()
        };
        for v in file.voices {
            cast.register(v);
        }
        Ok(cast)
    }

    /// Load from path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("io {}: {e}", path.display()))?;
        Self::from_json(&text)
    }

    /// Register / replace.
    pub fn register(&mut self, voice: CharacterVoice) {
        self.voices.insert(voice.speaker.clone(), voice);
    }

    /// Lookup.
    pub fn get(&self, speaker: &str) -> Option<&CharacterVoice> {
        self.voices.get(speaker)
    }

    /// Clip id for a line: `{prefix}_{line_no:04}`. None when speaker unlisted.
    pub fn clip_for_line(&self, speaker: &str, line_no: u32) -> Option<String> {
        self.voices
            .get(speaker)
            .map(|v| format!("{}_{:04}", v.clip_prefix, line_no))
    }

    /// Effective volume for a speaker.
    pub fn volume_for(&self, speaker: &str) -> f32 {
        self.voices
            .get(speaker)
            .map(|v| v.volume)
            .unwrap_or(self.default_volume)
            .clamp(0.0, 1.0)
    }

    /// Estimated voice seconds for text (used when clip length unknown).
    ///
    /// Base 14 chars/sec at rate 1.0, CJK longer (per-char cost), pitch shortens.
    pub fn duration_for_text(&self, speaker: &str, text: &str) -> f32 {
        let n = text.chars().filter(|c| !c.is_whitespace()).count().max(1) as f32;
        let cjk_boost = {
            let wide = text
                .chars()
                .filter(|&c| crate::rich_text::char_width(c) == 2)
                .count();
            1.0 + (wide as f32 / n) * 0.9
        };
        let rate = self
            .voices
            .get(speaker)
            .map(|v| v.rate)
            .unwrap_or(1.0)
            .clamp(0.4, 2.5);
        let pitch = self
            .voices
            .get(speaker)
            .map(|v| v.pitch)
            .unwrap_or(1.0)
            .clamp(0.5, 2.0);
        // higher pitch ~ slightly faster perceived
        let pitch_factor = 1.0 / (0.85 + pitch * 0.15);
        (n / (14.0 * rate)) * cjk_boost * pitch_factor + 0.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_routing_and_volume() {
        let mut cast = VoiceCast::new();
        cast.register(CharacterVoice::new("luna", "luna").with_volume(0.8));
        assert_eq!(cast.clip_for_line("luna", 42).as_deref(), Some("luna_0042"));
        assert!(cast.clip_for_line("nobody", 1).is_none());
        assert!((cast.volume_for("luna") - 0.8).abs() < 1e-5);
        assert_eq!(cast.volume_for("nobody"), 1.0);
    }

    #[test]
    fn duration_scales_with_text_and_rate() {
        let mut cast = VoiceCast::new();
        cast.register(CharacterVoice::new("luna", "luna"));
        let short = cast.duration_for_text("luna", "Hi");
        let long = cast.duration_for_text("luna", "Hello world, this is a much longer line");
        assert!(long > short);
        let mut fast = VoiceCast::new();
        let mut v = CharacterVoice::new("luna", "luna");
        v.rate = 2.0;
        fast.register(v);
        assert!(
            fast.duration_for_text("luna", "Hello world test")
                < cast.duration_for_text("luna", "Hello world test")
        );
    }

    #[test]
    fn cjk_takes_longer() {
        let cast = VoiceCast::new();
        let latin = cast.duration_for_text("x", "Hello hello");
        let cjk = cast.duration_for_text("x", "你好你好你好");
        assert!(cjk > latin * 0.9);
    }
}
