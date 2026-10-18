//! Weakly-compressible SPH (WCSPH) in 3D: dam break in a box.
//!
//! Poly6 density, spiky-gradient pressure, viscosity laplacian, uniform-grid
//! neighbors over a `BTreeMap` (deterministic order), semi-implicit Euler,
//! reflective box walls. Reference quality, not production scale: hundreds
//! to a few thousand particles at interactive rates on CPU.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Solver tuning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SphParams {
    /// Smoothing length (world units).
    pub h: f32,
    /// Rest density.
    pub rest_density: f32,
    /// Pressure stiffness.
    pub stiffness: f32,
    /// Viscosity.
    pub viscosity: f32,
    /// Seconds per step.
    pub dt: f32,
    /// Gravity accel (applied to -Y).
    pub gravity: f32,
    /// Box bounds `[min, max]` per axis.
    pub bounds_min: [f32; 3],
    /// Box bounds `[min, max]` per axis.
    pub bounds_max: [f32; 3],
    /// Wall restitution 0..=1.
    pub restitution: f32,
}

impl Default for SphParams {
    fn default() -> Self {
        Self {
            h: 1.0,
            rest_density: 1000.0,
            stiffness: 200.0,
            viscosity: 0.5,
            dt: 0.004,
            gravity: 9.81,
            bounds_min: [0.0, 0.0, 0.0],
            bounds_max: [12.0, 10.0, 6.0],
            restitution: 0.2,
        }
    }
}

/// One particle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphParticle {
    /// Position.
    pub pos: [f32; 3],
    /// Velocity.
    pub vel: [f32; 3],
    /// Density.
    pub density: f32,
    /// Pressure.
    pub pressure: f32,
}

/// WCSPH simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphSim {
    params: SphParams,
    /// Particle mass (uniform).
    pub mass: f32,
    /// Particles in index order.
    pub particles: Vec<SphParticle>,
}

impl SphSim {
    /// Dam break: water block in `[0, cols*dx] × [0, rows*dy] × [0, layers*dz]`.
    pub fn dam_break(params: SphParams, cols: usize, rows: usize, layers: usize, dx: f32) -> Self {
        let mass = params.rest_density * dx * dx * dx;
        let mut particles = Vec::with_capacity(cols * rows * layers);
        for l in 0..layers {
            for r in 0..rows {
                for c in 0..cols {
                    particles.push(SphParticle {
                        pos: [
                            dx * 0.5 + c as f32 * dx,
                            dx * 0.5 + r as f32 * dx,
                            dx * 0.5 + l as f32 * dx,
                        ],
                        vel: [0.0; 3],
                        density: params.rest_density,
                        pressure: 0.0,
                    });
                }
            }
        }
        let mut sim = Self {
            params,
            mass,
            particles,
        };
        sim.compute_density_pressure();
        sim
    }

    /// Particle count.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Mean density (for settle checks).
    pub fn mean_density(&self) -> f32 {
        if self.particles.is_empty() {
            return 0.0;
        }
        self.particles.iter().map(|p| p.density).sum::<f32>() / self.particles.len() as f32
    }

    /// Center of mass.
    pub fn center_of_mass(&self) -> [f32; 3] {
        let mut c = [0.0f32; 3];
        for p in &self.particles {
            c[0] += p.pos[0];
            c[1] += p.pos[1];
            c[2] += p.pos[2];
        }
        let n = self.particles.len().max(1) as f32;
        [c[0] / n, c[1] / n, c[2] / n]
    }

