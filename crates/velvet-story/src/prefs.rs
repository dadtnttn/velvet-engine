//! Player preferences for story presentation.

use serde::{Deserialize, Serialize};

/// Text reveal speed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TextSpeed {
    /// Instant.
    Instant,
    /// Characters per second.
    Cps(f32),
}

impl Default for TextSpeed {
    fn default() -> Self {
        Self::Cps(40.0)
    }
}

/// Skip mode policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SkipMode {
    /// Do not skip.
    #[default]
    Off,
    /// Skip all text.
    All,
    /// Skip only already-read lines.
    ReadOnly,
}

/// Story UI / playback preferences (saved separately from slots).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryPreferences {
    /// Text speed.
    pub text_speed: TextSpeed,
    /// Auto-advance mode.
    pub auto_mode: bool,
    /// Auto-advance delay seconds after line fully shown.
    pub auto_delay_secs: f32,
    /// Skip mode.
    pub skip_mode: SkipMode,
    /// Master volume 0..=1 (scales all buses).
    #[serde(default = "default_one")]
    pub master_volume: f32,
    /// Music bus volume 0..=1.
    #[serde(default = "default_one")]
    pub music_volume: f32,
    /// SFX bus volume 0..=1.
    #[serde(default = "default_one")]
    pub sfx_volume: f32,
    /// Master story volume scale 0..=1 (voice bus hint).
    pub voice_volume: f32,
    /// Wait for voice to finish before advance.
    pub wait_for_voice: bool,
    /// Prefer fullscreen window when host supports it.
    #[serde(default)]
    pub fullscreen: bool,
    /// Accessibility (font size, contrast, reduced motion). Persisted with prefs.
    #[serde(default)]
    pub accessibility: crate::accessibility::AccessibilityPrefs,
}

fn default_one() -> f32 {
    1.0
}

impl Default for StoryPreferences {
    fn default() -> Self {
        Self {
            text_speed: TextSpeed::default(),
            auto_mode: false,
            auto_delay_secs: 1.5,
            skip_mode: SkipMode::Off,
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            wait_for_voice: false,
            fullscreen: false,
            accessibility: crate::accessibility::AccessibilityPrefs::default(),
        }
    }
}

impl StoryPreferences {
    /// Effective music volume after master.
    pub fn effective_music_volume(&self) -> f32 {
        (self.master_volume * self.music_volume).clamp(0.0, 1.0)
    }

    /// Effective sfx volume after master.
    pub fn effective_sfx_volume(&self) -> f32 {
        (self.master_volume * self.sfx_volume).clamp(0.0, 1.0)
    }

    /// Effective voice volume after master.
    pub fn effective_voice_volume(&self) -> f32 {
        (self.master_volume * self.voice_volume).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_persists_with_prefs() {
        let mut prefs = StoryPreferences::default();
        prefs.accessibility.font_scale = 1.5;
        prefs.accessibility.high_contrast = true;
        let json = serde_json::to_string(&prefs).unwrap();
        let back: StoryPreferences = serde_json::from_str(&json).unwrap();
        assert!((back.accessibility.font_scale - 1.5).abs() < 1e-5);
        assert!(back.accessibility.high_contrast);
    }

    #[test]
    fn old_prefs_json_without_accessibility_loads() {
        let old = r#"{"text_speed":{"Cps":40.0},"auto_mode":false,"auto_delay_secs":1.5,"skip_mode":"Off","master_volume":1.0,"music_volume":1.0,"sfx_volume":1.0,"voice_volume":1.0,"wait_for_voice":false}"#;
        let prefs: StoryPreferences = serde_json::from_str(old).unwrap();
        assert_eq!(
            prefs.accessibility,
            crate::accessibility::AccessibilityPrefs::default()
        );
    }
}
