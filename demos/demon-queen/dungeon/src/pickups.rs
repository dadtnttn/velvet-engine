//! What a fallen foe leaves on the floor: coins, bones, ash, shards, tonics and herbs that burst
//! out of it, land, spin where they lie and fly to the Queen when she comes near (or, left long
//! enough, wherever she is: nothing earned is lost). The run takes what reaches her.
use velvet_dungeon::{gameplay::SpawnKind, rng, Blueprint};
use velvet_math::Vec3;

/// One thing on the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loot {
    Gold(u32),
    Material(&'static str, u32),
    Tonic,
    Herbs(u32),
}

impl Loot {
    /// How much it shines: plain things not at all, the rare ones brightly.
    pub fn rarity(self) -> f32 {
        match self {
            Loot::Gold(n) if n >= 20 => 0.8,
            Loot::Gold(_) => 0.35,
            Loot::Material("esquirla_corazon" | "craneo_campeon", _) => 1.0,
            Loot::Material("ceniza_arcana", _) => 0.6,
            Loot::Material(..) => 0.0,
            Loot::Tonic => 0.7,
            Loot::Herbs(_) => 0.3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pickup {
    pub loot: Loot,
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub landed: bool,
    /// Its own turn, so a heap does not spin as one.
    pub phase: f32,
}

/// Near this she draws what lies on the floor to her.
pub const MAGNET: f32 = 2.8;
/// Left this long, it comes to her wherever she is.
const WANDER: f32 = 10.0;

/// Throws `loot` out of a body at `at`, every piece its own way.
pub fn burst(at: Vec3, loot: &[Loot], seed: u64) -> Vec<Pickup> {
    let mut rng = rng::Rng::stream(seed, "pickups_v1", loot.len() as u64);
    loot.iter()
        .enumerate()
        .map(|(k, &loot)| {
            let a = rng.range(0.0, std::f32::consts::TAU) + k as f32 * 2.4;
            let out = rng.range(1.4, 3.0);
            Pickup {
                loot,
                position: at + Vec3::Y * 0.9,
                velocity: Vec3::new(a.cos() * out, rng.range(3.6, 5.4), a.sin() * out),
                age: 0.0,
                landed: false,
                phase: rng.range(0.0, std::f32::consts::TAU),
            }
        })
        .collect()
}

/// Moves what lies on the floor a step and returns what reached her.
pub fn update(pickups: &mut Vec<Pickup>, map: &Blueprint, player: Vec3, dt: f32) -> Vec<Loot> {
    let mut taken = Vec::new();
    let chest = player + Vec3::Y * 0.9;
    pickups.retain_mut(|p| {
        p.age += dt;
        let to = chest - p.position;
        let reach = to.length();
        let drawn = p.landed && (reach < MAGNET || p.age > WANDER) || p.age > WANDER + 3.0;
        if drawn {
            if reach < 0.55 {
                taken.push(p.loot);
                return false;
            }
            let speed =
                5.5 + 9.0 * (1.0 - (reach / MAGNET).min(1.0)) + (p.age - WANDER).max(0.0) * 2.0;
            p.position = p.position + to * ((speed * dt).min(reach) / reach.max(1.0e-4));
            return true;
        }
        if !p.landed {
            p.velocity.y -= 14.0 * dt;
            let next = p.position + p.velocity * dt;
            // into a wall: it drops straight down where it is
            let floor = map.surface(next.x, next.z, p.position.y + 0.4, 0.2);
            let floor = match floor {
                Some(y) => {
                    p.position.x = next.x;
                    p.position.z = next.z;
                    y
                }
                None => {
                    p.velocity.x = 0.0;
                    p.velocity.z = 0.0;
                    map.surface(p.position.x, p.position.z, p.position.y + 0.4, 0.2)
                        .unwrap_or(p.position.y - 50.0)
                }
            };
            p.position.y = next.y;
            if p.position.y <= floor + 0.12 && p.velocity.y < 0.0 {
                p.position.y = floor + 0.12;
                p.velocity = Vec3::ZERO;
                p.landed = true;
            }
            if p.age > 6.0 {
                p.landed = true;
            }
        }
        true
    });
    taken
}

/// What a foe of this kind leaves besides its materials: gold for most, the odd tonic and
/// herbs, more for elites and champions. Seeded by the run and the foe, the same every time.
pub fn extra_loot(seed: u64, spawn_id: usize, kind: SpawnKind) -> Vec<Loot> {
    let mut rng = rng::Rng::stream(seed, "kill_loot_v1", spawn_id as u64);
    let mut out = Vec::new();
    let (gold, tonic, herbs): ((u32, u32, f32), f32, (u32, f32)) = match kind {
        SpawnKind::Boss => ((110, 140, 1.0), 2.0, (3, 1.0)),
        SpawnKind::Miniboss => ((30, 45, 1.0), 1.0, (2, 1.0)),
        SpawnKind::Elite => ((8, 14, 1.0), 0.3, (1, 0.4)),
        _ => ((2, 5, 0.3), 0.08, (1, 0.15)),
    };
    if rng.chance(gold.2) {
        out.extend(coins(
            gold.0 + rng.index((gold.1 - gold.0 + 1) as usize) as u32,
        ));
    }
    let tonics = tonic.floor() as u32 + u32::from(rng.chance(tonic.fract()));
    for _ in 0..tonics {
        out.push(Loot::Tonic);
    }
    if rng.chance(herbs.1) {
        out.push(Loot::Herbs(herbs.0 + rng.index(2) as u32));
    }
    out
}

/// Gold as a few coins on the floor (each a share of it).
pub fn coins(total: u32) -> Vec<Loot> {
    if total == 0 {
        return vec![];
    }
    let n = total.min(1 + total / 6).min(7);
    (0..n)
        .map(|k| Loot::Gold(total / n + u32::from(k < total % n)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coins_share_the_gold() {
        for total in [1, 5, 13, 60, 140] {
            let sum: u32 = coins(total)
                .iter()
                .map(|l| match l {
                    Loot::Gold(n) => *n,
                    _ => 0,
                })
                .sum();
            assert_eq!(sum, total);
            assert!(coins(total).len() <= 7);
        }
    }

    #[test]
    fn champions_pay_more_than_foes() {
        let gold = |kind| -> u32 {
            (0..40)
                .flat_map(|id| extra_loot(7, id, kind))
                .map(|l| match l {
                    Loot::Gold(n) => n,
                    _ => 0,
                })
                .sum()
        };
        assert!(gold(SpawnKind::Boss) > gold(SpawnKind::Miniboss));
        assert!(gold(SpawnKind::Miniboss) > gold(SpawnKind::Elite));
        assert!(gold(SpawnKind::Elite) > gold(SpawnKind::Enemy));
        assert!(extra_loot(7, 3, SpawnKind::Boss).contains(&Loot::Tonic));
    }
}
