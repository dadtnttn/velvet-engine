//! Simple pressure diffusion for liquids / gases (not full CFD).

use crate::rules::RuleCtx;

/// Diffuse pressure scalar and gently push liquids from high → low.
pub fn rule_pressure_diffuse(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() {
        return false;
    }
    let mut c = cell;
    let mut acc = c.pressure;
    let mut n = 1.0f32;
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let nb = ctx.get(ctx.x + dx, ctx.y + dy);
        acc += nb.pressure;
        n += 1.0;
    }
    c.pressure = acc / n;
    // hydrostatic bias for liquids
    if matches!(
        ctx.phase(c.material),
        crate::material::Phase::Liquid | crate::material::Phase::Powder
    ) {
        c.pressure += ctx.world.config.gravity * 0.01 * ctx.mats().density(c.material);
    }

    // flow toward lower pressure horizontally
    if matches!(
        ctx.phase(c.material),
        crate::material::Phase::Liquid | crate::material::Phase::Gas
    ) {
        let left = ctx.get(ctx.x - 1, ctx.y);
        let right = ctx.get(ctx.x + 1, ctx.y);
        if left.is_air() && c.pressure > 0.1 && ctx.chance(0.2) {
            ctx.swap(ctx.x - 1, ctx.y);
            return true;
        }
        if right.is_air() && c.pressure > 0.1 && ctx.chance(0.2) {
            ctx.swap(ctx.x + 1, ctx.y);
            return true;
        }
    }

    let cc = crate::chunk::ChunkCoord::from_cell(ctx.x, ctx.y);
    let (ox, oy) = cc.origin_cell();
    if let Some(ch) = ctx.world.chunk_mut(cc) {
        let lx = (ctx.x - ox) as usize;
        let ly = (ctx.y - oy) as usize;
        ch.cells[crate::chunk::Chunk::idx(lx, ly)].pressure = c.pressure;
    }
    false
}
