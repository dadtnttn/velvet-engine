//! Skip engine integrating story preferences, seen-line tracking, and batch advance.

use serde::{Deserialize, Serialize};

use crate::prefs::{SkipMode, StoryPreferences};
use crate::runtime::{StoryPlayer, StoryWait};

/// Result of a skip attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipResult {
    /// Advanced one or more lines.
    Advanced {
        /// How many line advances occurred.
        lines: u32,
    },
    /// Stopped because of a choice.
    StoppedAtChoice,
    /// Stopped because the story ended.
    StoppedAtEnd,
    /// Skip mode is off or line is not eligible.
    NotEligible,
    /// Hit safety max steps.
    HitLimit,
}

/// Configuration for the skip engine beyond raw [`SkipMode`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkipConfig {
    /// Base skip mode (usually mirrored from preferences).
    pub mode: SkipMode,
    /// Maximum lines to skip in one `skip_while` call.
    pub max_batch: u32,
    /// Stop skip when a choice is reached (always recommended true).
    pub stop_on_choice: bool,
    /// Stop skip when ending is reached.
    pub stop_on_end: bool,
    /// If true, unread lines are never skipped even in `All` mode
    /// (useful for "fast-forward read only" UI toggles).
    pub force_read_only: bool,
    /// Skip delays (auto-mode interaction): ignore voice wait when skipping.
    pub ignore_voice_wait: bool,
}

impl Default for SkipConfig {
    fn default() -> Self {
        Self {
            mode: SkipMode::Off,
            max_batch: 256,
            stop_on_choice: true,
            stop_on_end: true,
            force_read_only: false,
            ignore_voice_wait: true,
        }
    }
}

impl SkipConfig {
    /// Build from story preferences.
    pub fn from_prefs(prefs: &StoryPreferences) -> Self {
        Self {
            mode: prefs.skip_mode,
            ..Default::default()
        }
    }

    /// Effective mode after force_read_only.
    pub fn effective_mode(&self) -> SkipMode {
        if self.force_read_only {
            match self.mode {
                SkipMode::Off => SkipMode::Off,
                SkipMode::All | SkipMode::ReadOnly => SkipMode::ReadOnly,
            }
        } else {
            self.mode
        }
    }
}

/// Skip controller held by UI / game layer.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SkipEngine {
    /// Config.
    pub config: SkipConfig,
    /// Whether the player is holding the skip button.
    pub holding: bool,
    /// Cumulative lines skipped this hold session.
    pub session_lines: u32,
}

impl SkipEngine {
    /// Create from preferences.
    pub fn from_prefs(prefs: &StoryPreferences) -> Self {
        Self {
            config: SkipConfig::from_prefs(prefs),
            holding: false,
            session_lines: 0,
        }
    }

    /// Sync mode from preferences without resetting hold state.
    pub fn sync_prefs(&mut self, prefs: &StoryPreferences) {
        self.config.mode = prefs.skip_mode;
    }

    /// Begin holding skip.
    pub fn begin_hold(&mut self) {
        self.holding = true;
        self.session_lines = 0;
    }

    /// End holding skip.
    pub fn end_hold(&mut self) {
        self.holding = false;
    }

    /// Whether a given line key is considered already-read.
    pub fn is_read(player: &StoryPlayer, scene: &str, op_index: usize) -> bool {
        let key = format!("{scene}:{op_index}");
        player.seen_line_keys().contains(&key)
    }

    /// Whether the current wait state can be skipped under config.
    pub fn can_skip_current(&self, player: &StoryPlayer) -> bool {
        match self.config.effective_mode() {
            SkipMode::Off => false,
            SkipMode::All => matches!(player.wait(), StoryWait::Line),
            SkipMode::ReadOnly => {
                if !matches!(player.wait(), StoryWait::Line) {
                    return false;
                }
                let snap = player.snapshot();
                Self::is_read(player, &snap.scene, snap.op_index)
            }
        }
    }

