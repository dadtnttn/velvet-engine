//! Weapon definitions.

use serde::{Deserialize, Serialize};

/// Weapon id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeaponId(pub String);

/// Weapon type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponKind {
    /// Melee arc / range.
    Melee,
    /// Hitscan instant.
    Hitscan,
    /// Projectile spawn.
    Projectile,
}

/// Weapon stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Weapon {
    /// Id.
    pub id: WeaponId,
    /// Kind.
    pub kind: WeaponKind,
    /// Damage per hit.
    pub damage: f32,
    /// Range (melee / hitscan).
    pub range: f32,
    /// Fire cooldown seconds.
    pub cooldown: f32,
    /// Time until ready.
    pub cooldown_left: f32,
    /// Magazine size (0 = infinite).
    pub magazine: u32,
    /// Current ammo in mag.
    pub ammo: u32,
    /// Reserve ammo.
    pub reserve: u32,
    /// Reload time.
    pub reload_secs: f32,
    /// Reloading left.
    pub reload_left: f32,
    /// Spread radians.
    pub spread: f32,
    /// Projectile speed.
    pub projectile_speed: f32,
    /// Knockback impulse.
    pub knockback: f32,
}

impl Weapon {
    /// Melee bat.
    pub fn melee(id: &str, damage: f32, range: f32, cooldown: f32) -> Self {
        Self {
            id: WeaponId(id.into()),
            kind: WeaponKind::Melee,
            damage,
            range,
            cooldown,
            cooldown_left: 0.0,
            magazine: 0,
            ammo: 0,
            reserve: 0,
            reload_secs: 0.0,
            reload_left: 0.0,
            spread: 0.0,
            projectile_speed: 0.0,
            knockback: 80.0,
        }
    }

    /// Pistol hitscan.
    pub fn pistol(id: &str) -> Self {
        Self {
            id: WeaponId(id.into()),
            kind: WeaponKind::Hitscan,
            damage: 25.0,
            range: 400.0,
            cooldown: 0.25,
            cooldown_left: 0.0,
            magazine: 12,
            ammo: 12,
            reserve: 48,
            reload_secs: 1.2,
            reload_left: 0.0,
            spread: 0.05,
            projectile_speed: 0.0,
            knockback: 40.0,
        }
    }

    /// Tick cooldowns.
    pub fn tick(&mut self, dt: f32) {
        self.cooldown_left = (self.cooldown_left - dt).max(0.0);
        if self.reload_left > 0.0 {
            self.reload_left = (self.reload_left - dt).max(0.0);
            if self.reload_left <= 0.0 && self.magazine > 0 {
                let need = self.magazine.saturating_sub(self.ammo);
                let take = need.min(self.reserve);
                self.ammo += take;
                self.reserve -= take;
            }
        }
    }

    /// Can fire.
    pub fn can_fire(&self) -> bool {
        self.cooldown_left <= 0.0
            && self.reload_left <= 0.0
            && (self.magazine == 0 || self.ammo > 0)
    }

    /// Begin reload if needed.
    pub fn start_reload(&mut self) {
        if self.magazine == 0 || self.ammo >= self.magazine || self.reserve == 0 {
            return;
        }
        if self.reload_left <= 0.0 {
            self.reload_left = self.reload_secs;
        }
    }

    /// Consume shot; returns false if cannot.
    pub fn fire(&mut self) -> bool {
        if !self.can_fire() {
            return false;
        }
        if self.magazine > 0 {
            self.ammo -= 1;
        }
        self.cooldown_left = self.cooldown;
        if self.magazine > 0 && self.ammo == 0 {
            self.start_reload();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_and_reload() {
        let mut w = Weapon::pistol("p");
        assert!(w.fire());
        assert_eq!(w.ammo, 11);
        w.ammo = 0;
        w.cooldown_left = 0.0;
        assert!(!w.fire());
        w.start_reload();
        w.tick(2.0);
        assert_eq!(w.ammo, 12);
    }
}
