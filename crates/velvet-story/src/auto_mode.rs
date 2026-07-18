//! Auto-advance dialogue timing based on text length and preferences.

use serde::{Deserialize, Serialize};

use crate::prefs::{StoryPreferences, TextSpeed};

/// Configuration for auto-advance delay computation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutoModeConfig {
    /// Base delay after the line is fully revealed (seconds).
    pub base_delay_secs: f32,
    /// Extra seconds per character of dialogue text.
    pub secs_per_char: f32,
    /// Minimum total wait after full reveal.
    pub min_delay_secs: f32,
    /// Maximum total wait after full reveal.
    pub max_delay_secs: f32,
    /// Multiplier applied when text speed is Instant (line already fully shown).
    pub instant_multiplier: f32,
}

impl Default for AutoModeConfig {
    fn default() -> Self {
        Self {
            base_delay_secs: 0.8,
            secs_per_char: 0.035,
            min_delay_secs: 0.6,
            max_delay_secs: 12.0,
            instant_multiplier: 0.85,
        }
    }
}

/// Compute how long auto-mode should wait after a line is fully shown.
pub fn compute_auto_delay(text: &str, prefs: &StoryPreferences, config: &AutoModeConfig) -> f32 {
    let char_count = text.chars().filter(|c| !c.is_whitespace()).count() as f32;
    let mut delay = config.base_delay_secs + char_count * config.secs_per_char;
    // Prefer user-configured auto delay as an additional floor.
    delay = delay.max(prefs.auto_delay_secs);
    if matches!(prefs.text_speed, TextSpeed::Instant) {
        delay *= config.instant_multiplier;
    }
    delay.clamp(config.min_delay_secs, config.max_delay_secs)
}

/// Estimate typewriter reveal duration for a line given text speed.
pub fn estimate_reveal_secs(text: &str, prefs: &StoryPreferences) -> f32 {
    match prefs.text_speed {
        TextSpeed::Instant => 0.0,
        TextSpeed::Cps(cps) => {
            let cps = cps.max(1.0);
            let n = text.chars().count() as f32;
            n / cps
        }
    }
}

/// State machine for auto-advance during a dialogue line.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AutoModeController {
    /// Whether auto-mode is enabled (mirrors prefs but can be toggled in UI).
    pub enabled: bool,
    /// Config used for delay calculation.
    pub config: AutoModeConfig,
    /// Seconds remaining until auto-advance fires (None = idle / not counting).
    timer: Option<f32>,
    /// Whether the current line has finished typewriter reveal.
    line_fully_shown: bool,
    /// Whether waiting on voice (blocks auto-advance when prefs say so).
    waiting_for_voice: bool,
}

impl AutoModeController {
    /// Create from preferences.
    pub fn from_prefs(prefs: &StoryPreferences) -> Self {
        Self {
            enabled: prefs.auto_mode,
            config: AutoModeConfig {
                base_delay_secs: prefs.auto_delay_secs.max(0.0),
                ..AutoModeConfig::default()
            },
            ..Default::default()
        }
    }

    /// Sync enabled flag / base delay from preferences.
    pub fn sync_prefs(&mut self, prefs: &StoryPreferences) {
        self.enabled = prefs.auto_mode;
        self.config.base_delay_secs = prefs.auto_delay_secs.max(0.0);
    }

    /// Begin tracking a new dialogue line.
    pub fn on_line_started(&mut self, text: &str, prefs: &StoryPreferences) {
        self.line_fully_shown = matches!(prefs.text_speed, TextSpeed::Instant);
        self.waiting_for_voice = prefs.wait_for_voice;
        if self.enabled && self.line_fully_shown && !self.waiting_for_voice {
            self.timer = Some(compute_auto_delay(text, prefs, &self.config));
        } else {
            self.timer = None;
        }
    }

    /// Notify that typewriter finished revealing the current line.
    pub fn on_line_fully_shown(&mut self, text: &str, prefs: &StoryPreferences) {
        self.line_fully_shown = true;
        if self.enabled && !self.waiting_for_voice {
            self.timer = Some(compute_auto_delay(text, prefs, &self.config));
        }
    }

    /// Voice clip finished (or was skipped).
    pub fn on_voice_finished(&mut self, text: &str, prefs: &StoryPreferences) {
        self.waiting_for_voice = false;
        if self.enabled && self.line_fully_shown {
            // Restart / start post-voice delay.
            self.timer = Some(compute_auto_delay(text, prefs, &self.config));
        }
    }