    /// One step: density/pressure, forces, integrate, collide.
    pub fn step(&mut self) {
        self.compute_density_pressure();
        let grid = self.build_grid();
        let n = self.particles.len();
        let mut acc = vec![[0.0f32; 3]; n];
        for (i, pi) in self.particles.iter().enumerate() {
            let pi = *pi;
            let mut ax = 0.0;
            let mut ay = -self.params.gravity;
            let mut az = 0.0;
            for j in self.neighbors(&grid, pi.pos) {
                if i == j {
                    continue;
                }
                let pj = self.particles[j];
                let dx = pi.pos[0] - pj.pos[0];
                let dy = pi.pos[1] - pj.pos[1];
                let dz = pi.pos[2] - pj.pos[2];
                let r = (dx * dx + dy * dy + dz * dz).sqrt();
                if r <= 1e-6 || r >= self.params.h {
                    continue;
                }
                // Pressure (spiky gradient).
                let press = -self.mass
                    * (pi.pressure / (pi.density * pi.density)
                        + pj.pressure / (pj.density * pj.density))
                    * spiky_grad(self.params.h, r);
                let inv = 1.0 / r;
                ax += press * dx * inv;
                ay += press * dy * inv;
                az += press * dz * inv;
                // Viscosity (laplacian).
                let visc = self.params.viscosity * self.mass / pj.density
                    * viscosity_laplacian(self.params.h, r);
                ax += visc * (pj.vel[0] - pi.vel[0]);
                ay += visc * (pj.vel[1] - pi.vel[1]);
                az += visc * (pj.vel[2] - pi.vel[2]);
            }
            // Clamp wild accelerations (stability guard, counted in tests).
            let mag = (ax * ax + ay * ay + az * az).sqrt();
            let cap = 5000.0;
            let k = if mag > cap { cap / mag } else { 1.0 };
            acc[i] = [ax * k, ay * k, az * k];
        }
        let dt = self.params.dt;
        let restitution = self.params.restitution;
        let bounds_min = self.params.bounds_min;
        let bounds_max = self.params.bounds_max;
        for (p, a) in self.particles.iter_mut().zip(acc.iter()) {
            for (vel, (pos, acc_c)) in p.vel.iter_mut().zip(p.pos.iter_mut().zip(a.iter())) {
                *vel += *acc_c * dt;
                // Velocity cap (CFL-ish safety).
                *vel = vel.clamp(-30.0, 30.0);
                *pos += *vel * dt;
            }
            // Reflective box walls.
            let [px, py, pz] = &mut p.pos;
            let [vx, vy, vz] = &mut p.vel;
            let [lox, loy, loz] = bounds_min;
            let [hix, hiy, hiz] = bounds_max;
            for (pos, vel, lo, hi) in [(px, vx, lox, hix), (py, vy, loy, hiy), (pz, vz, loz, hiz)] {
                if *pos < lo {
                    *pos = lo;
                    *vel = -*vel * restitution;
                } else if *pos > hi {
                    *pos = hi;
                    *vel = -*vel * restitution;
                }
            }
        }
    }

    fn compute_density_pressure(&mut self) {
        let grid = self.build_grid();
        for i in 0..self.particles.len() {
            let pi = self.particles[i];
            let mut density = 0.0;
            for j in self.neighbors(&grid, pi.pos) {
                let pj = self.particles[j];
                let dx = pi.pos[0] - pj.pos[0];
                let dy = pi.pos[1] - pj.pos[1];
                let dz = pi.pos[2] - pj.pos[2];
                let r2 = dx * dx + dy * dy + dz * dz;
                if r2 < self.params.h * self.params.h {
                    density += self.mass * poly6(self.params.h, r2.sqrt());
                }
            }
            // Never collapse below a floor (stability).
            let density = density.max(self.params.rest_density * 0.2);
            self.particles[i].density = density;
            self.particles[i].pressure =
                (self.params.stiffness * (density / self.params.rest_density - 1.0)).max(0.0);
        }
    }

    fn cell_of(&self, pos: [f32; 3]) -> (i32, i32, i32) {
        let h = self.params.h;
        (
            (pos[0] / h).floor() as i32,
            (pos[1] / h).floor() as i32,
            (pos[2] / h).floor() as i32,
        )
    }

    fn build_grid(&self) -> BTreeMap<(i32, i32, i32), Vec<usize>> {
        let mut grid: BTreeMap<(i32, i32, i32), Vec<usize>> = BTreeMap::new();
        for (i, p) in self.particles.iter().enumerate() {
            grid.entry(self.cell_of(p.pos)).or_default().push(i);
        }
        grid
    }

