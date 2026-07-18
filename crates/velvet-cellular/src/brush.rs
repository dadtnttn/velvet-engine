//! Author brush editor — the creation tool surface (not a full GUI chrome).
//!
//! Games and Studio call the same APIs: stroke, stamp, line, fill, replace, spray.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, MaterialId};
use crate::world::World;

/// Brush shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrushShape {
    /// Filled disk.
    #[default]
    Circle,
    /// Axis-aligned square.
    Square,
    /// Diamond (manhattan).
    Diamond,
    /// Single cell.
    Point,
    /// Horizontal spray noise.
    Spray,
}

/// What the brush does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrushMode {
    /// Paint material.
    #[default]
    Paint,
    /// Erase to air.
    Erase,
    /// Replace only cells matching `mask` material.
    Replace,
    /// Sample material under cursor into brush.
    Sample,
    /// Heat cells (raise temperature).
    Heat,
    /// Cool cells.
    Cool,
    /// Ignite flammables / spawn fire.
    Ignite,
    /// Spawn blood splat (uses blood material if registered).
    Bleed,
    /// Soften / dig solids into powder if possible.
    Dig,
}

/// Brush state for authors / editors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brush {
    /// Shape.
    pub shape: BrushShape,
    /// Mode.
    pub mode: BrushMode,
    /// Radius in cells (half-extent for square).
    pub radius: i32,
    /// Material to paint (or replace-to).
    pub material: MaterialId,
    /// Mask for Replace mode (None = any non-air).
    pub mask: Option<MaterialId>,
    /// Spray density 0..=1.
    pub spray_density: f32,
    /// Strength for heat/cool (°C).
    pub temp_delta: f32,
    /// Softness: chance to skip edge cells 0..=1.
    pub softness: f32,
    /// Whether stroke is active (mouse held).
    pub stroking: bool,
    /// Last cell painted (for line continuity).
    pub last: Option<(i32, i32)>,
    /// Cells painted this stroke (stats).
    pub stroke_cells: u32,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            shape: BrushShape::Circle,
            mode: BrushMode::Paint,
            radius: 3,
            material: MaterialId::AIR,
            mask: None,
            spray_density: 0.35,
            temp_delta: 80.0,
            softness: 0.0,
            stroking: false,
            last: None,
            stroke_cells: 0,
        }
    }
}

impl Brush {
    /// Paint with material id.
    pub fn with_material(mut self, id: MaterialId) -> Self {
        self.material = id;
        self
    }

    /// Radius.
    pub fn with_radius(mut self, r: i32) -> Self {
        self.radius = r.max(0);
        self
    }

    /// Shape.
    pub fn with_shape(mut self, s: BrushShape) -> Self {
        self.shape = s;
        self
    }

    /// Mode.
    pub fn with_mode(mut self, m: BrushMode) -> Self {
        self.mode = m;
        self
    }

    /// Begin stroke at cell.
    pub fn begin_stroke(&mut self, world: &mut World, x: i32, y: i32) -> u32 {
        self.stroking = true;
        self.stroke_cells = 0;
        self.last = Some((x, y));
        self.apply_stamp(world, x, y)
    }

    /// Continue stroke (line from last to current).
    pub fn move_stroke(&mut self, world: &mut World, x: i32, y: i32) -> u32 {
        if !self.stroking {
            return self.begin_stroke(world, x, y);
        }
        let mut n = 0u32;
        if let Some((lx, ly)) = self.last {
            n += self.apply_line(world, lx, ly, x, y);
        } else {
            n += self.apply_stamp(world, x, y);
        }
        self.last = Some((x, y));
        n
    }

    /// End stroke.
    pub fn end_stroke(&mut self) {
        self.stroking = false;
        self.last = None;
    }

    /// Single stamp at cell.
    pub fn apply_stamp(&mut self, world: &mut World, cx: i32, cy: i32) -> u32 {
        if self.mode == BrushMode::Sample {
            let c = world.get(cx, cy);
            if !c.is_air() {
                self.material = c.material;
            }
            return 0;
        }
        let r = self.radius;
        let mut painted = 0u32;
        for dy in -r..=r {
            for dx in -r..=r {
                if !self.in_shape(dx, dy, r) {
                    continue;
                }
                if self.softness > 0.0 {
                    let edge = ((dx * dx + dy * dy) as f32).sqrt() / (r as f32).max(1.0);
                    if edge > 0.5 && world.chance(self.softness * edge) {
                        continue;
                    }
                }
                if self.shape == BrushShape::Spray && !world.chance(self.spray_density) {
                    continue;
                }
                let x = cx + dx;
                let y = cy + dy;
                if self.apply_cell(world, x, y) {
                    painted += 1;
                }
            }
        }
        self.stroke_cells = self.stroke_cells.saturating_add(painted);
        painted
    }

    /// Bresenham line of stamps.
    pub fn apply_line(&mut self, world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32) -> u32 {
        let mut n = 0u32;
        let mut x = x0;
        let mut y = y0;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            n += self.apply_stamp(world, x, y);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        n
    }

    /// Flood fill with brush material (bounded).
    pub fn flood(&mut self, world: &mut World, x: i32, y: i32, max_cells: usize) -> u32 {
        if self.mode == BrushMode::Erase {
            return world.flood_fill(x, y, MaterialId::AIR, max_cells) as u32;
        }
        world.flood_fill(x, y, self.material, max_cells) as u32
    }

