//! Multi-layer cellular world — background / main / foreground overlays.
//!
//! Authors can simulate the main grid while keeping deco/FG layers for props.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, MaterialId};
use crate::material::MaterialRegistry;
use crate::sim::{step, SimConfig};
use crate::world::{World, WorldConfig};

/// Layer kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// Background (not simulated).
    Background,
    /// Main simulation layer.
    Main,
    /// Foreground overlay (not simulated).
    Foreground,
}

/// Named layer.
#[derive(Debug, Clone)]
pub struct WorldLayer {
    /// Kind.
    pub kind: LayerKind,
    /// Name.
    pub name: String,
    /// Grid world (shares materials via copy of registry on create).
    pub world: World,
    /// Simulate this layer.
    pub simulate: bool,
}

/// Stack of layers for a scene.
#[derive(Debug, Clone)]
pub struct LayerStack {
    /// Layers ordered back → front.
    pub layers: Vec<WorldLayer>,
}

impl LayerStack {
    /// Create with one main layer.
    pub fn with_main(materials: MaterialRegistry, config: WorldConfig) -> Self {
        Self {
            layers: vec![WorldLayer {
                kind: LayerKind::Main,
                name: "main".into(),
                world: World::new(materials, config),
                simulate: true,
            }],
        }
    }

    /// Add background layer (same materials).
    pub fn add_background(&mut self, name: impl Into<String>) {
        let mats = self.main().world.materials.clone();
        let cfg = self.main().world.config.clone();
        self.layers.insert(
            0,
            WorldLayer {
                kind: LayerKind::Background,
                name: name.into(),
                world: World::new(mats, cfg),
                simulate: false,
            },
        );
    }

    /// Add foreground.
    pub fn add_foreground(&mut self, name: impl Into<String>) {
        let mats = self.main().world.materials.clone();
        let cfg = self.main().world.config.clone();
        self.layers.push(WorldLayer {
            kind: LayerKind::Foreground,
            name: name.into(),
            world: World::new(mats, cfg),
            simulate: false,
        });
    }

    /// Main layer ref.
    pub fn main(&self) -> &WorldLayer {
        self.layers
            .iter()
            .find(|l| l.kind == LayerKind::Main)
            .expect("main layer")
    }

    /// Main mut.
    pub fn main_mut(&mut self) -> &mut WorldLayer {
        self.layers
            .iter_mut()
            .find(|l| l.kind == LayerKind::Main)
            .expect("main layer")
    }

    /// Step all simulated layers.
    pub fn step_all(&mut self, cfg: &SimConfig) {
        for layer in &mut self.layers {
            if layer.simulate {
                step(&mut layer.world, cfg);
            }
        }
    }

    /// Composite sample: first non-air from front to back.
    pub fn sample_visible(&self, x: i32, y: i32) -> Cell {
        for layer in self.layers.iter().rev() {
            let c = layer.world.get(x, y);
            if !c.is_air() {
                return c;
            }
        }
        Cell::air()
    }

    /// Paint on layer by name.
    pub fn paint_on(&mut self, layer_name: &str, x: i32, y: i32, r: i32, mat: MaterialId) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.name == layer_name) {
            layer.world.paint_circle(x, y, r, mat);
            true
        } else {
            false
        }
    }

    /// Occupied on main.
    pub fn main_occupied(&self) -> usize {
        self.main().world.occupied_cells()
    }
}

/// Parallax scroll helper (offset sample).
pub fn sample_parallax(stack: &LayerStack, x: i32, y: i32, scroll_x: f32) -> Cell {
    // sample BG shifted
    for layer in stack.layers.iter().rev() {
        let sx = if layer.kind == LayerKind::Background {
            x + (scroll_x * 0.5) as i32
        } else if layer.kind == LayerKind::Foreground {
            x + (scroll_x * 1.5) as i32
        } else {
            x
        };
        let c = layer.world.get(sx, y);
        if !c.is_air() {
            return c;
        }
    }
    Cell::air()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;

    #[test]
    fn layers_composite_and_step_main_only() {
        let (reg, ids) = builtin_registry();
        let mut stack = LayerStack::with_main(reg, WorldConfig::default());
        stack.add_background("bg");
        stack.add_foreground("fg");
        assert!(stack.paint_on("bg", 0, 0, 2, ids.stone));
        stack.main_mut().world.set(5, 5, Cell::of(ids.sand));
        stack.paint_on("fg", 5, 5, 1, ids.water);
        // visible at 5,5 is FG water
        assert_eq!(stack.sample_visible(5, 5).material, ids.water);
        // visible at 0,0 is BG stone
        assert_eq!(stack.sample_visible(0, 0).material, ids.stone);
        let cfg = SimConfig::default();
        stack.step_all(&cfg);
        assert!(stack.main_occupied() >= 1 || stack.main().world.get(5, 5).material == ids.sand);
    }
}