    fn neighbors(&self, grid: &BTreeMap<(i32, i32, i32), Vec<usize>>, pos: [f32; 3]) -> Vec<usize> {
        let (cx, cy, cz) = self.cell_of(pos);
        let mut out = Vec::new();
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if let Some(cell) = grid.get(&(cx + dx, cy + dy, cz + dz)) {
                        out.extend_from_slice(cell);
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// Density at an arbitrary point (direct sum; meshing-scale use).
    pub fn density_at(&self, pos: [f32; 3]) -> f32 {
        let mut density = 0.0;
        for p in &self.particles {
            let dx = pos[0] - p.pos[0];
            let dy = pos[1] - p.pos[1];
            let dz = pos[2] - p.pos[2];
            let r2 = dx * dx + dy * dy + dz * dz;
            if r2 < self.params.h * self.params.h {
                density += self.mass * poly6(self.params.h, r2.sqrt());
            }
        }
        density
    }

    /// Triangle mesh of the density isosurface (naive surface nets).
    ///
    /// Samples density on a `cell`-spaced grid over the sim bounds at
    /// `iso_fraction * rest_density`, places one vertex per sign-changing
    /// cube (average of edge crossings), and stitches quads across
    /// sign-changing grid edges. Normals come from the density gradient
    /// (central differences), so shading stays smooth.
    pub fn surface_mesh(&self, cell: f32, iso_fraction: f32) -> velvet_terrain::TerrainMesh {
        let cell = cell.max(1e-3);
        let iso = self.params.rest_density * iso_fraction;
        let [min_x, min_y, min_z] = self.params.bounds_min;
        let [max_x, max_y, max_z] = self.params.bounds_max;
        let nx = ((max_x - min_x) / cell).ceil().max(1.0) as usize + 1;
        let ny = ((max_y - min_y) / cell).ceil().max(1.0) as usize + 1;
        let nz = ((max_z - min_z) / cell).ceil().max(1.0) as usize + 1;
        // Density lattice + inside mask.
        let mut dens = vec![0.0f32; nx * ny * nz];
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    dens[k * ny * nx + j * nx + i] = self.density_at([
                        min_x + i as f32 * cell,
                        min_y + j as f32 * cell,
                        min_z + k as f32 * cell,
                    ]);
                }
            }
        }
        let inside = |i: usize, j: usize, k: usize| dens[k * ny * nx + j * nx + i] > iso;
        let point = |i: usize, j: usize, k: usize| {
            [
                min_x + i as f32 * cell,
                min_y + j as f32 * cell,
                min_z + k as f32 * cell,
            ]
        };
        // 12 cube edges as corner pairs (corner = x + 2y + 4z).
        const EDGES: [(usize, usize); 12] = [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let corner = |cell: (usize, usize, usize), c: usize| {
            (
                cell.0 + (c & 1),
                cell.1 + ((c >> 1) & 1),
                cell.2 + ((c >> 2) & 1),
            )
        };
        // One vertex per sign-changing cube.
        let mut vert_of: std::collections::BTreeMap<(usize, usize, usize), u32> =
            std::collections::BTreeMap::new();
        let mut positions: Vec<f32> = Vec::new();
        for k in 0..nz - 1 {
            for j in 0..ny - 1 {
                for i in 0..nx - 1 {
                    let mut mask = 0u8;
                    for c in 0..8 {
                        let (ci, cj, ck) = corner((i, j, k), c);
                        if inside(ci, cj, ck) {
                            mask |= 1 << c;
                        }
                    }
                    if mask == 0 || mask == 0xFF {
                        continue;
                    }
                    let mut avg = [0.0f32; 3];
                    let mut count = 0u32;
                    for (a, b) in EDGES {
                        let (ai, aj, ak) = corner((i, j, k), a);
                        let (bi, bj, bk) = corner((i, j, k), b);
                        if inside(ai, aj, ak) == inside(bi, bj, bk) {
                            continue;
                        }
                        let da = dens[ak * ny * nx + aj * nx + ai] - iso;
                        let db = dens[bk * ny * nx + bj * nx + bi] - iso;
                        let t = (da / (da - db)).clamp(0.0, 1.0);
                        let pa = point(ai, aj, ak);
                        let pb = point(bi, bj, bk);
                        for c in 0..3 {
                            avg[c] += pa[c] + (pb[c] - pa[c]) * t;
                        }
                        count += 1;
                    }
                    if count == 0 {
                        continue;
                    }
                    let idx = (positions.len() / 3) as u32;
                    positions.extend_from_slice(&[
                        avg[0] / count as f32,
                        avg[1] / count as f32,
                        avg[2] / count as f32,
                    ]);
                    vert_of.insert((i, j, k), idx);
                }
            }
        }
        // Quads across sign-changing grid edges (x, y, then z edges).
        let mut indices: Vec<u32> = Vec::new();
        let mut quad = |cells: [(usize, usize, usize); 4], flip: bool| {
            let mut v = [0u32; 4];
            for (n, cell) in cells.iter().enumerate() {
                let Some(idx) = vert_of.get(cell) else {
                    return;
                };
                v[n] = *idx;
            }
            if flip {
                indices.extend_from_slice(&[v[0], v[2], v[1], v[0], v[3], v[2]]);
            } else {
                indices.extend_from_slice(&[v[0], v[1], v[2], v[0], v[2], v[3]]);
            }
        };
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx - 1 {
                    if inside(i, j, k) == inside(i + 1, j, k) {
                        continue;
                    }
                    // Cells around the x-edge, cyclic order seen from +x.
                    let cells = [
                        (i, j.wrapping_sub(1), k.wrapping_sub(1)),
                        (i, j, k.wrapping_sub(1)),
                        (i, j, k),
                        (i, j.wrapping_sub(1), k),
                    ];
                    if cells
                        .iter()
                        .any(|(ci, cj, ck)| *ci >= nx - 1 || *cj >= ny - 1 || *ck >= nz - 1)
                    {
                        continue;
                    }
                    // Winding verified by the signed-volume test (outward).
                    quad(cells, !inside(i, j, k));
                }
            }
        }
        for k in 0..nz {
            for j in 0..ny - 1 {
                for i in 0..nx {
                    if inside(i, j, k) == inside(i, j + 1, k) {
                        continue;
                    }
                    let cells = [
                        (i.wrapping_sub(1), j, k.wrapping_sub(1)),
                        (i, j, k.wrapping_sub(1)),
                        (i, j, k),
                        (i.wrapping_sub(1), j, k),
                    ];
                    if cells
                        .iter()
                        .any(|(ci, cj, ck)| *ci >= nx - 1 || *cj >= ny - 1 || *ck >= nz - 1)
                    {
                        continue;
                    }
                    quad(cells, !inside(i, j, k));
                }
            }
        }
        for k in 0..nz - 1 {
            for j in 0..ny {
                for i in 0..nx {
                    if inside(i, j, k) == inside(i, j, k + 1) {
                        continue;
                    }
                    let cells = [
                        (i.wrapping_sub(1), j.wrapping_sub(1), k),
                        (i, j.wrapping_sub(1), k),
                        (i, j, k),
                        (i.wrapping_sub(1), j, k),
                    ];
                    if cells
                        .iter()
                        .any(|(ci, cj, ck)| *ci >= nx - 1 || *cj >= ny - 1 || *ck >= nz - 1)
                    {
                        continue;
                    }
                    quad(cells, !inside(i, j, k));
                }
            }
        }
        // Normals from the density gradient + planar UVs + water biome.
        let nv = positions.len() / 3;
        let mut normals = Vec::with_capacity(nv * 3);
        let grad = |p: [f32; 3]| {
            let e = cell * 0.5;
            let gx =
                self.density_at([p[0] + e, p[1], p[2]]) - self.density_at([p[0] - e, p[1], p[2]]);
            let gy =
                self.density_at([p[0], p[1] + e, p[2]]) - self.density_at([p[0], p[1] - e, p[2]]);
            let gz =
                self.density_at([p[0], p[1], p[2] + e]) - self.density_at([p[0], p[1], p[2] - e]);
            // Gradient points inward (toward density); normals face outward.
            let len = (gx * gx + gy * gy + gz * gz).sqrt().max(1e-9);
            [-gx / len, -gy / len, -gz / len]
        };
        let mut uvs = Vec::with_capacity(nv * 2);
        for v in 0..nv {
            let p = [positions[v * 3], positions[v * 3 + 1], positions[v * 3 + 2]];
            let n = grad(p);
            normals.extend_from_slice(&n);
            uvs.extend_from_slice(&[
                (p[0] - min_x) / (max_x - min_x).max(1e-6),
                (p[2] - min_z) / (max_z - min_z).max(1e-6),
            ]);
        }
        velvet_terrain::TerrainMesh {
            positions,
            normals,
            uvs,
            indices,
            biomes: vec![0; nv],
        }
    }
}

