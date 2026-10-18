//! Water brought to life ([`WaterLife3D`]): what disturbs the bodies of water a game has placed,
//! frame by frame, by each one's look ([`WaterLook3D`]). Someone wading through the water, or
//! walking by its edge, rings it and kicks it up; drops fall now and then on a pond and down a
//! well; bubbles break in a hot spring; a spout pours into a brook and a wheel's paddles beat it.
//! Each makes rings ([`Wgpu3DRenderer::water_touch`]) and throws droplets
//! ([`Wgpu3DRenderer::water_emit`]); chance comes from a small seeded generator, so a run
//! replays the same.
use crate::{WaterBody3D, WaterBodyHandle, WaterLook3D, Wgpu3DRenderer};
use velvet_math::Vec3;

struct Placed {
    handle: WaterBodyHandle,
    look: WaterLook3D,
    group: u8,
}

/// What disturbs a game's water, frame by frame. Bodies are placed in groups (a place each: a
/// wood, a village); only the group in view is drawn, run and disturbed.
pub struct WaterLife3D {
    bodies: Vec<Placed>,
    /// Where a spout's stream meets the brooks, and a mill wheel's foot.
    spout: Option<Vec3>,
    mill: Option<Vec3>,
    last: Option<f32>,
    rng: u32,
    next_drop: f32,
    next_paddle: f32,
    last_touch: Option<Vec3>,
}

impl Default for WaterLife3D {
    fn default() -> Self {
        Self::new()
    }
}

