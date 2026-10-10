//! Shallow-water heightfield solver (virtual-pipes model).
//!
//! Water depth rides over a bedrock heightfield; each step, flux through the
//! four cell faces moves volume downhill, then depth integrates. Rain adds,
//! evaporation removes. Deterministic and volume-conserving up to boundary
//! outflow (documented, bounded in tests).

use serde::{Deserialize, Serialize};

/// Solver tuning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShallowParams {
    /// Grid resolution (cells per side).
    pub resolution: u32,
    /// World extent (XZ).
    pub size: f32,
    /// Seconds per step.
    pub dt: f32,
    /// Pipe cross-section factor.
    pub pipe_area: f32,
    /// Gravity accel.
    pub gravity: f32,
    /// Max flux damping (stability).
    pub damping: f32,
    /// Open rim: boundary faces drain out of the domain (rivers to sea).
    /// Off by default: closed walls conserve volume exactly.
    #[serde(default)]
    pub open_rim: bool,
}

impl Default for ShallowParams {
    fn default() -> Self {
        Self {
            resolution: 64,
            size: 64.0,
            dt: 0.033,
            pipe_area: 1.0,
            gravity: 9.81,
            damping: 0.999,
            open_rim: false,
        }
    }
}

/// Shallow water over bedrock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShallowWater {
    params: ShallowParams,
    /// Bedrock heights (world units), row-major.
    pub bedrock: Vec<f32>,
    /// Water depth per cell (world units, ≥ 0).
    pub depth: Vec<f32>,
    /// Rightward face fluxes `(n+1) × n` (shared faces: exact conservation).
    fx: Vec<f32>,
    /// Downward face fluxes `n × (n+1)` (shared faces: exact conservation).
    fz: Vec<f32>,
}

impl ShallowWater {
    /// New solver over a bedrock grid (must match `resolution²`).
    pub fn new(params: ShallowParams, bedrock: Vec<f32>) -> Result<Self, String> {
        let n = params.resolution.max(2) as usize;
        if bedrock.len() != n * n {
            return Err(format!(
                "bedrock has {} cells, need {}",
                bedrock.len(),
                n * n
            ));
        }
        Ok(Self {
            params,
            bedrock,
            depth: vec![0.0; n * n],
            fx: vec![0.0; (n + 1) * n],
            fz: vec![0.0; n * (n + 1)],
        })
    }

    /// Resolution.
    pub fn resolution(&self) -> usize {
        self.params.resolution.max(2) as usize
    }

    /// Fill depressions up to `level` (lake init): depth = max(0, level - bedrock).
    pub fn fill_to_level(&mut self, level: f32) {
        for (d, b) in self.depth.iter_mut().zip(self.bedrock.iter()) {
            *d = (level - b).max(0.0);
        }
    }

    /// Add rain depth uniformly.
    pub fn rain(&mut self, amount: f32) {
        for d in &mut self.depth {
            *d += amount;
        }
    }

    /// Evaporate a fixed depth from every cell (a surface process: deep
    /// pools barely notice, thin films vanish). With [`Self::rain`] this
    /// closes the water cycle: rain feeds streams and pools while
    /// evaporation and open rims drain them, so the map settles at a
    /// living equilibrium instead of filling forever.
    pub fn evaporate(&mut self, amount: f32) {
        if amount <= 0.0 {
            return;
        }
        for d in &mut self.depth {
            *d = (*d - amount).max(0.0);
        }
    }

    /// Extra evaporation over open water (cells deeper than `deep`).
    /// Lakes evaporate at potential rate while damp soil lags behind;
    /// without this a rain-fed closed basin rises forever, since its
    /// catchment outweighs its surface.
    pub fn evaporate_open_water(&mut self, amount: f32, deep: f32) {
        if amount <= 0.0 {
            return;
        }
        for d in &mut self.depth {
            if *d > deep {
                *d = (*d - amount).max(0.0);
            }
        }
    }

