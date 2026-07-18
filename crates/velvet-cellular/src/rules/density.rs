//! Density sorting: heavier powders/liquids sink through lighter ones.

use crate::cell::CellFlags;
use crate::material::Phase;
use crate::rules::RuleCtx;

/// Sink through less dense neighbors below.
pub fn rule_density_sink(ctx: &mut RuleCtx<'_>) -> bool {
    let cell = ctx.cell();
    if cell.is_air() || cell.flags.contains(CellFlags::MOVED) {
        return false;
    }
    let phase = ctx.phase(cell.material);
    if !matches!(phase, Phase::Powder | Phase::Liquid) {
        return false;
    }
    let below = ctx.get(ctx.x, ctx.y - 1);
    if below.is_air() || below.flags.contains(CellFlags::MOVED) {
        return false;
    }
    let bp = ctx.phase(below.material);
    if !matches!(bp, Phase::Powder | Phase::Liquid | Phase::Gas) {
        return false;
    }
    let d0 = ctx.mats().density(cell.material);
    let d1 = ctx.mats().density(below.material);
    if d0 > d1 + 0.05 {
        ctx.swap(ctx.x, ctx.y - 1);
        return true;
    }
    // slight diagonal density sort
    if d0 > d1 {
        let dx = if ctx.chance(0.5) { 1 } else { -1 };
        let n = ctx.get(ctx.x + dx, ctx.y - 1);
        if !n.is_air()
            && !n.flags.contains(CellFlags::MOVED)
            && matches!(ctx.phase(n.material), Phase::Powder | Phase::Liquid)
            && d0 > ctx.mats().density(n.material) + 0.05
        {
            ctx.swap(ctx.x + dx, ctx.y - 1);
            return true;
        }
    }
    false
}
