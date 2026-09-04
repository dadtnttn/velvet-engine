//! Deterministic Fx sim — tick + raster to framebuffer.
//! Pixel-perfect: particles are 1–2 px squares, drawn at integer canvas coords.

use velvet_math::Pcg32;

use crate::emitter::EmitterShape;
use crate::palette::PaletteId;
use crate::path::PathKind;
use crate::preset::FxPreset;

/// One rasterized frame (RGBA8).
#[derive(Debug, Clone)]
pub struct SimFrame {
    /// Width px.
    pub width: u32,
    /// Height px.
    pub height: u32,
    /// RGBA pixels row-major.
    pub pixels: Vec<u8>,
}

impl SimFrame {
    /// New with background filled.
    pub fn new(width: u32, height: u32, bg: [u8; 4]) -> Self {
        let n = (width as usize) * (height as usize);
        let mut pixels = vec![0u8; n * 4];
        for chunk in pixels.chunks_mut(4) {
            chunk.copy_from_slice(&bg);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    #[inline]
    fn put(&mut self, x: i32, y: i32, col: [u8; 4]) {
        if col[3] == 0 {
            return;
        }
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        if col[3] == 255 {
            self.pixels[i..i + 4].copy_from_slice(&col);
        } else {
            // alpha blend over existing
            let sa = col[3] as f32 / 255.0;
            let da = self.pixels[i + 3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a > 1e-4 {
                for (c, &s) in col.iter().take(3).enumerate() {
                    let s = s as f32;
                    let d = self.pixels[i + c] as f32;
                    self.pixels[i + c] = ((s * sa + d * da * (1.0 - sa)) / out_a).round() as u8;
                }
                self.pixels[i + 3] = (out_a * 255.0).round() as u8;
            }
        }
    }

    fn additive_put(&mut self, x: i32, y: i32, col: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if col[3] == 0 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        let a = col[3] as f32 / 255.0;
        self.pixels[i] = (self.pixels[i] as f32 + col[0] as f32 * a * 0.55).min(255.0) as u8;
        self.pixels[i + 1] =
            (self.pixels[i + 1] as f32 + col[1] as f32 * a * 0.55).min(255.0) as u8;
        self.pixels[i + 2] =
            (self.pixels[i + 2] as f32 + col[2] as f32 * a * 0.55).min(255.0) as u8;
        self.pixels[i + 3] = 255;
    }
}

/// Runtime FX sim — deterministic from preset + seed.
#[derive(Debug, Clone)]
pub struct FxSim {
    /// Preset (cloned).
    pub preset: FxPreset,
    /// Time accumulator for continuous emission bookkeeping (not needed for stateless render).
    pub time: f32,
    /// Internal rng state for spawning (derived from seed).
    rng: u64,
}

impl FxSim {
    /// New sim from preset.
    pub fn new(preset: FxPreset) -> Self {
        let preset = preset.validated();
        let seed = preset.seed | 1;
        Self {
            preset,
            time: 0.0,
            rng: seed,
        }
    }

    /// Reset time.
    pub fn reset(&mut self) {
        self.time = 0.0;
        self.rng = self.preset.seed | 1;
    }

    /// Advance time (for interactive preview, not needed for frame rendering which is stateless by t).
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
    }

    /// Render a single frame at normalized time `phase` 0..=1 (loop position) or absolute seconds.
    /// For determinism, this renders stateless: spawns particles needed for that moment
    /// by simulating the preceding `max_lifetime` window at 60 Hz substeps.
    pub fn render_frame_at(&self, time: f32) -> SimFrame {
        let preset = &self.preset;
        let dims = preset.dimensions.clamped();
        let bg = preset.background;
        let mut frame = SimFrame::new(dims.width, dims.height, bg);

        let palette = preset.effective_palette();
        let palette = if palette.is_empty() {
            crate::palette::palette_colors(PaletteId::Fire).to_vec()
        } else {
            palette
        };
        let duration = preset.duration().max(0.2);
        // normalize time to loop
        let t = if preset.loop_mode == crate::preset::LoopMode::PingPong {
            let phase = (time / duration) % 2.0;
            (if phase > 1.0 { 2.0 - phase } else { phase }) * duration
        } else {
            time.rem_euclid(duration)
        };

        let cx = dims.width as f32 * 0.5;
        let cy = dims.height as f32 * 0.5;
        let scale = dims.width.min(dims.height) as f32 * 0.45;

        let pc = &preset.particles;
        let max_age = pc.lifetime_max.max(0.2);

        // Spawn window: particles spawned in [t - max_age, t]
        let dt = 1.0 / 60.0;
        let _steps_back = (max_age / dt).ceil() as i32 + 1;
        let spawn_dt = 1.0 / preset.emitter.rate.max(1.0);

        // Use deterministic RNG seeded by preset seed + frame index behaviour;
        // we seed per simulation with Pcg32
        let mut rng = Pcg32::new(
            preset.seed ^ 0x9E37_79B9_7F4A_7C15,
            0x9E37_79B9_7F4A_7C15 | 1,
        );
        // For each potential spawn time, decide if we spawn and generate particle
        // We step backwards and emit at spawn_dt interval with jitter from rng
        // Simpler: iterate spawn times with deterministic offsets
        let mut spawn_times: Vec<f32> = Vec::new();
        let start = t - max_age;
        let mut s = start;
        // precompute spawn times with slight jitter using rng
        let mut spawn_rng = Pcg32::new(preset.seed ^ 0x1234, 0xABCD);
        while s <= t {
            // jitter spawn interval ±15%
            let jitter = (spawn_rng.next_f32() - 0.5) * spawn_dt * 0.3;
            let at = s + jitter;
            if at >= start && at <= t {
                spawn_times.push(at);
            }
            s += spawn_dt;
            // avoid infinite; cap 2048 spawns
            if spawn_times.len() > 2048 {
                break;
            }
        }
        // For burst mode (non-continuous), only keep spawns near loop start
        if !preset.emitter.continuous {
            let burst_phase = (t / duration) % 1.0;
            // emit burst at phase 0..0.1
            if burst_phase > 0.18 {
                // outside burst window, only keep long-lived from burst
                spawn_times.retain(|&at| {
                    let at_phase = (at.rem_euclid(duration) / duration) % 1.0;
                    at_phase < 0.18
                });
            }
        }

        // For each spawn, simulate to t
        let gravity = pc.gravity;
        let drag = pc.drag.clamp(0.0, 0.97);
        let jitter = pc.jitter;
        for &spawn_at in &spawn_times {
            let age = t - spawn_at;
            if age < 0.0 {
                continue;
            }
            let life = rng_range(&mut rng, pc.lifetime_min, pc.lifetime_max);
            if age > life {
                continue;
            }
            // skip if beyond max_particles cap: sample with deterministic thinning
            // we just render all and rely on max limit for perf; thin if too many
            // Use spawn index hashed
            let norm = age / life; // 0 birth → 1 death

            // path sample: t_path derived from spawn_at (so particles from path move over time)
            // Use spawn path t = (spawn_at / duration + rng offset) % 1
            let mut prng = Pcg32::new(spawn_at.to_bits() as u64 ^ preset.seed, 0x55);
            let path_t = prng.next_f32();
            let path_time = spawn_at * 0.7;
            let ps = preset.path.sample(path_t, path_time);
            let px0 = cx + ps.x * scale;
            let py0 = cy + ps.y * scale;

            // initial velocity: emitter shape + speed
            let speed = rng_range(&mut rng, pc.speed_min, pc.speed_max);
            let (vx0, vy0) = emitter_velocity(&preset.emitter, &ps, speed, &mut prng);

            // integrate from spawn to t with simple euler + drag + gravity + jitter
            let steps = (age / dt).ceil() as usize;
            let mut x = px0;
            let mut y = py0;
            let mut vx = vx0;
            let mut vy = vy0;
            for step in 0..steps {
                let _frac = step as f32 / steps.max(1) as f32;
                // apply gravity
                vy += gravity * dt;
                // drag
                let d = (1.0 - drag * dt * 6.0).clamp(0.0, 1.0);
                vx *= d;
                vy *= d;
                // jitter (per-frame wobble)
                let jx = ((prng.next_f32() - 0.5) * 2.0) * jitter * dt * 18.0;
                let jy = ((prng.next_f32() - 0.5) * 2.0) * jitter * dt * 18.0;
                vx += jx;
                vy += jy;
                x += vx * dt;
                y += vy * dt;
            }

            // palette index: age → 0..1
            let pal_t = norm.powf(pc.fade_pow.max(0.2));
            let pal_idx =
                (pal_t * (palette.len() as f32 - 1e-4)).clamp(0.0, palette.len() as f32 - 1.0);
            let lo = pal_idx.floor() as usize;
            let hi = (lo + 1).min(palette.len() - 1);
            let f = pal_idx - lo as f32;
            let c0 = palette[lo];
            let c1 = palette[hi];
            let r = lerp_u8(c0[0], c1[0], f);
            let g = lerp_u8(c0[1], c1[1], f);
            let b = lerp_u8(c0[2], c1[2], f);
            let mut a = ((1.0 - norm).powf(1.0_f32) * 255.0).round() as i32;
            // flicker
            if pc.flicker > 0.01 {
                let flick = (prng.next_f32() - 0.5) * pc.flicker * 2.0;
                a = ((a as f32 * (1.0 + flick)).round() as i32).clamp(0, 255);
            }
            if a <= 8 {
                continue;
            }
            let col = [r, g, b, a as u8];

            // size and shrink
            let mut sz = pc.size as i32;
            if pc.shrink > 0.0 {
                let s = 1.0 - norm * pc.shrink;
                if s < 0.6 {
                    sz = 1;
                }
            }
            // if particle outside 1px border, skip
            let ix = x.round() as i32;
            let iy = y.round() as i32;
            // Special FX tweaks per path kind
            let draw_x = ix;
            let draw_y = iy;
            // For Dna, add connecting bridge occasionally
            let _ = ps;
            // Draw
            if self.preset.additive {
                for oy in 0..sz {
                    for ox in 0..sz {
                        frame.additive_put(draw_x + ox, draw_y + oy, col);
                    }
                }
                // small glow for high speed
                if speed > 40.0 {
                    frame.additive_put(draw_x + 1, draw_y, [r, g, b, (a / 3) as u8]);
                }
            } else {
                for oy in 0..sz {
                    for ox in 0..sz {
                        frame.put(draw_x + ox, draw_y + oy, col);
                    }
                }
            }
            // second pixel for size 1: sometimes add 2px for bright core near birth
            if pc.size == 1 && norm < 0.15 && rng_range(&mut rng, 0.0, 1.0) > 0.6 {
                if self.preset.additive {
                    frame.additive_put(draw_x + 1, draw_y, [255, 255, 255, 80]);
                } else {
                    frame.put(draw_x + 1, draw_y, [255, 255, 255, 120]);
                }
            }
            // dna connector: faint line between two helices every Nth
            if preset.path.kind == PathKind::Dna && (spawn_at.to_bits() % 7 == 0) {
                // draw tiny bridge pixel
                let bx = (cx + ps.x * scale * 0.5).round() as i32;
                let by = iy;
                frame.put(bx, by, [r, g, b, (a / 4).clamp(0, 255) as u8]);
            }
        }

        frame
    }

    /// Convenience: render frame index `i` of `preset.frames` looped.
    pub fn render_index(&self, index: u32) -> SimFrame {
        let t = index as f32 / self.preset.fps as f32;
        self.render_frame_at(t)
    }

    /// Render all frames in order.
    pub fn render_all_frames(&self) -> Vec<SimFrame> {
        (0..self.preset.frames)
            .map(|i| self.render_index(i))
            .collect()
    }
}

fn emitter_velocity(
    ec: &crate::emitter::EmitterConfig,
    path_sample: &crate::path::PathSample,
    speed: f32,
    rng: &mut Pcg32,
) -> (f32, f32) {
    let base = match ec.shape {
        EmitterShape::Point => ec.angle,
        EmitterShape::Cone => {
            let spread = ec.spread;
            ec.angle + (rng.next_f32() - 0.5) * 2.0 * spread
        }
        EmitterShape::Circle => rng.next_f32() * std::f32::consts::TAU,
        EmitterShape::Ring => {
            // outward from center — angle from path normal
            let nx = path_sample.nx;
            let ny = path_sample.ny;
            ny.atan2(nx) + (rng.next_f32() - 0.5) * ec.spread * 0.5
        }
        EmitterShape::Directed => ec.angle + (rng.next_f32() - 0.5) * ec.spread * 0.4,
    };
    let angle = if ec.align_to_path && ec.shape != EmitterShape::Circle {
        // blend path normal with base (35% normal)
        let pn = path_sample.ny.atan2(path_sample.nx);
        base * 0.65 + pn * 0.35
    } else {
        base
    };
    (angle.cos() * speed, angle.sin() * speed)
}

fn rng_range(rng: &mut Pcg32, a: f32, b: f32) -> f32 {
    a + (b - a) * rng.next_f32()
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::FxPreset;

    #[test]
    fn render_deterministic() {
        let p = FxPreset::fire();
        let sim = FxSim::new(p);
        let a = sim.render_frame_at(0.2);
        let b = sim.render_frame_at(0.2);
        assert_eq!(a.pixels, b.pixels);
    }

    #[test]
    fn all_presets_render() {
        for preset in FxPreset::examples() {
            let sim = FxSim::new(preset.clone());
            let f = sim.render_frame_at(0.0);
            assert_eq!(f.width, preset.dimensions.width);
            assert!(f.pixels.iter().any(|&v| v != 0) || preset.background[3] == 0);
        }
    }
}