    /// Try to skip a single line using player.try_skip or direct advance when eligible.
    pub fn skip_once(&mut self, player: &mut StoryPlayer) -> SkipResult {
        match player.wait() {
            StoryWait::Choice if self.config.stop_on_choice => return SkipResult::StoppedAtChoice,
            StoryWait::Ended if self.config.stop_on_end => return SkipResult::StoppedAtEnd,
            _ => {}
        }

        if !self.can_skip_current(player) {
            // Fall back to player's built-in try_skip for consistency.
            if player.try_skip() {
                self.session_lines = self.session_lines.saturating_add(1);
                return SkipResult::Advanced { lines: 1 };
            }
            return SkipResult::NotEligible;
        }

        if matches!(player.wait(), StoryWait::Line) {
            player.advance();
            self.session_lines = self.session_lines.saturating_add(1);
            SkipResult::Advanced { lines: 1 }
        } else {
            SkipResult::NotEligible
        }
    }

    /// Skip while eligible, up to max_batch (or until choice/end).
    pub fn skip_batch(&mut self, player: &mut StoryPlayer) -> SkipResult {
        let mut lines = 0u32;
        let limit = self.config.max_batch.max(1);
        for _ in 0..limit {
            match player.wait() {
                StoryWait::Choice if self.config.stop_on_choice => {
                    return if lines > 0 {
                        SkipResult::Advanced { lines }
                    } else {
                        SkipResult::StoppedAtChoice
                    };
                }
                StoryWait::Ended if self.config.stop_on_end => {
                    return if lines > 0 {
                        SkipResult::Advanced { lines }
                    } else {
                        SkipResult::StoppedAtEnd
                    };
                }
                _ => {}
            }

            if !self.can_skip_current(player) {
                break;
            }
            if matches!(player.wait(), StoryWait::Line) {
                player.advance();
                lines += 1;
                self.session_lines = self.session_lines.saturating_add(1);
            } else {
                break;
            }
        }
        if lines == 0 {
            if matches!(player.wait(), StoryWait::Choice) {
                SkipResult::StoppedAtChoice
            } else if matches!(player.wait(), StoryWait::Ended) {
                SkipResult::StoppedAtEnd
            } else if lines == 0 && self.config.max_batch > 0 && !self.can_skip_current(player) {
                SkipResult::NotEligible
            } else {
                SkipResult::HitLimit
            }
        } else if lines >= limit {
            SkipResult::HitLimit
        } else {
            SkipResult::Advanced { lines }
        }
    }

    /// Tick while holding: if holding and mode active, batch skip.
    pub fn tick_hold(&mut self, player: &mut StoryPlayer) -> SkipResult {
        if !self.holding {
            return SkipResult::NotEligible;
        }
        self.skip_batch(player)
    }
}

