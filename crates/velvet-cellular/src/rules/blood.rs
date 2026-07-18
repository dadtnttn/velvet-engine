//! Blood physics — viscous liquid with clotting and drying.

use crate::cell::{Cell, MaterialId};
use crate::rules::RuleCtx;

/// Blood-specific behaviour: high viscosity spread, clot → dried blood, drip trails.
pub fn rule_blood(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        return false;
    }
    let key = ctx.mats().get(cell.material).key.as_str();
    if key != "blood" && key != "fresh_blood" {
        // flesh leaks blood when damaged (hot or low life)
        if key == "flesh" && (cell.life > 0 && cell.life < 20 || cell.temp > 60.0) {
            if let Ok(blood) = ctx.world.materials.id("blood") {
                let below = ctx.get(ctx.x, ctx.y - 1);
                if below.is_air() && ctx.chance(0.08) {
                    ctx.world
                        .set(ctx.x, ctx.y - 1, Cell::of(blood).with_life(90));
                    return true;
                }
            }
        }
        return false;
    }

    let mut c = cell;
    // dry / clot over time
    if c.life == 0 {
        c.life = 180;
    }
    if c.life > 0 {
        c.life -= 1;
    }
    if c.life == 0 {
        if let Ok(dried) = ctx.world.materials.id("dried_blood") {
            ctx.set_here(Cell::of(dried));
            return true;
        }
        // else stay as powder-ish
        if let Ok(ash) = ctx.world.materials.id("ash") {
            ctx.set_here(Cell::of(ash));
            return true;
        }
    }

    // sticky: slower horizontal than normal liquid (extra viscosity pass)
    let below = ctx.get(ctx.x, ctx.y - 1);
    if !below.is_air() {
        // smear sideways slowly
        let dx = if ctx.chance(0.5) { 1 } else { -1 };
        let n = ctx.get(ctx.x + dx, ctx.y);
        if n.is_air() && ctx.chance(0.12) {
            ctx.swap(ctx.x + dx, ctx.y);
            return true;
        }
        // stain solids occasionally
        if matches!(
            ctx.phase(below.material),
            crate::material::Phase::Solid | crate::material::Phase::Static
        ) && ctx.chance(0.01)
        {
            if let Ok(dried) = ctx.world.materials.id("dried_blood") {
                // leave a stain nearby if air diagonal
                let stain = ctx.get(ctx.x + dx, ctx.y - 1);
                if stain.is_air() {
                    ctx.world
                        .set(ctx.x + dx, ctx.y - 1, Cell::of(dried).with_life(0));
                }
            }
        }
    }

    // write life decay
    let cc = crate::chunk::ChunkCoord::from_cell(ctx.x, ctx.y);
    let (ox, oy) = cc.origin_cell();
    if let Some(ch) = ctx.world.chunk_mut(cc) {
        ch.cells[crate::chunk::Chunk::idx((ctx.x - ox) as usize, (ctx.y - oy) as usize)].life =
            c.life;
    }
    false
}

/// Spawn a blood burst (weapon / death helper).
pub fn splatter_blood(world: &mut crate::world::World, x: i32, y: i32, radius: i32) {
    let blood = world.mat("blood");
    if blood.is_air() {
        return;
    }
    world.paint_circle(x, y, radius, blood);
    // life on blood cells for clotting timer
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let cx = x + dx;
            let cy = y + dy;
            let mut c = world.get(cx, cy);
            if c.material == blood {
                c.life = 100 + (world.next_u32() % 80) as u16;
                world.set(cx, cy, c);
            }
        }
    }
    world.events.push(crate::events::SimEvent::BloodSplatter {
        x,
        y,
        radius,
    });
    let _ = MaterialId::AIR;
}
