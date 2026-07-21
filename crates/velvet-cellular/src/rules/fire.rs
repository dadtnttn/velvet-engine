//! Fire / plasma: ignition, burn life, heat, residue.

use crate::cell::{Cell, CellFlags, MaterialId};
use crate::events::SimEvent;
use crate::material::Phase;
use crate::rules::RuleCtx;

/// Fire and flammable interactions.
pub fn rule_fire(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        return false;
    }
    let def = ctx.mats().get(cell.material).clone();
    let phase = def.phase;

    // Burning cell: tick life, spread heat, produce residue
    if cell.flags.contains(CellFlags::BURNING) || phase == Phase::Plasma {
        let mut c = cell;
        // heat neighbors
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (0, 1)] {
            let nx = ctx.x + dx;
            let ny = ctx.y + dy;
            let mut n = ctx.get(nx, ny);
            if n.is_air() {
                continue;
            }
            n.temp += def.reaction.burn_heat * 0.25;
            // write temp without full set event storm
            write_cell(ctx, nx, ny, n);
            try_ignite(ctx, nx, ny);
        }

        if phase == Phase::Plasma {
            // fire dies over life
            if c.life == 0 {
                c.life = def.reaction.burn_life.max(8);
            }
            if c.life > 0 {
                c.life -= 1;
            }
            if c.life == 0 {
                let to = def.reaction.burn_residue.unwrap_or(MaterialId::AIR);
                let prev = c.material;
                c = Cell::of(to);
                ctx.set_here(c);
                ctx.world.events.push(SimEvent::MaterialChanged {
                    x: ctx.x,
                    y: ctx.y,
                    from: prev,
                    to,
                });
                return true;
            } else {
                c.temp = c.temp.max(400.0);
                ctx.set_here(c);
            }
            // spawn smoke product above
            if let Some(prod) = def.reaction.burn_product {
                let above = ctx.get(ctx.x, ctx.y + 1);
                if above.is_air() && ctx.chance(0.15) {
                    ctx.world
                        .set(ctx.x, ctx.y + 1, Cell::of(prod).with_life(30));
                }
            }
            return false;
        }

        // flammable solid/powder burning
        if c.life == 0 {
            c.life = def.reaction.burn_life.max(1);
        }
        if c.life > 0 {
            c.life -= 1;
        }
        c.temp += def.reaction.burn_heat;
        if c.life == 0 {
            let to = def.reaction.burn_residue.unwrap_or(MaterialId::AIR);
            let prev = c.material;
            ctx.set_here(Cell::of(to));
            ctx.world.events.push(SimEvent::MaterialChanged {
                x: ctx.x,
                y: ctx.y,
                from: prev,
                to,
            });
            return true;
        }
        c.flags.insert(CellFlags::BURNING);
        ctx.set_here(c);
        return false;
    }

    // hot enough flammable?
    if def.reaction.flammable && cell.temp >= def.reaction.ignite_temp {
        try_ignite(ctx, ctx.x, ctx.y);
    }

    // extinguish fire with water-like
    if def.reaction.extinguishes {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = ctx.x + dx;
            let ny = ctx.y + dy;
            let n = ctx.get(nx, ny);
            if n.flags.contains(CellFlags::BURNING) || ctx.phase(n.material) == Phase::Plasma {
                ctx.world.set(nx, ny, Cell::air());
            }
        }
    }

    false
}

fn try_ignite(ctx: &mut RuleCtx<'_>, x: i32, y: i32) {
    let mut c = ctx.world.get(x, y);
    if c.is_air() {
        return;
    }
    let def = ctx.world.materials.get(c.material);
    if !def.reaction.flammable && def.phase != Phase::Plasma {
        return;
    }
    if c.temp < def.reaction.ignite_temp && def.phase != Phase::Plasma {
        return;
    }
    if c.flags.contains(CellFlags::BURNING) {
        return;
    }
    c.flags.insert(CellFlags::BURNING);
    if c.life == 0 {
        c.life = def.reaction.burn_life.max(1);
    }
    // convert wood etc. to fire plasma if configured burn product path
    if def.phase != Phase::Plasma {
        if let Ok(fire_id) = ctx.world.materials.id("fire") {
            let life = def.reaction.burn_life;
            ctx.world
                .set(x, y, Cell::of(fire_id).with_life(life).with_temp(800.0));
            // mark burning on fire
            let mut f = ctx.world.get(x, y);
            f.flags.insert(CellFlags::BURNING);
            write_cell(ctx, x, y, f);
            ctx.world.events.push(SimEvent::Ignited {
                x,
                y,
                material: c.material,
            });
            return;
        }
    }
    write_cell(ctx, x, y, c);
    ctx.world.events.push(SimEvent::Ignited {
        x,
        y,
        material: c.material,
    });
}

fn write_cell(ctx: &mut RuleCtx<'_>, x: i32, y: i32, cell: Cell) {
    let cc = crate::chunk::ChunkCoord::from_cell(x, y);
    let (ox, oy) = cc.origin_cell();
    let chunk = ctx.world.ensure_chunk(cc);
    chunk.set((x - ox) as usize, (y - oy) as usize, cell);
}