    /// Rectangle stamp (axis-aligned).
    pub fn apply_rect(
        &mut self,
        world: &mut World,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    ) -> u32 {
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        let mut n = 0u32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.apply_cell(world, x, y) {
                    n += 1;
                }
            }
        }
        self.stroke_cells = self.stroke_cells.saturating_add(n);
        n
    }

    fn in_shape(&self, dx: i32, dy: i32, r: i32) -> bool {
        match self.shape {
            BrushShape::Point => dx == 0 && dy == 0,
            BrushShape::Circle | BrushShape::Spray => dx * dx + dy * dy <= r * r,
            BrushShape::Square => dx.abs() <= r && dy.abs() <= r,
            BrushShape::Diamond => dx.abs() + dy.abs() <= r,
        }
    }

    fn apply_cell(&self, world: &mut World, x: i32, y: i32) -> bool {
        let cur = world.get(x, y);
        match self.mode {
            BrushMode::Paint => {
                world.set(x, y, Cell::of(self.material));
                true
            }
            BrushMode::Erase => {
                if cur.is_air() {
                    return false;
                }
                world.set(x, y, Cell::air());
                true
            }
            BrushMode::Replace => {
                let ok = match self.mask {
                    Some(m) => cur.material == m,
                    None => !cur.is_air(),
                };
                if !ok {
                    return false;
                }
                world.set(x, y, Cell::of(self.material));
                true
            }
            BrushMode::Sample => false,
            BrushMode::Heat => {
                if cur.is_air() {
                    return false;
                }
                let mut c = cur;
                c.temp += self.temp_delta;
                world.set(x, y, c);
                true
            }
            BrushMode::Cool => {
                if cur.is_air() {
                    return false;
                }
                let mut c = cur;
                c.temp -= self.temp_delta;
                world.set(x, y, c);
                true
            }
            BrushMode::Ignite => {
                if let Ok(fire) = world.materials.id("fire") {
                    world.set(x, y, Cell::of(fire).with_life(16).with_temp(900.0));
                    true
                } else {
                    let mut c = cur;
                    c.temp = c.temp.max(400.0);
                    c.flags.insert(crate::cell::CellFlags::BURNING);
                    world.set(x, y, c);
                    true
                }
            }
            BrushMode::Bleed => {
                let blood = world.materials.id("blood").unwrap_or(self.material);
                // blood prefers empty or replaces non-static
                if matches!(
                    world.materials.phase(cur.material),
                    crate::material::Phase::Static
                ) {
                    return false;
                }
                if cur.is_air() || world.chance(0.7) {
                    world.set(x, y, Cell::of(blood).with_life(120));
                    true
                } else {
                    false
                }
            }
            BrushMode::Dig => {
                use crate::material::Phase;
                let p = world.materials.phase(cur.material);
                if matches!(p, Phase::Solid | Phase::Powder | Phase::Static) {
                    if world.materials.get(cur.material).key == "bedrock" {
                        return false;
                    }
                    // turn into sand/ash if available else air
                    let powder = world
                        .materials
                        .id("sand")
                        .or_else(|_| world.materials.id("ash"))
                        .unwrap_or(MaterialId::AIR);
                    world.set(x, y, Cell::of(powder));
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// Preset palette slot for UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrushPreset {
    /// Name.
    pub name: String,
    /// Material key.
    pub material_key: String,
    /// Shape.
    pub shape: BrushShape,
    /// Mode.
    pub mode: BrushMode,
    /// Radius.
    pub radius: i32,
}

/// Default author palette.
pub fn default_brush_presets() -> Vec<BrushPreset> {
    vec![
        BrushPreset {
            name: "Sand".into(),
            material_key: "sand".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Paint,
            radius: 4,
        },
        BrushPreset {
            name: "Water".into(),
            material_key: "water".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Paint,
            radius: 3,
        },
        BrushPreset {
            name: "Blood".into(),
            material_key: "blood".into(),
            shape: BrushShape::Spray,
            mode: BrushMode::Bleed,
            radius: 5,
        },
        BrushPreset {
            name: "Stone wall".into(),
            material_key: "stone".into(),
            shape: BrushShape::Square,
            mode: BrushMode::Paint,
            radius: 2,
        },
        BrushPreset {
            name: "Eraser".into(),
            material_key: "air".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Erase,
            radius: 4,
        },
        BrushPreset {
            name: "Fire".into(),
            material_key: "fire".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Ignite,
            radius: 2,
        },
        BrushPreset {
            name: "Acid".into(),
            material_key: "acid".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Paint,
            radius: 2,
        },
        BrushPreset {
            name: "Dig".into(),
            material_key: "sand".into(),
            shape: BrushShape::Circle,
            mode: BrushMode::Dig,
            radius: 3,
        },
    ]
}

/// Apply a named preset onto a brush (resolves material key).
pub fn apply_preset(brush: &mut Brush, world: &World, preset: &BrushPreset) {
    brush.shape = preset.shape;
    brush.mode = preset.mode;
    brush.radius = preset.radius;
    brush.material = world.mat(&preset.material_key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn brush_paints_circle_of_sand() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut brush = Brush::default()
            .with_material(ids.sand)
            .with_radius(3)
            .with_shape(BrushShape::Circle);
        let n = brush.begin_stroke(&mut world, 0, 10);
        assert!(n > 5);
        assert_eq!(world.get(0, 10).material, ids.sand);
        brush.end_stroke();
    }

    #[test]
    fn brush_line_and_erase() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut brush = Brush::default()
            .with_material(ids.stone)
            .with_radius(0)
            .with_shape(BrushShape::Point);
        brush.apply_line(&mut world, 0, 0, 10, 0);
        assert_eq!(world.get(5, 0).material, ids.stone);
        brush.mode = BrushMode::Erase;
        brush.apply_stamp(&mut world, 5, 0);
        assert!(world.get(5, 0).is_air());
    }
}
