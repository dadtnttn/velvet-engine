//! The Queen's castle from the inside: a stronghold lit by torches and
//! candles rather than by the day outside, cool violet stone and a little
//! moonlight through the windows. castle3d builds the rooms and their flames;
//! this sets the light they are seen in.
use super::*;

/// The interior's light. Independent of the hour outside, except that a
/// window lets in warmer light by day and cold moonlight by night.
pub(super) fn mood(mut env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let night = env.celestial.moon_visibility.clamp(0.0, 1.0);
    let window = [
        0.62 - 0.22 * night,
        0.62 - 0.04 * night,
        0.78 + 0.16 * night,
    ];
    let lighting = &mut env.lighting;
    lighting.key_color = [window[0] * 0.62, window[1] * 0.62, window[2] * 0.62];
    lighting.ambient_color = [0.185, 0.135, 0.255];
    lighting.bounce_color = [0.24, 0.115, 0.085];
    let fog = &mut env.atmosphere;
    fog.fog_color = [0.040, 0.024, 0.060];
    fog.fog_start = 15.0;
    fog.fog_end = 54.0;
    fog.fog_density = 0.004;
    fog.height_falloff = 0.0;
    env.clear_color = [0.020, 0.013, 0.031, 1.0];
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interior_is_dim_cool_and_independent_of_the_hour() {
        let day = mood(EnvironmentPreset3D::Lived.sample(0.5));
        let night = mood(EnvironmentPreset3D::Lived.sample(0.0));
        for env in [day, night] {
            let key = env.lighting.key_color;
            assert!(
                key.iter().all(|c| *c < 0.75),
                "the window is not the sun: {key:?}"
            );
            assert!(
                env.lighting.ambient_color[2] > env.lighting.ambient_color[0],
                "violet shadows"
            );
            assert!(
                env.lighting.bounce_color[0] > env.lighting.bounce_color[2],
                "warm bounce off the floor"
            );
            assert!(env.clear_color[..3].iter().all(|c| *c < 0.05));
        }
        assert!(
            night.lighting.key_color[2] > night.lighting.key_color[0],
            "moonlight is cold"
        );
        assert!(
            day.lighting.key_color[0] > night.lighting.key_color[0],
            "a window is warmer by day"
        );
    }
}
