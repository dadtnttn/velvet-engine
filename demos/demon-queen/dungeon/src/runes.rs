//! Ritual state for the procedural spiral carved into each runestone.
//! Geometry lives in visual.rs; only that rune rotates, never the monolith.
use super::visual;
use velvet_math::Vec3;
use velvet_render::PointLight3D;

pub fn stone_name(index: usize) -> &'static str {
    ["I", "II", "III"][index.min(2)]
}

pub fn phase_name(phase: u8) -> &'static str {
    ["APAGADA", "ALBA", "CÉNIT", "OCASO"][phase.min(3) as usize]
}

pub fn phase_tint(phase: u8) -> [f32; 3] {
    match phase {
        1 => [1.37, 0.93, 0.78],
        2 => [1.43, 1.32, 0.91],
        3 => [1.18, 0.85, 1.29],
        _ => [1.0; 3],
    }
}

pub fn add_light(
    lights: &mut Vec<PointLight3D>,
    position: Vec3,
    yaw: f32,
    phase: u8,
    elapsed: f32,
    changed_at: f32,
) {
    let burst = (1.0 - (elapsed - changed_at).max(0.0) / 0.85).clamp(0.0, 1.0);
    if phase == 0 && burst == 0.0 {
        return;
    }
    let color = match phase {
        1 => [1.65, 0.68, 0.24],
        2 => [1.65, 1.34, 0.47],
        3 => [1.33, 0.44, 1.12],
        _ => [0.30, 0.47, 0.45],
    };
    let normal = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    lights.push(PointLight3D::new(
        position + Vec3::Y * 0.9 + normal * 0.25,
        visual::tint(color, 0.72),
        1.35 + burst * 1.35,
        3.8,
    ));
}

#[cfg(test)]
mod tests {
    use super::super::Dungeon;

    #[test]
    fn original_spiral_rotates_quarter_turn_and_eases() {
        let mut run = Dungeon::generate(42);
        run.elapsed = 1.0;
        run.rune_changed_at[0] = 1.0;
        run.rune_target_angle[0] = std::f32::consts::FRAC_PI_2;
        assert_eq!(run.rune_angle(0), 0.0);
        run.elapsed = 1.41;
        assert!((run.rune_angle(0) - std::f32::consts::FRAC_PI_4).abs() < 0.0001);
        run.elapsed = 1.82;
        assert!((run.rune_angle(0) - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    }
}
