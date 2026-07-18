//! Heat diffusion and phase transitions (melt / boil / freeze).

use crate::cell::Cell;
use crate::events::SimEvent;
use crate::rules::RuleCtx;

/// Diffuse temperature and apply phase changes.
pub fn rule_temperature(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        // ambient bleed
        return false;
    }
    let def = ctx.mats().get(cell.material).clone();
    let mut c = cell;
    let cond = def.physics.conductivity.clamp(0.0, 1.0);
    let ambient = ctx.world.config.ambient_temp;

    // average with 4-neighbors
    let mut acc = c.temp;
    let mut n_count = 1.0f32;
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let n = ctx.get(ctx.x + dx, ctx.y + dy);
        if n.is_air() {
            acc += ambient;
        } else {
            acc += n.temp;
        }
        n_count += 1.0;
    }
    let target = acc / n_count;
    c.temp += (target - c.temp) * cond * 0.35;
    // relax toward ambient slowly
    c.temp += (ambient - c.temp) * 0.01;

    // phase transitions
    if let (Some(mp), Some(into)) = (def.physics.melt_point, def.physics.melt_into) {
        if c.temp >= mp {
            let prev = c.material;
            c = Cell::of(into).with_temp(c.temp);
            ctx.set_here(c);
            ctx.world.events.push(SimEvent::MaterialChanged {
                x: ctx.x,
                y: ctx.y,
                from: prev,
                to: into,
            });
            return true;
        }
    }
    if let (Some(bp), Some(into)) = (def.physics.boil_point, def.physics.boil_into) {
        if c.temp >= bp {
            let prev = c.material;
            c = Cell::of(into).with_temp(c.temp);
            ctx.set_here(c);
            ctx.world.events.push(SimEvent::MaterialChanged {
                x: ctx.x,
                y: ctx.y,
                from: prev,
                to: into,
            });
            return true;
        }
    }
    if let (Some(fp), Some(into)) = (def.physics.freeze_point, def.physics.freeze_into) {
        if c.temp <= fp {
            let prev = c.material;
            c = Cell::of(into).with_temp(c.temp);
            ctx.set_here(c);
            ctx.world.events.push(SimEvent::MaterialChanged {
                x: ctx.x,
                y: ctx.y,
                from: prev,
                to: into,
            });
            return true;
        }
    }

    // write temp if changed
    if (c.temp - cell.temp).abs() > 0.05 {
        let cc = crate::chunk::ChunkCoord::from_cell(ctx.x, ctx.y);
        let (ox, oy) = cc.origin_cell();
        if let Some(ch) = ctx.world.chunk_mut(cc) {
            ch.set((ctx.x - ox) as usize, (ctx.y - oy) as usize, c);
        }
    }
    false
}