    /// Add a puddle (gaussian splat of depth at world `(x, z)`).
    pub fn splash(&mut self, x: f32, z: f32, radius: f32, amount: f32) {
        let n = self.resolution();
        for j in 0..n {
            for i in 0..n {
                let wx = (i as f32 / (n - 1) as f32 - 0.5) * self.params.size;
                let wz = (j as f32 / (n - 1) as f32 - 0.5) * self.params.size;
                let d2 = (wx - x) * (wx - x) + (wz - z) * (wz - z);
                self.depth[j * n + i] += amount * (-d2 / (radius * radius).max(1e-6)).exp();
            }
        }
    }

    /// Total water volume (depth summed over cells, × cell area).
    pub fn volume(&self) -> f64 {
        let n = self.resolution() as f64;
        let area = (self.params.size as f64 / (n - 1.0)).powi(2);
        self.depth.iter().map(|d| (*d as f64).max(0.0)).sum::<f64>() * area
    }

    /// One step: update shared face fluxes, then depths.
    ///
    /// Closed walls (default): each interior face is one variable, so what
    /// leaves a cell enters its neighbor bit-identically — volume is
    /// conserved exactly (rain/splash are the only sources). With
    /// `open_rim`, rim faces drain against a dry ghost (rivers to sea);
    /// conservation no longer holds by design.
    pub fn step(&mut self) {
        let n = self.resolution();
        let cell = self.params.size / (n - 1) as f32;
        let dt = self.params.dt;
        let area = self.params.pipe_area;
        let gravity = self.params.gravity;
        let damping = self.params.damping;
        let open = self.params.open_rim;
        // Vertical faces (rightward positive).
        for j in 0..n {
            for i in 0..=n {
                // Ghost dry bed beyond open rims; the limiter then uses the
                // real cell alone (the ghost is a bottomless sink).
                let (bl, dl, br, dr, cap) = if i == 0 {
                    if !open {
                        continue;
                    }
                    let r = j * n;
                    (
                        self.bedrock[r],
                        0.0,
                        self.bedrock[r],
                        self.depth[r],
                        self.depth[r],
                    )
                } else if i == n {
                    if !open {
                        continue;
                    }
                    let l = j * n + n - 1;
                    (
                        self.bedrock[l],
                        self.depth[l],
                        self.bedrock[l],
                        0.0,
                        self.depth[l],
                    )
                } else {
                    let l = j * n + i - 1;
                    let r = j * n + i;
                    (
                        self.bedrock[l],
                        self.depth[l],
                        self.bedrock[r],
                        self.depth[r],
                        self.depth[l].min(self.depth[r]),
                    )
                };
                let dh = (bl + dl) - (br + dr);
                let mut f = self.fx[j * (n + 1) + i];
                f = (f + dt * area * gravity * dh / cell) * damping;
                // Limiter: no side gives more than a quarter of the capped
                // volume, so cells can never overdraft.
                let limit = 0.25 * cap * cell * cell / dt.max(1e-6);
                self.fx[j * (n + 1) + i] = f.clamp(-limit, limit);
            }
        }
        // Horizontal faces (downward positive).
        for j in 0..=n {
            for i in 0..n {
                let (bu, du, bd, dd, cap) = if j == 0 {
                    if !open {
                        continue;
                    }
                    let d = i;
                    (
                        self.bedrock[d],
                        0.0,
                        self.bedrock[d],
                        self.depth[d],
                        self.depth[d],
                    )
                } else if j == n {
                    if !open {
                        continue;
                    }
                    let u = (n - 1) * n + i;
                    (
                        self.bedrock[u],
                        self.depth[u],
                        self.bedrock[u],
                        0.0,
                        self.depth[u],
                    )
                } else {
                    let u = (j - 1) * n + i;
                    let d = j * n + i;
                    (
                        self.bedrock[u],
                        self.depth[u],
                        self.bedrock[d],
                        self.depth[d],
                        self.depth[u].min(self.depth[d]),
                    )
                };
                let dh = (bu + du) - (bd + dd);
                let mut f = self.fz[j * n + i];
                f = (f + dt * area * gravity * dh / cell) * damping;
                let limit = 0.25 * cap * cell * cell / dt.max(1e-6);
                self.fz[j * n + i] = f.clamp(-limit, limit);
            }
        }
        // Depth integration from shared faces.
        for j in 0..n {
            for i in 0..n {
                let net = self.fx[j * (n + 1) + i] - self.fx[j * (n + 1) + i + 1]
                    + self.fz[j * n + i]
                    - self.fz[(j + 1) * n + i];
                self.depth[j * n + i] = (self.depth[j * n + i] + net * dt / (cell * cell)).max(0.0);
            }
        }
    }