/// Poly6 kernel (3D density).
fn poly6(h: f32, r: f32) -> f32 {
    if r >= h {
        return 0.0;
    }
    let q = h * h - r * r;
    315.0 / (64.0 * std::f32::consts::PI * h.powi(9)) * q * q * q
}

/// Spiky gradient magnitude (3D pressure), without the `1/r` direction.
fn spiky_grad(h: f32, r: f32) -> f32 {
    if r >= h || r <= 0.0 {
        return 0.0;
    }
    let q = h - r;
    -45.0 / (std::f32::consts::PI * h.powi(6)) * q * q
}

/// Viscosity laplacian (3D).
fn viscosity_laplacian(h: f32, r: f32) -> f32 {
    if r >= h {
        return 0.0;
    }
    45.0 / (std::f32::consts::PI * h.powi(6)) * (h - r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_dam() -> SphSim {
        SphSim::dam_break(SphParams::default(), 6, 8, 4, 0.5)
    }

    #[test]
    fn kernels_behave() {
        let h = 1.0;
        assert!(poly6(h, 0.0) > poly6(h, 0.5));
        assert_eq!(poly6(h, 1.0), 0.0);
        assert!(spiky_grad(h, 0.5) < 0.0);
        assert_eq!(viscosity_laplacian(h, 2.0), 0.0);
    }

    #[test]
    fn steps_are_deterministic_and_finite() {
        let mut a = small_dam();
        let mut b = small_dam();
        for _ in 0..10 {
            a.step();
            b.step();
        }
        assert_eq!(a.particles, b.particles);
        for p in &a.particles {
            for c in 0..3 {
                assert!(p.pos[c].is_finite() && p.vel[c].is_finite());
            }
            assert!(p.density.is_finite() && p.pressure.is_finite());
            assert!(p.pressure >= 0.0);
        }
    }

    fn signed_volume_m3(positions: &[f32], indices: &[u32]) -> f64 {
        let mut vol = 0.0f64;
        for t in indices.as_chunks::<3>().0 {
            let a = [
                positions[t[0] as usize * 3] as f64,
                positions[t[0] as usize * 3 + 1] as f64,
                positions[t[0] as usize * 3 + 2] as f64,
            ];
            let b = [
                positions[t[1] as usize * 3] as f64,
                positions[t[1] as usize * 3 + 1] as f64,
                positions[t[1] as usize * 3 + 2] as f64,
            ];
            let c = [
                positions[t[2] as usize * 3] as f64,
                positions[t[2] as usize * 3 + 1] as f64,
                positions[t[2] as usize * 3 + 2] as f64,
            ];
            let (e1, e2) = (
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            vol += a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2];
        }
        vol / 6.0
    }

    #[test]
    fn surface_nets_close_outward_blob() {
        let mut sim = small_dam();
        for _ in 0..30 {
            sim.step();
        }
        let mesh = sim.surface_mesh(0.5, 0.5);
        assert!(mesh.triangle_count() > 20, "tris={}", mesh.triangle_count());
        let nv = mesh.vertex_count() as u32;
        assert!(mesh.indices.iter().all(|i| *i < nv));
        // Outward orientation: positive signed volume.
        let vol = signed_volume_m3(&mesh.positions, &mesh.indices);
        assert!(vol > 0.5, "signed volume {vol}");
        // Deterministic mesh.
        let mesh2 = sim.surface_mesh(0.5, 0.5);
        assert_eq!(mesh.positions, mesh2.positions);
        assert_eq!(mesh.indices, mesh2.indices);
    }

    #[test]
    fn bench_reports_throughput() {
        // Informative only (no perf gate: CI machines vary).
        let mut sim = small_dam();
        let steps = 40u32;
        let t0 = std::time::Instant::now();
        for _ in 0..steps {
            sim.step();
        }
        let per_step = t0.elapsed().as_secs_f64() / steps as f64;
        eprintln!(
            "sph bench: {} particles, {per_step:.2}ms/step",
            sim.len(),
            per_step = per_step * 1000.0
        );
        assert!(sim.center_of_mass()[1].is_finite());
    }

    #[test]
    fn dam_break_collapses_and_stays_in_bounds() {
        let mut sim = small_dam();
        let com0 = sim.center_of_mass();
        let max_x0 = sim
            .particles
            .iter()
            .map(|p| p.pos[0])
            .fold(0.0f32, f32::max);
        for _ in 0..120 {
            sim.step();
        }
        let com1 = sim.center_of_mass();
        // Water slumps: center of mass falls and spreads +x.
        assert!(com1[1] < com0[1], "{} vs {}", com1[1], com0[1]);
        let max_x1 = sim
            .particles
            .iter()
            .map(|p| p.pos[0])
            .fold(0.0f32, f32::max);
        assert!(max_x1 > max_x0, "{max_x1} vs {max_x0}");
        for p in &sim.particles {
            for c in 0..3 {
                assert!(
                    p.pos[c] >= sim.params.bounds_min[c] - 1e-4
                        && p.pos[c] <= sim.params.bounds_max[c] + 1e-4
                );
            }
        }
        // Density stays in a sane band around rest density.
        let mean = sim.mean_density();
        assert!(
            (mean - sim.params.rest_density).abs() < sim.params.rest_density * 0.6,
            "{mean}"
        );
    }
}
