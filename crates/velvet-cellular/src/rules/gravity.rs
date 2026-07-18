//! Gravity: powders and liquids fall into air / lighter gas.

use crate::cell::CellFlags;
use crate::material::Phase;
use crate::rules::RuleCtx;

/// Apply gravity at (x,y). Returns true if moved.
pub fn rule_gravity(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() || cell.flags.contains(CellFlags::MOVED) {
        return false;
    }
    if !ctx.mats().gravity_applies(cell.material) {
        return false;
    }
    let phase = ctx.phase(cell.material);
    if matches!(phase, Phase::Solid | Phase::Static) {
        return false;
    }

    let below = ctx.get(ctx.x, ctx.y - 1);
    // Fall straight down into air or gas
    if can_fall_into(ctx, cell.material, below) {
        ctx.swap(ctx.x, ctx.y - 1);
        mark_moved(ctx, ctx.x, ctx.y - 1);
        return true;
    }

    // Diagonal fall for powders / free liquids
    if matches!(phase, Phase::Powder | Phase::Liquid | Phase::Plasma) {
        let prefer_right = ctx.chance(0.5);
        let order = if prefer_right {
            [1, -1]
        } else {
            [-1, 1]
        };
        for dx in order {
            let nx = ctx.x + dx;
            let ny = ctx.y - 1;
            let n = ctx.get(nx, ny);
            if can_fall_into(ctx, cell.material, n) {
                ctx.swap(nx, ny);
                mark_moved(ctx, nx, ny);
                return true;
            }
        }
    }

    // Liquids spread horizontally when blocked
    if phase == Phase::Liquid {
        let prefer_right = ctx.chance(0.5);
        let order = if prefer_right {
            [1, -1]
        } else {
            [-1, 1]
        };
        for dx in order {
            let nx = ctx.x + dx;
            let n = ctx.get(nx, ctx.y);
            if can_fall_into(ctx, cell.material, n) {
                // viscosity: random skip
                let visc = ctx.mats().get(cell.material).physics.viscosity;
                if ctx.chance(1.0 - visc.min(0.95)) {
                    ctx.swap(nx, ctx.y);
                    mark_moved(ctx, nx, ctx.y);
                    return true;
                }
            }
        }
    }

    // Plasma / fire rises
    if phase == Phase::Plasma {
        let above = ctx.get(ctx.x, ctx.y + 1);
        if above.is_air() || ctx.phase(above.material) == Phase::Gas {
            if ctx.chance(0.7) {
                ctx.swap(ctx.x, ctx.y + 1);
                mark_moved(ctx, ctx.x, ctx.y + 1);
                return true;
            }
        }
        let dx = if ctx.chance(0.5) { 1 } else { -1 };
        let n = ctx.get(ctx.x + dx, ctx.y + 1);
        if n.is_air() {
            ctx.swap(ctx.x + dx, ctx.y + 1);
            mark_moved(ctx, ctx.x + dx, ctx.y + 1);
            return true;
        }
    }

    false
}

fn can_fall_into(ctx: &RuleCtx<'_>, self_mat: crate::cell::MaterialId, into: crate::cell::Cell) -> bool {
    if into.flags.contains(CellFlags::MOVED) {
        return false;
    }
    if into.is_air() {
        return true;
    }
    let p = ctx.phase(into.material);
    if p == Phase::Gas {
        // denser materials displace gas
        return ctx.mats().density(self_mat) > ctx.mats().density(into.material);
    }
    false
}

fn mark_moved(ctx: &mut RuleCtx<'_>, x: i32, y: i32) {
    let mut c = ctx.world.get(x, y);
    c.flags.insert(CellFlags::MOVED);
    // avoid event spam: write through chunk
    let cc = crate::chunk::ChunkCoord::from_cell(x, y);
    if let Some(ch) = ctx.world.chunk_mut(cc) {
        let (ox, oy) = cc.origin_cell();
        let lx = (x - ox) as usize;
        let ly = (y - oy) as usize;
        if lx < crate::chunk::CHUNK_SIZE && ly < crate::chunk::CHUNK_SIZE {
            ch.cells[crate::chunk::Chunk::idx(lx, ly)].flags.insert(CellFlags::MOVED);
            ch.active = true;
        }
    }
}