    /// Bilinear depth at world `(x, z)` (clamped to the rim).
    pub fn sample_depth_world(&self, x: f32, z: f32) -> f32 {
        let n = self.resolution();
        let half = self.params.size * 0.5;
        bilinear_grid(
            &self.depth,
            n,
            world_to_grid(x, half, self.params.size, n),
            world_to_grid(z, half, self.params.size, n),
        )
    }

    /// Bilinear surface height at world `(x, z)` (clamped to the rim).
    pub fn sample_surface_world(&self, x: f32, z: f32) -> f32 {
        let n = self.resolution();
        let half = self.params.size * 0.5;
        bilinear_grid(
            &self.surface(),
            n,
            world_to_grid(x, half, self.params.size, n),
            world_to_grid(z, half, self.params.size, n),
        )
    }

    /// Water surface heights (bedrock + depth) for meshing/export.
    pub fn surface(&self) -> Vec<f32> {
        self.bedrock
            .iter()
            .zip(self.depth.iter())
            .map(|(b, d)| b + d.max(0.0))
            .collect()
    }

    /// Discharge vector at a cell: average of face fluxes over depth.
    ///
    /// Units are solver-internal (direction + relative magnitude for
    /// steering); documented as such so nobody mistakes it for m/s.
    /// Returns `(0, 0)` on dry cells.
    pub fn velocity_at(&self, i: usize, j: usize) -> (f32, f32) {
        let n = self.resolution();
        if i >= n || j >= n {
            return (0.0, 0.0);
        }
        let depth = self.depth[j * n + i].max(1e-6);
        let cell = self.params.size / (n - 1) as f32;
        let vx = 0.5 * (self.fx[j * (n + 1) + i] + self.fx[j * (n + 1) + i + 1]) / (depth * cell);
        let vz = 0.5 * (self.fz[j * n + i] + self.fz[(j + 1) * n + i]) / (depth * cell);
        (vx, vz)
    }