impl WaterLife3D {
    /// No water yet.
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            spout: None,
            mill: None,
            last: None,
            rng: 0x9E37_79B9,
            next_drop: 0.0,
            next_paddle: 0.0,
            last_touch: None,
        }
    }

    /// Places `body` with the renderer, in `group`; its look says what disturbs it.
    pub fn add(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        body: WaterBody3D,
        group: u8,
    ) {
        let look = body.style.look;
        let handle = renderer.add_water_body(device, body);
        self.bodies.push(Placed {
            handle,
            look,
            group,
        });
    }

    /// Where a spout's stream meets the brooks.
    pub fn set_spout(&mut self, at: Vec3) {
        self.spout = Some(at);
    }

    /// Where a mill wheel's paddles beat the brooks.
    pub fn set_mill(&mut self, at: Vec3) {
        self.mill = Some(at);
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xFFFF) as f32 / 65535.0
    }

    /// A random point on the water of `handle`.
    fn random_wet(
        &mut self,
        renderer: &Wgpu3DRenderer,
        handle: WaterBodyHandle,
    ) -> Option<[f32; 2]> {
        let body = renderer.water_body(handle)?;
        let (origin, size, cell) = (body.origin, body.size, body.cell);
        for _ in 0..12 {
            let x = origin[0] + self.random() * size.0 as f32 * cell;
            let z = origin[1] + self.random() * size.1 as f32 * cell;
            if renderer
                .water_body(handle)
                .is_some_and(|b| b.contains(x, z))
            {
                return Some([x, z]);
            }
        }
        None
    }

    /// Droplets thrown up from (x, z) at the surface `level`.
    #[allow(clippy::too_many_arguments)]
    fn splash(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        x: f32,
        level: f32,
        z: f32,
        count: usize,
        up: f32,
        out: f32,
        along: Vec3,
    ) {
        for _ in 0..count {
            let a = self.random() * std::f32::consts::TAU;
            let r = out * (0.3 + 0.7 * self.random());
            let v = Vec3::new(a.cos() * r, up * (0.6 + 0.6 * self.random()), a.sin() * r) + along;
            renderer.water_emit(Vec3::new(x, level + 0.03, z), v, 1.4);
        }
    }

    /// One frame at time `elapsed` (seconds): shows the bodies of group `shown` (none: all
    /// hidden), runs that water, and disturbs it round `walker` and by each body's look.
    pub fn frame(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        elapsed: f32,
        walker: Vec3,
        shown: Option<u8>,
    ) {
        let dt = self
            .last
            .map(|l| (elapsed - l).clamp(0.0, 0.1))
            .unwrap_or(0.0);
        self.last = Some(elapsed);
        for b in &self.bodies {
            renderer.set_water_visible(b.handle, Some(b.group) == shown);
        }
        if shown.is_none() {
            return;
        }
        renderer.advance_water(dt);
        let in_view: Vec<(WaterBodyHandle, WaterLook3D)> = self
            .bodies
            .iter()
            .filter(|b| Some(b.group) == shown)
            .map(|b| (b.handle, b.look))
            .collect();
        // Steps along the shore.
        let moved = self.last_touch.is_none_or(|t| (t - walker).length() > 0.3);
        if moved {
            for &(handle, _) in &in_view {
                let Some(body) = renderer.water_body(handle) else {
                    continue;
                };
                let level = body.level;
                if body.contains(walker.x, walker.z) {
                    // Wading: a ring round the legs and water kicked up at each step.
                    renderer.water_touch(walker.x, walker.z, 0.28, 0.016);
                    self.splash(renderer, walker.x, level, walker.z, 6, 1.4, 0.7, Vec3::ZERO);
                    self.last_touch = Some(walker);
                } else if let Some(p) = body.nearest_water(walker.x, walker.z, 0.8) {
                    renderer.water_touch(p[0], p[1], 0.16, 0.008);
                    if self.random() < 0.3 {
                        self.splash(renderer, p[0], level, p[1], 2, 1.0, 0.4, Vec3::ZERO);
                    }
                    self.last_touch = Some(walker);
                }
            }
        }
        // Drops, bubbles, the spout and the mill.
        self.next_drop -= dt;
        let drop = self.next_drop <= 0.0;
        if drop {
            self.next_drop = 0.5 + self.random() * 1.3;
        }
        self.next_paddle -= dt;
        let paddle = self.next_paddle <= 0.0;
        if paddle {
            self.next_paddle = 0.32;
        }
        for (handle, look) in in_view {
            let Some(level) = renderer.water_body(handle).map(|b| b.level) else {
                continue;
            };
            match look {
                WaterLook3D::Pond if drop && self.random() < 0.5 => {
                    if let Some(p) = self.random_wet(renderer, handle) {
                        renderer.water_touch(p[0], p[1], 0.1, 0.006);
                        self.splash(renderer, p[0], level, p[1], 2, 0.9, 0.2, Vec3::ZERO);
                    }
                }
                WaterLook3D::Well if drop && self.random() < 0.35 => {
                    if let Some(p) = self.random_wet(renderer, handle) {
                        renderer.water_touch(p[0], p[1], 0.04, 0.01);
                    }
                }
                WaterLook3D::Spring => {
                    // Bubbles rising: the surface bulges where they break.
                    if self.random() < dt * 5.0 {
                        if let Some(p) = self.random_wet(renderer, handle) {
                            renderer.water_touch(p[0], p[1], 0.05, -0.006);
                        }
                    }
                }
                WaterLook3D::Brook => {
                    if let Some(s) = self.spout {
                        renderer.water_touch(s.x, s.z, 0.12, 0.0025);
                        let n = (dt * 45.0 + self.random()) as usize;
                        self.splash(renderer, s.x, level, s.z, n, 1.3, 0.7, Vec3::ZERO);
                    }
                    if let (Some(m), true) = (self.mill, paddle) {
                        renderer.water_touch(m.x, m.z, 0.18, 0.012);
                        self.splash(renderer, m.x, level, m.z, 5, 1.4, 0.5, Vec3::ZERO);
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_chance_replays_the_same_and_stays_in_range() {
        let (mut a, mut b) = (WaterLife3D::new(), WaterLife3D::new());
        let draws: Vec<f32> = (0..1000).map(|_| a.random()).collect();
        assert!(draws.iter().all(|r| (0.0..=1.0).contains(r)));
        assert_eq!(draws, (0..1000).map(|_| b.random()).collect::<Vec<_>>());
        let mean = draws.iter().sum::<f32>() / draws.len() as f32;
        assert!((mean - 0.5).abs() < 0.05, "{mean}");
    }
}