    /// Mark that voice is still playing.
    pub fn set_waiting_for_voice(&mut self, waiting: bool) {
        self.waiting_for_voice = waiting;
        if waiting {
            self.timer = None;
        }
    }

    /// Cancel auto-advance (manual input / choice).
    pub fn cancel(&mut self) {
        self.timer = None;
        self.line_fully_shown = false;
        self.waiting_for_voice = false;
    }

    /// Toggle enabled; cancels current timer when disabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.timer = None;
        }
    }

    /// Seconds remaining, if counting.
    pub fn remaining(&self) -> Option<f32> {
        self.timer
    }

    /// Whether currently counting down.
    pub fn is_counting(&self) -> bool {
        self.timer.is_some()
    }

    /// Tick the timer. Returns `true` when auto-advance should fire.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(ref mut t) = self.timer {
            *t -= dt.max(0.0);
            if *t <= 0.0 {
                self.timer = None;
                return true;
            }
        }
        false
    }

    /// Full delay for a line including typewriter reveal estimate.
    pub fn total_line_duration(&self, text: &str, prefs: &StoryPreferences) -> f32 {
        estimate_reveal_secs(text, prefs) + compute_auto_delay(text, prefs, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs_auto() -> StoryPreferences {
        StoryPreferences {
            auto_mode: true,
            auto_delay_secs: 1.0,
            text_speed: TextSpeed::Cps(40.0),
            wait_for_voice: false,
            ..Default::default()
        }
    }

    #[test]
    fn delay_scales_with_text_length() {
        let prefs = prefs_auto();
        let cfg = AutoModeConfig::default();
        let short = compute_auto_delay("Hi", &prefs, &cfg);
        let long = compute_auto_delay(
            "This is a much longer line of dialogue that should take longer to read.",
            &prefs,
            &cfg,
        );
        assert!(long > short);
        assert!(short >= cfg.min_delay_secs);
        assert!(long <= cfg.max_delay_secs);
    }

    #[test]
    fn instant_text_shortens_delay() {
        let mut prefs = prefs_auto();
        prefs.text_speed = TextSpeed::Instant;
        let cfg = AutoModeConfig::default();
        let d = compute_auto_delay("Hello world", &prefs, &cfg);
        prefs.text_speed = TextSpeed::Cps(40.0);
        let d2 = compute_auto_delay("Hello world", &prefs, &cfg);
        assert!(d < d2);
    }

    #[test]
    fn reveal_estimate() {
        let prefs = StoryPreferences {
            text_speed: TextSpeed::Cps(10.0),
            ..Default::default()
        };
        assert!((estimate_reveal_secs("abcd", &prefs) - 0.4).abs() < 1e-4);
        let instant = StoryPreferences {
            text_speed: TextSpeed::Instant,
            ..Default::default()
        };
        assert_eq!(estimate_reveal_secs("abcd", &instant), 0.0);
    }

    #[test]
    fn controller_fires_after_timer() {
        let prefs = prefs_auto();
        let mut auto = AutoModeController::from_prefs(&prefs);
        auto.on_line_started("Hello", &prefs);
        // Not fully shown yet with CPS
        assert!(!auto.is_counting());
        auto.on_line_fully_shown("Hello", &prefs);
        assert!(auto.is_counting());
        let mut fired = false;
        for _ in 0..200 {
            if auto.tick(0.1) {
                fired = true;
                break;
            }
        }
        assert!(fired);
        assert!(!auto.is_counting());
    }

    #[test]
    fn voice_blocks_until_finished() {
        let mut prefs = prefs_auto();
        prefs.wait_for_voice = true;
        prefs.text_speed = TextSpeed::Instant;
        let mut auto = AutoModeController::from_prefs(&prefs);
        auto.on_line_started("Voice line", &prefs);
        assert!(!auto.is_counting());
        auto.on_voice_finished("Voice line", &prefs);
        assert!(auto.is_counting());
        assert!(auto.tick(100.0));
    }

    #[test]
    fn cancel_and_disable() {
        let prefs = prefs_auto();
        let mut auto = AutoModeController::from_prefs(&prefs);
        auto.on_line_started("x", &prefs);
        auto.on_line_fully_shown("x", &prefs);
        auto.cancel();
        assert!(!auto.is_counting());
        auto.on_line_fully_shown("x", &prefs);
        auto.set_enabled(false);
        assert!(!auto.tick(10.0));
    }

    #[test]
    fn total_duration_positive() {
        let prefs = prefs_auto();
        let auto = AutoModeController::from_prefs(&prefs);
        let t = auto.total_line_duration("Hello there friend", &prefs);
        assert!(t > 0.5);
    }
}