    /// Bilinear velocity at world `(x, z)` (for erosion steering, foam…).
    pub fn velocity_world(&self, x: f32, z: f32) -> (f32, f32) {
        let n = self.resolution();
        let half = self.params.size * 0.5;
        let fx = ((x + half) / self.size() * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let fz = ((z + half) / self.size() * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(n - 1), (j0 + 1).min(n - 1));
        let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
        let (ax, az) = self.velocity_at(i0, j0);
        let (bx, bz) = self.velocity_at(i1, j0);
        let (cx, cz) = self.velocity_at(i0, j1);
        let (dx, dz) = self.velocity_at(i1, j1);
        (
            ax * (1.0 - tx) * (1.0 - tz)
                + bx * tx * (1.0 - tz)
                + cx * (1.0 - tx) * tz
                + dx * tx * tz,
            az * (1.0 - tx) * (1.0 - tz)
                + bz * tx * (1.0 - tz)
                + cz * (1.0 - tx) * tz
                + dz * tx * tz,
        )
    }

    /// Grid size in world units (for world↔cell mapping).
    fn size(&self) -> f32 {
        self.params.size
    }
}

/// Marching-squares triangles of the region where `values >= iso`.
///
/// Returns `(x, z)` triangles in grid units. Boundary vertices sit exactly
/// on the interpolated isoline (no stair-steps); winding matches the
/// engine grid convention (negative signed area in XZ, same as the terrain
/// mesher). Saddles resolve by center average. Pure topology for presentation
/// meshes: shading happens at the interpolated vertices downstream.
pub fn contour_triangles(values: &[f32], n: usize, iso: f32) -> Vec<[[f32; 2]; 3]> {
    if n < 2 || values.len() != n * n {
        return Vec::new();
    }
    let at = |i: usize, j: usize| values[j * n + i];
    // Edge crossing between grid nodes a -> b (values differ across iso).
    let cross = |ax: usize, az: usize, bx: usize, bz: usize| {
        let da = at(ax, az) - iso;
        let db = at(bx, bz) - iso;
        let t = if (db - da).abs() < 1e-9 {
            0.5
        } else {
            ((-da) / (db - da)).clamp(0.0, 1.0)
        };
        [
            ax as f32 + (bx as f32 - ax as f32) * t,
            az as f32 + (bz as f32 - az as f32) * t,
        ]
    };
    let mut tris: Vec<[[f32; 2]; 3]> = Vec::new();
    // Emit with engine winding (negative XZ signed area), enforced below.
    let mut emit = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| {
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if area <= 0.0 {
            tris.push([a, b, c]);
        } else {
            tris.push([a, c, b]);
        }
    };
    let mut fan = |poly: &[[f32; 2]]| {
        for k in 1..poly.len() - 1 {
            emit(poly[0], poly[k], poly[k + 1]);
        }
    };
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let v00 = at(i, j) >= iso;
            let v10 = at(i + 1, j) >= iso;
            let v11 = at(i + 1, j + 1) >= iso;
            let v01 = at(i, j + 1) >= iso;
            let c0 = [i as f32, j as f32];
            let c1 = [i as f32 + 1.0, j as f32];
            let c2 = [i as f32 + 1.0, j as f32 + 1.0];
            let c3 = [i as f32, j as f32 + 1.0];
            // Edge crossings (only computed when the edge straddles).
            let xb = cross(i, j, i + 1, j);
            let xr = cross(i + 1, j, i + 1, j + 1);
            let xt = cross(i + 1, j + 1, i, j + 1);
            let xl = cross(i, j + 1, i, j);
            match (v00, v10, v11, v01) {
                (false, false, false, false) => {}
                (true, true, true, true) => {
                    fan(&[c0, c1, c2]);
                    fan(&[c0, c2, c3]);
                }
                (true, false, false, false) => fan(&[c0, xb, xl]),
                (false, true, false, false) => fan(&[c1, xr, xb]),
                (false, false, true, false) => fan(&[c2, xt, xr]),
                (false, false, false, true) => fan(&[c3, xl, xt]),
                (true, true, false, false) => fan(&[c0, c1, xr, xl]),
                (false, true, true, false) => fan(&[c1, c2, xt, xb]),
                (true, false, false, true) => fan(&[c0, xb, xt, c3]),
                (false, false, true, true) => fan(&[c3, c2, xr, xl]),
                (true, true, true, false) => fan(&[c0, c1, c2, xt, xl]),
                (true, true, false, true) => fan(&[c0, c1, xr, xt, c3]),
                (true, false, true, true) => fan(&[c0, xb, xr, c2, c3]),
                (false, true, true, true) => fan(&[xb, c1, c2, c3, xl]),
                (true, false, true, false) | (false, true, false, true) => {
                    // Saddle: center average decides joined vs disjoint.
                    let center =
                        (at(i, j) + at(i + 1, j) + at(i + 1, j + 1) + at(i, j + 1)) * 0.25 >= iso;
                    if v00 {
                        fan(&[c0, xb, xl]);
                        fan(&[c2, xt, xr]);
                        if center {
                            let cc = [i as f32 + 0.5, j as f32 + 0.5];
                            fan(&[xb, c1, cc]);
                            fan(&[cc, c1, xr]);
                            fan(&[xl, cc, c3]);
                            fan(&[cc, xt, c3]);
                        }
                    } else {
                        fan(&[c1, xr, xb]);
                        fan(&[c3, xl, xt]);
                        if center {
                            let cc = [i as f32 + 0.5, j as f32 + 0.5];
                            fan(&[xb, xr, cc]);
                            fan(&[xr, xt, cc]);
                            fan(&[xt, xl, cc]);
                            fan(&[xl, xb, cc]);
                        }
                    }
                }
            }
        }
    }
    tris
}

