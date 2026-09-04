//! Node-anchored UI particles: bursts on clicks, sparkles on unlocks.
//!
//! Deliberately dependency-free (no GPU types): positions/velocities/
//! lifetimes simulate on CPU with a deterministic jitter cycle, and hosts
//! paint the [`UiParticles::points`] view (position, size, alpha). For
//! GPU upload, the packed layout mirrors `velvet_render::ParticleGpu`
//! field order (position, size, color, age).

use velvet_math::Vec2;

use crate::node::NodeId;
use crate::tree::UiTree;

/// One live UI particle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiParticle {
    /// Position in UI px.
    pub pos: Vec2,
    /// Velocity in UI px/s.
    pub vel: Vec2,
    /// Remaining life seconds.
    pub life: f32,
    /// Initial life seconds.
    pub max_life: f32,
    /// Base size in px.
    pub size: f32,
}

/// Paint-ready particle view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiParticlePoint {
    /// Position.
    pub pos: Vec2,
    /// Size.
    pub size: f32,
    /// Fade alpha 0..=1 (1 = born).
    pub alpha: f32,
}

/// Burst template (reused per effect kind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiBurst {
    /// Initial speed px/s.
    pub speed: f32,
    /// Lifetime seconds.
    pub life: f32,
    /// Base size px.
    pub size: f32,
}

impl UiBurst {
    /// Click confetti: fast, short, small.
    pub fn click() -> Self {
        Self {
            speed: 220.0,
            life: 0.45,
            size: 5.0,
        }
    }

    /// Unlock sparkle: slow rise, longer life.
    pub fn sparkle() -> Self {
        Self {
            speed: 60.0,
            life: 1.1,
            size: 7.0,
        }
    }
}

/// Particle layer over a UI tree.
#[derive(Debug, Clone, Default)]
pub struct UiParticles {
    particles: Vec<UiParticle>,
    /// Deterministic jitter cycle counter.
    tick_counter: u64,
    /// Hard cap (oldest culled).
    pub capacity: usize,
}

impl UiParticles {
    /// Empty layer with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            ..Default::default()
        }
    }

    /// Live count.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Burst `count` particles at a point.
    pub fn burst_at(&mut self, pos: Vec2, count: usize, burst: UiBurst) {
        for i in 0..count {
            // Deterministic fan: golden-angle spiral, no RNG needed.
            let a = self.tick_counter.wrapping_add(i as u64) as f32 * 2.399963;
            let speed = burst.speed * (0.4 + 0.6 * ((i % 3) as f32 / 2.0));
            self.push(UiParticle {
                pos,
                vel: Vec2::new(a.cos() * speed, a.sin() * speed),
                life: burst.life,
                max_life: burst.life,
                size: burst.size,
            });
        }
        self.tick_counter = self.tick_counter.wrapping_add(count as u64);
    }

    /// Burst at a node's center (missing nodes are ignored).
    pub fn burst_at_node(&mut self, tree: &UiTree, id: NodeId, count: usize, burst: UiBurst) {
        if let Some(node) = tree.get(id) {
            let center = node.rect.pos + node.rect.size * 0.5;
            self.burst_at(center, count, burst);
        }
    }

    /// Burst at the node named `name` (first match wins).
    pub fn burst_at_named(&mut self, tree: &UiTree, name: &str, count: usize, burst: UiBurst) {
        let mut found = None;
        tree.for_each_node(&mut |node| {
            if found.is_none() && node.name == name {
                found = Some(node.id);
            }
        });
        if let Some(id) = found {
            self.burst_at_node(tree, id, count, burst);
        }
    }

    fn push(&mut self, particle: UiParticle) {
        if self.particles.len() >= self.capacity {
            self.particles.remove(0);
        }
        self.particles.push(particle);
    }

    /// Advance simulation; dead particles drop.
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        for p in &mut self.particles {
            p.life -= dt;
            p.pos = Vec2::new(p.pos.x + p.vel.x * dt, p.pos.y + p.vel.y * dt);
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Paint view (faded alpha).
    pub fn points(&self) -> Vec<UiParticlePoint> {
        self.particles
            .iter()
            .map(|p| UiParticlePoint {
                pos: p.pos,
                size: p.size,
                alpha: (p.life / p.max_life).clamp(0.0, 1.0),
            })
            .collect()
    }

    /// Clear all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    fn tree_with_button() -> (UiTree, NodeId) {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let btn = ui.add_node(
            "go",
            WidgetKind::Button {
                label: "Go".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
            Some(root),
        );
        ui.layout(Vec2::new(800.0, 600.0));
        (ui, btn)
    }

    #[test]
    fn burst_spawns_and_fades() {
        let (ui, btn) = tree_with_button();
        let mut fx = UiParticles::with_capacity(64);
        fx.burst_at_node(&ui, btn, 8, UiBurst::click());
        assert_eq!(fx.len(), 8);
        // Center of the button rect.
        let c = ui.get(btn).unwrap().rect.pos + ui.get(btn).unwrap().rect.size * 0.5;
        assert!(fx.points().iter().all(|p| p.alpha == 1.0));
        fx.tick(0.225);
        assert!(fx.points().iter().all(|p| (p.alpha - 0.5).abs() < 1e-4));
        assert_ne!(fx.points()[0].pos, c);
        fx.tick(1.0);
        assert!(fx.is_empty());
    }

    #[test]
    fn missing_nodes_and_capacity() {
        let (ui, _) = tree_with_button();
        let mut fx = UiParticles::with_capacity(4);
        fx.burst_at_named(&ui, "nope", 10, UiBurst::click());
        assert!(fx.is_empty());
        fx.burst_at_named(&ui, "go", 10, UiBurst::click());
        assert_eq!(fx.len(), 4);
        // Deterministic: same sequence replays the same positions.
        let mut replay = UiParticles::with_capacity(4);
        replay.burst_at_named(&ui, "go", 10, UiBurst::click());
        assert_eq!(
            fx.points().iter().map(|p| p.pos).collect::<Vec<_>>(),
            replay.points().iter().map(|p| p.pos).collect::<Vec<_>>()
        );
    }
}
