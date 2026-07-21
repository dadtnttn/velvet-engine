//! Multi-target animation director (cards, actors, UI ids).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::effect::{build_effect, tick_tweens, EffectKind, EffectParams};
use crate::pose::AnimPose;
use crate::tween::{parse_ease, FloatTween};

/// One named animated object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimTarget {
    /// Stable id (e.g. `card_0`, `hero`, `ui.banner`).
    pub id: String,
    /// Current pose.
    pub pose: AnimPose,
    /// Active tweens.
    pub tweens: Vec<FloatTween>,
    /// Optional tag for batch effects.
    pub tag: Option<String>,
}

impl AnimTarget {
    /// New target at pose.
    pub fn new(id: impl Into<String>, pose: AnimPose) -> Self {
        Self {
            id: id.into(),
            pose,
            tweens: Vec::new(),
            tag: None,
        }
    }

    /// Idle / no active tweens.
    pub fn is_idle(&self) -> bool {
        self.tweens.is_empty() || self.tweens.iter().all(|t| t.finished())
    }

    /// Replace tweens (clears finished).
    pub fn play_tweens(&mut self, tweens: Vec<FloatTween>) {
        self.tweens = tweens;
    }

    /// Append tweens.
    pub fn push_tweens(&mut self, tweens: Vec<FloatTween>) {
        self.tweens.extend(tweens);
    }

    fn prune_finished(&mut self) {
        self.tweens.retain(|t| !t.finished());
    }
}

/// Central animation tool: spawn targets, run effects, tick.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnimDirector {
    /// Targets by id.
    pub targets: IndexMap<String, AnimTarget>,
    /// Log of recent effect spawns (script/debug).
    pub log: Vec<String>,
}

impl AnimDirector {
    /// Empty director.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensure target exists (creates at default pose).
    pub fn ensure(&mut self, id: &str) -> &mut AnimTarget {
        if !self.targets.contains_key(id) {
            self.targets
                .insert(id.into(), AnimTarget::new(id, AnimPose::default()));
        }
        self.targets.get_mut(id).expect("just inserted")
    }

    /// Spawn or reset target at position.
    pub fn spawn_at(&mut self, id: impl Into<String>, pos: Vec2) -> &mut AnimTarget {
        let id = id.into();
        let t = AnimTarget::new(id.clone(), AnimPose::at(pos));
        self.targets.insert(id.clone(), t);
        self.targets.get_mut(&id).expect("inserted")
    }

    /// Current pose (or None).
    pub fn pose(&self, id: &str) -> Option<&AnimPose> {
        self.targets.get(id).map(|t| &t.pose)
    }

    /// Play a named effect on a target.
    pub fn play_effect(&mut self, id: &str, kind: EffectKind, params: EffectParams) -> bool {
        let target = self.ensure(id);
        let tweens = build_effect(kind, &target.pose, params);
        // For deal/fade_in, seed opacity if needed
        if matches!(
            kind,
            EffectKind::FadeIn | EffectKind::Deal | EffectKind::PopIn | EffectKind::BounceIn
        ) {
            target.pose.opacity = 0.0;
        }
        if matches!(kind, EffectKind::Deal) && params.to.length_squared() > 1e-6 {
            // leave pose; tweens drive from offset
        }
        target.play_tweens(tweens);
        self.log.push(format!(
            "fx {} {} d={:.2}",
            id,
            kind.as_str(),
            params.duration
        ));
        if self.log.len() > 64 {
            let n = self.log.len() - 64;
            self.log.drain(0..n);
        }
        true
    }

    /// Move target to (x,y) over duration.
    pub fn move_to(&mut self, id: &str, x: f32, y: f32, duration: f32, ease: &str) {
        self.play_effect(
            id,
            EffectKind::MoveTo,
            EffectParams {
                to: Vec2::new(x, y),
                duration,
                ease: parse_ease(ease),
                ..Default::default()
            },
        );
    }

    /// Stop tweens on target (pose freezes).
    pub fn stop(&mut self, id: &str) {
        if let Some(t) = self.targets.get_mut(id) {
            t.tweens.clear();
        }
    }

    /// Remove target.
    pub fn despawn(&mut self, id: &str) {
        self.targets.shift_remove(id);
    }

    /// Advance all animations by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        for t in self.targets.values_mut() {
            if t.tweens.is_empty() {
                continue;
            }
            tick_tweens(&mut t.pose, &mut t.tweens, dt);
            t.prune_finished();
        }
    }

    /// True if every target is idle.
    pub fn all_idle(&self) -> bool {
        self.targets.values().all(|t| t.is_idle())
    }

    /// Snapshot poses for rendering.
    pub fn poses(&self) -> impl Iterator<Item = (&str, &AnimPose)> {
        self.targets.iter().map(|(k, v)| (k.as_str(), &v.pose))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deal_card_animates_to_slot() {
        let mut d = AnimDirector::new();
        d.spawn_at("card0", Vec2::new(0.0, 0.0));
        d.play_effect(
            "card0",
            EffectKind::Deal,
            EffectParams {
                to: Vec2::new(200.0, 300.0),
                duration: 0.4,
                ..Default::default()
            },
        );
        assert!(!d.all_idle());
        for _ in 0..30 {
            d.tick(1.0 / 60.0);
        }
        let p = d.pose("card0").unwrap();
        assert!(p.opacity > 0.5, "opacity={}", p.opacity);
        assert!((p.pos.x - 200.0).abs() < 30.0, "x={}", p.pos.x);
        assert!((p.pos.y - 300.0).abs() < 40.0, "y={}", p.pos.y);
    }

    #[test]
    fn move_and_idle() {
        let mut d = AnimDirector::new();
        d.spawn_at("ui", Vec2::new(0.0, 0.0));
        d.move_to("ui", 100.0, 50.0, 0.2, "cubic_out");
        for _ in 0..20 {
            d.tick(0.05);
        }
        assert!(d.all_idle());
        let p = d.pose("ui").unwrap();
        assert!((p.pos.x - 100.0).abs() < 1.0);
        assert!((p.pos.y - 50.0).abs() < 1.0);
    }
}