/// Whitecap mask from speed + convergence (0..=1 per cell).
///
/// Still water reads ~0; jets and pile-ups read high. Shader input, not
/// physics: cheap, stable, and monotone in agitation (tested).
pub fn foam_mask(sim: &ShallowWater) -> Vec<f32> {
    let n = sim.resolution();
    let cell = sim.params.size / (n - 1) as f32;
    let mut out = vec![0.0f32; n * n];
    for j in 0..n {
        for i in 0..n {
            let (vx, vz) = sim.velocity_at(i, j);
            let speed = (vx * vx + vz * vz).sqrt();
            // Divergence from shared faces (negative = converging).
            let div = (sim.fx[j * (n + 1) + i + 1] - sim.fx[j * (n + 1) + i]
                + sim.fz[(j + 1) * n + i]
                - sim.fz[j * n + i])
                / cell.max(1e-6);
            let foam = (speed * 0.5 + (-div).max(0.0) * 1.5).clamp(0.0, 1.0);
            out[j * n + i] = foam;
        }
    }
    out
}

/// Archimedes buoyancy force in newtons for a displaced volume.
pub fn archimedes_force(displaced_m3: f32, fluid_density_kg_m3: f32) -> f32 {
    const G: f32 = 9.81;
    displaced_m3.max(0.0) * fluid_density_kg_m3.max(0.0) * G
}

/// One Gerstner wave component (visual chop over the simulated surface).
///
/// Real Gerstner displaces horizontally too (`sharpness` 0..=1 steepens
/// crests into breaker-like peaks); `speed` is angular frequency.
#[derive(Debug, Clone, Copy)]
pub struct GerstnerWave {
    /// XZ travel direction (normalized internally).
    pub dir: [f32; 2],
    /// Crest height in world units.
    pub amplitude: f32,
    /// Wavelength in world units.
    pub wavelength: f32,
    /// Angular speed (rad/s).
    pub speed: f32,
    /// Crest sharpness 0..=1.
    pub sharpness: f32,
}

/// Calm-lake default chop: three small crossing swells.
pub fn calm_chop() -> [GerstnerWave; 3] {
    [
        GerstnerWave {
            dir: [1.0, 0.3],
            amplitude: 0.22,
            wavelength: 16.0,
            speed: 1.1,
            sharpness: 0.55,
        },
        GerstnerWave {
            dir: [-0.4, 1.0],
            amplitude: 0.11,
            wavelength: 7.0,
            speed: 1.7,
            sharpness: 0.5,
        },
        GerstnerWave {
            dir: [0.7, -0.7],
            amplitude: 0.05,
            wavelength: 3.1,
            speed: 2.6,
            sharpness: 0.45,
        },
    ]
}

/// Gerstner displacement `[dx, dy, dz]` at `(x, z, t)` (visual only:
/// physics keeps running on the shallow sim underneath).
pub fn gerstner_displace(x: f32, z: f32, t: f32, waves: &[GerstnerWave]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for w in waves {
        let len = (w.dir[0] * w.dir[0] + w.dir[1] * w.dir[1]).sqrt().max(1e-6);
        let (dx, dz) = (w.dir[0] / len, w.dir[1] / len);
        let k = std::f32::consts::TAU / w.wavelength.max(0.01);
        let q = w.sharpness.clamp(0.0, 1.0) / (k * w.amplitude.max(1e-6) * waves.len() as f32);
        let phase = k * (dx * x + dz * z) - w.speed * t;
        let (s, c) = phase.sin_cos();
        out[0] += q * w.amplitude * dx * c;
        out[1] += w.amplitude * s;
        out[2] += q * w.amplitude * dz * c;
    }
    out
}

