//! Dissolution chemistry (acid etc.).

use crate::cell::Cell;
use crate::events::SimEvent;
use crate::rules::RuleCtx;

/// Agent dissolves adjacent targets listed in material.reaction.dissolves.
pub fn rule_dissolve(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        return false;
    }
    let def = ctx.mats().get(cell.material);
    if def.reaction.dissolves.is_empty() {
        return false;
    }
    let rate = def.reaction.dissolve_rate.max(1);
    let dissolves = def.reaction.dissolves.clone();
    let agent = cell.material;

    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let nx = ctx.x + dx;
        let ny = ctx.y + dy;
        let mut n = ctx.get(nx, ny);
        if n.is_air() {
            continue;
        }
        if !dissolves.contains(&n.material) {
            continue;
        }
        // accumulate damage in life
        let dmg = rate;
        if n.life < dmg {
            let target = n.material;
            ctx.world.set(nx, ny, Cell::air());
            ctx.world.events.push(SimEvent::Dissolved {
                x: nx,
                y: ny,
                target,
                agent,
            });
            // sometimes consume agent
            if ctx.chance(0.05) {
                ctx.set_here(Cell::air());
                return true;
            }
            return true;
        } else {
            n.life = n.life.saturating_sub(dmg);
            let cc = crate::chunk::ChunkCoord::from_cell(nx, ny);
            let (ox, oy) = cc.origin_cell();
            if let Some(ch) = ctx.world.chunk_mut(cc) {
                ch.set((nx - ox) as usize, (ny - oy) as usize, n);
            }
        }
    }
    false
}
