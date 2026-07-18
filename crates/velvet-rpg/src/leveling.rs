//! XP curves, level-up resolution, and stat growth tables.

use serde::{Deserialize, Serialize};

use crate::stats::{Attributes, LevelProgress, StatBlock};

/// How XP required for each level is computed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum XpCurve {
    /// Linear: `base + (level - 1) * per_level`.
    Linear {
        /// XP for level 1 → 2.
        base: u32,
        /// Extra XP per level.
        per_level: u32,
    },
    /// Polynomial-ish: `base * level^power` (rounded).
    Power {
        /// Scale.
        base: f32,
        /// Exponent.
        power: f32,
    },
    /// Explicit table; index 0 = XP for level 1→2. Beyond table uses last gap.
    Table(Vec<u32>),
}

impl Default for XpCurve {
    fn default() -> Self {
        Self::Linear {
            base: 100,
            per_level: 50,
        }
    }
}

impl XpCurve {
    /// XP required to go from `level` to `level + 1` (level is 1-based).
    pub fn xp_to_next(&self, level: u32) -> u32 {
        let level = level.max(1);
        match self {
            Self::Linear { base, per_level } => base + (level - 1) * per_level,
            Self::Power { base, power } => {
                let v = (*base as f64) * (level as f64).powf(*power as f64);
                v.round().max(1.0) as u32
            }
            Self::Table(t) => {
                if t.is_empty() {
                    return 100;
                }
                let idx = (level as usize).saturating_sub(1);
                if idx < t.len() {
                    t[idx].max(1)
                } else {
                    *t.last().unwrap_or(&100)
                }
            }
        }
    }

    /// Total XP required to reach `level` from 1 (sum of thresholds).
    pub fn total_xp_for_level(&self, level: u32) -> u32 {
        if level <= 1 {
            return 0;
        }
        (1..level).map(|l| self.xp_to_next(l)).sum()
    }
}

/// Per-level attribute growth (added on each level-up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatGrowth {
    /// Strength growth.
    pub strength: i32,
    /// Agility growth.
    pub agility: i32,
    /// Intellect growth.
    pub intellect: i32,
    /// Vitality growth.
    pub vitality: i32,
    /// Luck growth.
    pub luck: i32,
}

impl Default for StatGrowth {
    fn default() -> Self {
        Self {
            strength: 1,
            agility: 1,
            intellect: 1,
            vitality: 1,
            luck: 0,
        }
    }
}

impl StatGrowth {
    /// Flat growth for all combat stats.
    pub fn flat(n: i32) -> Self {
        Self {
            strength: n,
            agility: n,
            intellect: n,
            vitality: n,
            luck: n,
        }
    }

    /// Apply growth to attributes.
    pub fn apply(self, attrs: &mut Attributes) {
        attrs.strength += self.strength;
        attrs.agility += self.agility;
        attrs.intellect += self.intellect;
        attrs.vitality += self.vitality;
        attrs.luck += self.luck;
    }
}

/// Result of granting XP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelUpResult {
    /// Levels gained this grant.
    pub levels_gained: u32,
    /// New level.
    pub new_level: u32,
    /// Attribute deltas applied (summed across multi-level).
    pub growth_applied: StatGrowth,
    /// Whether HP/MP were refilled on level-up.
    pub refilled: bool,
}

/// Leveling system binding curve + growth + progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LevelingSystem {
    /// XP curve.
    pub curve: XpCurve,
    /// Growth per level.
    pub growth: StatGrowth,
    /// Progress.
    pub progress: LevelProgress,
    /// Refill HP/MP fully on each level-up.
    pub refill_on_level: bool,
    /// Soft level cap (0 = none).
    pub level_cap: u32,
}

impl Default for LevelingSystem {
    fn default() -> Self {
        let curve = XpCurve::default();
        Self {
            progress: LevelProgress {
                level: 1,
                xp: 0,
                xp_to_next: curve.xp_to_next(1),
            },
            curve,
            growth: StatGrowth::default(),
            refill_on_level: true,
            level_cap: 0,
        }
    }
}

impl LevelingSystem {
    /// Create with curve and growth.
    pub fn new(curve: XpCurve, growth: StatGrowth) -> Self {
        let xp_to_next = curve.xp_to_next(1);
        Self {
            curve,
            growth,
            progress: LevelProgress {
                level: 1,
                xp: 0,
                xp_to_next,
            },
            refill_on_level: true,
            level_cap: 0,
        }
    }

    /// Sync `xp_to_next` from the curve for the current level.
    pub fn resync_threshold(&mut self) {
        self.progress.xp_to_next = self.curve.xp_to_next(self.progress.level);
    }

    /// Whether at soft cap.
    pub fn at_cap(&self) -> bool {
        self.level_cap > 0 && self.progress.level >= self.level_cap
    }

