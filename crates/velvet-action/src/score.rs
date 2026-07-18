//! Score and combos.

use serde::{Deserialize, Serialize};

/// Combo tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComboState {
    /// Current combo count.
    pub count: u32,
    /// Timer until combo resets.
    pub timer: f32,
    /// Window seconds.
    pub window: f32,
}

impl Default for ComboState {
    fn default() -> Self {
        Self {
            count: 0,
            timer: 0.0,
            window: 2.5,
        }
    }
}

impl ComboState {
    /// Tick timer.
    pub fn tick(&mut self, dt: f32) {
        if self.count > 0 {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.count = 0;
            }
        }
    }

    /// Register hit; returns multiplier.
    pub fn register_hit(&mut self) -> f32 {
        self.count += 1;
        self.timer = self.window;
        1.0 + (self.count.saturating_sub(1) as f32) * 0.25
    }
}

/// Score board.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreBoard {
    /// Points.
    pub score: i64,
    /// Kills.
    pub kills: u32,
    /// Deaths.
    pub deaths: u32,
    /// Combo.
    pub combo: ComboState,
    /// Best combo.
    pub best_combo: u32,
}

impl ScoreBoard {
    /// Add kill score.
    pub fn add_kill(&mut self, base: i64) {
        let mul = self.combo.register_hit();
        self.kills += 1;
        self.score += (base as f32 * mul) as i64;
        self.best_combo = self.best_combo.max(self.combo.count);
    }

    /// Player died.
    pub fn add_death(&mut self) {
        self.deaths += 1;
        self.combo.count = 0;
        self.combo.timer = 0.0;
    }

    /// Tick.
    pub fn tick(&mut self, dt: f32) {
        self.combo.tick(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combo_multiplies() {
        let mut s = ScoreBoard::default();
        s.add_kill(100);
        s.add_kill(100);
        assert!(s.score > 200);
        assert_eq!(s.kills, 2);
    }
}
