//! Damage application helpers.

use velvet_math::Vec2;
use velvet_play::Health;

/// Damage event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageEvent {
    /// Target id.
    pub target: usize,
    /// Source id.
    pub source: usize,
    /// Amount.
    pub amount: f32,
    /// Hit position.
    pub point: Vec2,
}

/// Death event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathEvent {
    /// Entity.
    pub target: usize,
    /// Killer.
    pub source: usize,
}

/// Apply damage; returns death event if fatal.
pub fn apply_damage(
    health: &mut Health,
    target: usize,
    source: usize,
    amount: f32,
    point: Vec2,
) -> (DamageEvent, Option<DeathEvent>) {
    let dmg = DamageEvent {
        target,
        source,
        amount,
        point,
    };
    let dead = health.damage(amount);
    let death = if dead {
        Some(DeathEvent { target, source })
    } else {
        None
    };
    (dmg, death)
}

/// Melee query: targets in range and facing cone.
pub fn melee_targets(
    origin: Vec2,
    facing: Vec2,
    range: f32,
    half_arc: f32,
    candidates: &[(usize, Vec2)],
) -> Vec<usize> {
    let face = facing.normalize_or_zero();
    let mut out = Vec::new();
    for (id, pos) in candidates {
        let offset = *pos - origin;
        let dist = offset.length();
        if dist > range || dist < 1e-4 {
            continue;
        }
        let dir = offset * (1.0 / dist);
        let angle = face.dot(dir).clamp(-1.0, 1.0).acos();
        if angle <= half_arc {
            out.push(*id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn melee_cone() {
        let hits = melee_targets(
            Vec2::ZERO,
            Vec2::X,
            50.0,
            0.5,
            &[(1, Vec2::new(20.0, 0.0)), (2, Vec2::new(0.0, 40.0))],
        );
        assert_eq!(hits, vec![1]);
    }

    #[test]
    fn damage_kills() {
        let mut h = Health::full(10.0);
        let (_, death) = apply_damage(&mut h, 1, 0, 50.0, Vec2::ZERO);
        assert!(death.is_some());
    }

    #[test]
    fn damage_nonlethal_and_event_fields() {
        let mut h = Health::full(100.0);
        let (dmg, death) = apply_damage(&mut h, 7, 3, 15.0, Vec2::new(1.0, 2.0));
        assert!(death.is_none());
        assert_eq!(dmg.target, 7);
        assert_eq!(dmg.source, 3);
        assert!((dmg.amount - 15.0).abs() < 1e-5);
        assert!((dmg.point.x - 1.0).abs() < 1e-5);
        assert!(h.current < 100.0);
        assert!(h.current > 0.0);
    }

    #[test]
    fn melee_range_and_behind() {
        let origin = Vec2::ZERO;
        let facing = Vec2::X;
        let candidates = [
            (1, Vec2::new(10.0, 0.0)),  // front in range
            (2, Vec2::new(100.0, 0.0)), // front out of range
            (3, Vec2::new(-10.0, 0.0)), // behind
            (4, Vec2::new(5.0, 5.0)),   // angled
        ];
        let hits = melee_targets(origin, facing, 20.0, 0.4, &candidates);
        assert!(hits.contains(&1));
        assert!(!hits.contains(&2));
        assert!(!hits.contains(&3));
        // Wide arc should include angled target.
        let wide = melee_targets(origin, facing, 20.0, 1.2, &candidates);
        assert!(wide.contains(&1));
        assert!(wide.contains(&4), "wide arc misses angled target: {wide:?}");
        assert!(!wide.contains(&2));
        assert!(!wide.contains(&3));
    }

    #[test]
    fn melee_zero_range_and_self() {
        let hits = melee_targets(Vec2::ZERO, Vec2::Y, 0.0, 1.0, &[(1, Vec2::new(0.0, 1.0))]);
        assert!(hits.is_empty());
        // Candidate on origin skipped (dist ~ 0).
        let hits2 = melee_targets(Vec2::ZERO, Vec2::X, 10.0, 1.0, &[(9, Vec2::ZERO)]);
        assert!(hits2.is_empty());
    }

    #[test]
    fn sequential_damage_to_death() {
        let mut h = Health::full(30.0);
        let mut deaths = 0;
        for _ in 0..5 {
            let (_, d) = apply_damage(&mut h, 1, 0, 10.0, Vec2::ZERO);
            if d.is_some() {
                deaths += 1;
            }
        }
        assert_eq!(deaths, 1);
        assert!(!h.is_alive());
        assert_eq!(h.current, 0.0);
    }
}
