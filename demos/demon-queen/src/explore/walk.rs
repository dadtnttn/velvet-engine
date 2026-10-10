//! El paso de la Reina: la ayuda para rodear esquinas cuando choca de frente.

/// When a step is blocked straight on and along both axes (caught on a corner, or at the mouth
/// of a doorway a little to one side), the move that eases her round it: first a turn of the
/// step that is free and still goes forward (25° to 65° either way), else a sidestep toward the
/// nearest opening within half a metre to either side (where the way forward is free). `free`
/// says whether an offset from where she stands is clear. Returns the offset to take.
pub(crate) fn corner_assist(
    dx: f32,
    dz: f32,
    free: impl Fn(f32, f32) -> bool,
) -> Option<(f32, f32)> {
    let len = dx.hypot(dz);
    if len < 1e-6 {
        return None;
    }
    for degrees in [25.0f32, 45.0, 65.0] {
        let turn = degrees.to_radians();
        for sign in [1.0f32, -1.0] {
            let (s, c) = (turn * sign).sin_cos();
            let (ax, az) = ((dx * c - dz * s) * c, (dx * s + dz * c) * c);
            if free(ax, az) {
                return Some((ax, az));
            }
        }
    }
    let (fx, fz) = (dx / len, dz / len);
    let mut best: Option<(f32, f32, f32)> = None;
    for side in [1.0f32, -1.0] {
        let (px, pz) = (-fz * side, fx * side);
        for k in 1..=10 {
            let d = k as f32 * 0.05;
            if !free(px * d, pz * d) {
                break;
            }
            if free(px * d + dx, pz * d + dz) {
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, px, pz));
                }
                break;
            }
        }
    }
    best.map(|(d, px, pz)| {
        let step = d.min(len);
        (px * step, pz * step)
    })
}
