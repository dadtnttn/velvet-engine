//! Explosive materials (gunpowder etc.).

use crate::cell::{Cell, MaterialId};
use crate::events::SimEvent;
use crate::rules::RuleCtx;

/// If a burning/hot explosive cell detonates, clear a radius and spawn fire.
pub fn rule_explosion(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        return false;
    }
    let def = ctx.mats().get(cell.material);
    if !def.reaction.explosive {
        return false;
    }
    let hot = cell.temp >= def.reaction.ignite_temp.max(50.0)
        || cell.flags.contains(crate::cell::CellFlags::BURNING);
    if !hot {
        return false;
    }
    let r = def.reaction.explosion_radius.max(1) as i32;
    let fire = ctx.world.materials.id("fire").unwrap_or(MaterialId::AIR);
    let cx = ctx.x;
    let cy = ctx.y;
    ctx.world.events.push(SimEvent::Exploded {
        x: cx,
        y: cy,
        radius: r as u8,
    });
    // destroy center first
    ctx.set_here(Cell::air());
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy > r * r {
                continue;
            }
            let nx = cx + dx;
            let ny = cy + dy;
            let n = ctx.world.get(nx, ny);
            // don't erase bedrock
            if ctx.world.materials.get(n.material).phase == crate::material::Phase::Static {
                continue;
            }
            if dx == 0 && dy == 0 {
                continue;
            }
            if !fire.is_air() && (dx * dx + dy * dy) <= (r * r / 2).max(1) {
                ctx.world
                    .set(nx, ny, Cell::of(fire).with_life(10).with_temp(900.0));
            } else {
                ctx.world.set(nx, ny, Cell::air());
            }
        }
    }
    true
}