/// Estimate how many consecutive already-read lines remain from the current cursor.
/// This walks a cloned lightweight path using only dialogue ops (best-effort).
pub fn count_skippable_read_lines(player: &StoryPlayer, max: u32) -> u32 {
    let snap = player.snapshot();
    let Some(scene) = player.program().scene(&snap.scene) else {
        return 0;
    };
    let mut count = 0u32;
    let mut idx = snap.op_index;
    while count < max && idx < scene.ops.len() {
        let key = format!("{}:{}", snap.scene, idx);
        match &scene.ops[idx] {
            crate::ir::StoryOp::Dialogue { .. } => {
                if player.seen_line_keys().contains(&key) {
                    count += 1;
                    idx += 1;
                } else {
                    break;
                }
            }
            crate::ir::StoryOp::Choice { .. } | crate::ir::StoryOp::End { .. } => break,
            crate::ir::StoryOp::Jump { .. } | crate::ir::StoryOp::Call { .. } => break,
            _ => idx += 1,
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::prefs::SkipMode;

    fn multi_line() -> StoryPlayer {
        let src = r##"
character n { name: "N" }
scene start {
    n "A"
    n "B"
    n "C"
    end
}
"##;
        let program = load_program_from_source(src, None, "skip").unwrap();
        StoryPlayer::start(program)
    }

    #[test]
    fn skip_all_advances_lines() {
        let mut player = multi_line();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::All;
        let r = engine.skip_batch(&mut player);
        match r {
            SkipResult::Advanced { lines } => assert!(lines >= 1),
            SkipResult::StoppedAtEnd => {}
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn skip_off_not_eligible() {
        let mut player = multi_line();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::Off;
        assert_eq!(engine.skip_once(&mut player), SkipResult::NotEligible);
    }

    #[test]
    fn read_only_skips_after_seen() {
        let mut player = multi_line();
        // Manually advance once so line is seen, then go back is hard —
        // instead: read line A (seen), advance to B, enable read-only and try skip on B (unread).
        assert_eq!(player.wait(), &StoryWait::Line);
        let a = player.current_text().to_string();
        player.advance(); // now B, A is seen
        assert_ne!(player.current_text(), a);

        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::ReadOnly;
        // B is currently showing and was just applied as seen when we landed... dialogue marks seen on show.
        // So B is also seen. Skip should work.
        let can = engine.can_skip_current(&player);
        assert!(can);
        let _ = engine.skip_once(&mut player);
    }

    #[test]
    fn hold_session_counts_lines() {
        let mut player = multi_line();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::All;
        engine.begin_hold();
        let _ = engine.tick_hold(&mut player);
        assert!(engine.session_lines >= 1);
        engine.end_hold();
        assert!(!engine.holding);
    }

    #[test]
    fn force_read_only_downgrades_all() {
        let cfg = SkipConfig {
            mode: SkipMode::All,
            force_read_only: true,
            ..Default::default()
        };
        assert_eq!(cfg.effective_mode(), SkipMode::ReadOnly);
    }

    #[test]
    fn count_skippable() {
        let mut player = multi_line();
        // All current lines get marked seen as displayed.
        let n = count_skippable_read_lines(&player, 10);
        assert!(n >= 1);
        player.advance();
        let _ = count_skippable_read_lines(&player, 10);
    }

    #[test]
    fn skip_batch_reaches_end() {
        let mut player = multi_line();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::All;
        engine.config.max_batch = 32;
        let mut total = 0u32;
        for _ in 0..16 {
            match engine.skip_batch(&mut player) {
                SkipResult::Advanced { lines } => total += lines,
                SkipResult::StoppedAtEnd => break,
                SkipResult::NotEligible => break,
                other => panic!("unexpected {other:?}"),
            }
            if player.is_ended() {
                break;
            }
        }
        assert!(
            player.is_ended() || total >= 2,
            "total={total} ended={}",
            player.is_ended()
        );
    }

    #[test]
    fn skip_once_advances_one_line() {
        let mut player = multi_line();
        let first = player.current_text().to_string();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::All;
        let r = engine.skip_once(&mut player);
        match r {
            SkipResult::Advanced { lines } => assert!(lines >= 1),
            SkipResult::StoppedAtEnd => {}
            other => panic!("{other:?}"),
        }
        if !player.is_ended() {
            assert_ne!(player.current_text(), first);
        }
    }

    #[test]
    fn hold_then_end_clears_holding() {
        let mut player = multi_line();
        let mut engine = SkipEngine::default();
        engine.config.mode = SkipMode::All;
        engine.begin_hold();
        assert!(engine.holding);
        for _ in 0..5 {
            let _ = engine.tick_hold(&mut player);
            if player.is_ended() {
                break;
            }
        }
        assert!(engine.session_lines >= 1);
        engine.end_hold();
        assert!(!engine.holding);
        // begin_hold resets session counter again
        engine.begin_hold();
        assert_eq!(engine.session_lines, 0);
        assert!(engine.holding);
    }
}