    /// Grant XP and apply growth to `stats` for each level gained.
    pub fn add_xp(&mut self, amount: u32, stats: &mut StatBlock) -> LevelUpResult {
        if amount == 0 || self.at_cap() {
            return LevelUpResult {
                levels_gained: 0,
                new_level: self.progress.level,
                growth_applied: StatGrowth {
                    strength: 0,
                    agility: 0,
                    intellect: 0,
                    vitality: 0,
                    luck: 0,
                },
                refilled: false,
            };
        }

        self.progress.xp = self.progress.xp.saturating_add(amount);
        let mut levels = 0u32;
        let mut growth_sum = StatGrowth {
            strength: 0,
            agility: 0,
            intellect: 0,
            vitality: 0,
            luck: 0,
        };

        while self.progress.xp >= self.progress.xp_to_next {
            if self.level_cap > 0 && self.progress.level >= self.level_cap {
                self.progress.xp = self.progress.xp_to_next.saturating_sub(1);
                break;
            }
            self.progress.xp -= self.progress.xp_to_next;
            self.progress.level += 1;
            levels += 1;
            self.growth.apply(&mut stats.attributes);
            growth_sum.strength += self.growth.strength;
            growth_sum.agility += self.growth.agility;
            growth_sum.intellect += self.growth.intellect;
            growth_sum.vitality += self.growth.vitality;
            growth_sum.luck += self.growth.luck;
            if self.refill_on_level {
                stats.hp = stats.attributes.max_hp();
                stats.mp = stats.attributes.max_mp();
            } else {
                // Keep ratios roughly; at least raise caps without killing current.
                stats.hp = stats.hp.min(stats.attributes.max_hp()).max(stats.hp);
                stats.mp = stats.mp.min(stats.attributes.max_mp()).max(stats.mp);
            }
            self.progress.xp_to_next = self.curve.xp_to_next(self.progress.level);
            if self.level_cap > 0 && self.progress.level >= self.level_cap {
                break;
            }
        }

        LevelUpResult {
            levels_gained: levels,
            new_level: self.progress.level,
            growth_applied: growth_sum,
            refilled: levels > 0 && self.refill_on_level,
        }
    }

    /// Progress fraction toward next level 0..=1.
    pub fn fraction(&self) -> f32 {
        if self.progress.xp_to_next == 0 {
            return 1.0;
        }
        self.progress.xp as f32 / self.progress.xp_to_next as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_curve_thresholds() {
        let c = XpCurve::Linear {
            base: 100,
            per_level: 50,
        };
        assert_eq!(c.xp_to_next(1), 100);
        assert_eq!(c.xp_to_next(2), 150);
        assert_eq!(c.total_xp_for_level(3), 250);
    }

    #[test]
    fn power_curve_increases() {
        let c = XpCurve::Power {
            base: 50.0,
            power: 1.5,
        };
        assert!(c.xp_to_next(5) > c.xp_to_next(2));
    }

    #[test]
    fn table_curve() {
        let c = XpCurve::Table(vec![10, 20, 40]);
        assert_eq!(c.xp_to_next(1), 10);
        assert_eq!(c.xp_to_next(3), 40);
        assert_eq!(c.xp_to_next(10), 40);
    }

    #[test]
    fn add_xp_levels_and_grows() {
        let mut sys = LevelingSystem::new(
            XpCurve::Linear {
                base: 50,
                per_level: 0,
            },
            StatGrowth::flat(2),
        );
        let mut stats = StatBlock::default();
        let str0 = stats.attributes.strength;
        let r = sys.add_xp(120, &mut stats);
        assert_eq!(r.levels_gained, 2);
        assert_eq!(stats.attributes.strength, str0 + 4);
        assert_eq!(stats.hp, stats.attributes.max_hp());
    }

    #[test]
    fn level_cap() {
        let mut sys = LevelingSystem::new(XpCurve::default(), StatGrowth::default());
        sys.level_cap = 2;
        let mut stats = StatBlock::default();
        let r = sys.add_xp(10_000, &mut stats);
        assert_eq!(r.new_level, 2);
        assert!(sys.at_cap());
    }

    #[test]
    fn fraction_midway() {
        let mut sys = LevelingSystem::default();
        let mut stats = StatBlock::default();
        sys.add_xp(50, &mut stats);
        let f = sys.fraction();
        assert!(f > 0.0 && f < 1.0);
    }

    #[test]
    fn multi_level_xp_dump() {
        let mut sys = LevelingSystem::new(
            XpCurve::Linear {
                base: 100,
                per_level: 0,
            },
            StatGrowth::flat(1),
        );
        let mut stats = StatBlock::default();
        let r = sys.add_xp(350, &mut stats);
        assert_eq!(r.levels_gained, 3);
        assert_eq!(r.new_level, 4);
        assert!(sys.fraction() >= 0.0 && sys.fraction() <= 1.0);
    }

    #[test]
    fn no_xp_no_level() {
        let mut sys = LevelingSystem::default();
        let mut stats = StatBlock::default();
        let before = stats.attributes.strength;
        let r = sys.add_xp(0, &mut stats);
        assert_eq!(r.levels_gained, 0);
        assert_eq!(stats.attributes.strength, before);
    }

    #[test]
    fn table_curve_progression() {
        let mut sys =
            LevelingSystem::new(XpCurve::Table(vec![10, 20, 40, 80]), StatGrowth::flat(1));
        let mut stats = StatBlock::default();
        let r = sys.add_xp(30, &mut stats);
        assert!(r.levels_gained >= 2);
        assert!(r.new_level >= 3);
    }

    #[test]
    fn cap_blocks_further_growth() {
        let mut sys = LevelingSystem::new(
            XpCurve::Linear {
                base: 10,
                per_level: 0,
            },
            StatGrowth::flat(5),
        );
        sys.level_cap = 3;
        let mut stats = StatBlock::default();
        let r1 = sys.add_xp(1000, &mut stats);
        assert_eq!(r1.new_level, 3);
        let str_at_cap = stats.attributes.strength;
        let r2 = sys.add_xp(1000, &mut stats);
        assert_eq!(r2.levels_gained, 0);
        assert_eq!(stats.attributes.strength, str_at_cap);
        assert!(sys.at_cap());
    }

    #[test]
    fn power_curve_total_increases() {
        let c = XpCurve::Power {
            base: 20.0,
            power: 1.8,
        };
        let mut prev = 0u32;
        for lvl in 1..10 {
            let t = c.total_xp_for_level(lvl);
            assert!(t >= prev);
            prev = t;
        }
    }
}
