//! Character statistics and leveling.

use serde::{Deserialize, Serialize};

/// Core attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attributes {
    /// Strength.
    pub strength: i32,
    /// Agility.
    pub agility: i32,
    /// Intelligence.
    pub intellect: i32,
    /// Vitality.
    pub vitality: i32,
    /// Luck.
    pub luck: i32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            strength: 5,
            agility: 5,
            intellect: 5,
            vitality: 5,
            luck: 5,
        }
    }
}

impl Attributes {
    /// Max HP derived.
    pub fn max_hp(self) -> f32 {
        20.0 + self.vitality as f32 * 8.0
    }

    /// Max MP derived.
    pub fn max_mp(self) -> f32 {
        5.0 + self.intellect as f32 * 4.0
    }

    /// Physical attack.
    pub fn attack(self) -> f32 {
        2.0 + self.strength as f32 * 1.5
    }

    /// Defense.
    pub fn defense(self) -> f32 {
        1.0 + self.vitality as f32 * 0.8 + self.agility as f32 * 0.2
    }
}

/// Runtime combat/ stat block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatBlock {
    /// Attributes.
    pub attributes: Attributes,
    /// Current HP.
    pub hp: f32,
    /// Current MP.
    pub mp: f32,
}

impl Default for StatBlock {
    fn default() -> Self {
        let attributes = Attributes::default();
        Self {
            hp: attributes.max_hp(),
            mp: attributes.max_mp(),
            attributes,
        }
    }
}

impl StatBlock {
    /// From attributes at full resources.
    pub fn from_attributes(attributes: Attributes) -> Self {
        Self {
            hp: attributes.max_hp(),
            mp: attributes.max_mp(),
            attributes,
        }
    }

    /// Apply damage after defense; returns true if fatal.
    pub fn take_damage(&mut self, raw: f32) -> bool {
        let mitigated = (raw - self.attributes.defense() * 0.5).max(1.0);
        self.hp = (self.hp - mitigated).max(0.0);
        self.hp <= 0.0
    }

    /// Heal HP.
    pub fn heal(&mut self, amount: f32) {
        self.hp = (self.hp + amount).min(self.attributes.max_hp());
    }

    /// Spend MP; returns false if insufficient.
    pub fn spend_mp(&mut self, amount: f32) -> bool {
        if self.mp < amount {
            return false;
        }
        self.mp -= amount;
        true
    }

    /// Restore MP (clamped to max).
    pub fn restore_mp(&mut self, amount: f32) {
        self.mp = (self.mp + amount).min(self.attributes.max_mp());
    }

    /// HP fraction 0..=1.
    pub fn hp_fraction(&self) -> f32 {
        let max = self.attributes.max_hp();
        if max <= 0.0 {
            0.0
        } else {
            (self.hp / max).clamp(0.0, 1.0)
        }
    }

    /// MP fraction 0..=1.
    pub fn mp_fraction(&self) -> f32 {
        let max = self.attributes.max_mp();
        if max <= 0.0 {
            0.0
        } else {
            (self.mp / max).clamp(0.0, 1.0)
        }
    }

    /// Fully restore HP and MP.
    pub fn refill(&mut self) {
        self.hp = self.attributes.max_hp();
        self.mp = self.attributes.max_mp();
    }

    /// Alive.
    pub fn is_alive(&self) -> bool {
        self.hp > 0.0
    }
}

/// Level and experience.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelProgress {
    /// Level (1-based).
    pub level: u32,
    /// XP toward next level.
    pub xp: u32,
    /// XP required for next level.
    pub xp_to_next: u32,
}

impl Default for LevelProgress {
    fn default() -> Self {
        Self {
            level: 1,
            xp: 0,
            xp_to_next: 100,
        }
    }
}

impl LevelProgress {
    /// Add XP; returns number of levels gained.
    pub fn add_xp(&mut self, amount: u32) -> u32 {
        self.xp += amount;
        let mut gained = 0;
        while self.xp >= self.xp_to_next {
            self.xp -= self.xp_to_next;
            self.level += 1;
            gained += 1;
            self.xp_to_next = 100 + (self.level - 1) * 50;
        }
        gained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_up() {
        let mut lp = LevelProgress::default();
        assert_eq!(lp.add_xp(250), 2);
        assert!(lp.level >= 3);
    }

    #[test]
    fn damage_and_heal() {
        let mut s = StatBlock::default();
        let max = s.hp;
        s.take_damage(5.0);
        assert!(s.hp < max);
        s.heal(1000.0);
        assert_eq!(s.hp, s.attributes.max_hp());
    }
}