fn world_to_grid(v: f32, half: f32, size: f32, n: usize) -> f32 {
    ((v + half) / size * (n - 1) as f32).clamp(0.0, (n - 1) as f32)
}

fn bilinear_grid(values: &[f32], n: usize, fx: f32, fz: f32) -> f32 {
    let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
    let (i1, j1) = ((i0 + 1).min(n - 1), (j0 + 1).min(n - 1));
    let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
    let a = values[j0 * n + i0];
    let b = values[j0 * n + i1];
    let c = values[j1 * n + i0];
    let d = values[j1 * n + i1];
    a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + d * tx * tz
}

impl ShallowWater {
    /// Water surface as a terrain [`Heightfield`](velvet_terrain::Heightfield),
    /// ready for the terrain mesher (`build_mesh`) or Scene3D export.
    ///
    /// This is the rain/puddle coupling: a terrain heightfield becomes
    /// bedrock (via [`ShallowWater::new`]), and the simulated surface comes
    /// back as a meshable field.
    pub fn surface_field(&self) -> velvet_terrain::Heightfield {
        velvet_terrain::Heightfield {
            resolution: self.resolution() as u32,
            size: self.params.size,
            heights: self.surface(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_bedrock(n: usize, h: f32) -> Vec<f32> {
        vec![h; n * n]
    }

    #[test]
    fn rain_and_evaporation_balance() {
        // Flat bed, matched rain/evaporation: the film hovers near one
        // step of rain instead of growing forever; nothing goes negative.
        let params = ShallowParams {
            resolution: 16,
            ..Default::default()
        };
        let mut sim = ShallowWater::new(params, flat_bedrock(16, 0.0)).unwrap();
        for _ in 0..2000 {
            sim.rain(0.0002);
            sim.evaporate(0.0002);
            sim.step();
        }
        let d0 = sim.depth[0];
        assert!(d0 < 0.01, "no water-cycle equilibrium: film={d0}");
        assert!(sim.depth.iter().all(|d| *d >= 0.0));
        // A big evaporation dries the map exactly.
        sim.rain(0.5);
        sim.evaporate(1.0);
        assert!(sim.depth.iter().all(|d| *d == 0.0));
        // Open-water boost only touches deep cells, never below zero.
        sim.rain(0.3);
        sim.evaporate_open_water(0.1, 0.4);
        assert!((sim.depth[0] - 0.3).abs() < 1e-6);
        sim.rain(0.3);
        sim.evaporate_open_water(0.1, 0.4);
        assert!((sim.depth[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn still_water_stays_still() {
        let params = ShallowParams {
            resolution: 16,
            ..Default::default()
        };
        let mut sim = ShallowWater::new(params, flat_bedrock(16, 0.0)).unwrap();
        sim.rain(0.5);
        let v0 = sim.volume();
        for _ in 0..20 {
            sim.step();
        }
        // Flat bed + uniform depth: nothing moves (up to fp noise).
        assert!((sim.volume() - v0).abs() < v0 * 1e-4 + 1e-6);
        assert!(sim.depth.iter().all(|d| (*d - 0.5).abs() < 1e-3));
    }

    #[test]
    fn water_flows_downhill_and_conserves_inland() {
        // Tilted bedrock: water must migrate to the low corner.
        let n = 16usize;
        let mut bedrock = vec![0.0; n * n];
        for j in 0..n {
            for i in 0..n {
                bedrock[j * n + i] = (i + j) as f32 * 0.2;
            }
        }
        let params = ShallowParams {
            resolution: 16,
            ..Default::default()
        };
        let mut sim = ShallowWater::new(params, bedrock).unwrap();
        // Puddle on the high corner.
        sim.splash(28.0, 28.0, 6.0, 2.0);
        let v0 = sim.volume();
        let centroid = |sim: &ShallowWater| {
            let (mut sx, mut sz, mut m) = (0.0f64, 0.0f64, 0.0f64);
            for j in 0..n {
                for i in 0..n {
                    let w = sim.depth[j * n + i] as f64;
                    sx += i as f64 * w;
                    sz += j as f64 * w;
                    m += w;
                }
            }
            (sx / m, sz / m)
        };
        let c0 = centroid(&sim);
        for _ in 0..300 {
            sim.step();
        }
        // Closed walls: exact conservation.
        assert!((sim.volume() - v0).abs() < v0 * 1e-4 + 1e-6);
        // Water centroid migrated toward the low corner (0,0).
        let c1 = centroid(&sim);
        assert!(c1.0 < c0.0 && c1.1 < c0.1, "{c0:?} -> {c1:?}");
    }

    #[test]
    fn bench_reports_throughput() {
        // Informative only (no perf gate: CI machines vary).
        let params = ShallowParams {
            resolution: 64,
            ..Default::default()
        };
        let mut sim = ShallowWater::new(params, vec![0.0; 64 * 64]).unwrap();
        sim.rain(0.5);
        let steps = 60u32;
        let t0 = std::time::Instant::now();
        for _ in 0..steps {
            sim.step();
        }
        let per_step = t0.elapsed().as_secs_f64() / steps as f64;
        eprintln!(
            "shallow bench: 64x64, {per_step:.3}ms/step",
            per_step = per_step * 1000.0
        );
        assert!(sim.volume() > 0.0);
    }

    #[test]
    fn gerstner_is_bounded_and_moves() {
        use super::{calm_chop, gerstner_displace};
        let waves = calm_chop();
        let max_amp: f32 = waves.iter().map(|w| w.amplitude).sum();
        // Bounded by the amplitude sum, zero-mean-ish over space.
        let mut peak = 0.0f32;
        let mut mean = 0.0f64;
        let mut n = 0u32;
        for j in 0..16 {
            for i in 0..16 {
                let d = gerstner_displace(i as f32, j as f32, 1.7, &waves);
                peak = peak.max(d[1].abs());
                mean += d[1] as f64;
                n += 1;
            }
        }
        assert!(peak <= max_amp + 1e-4, "{peak} vs {max_amp}");
        assert!(peak > max_amp * 0.25, "flat chop: {peak}");
        assert!((mean / n as f64).abs() < max_amp as f64 * 0.25);
        // Time moves the surface but never the mean.
        let a = gerstner_displace(3.0, 5.0, 0.0, &waves);
        let b = gerstner_displace(3.0, 5.0, 2.0, &waves);
        assert!((a[1] - b[1]).abs() > 1e-4);
        // Deterministic.
        assert_eq!(a, gerstner_displace(3.0, 5.0, 0.0, &waves));
    }

    #[test]
    fn open_rim_drains_while_closed_conserves() {
        // Tilted bed, puddle high: open rims lose volume, closed keep it.
        let n = 16usize;
        let mut bedrock = vec![0.0; n * n];
        for j in 0..n {
            for i in 0..n {
                bedrock[j * n + i] = (i + j) as f32 * 0.2;
            }
        }
        let run = |open_rim: bool| {
            let params = ShallowParams {
                resolution: 16,
                open_rim,
                ..Default::default()
            };
            let mut sim = ShallowWater::new(params, bedrock.clone()).unwrap();
            // Puddle near the low rim so it can reach the edge in time.
            sim.splash(-25.0, -25.0, 5.0, 2.0);
            let v0 = sim.volume();
            for _ in 0..150 {
                sim.step();
            }
            (v0, sim.volume())
        };
        let (v0, closed) = run(false);
        assert!((closed - v0).abs() < v0 * 1e-4 + 1e-6);
        let (_, open) = run(true);
        assert!(open < v0 * 0.9, "rim did not drain: {open} vs {v0}");
        assert!(open >= 0.0);
    }

    #[test]
    fn contour_follows_the_isoline() {
        use super::contour_triangles;
        // 4x4 pond in an 8x8 grid, iso 0.5: area must be ~16 cells.
        let n = 8usize;
        let mut depth = vec![0.0f32; n * n];
        for j in 2..6 {
            for i in 2..6 {
                depth[j * n + i] = 1.0;
            }
        }
        let tris = contour_triangles(&depth, n, 0.5);
        assert!(!tris.is_empty());
        let mut area = 0.0f64;
        for t in &tris {
            // Cross product = 2x geometric area; winding must stay negative.
            let a = (t[1][0] - t[0][0]) as f64 * (t[2][1] - t[0][1]) as f64
                - (t[2][0] - t[0][0]) as f64 * (t[1][1] - t[0][1]) as f64;
            assert!(a <= 0.0, "winding flipped: {a}");
            area += -a / 2.0;
        }
        assert!((area - 16.0).abs() < 2.5, "area {area}");
        // Empty and full grids: nothing and everything, no garbage.
        assert!(contour_triangles(&[0.0; 16], 4, 0.5).is_empty());
        let full = contour_triangles(&[1.0; 16], 4, 0.5);
        let mut full_area = 0.0f64;
        for t in &full {
            full_area += -((t[1][0] - t[0][0]) as f64 * (t[2][1] - t[0][1]) as f64
                - (t[2][0] - t[0][0]) as f64 * (t[1][1] - t[0][1]) as f64)
                / 2.0;
        }
        assert!((full_area - 9.0).abs() < 1e-4, "{full_area}");
    }

    #[test]
    fn bad_bedrock_rejected() {
        let params = ShallowParams::default();
        assert!(ShallowWater::new(params, vec![0.0; 10]).is_err());
    }

    #[test]
    fn foam_marks_agitation_not_calm() {
        use super::{archimedes_force, foam_mask};
        let params = ShallowParams {
            resolution: 16,
            ..Default::default()
        };
        let mut calm = ShallowWater::new(params, vec![0.0; 16 * 16]).unwrap();
        calm.rain(0.3);
        for _ in 0..10 {
            calm.step();
        }
        let max_calm = foam_mask(&calm).iter().cloned().fold(0.0f32, f32::max);
        assert!(max_calm < 0.05, "{max_calm}");
        calm.splash(0.0, 0.0, 4.0, 3.0);
        for _ in 0..10 {
            calm.step();
        }
        let max_wild = foam_mask(&calm).iter().cloned().fold(0.0f32, f32::max);
        assert!(max_wild > 0.35, "{max_wild}");
        assert!((archimedes_force(1.0, 1000.0) - 9810.0).abs() < 1e-2);
        assert_eq!(archimedes_force(-2.0, 1000.0), 0.0);
    }

    #[test]
    fn terrain_rain_coupling_meshes() {
        use velvet_terrain::{build_mesh, TerrainGraph, TerrainParams};
        // Terrain heightfield becomes bedrock; rain falls; the surface
        // comes back as a meshable field.
        let terrain = TerrainGraph::new(TerrainParams {
            seed: 3,
            resolution: 33,
            size: 32.0,
            ..Default::default()
        })
        .bake();
        let params = ShallowParams {
            resolution: 33,
            size: 32.0,
            ..Default::default()
        };
        let mut sim = ShallowWater::new(params, terrain.heights.clone()).unwrap();
        sim.rain(0.4);
        for _ in 0..30 {
            sim.step();
        }
        let surface = sim.surface_field();
        assert_eq!(surface.heights.len(), 33 * 33);
        // Surface sits at or above bedrock everywhere.
        assert!(surface
            .heights
            .iter()
            .zip(terrain.heights.iter())
            .all(|(s, b)| *s >= *b - 1e-4));
        let mesh = build_mesh(&surface);
        assert_eq!(mesh.vertex_count(), 33 * 33);
    }
}
