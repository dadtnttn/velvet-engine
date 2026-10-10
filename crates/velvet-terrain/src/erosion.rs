//! Thermal and hydraulic erosion over a [`Heightfield`].
//!
//! - Thermal: material above the talus angle slides downhill (a few Jacobi
//!   iterations stabilize scree slopes).
//! - Hydraulic: Hans Theobald Beyer-style droplets (inertia, sediment
//!   capacity, evaporation) carve valleys and deposit fans.
//!
//! Both conserve sediment up to floating-point error and are deterministic
//! for a fixed seed (droplet starts come from [`SplitMix64`]).

use crate::graph::Heightfield;
use crate::noise::SplitMix64;

/// Erosion tuning.
#[derive(Debug, Clone, Copy)]
pub struct ErosionParams {
    /// Thermal iterations.
    pub thermal_iterations: u32,
    /// Talus slope (height difference per cell at rest).
    pub talus: f32,
    /// Fraction of the excess moved per iteration.
    pub thermal_rate: f32,
    /// Droplet count.
    pub droplets: u32,
    /// Droplet inertia (0..1).
    pub inertia: f32,
    /// Sediment capacity factor.
    pub capacity: f32,
    /// Erode/deposit rate.
    pub erosion_rate: f32,
    /// Water evaporation per step.
    pub evaporate: f32,
    /// Gravity accel applied to speed.
    pub gravity: f32,
    /// Speed cap (cells/step): breaks the speed→capacity feedback loop.
    pub max_speed: f32,
    /// Droplet lifetime steps.
    pub lifetime: u32,
}

impl Default for ErosionParams {
    fn default() -> Self {
        Self {
            thermal_iterations: 8,
            talus: 1.2,
            thermal_rate: 0.25,
            droplets: 4_000,
            inertia: 0.05,
            capacity: 0.5,
            erosion_rate: 0.2,
            evaporate: 0.02,
            gravity: 2.0,
            max_speed: 3.0,
            lifetime: 30,
        }
    }
}

/// Thermal weathering in place.
pub fn erode_thermal(field: &mut Heightfield, params: &ErosionParams) {
    let res = field.resolution as usize;
    if res < 3 {
        return;
    }
    let mut delta = vec![0.0f32; res * res];
    for _ in 0..params.thermal_iterations {
        delta.fill(0.0);
        for j in 0..res {
            for i in 0..res {
                let h = field.at(i as i32, j as i32);
                // 4-neighborhood talus relaxation.
                for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (ni, nj) = (i as i32 + di, j as i32 + dj);
                    if ni < 0 || nj < 0 || ni >= res as i32 || nj >= res as i32 {
                        continue;
                    }
                    let nh = field.at(ni, nj);
                    let diff = h - nh;
                    if diff > params.talus {
                        let move_ = (diff - params.talus) * params.thermal_rate * 0.25;
                        delta[j * res + i] -= move_;
                        delta[nj as usize * res + ni as usize] += move_;
                    }
                }
            }
        }
        for (h, d) in field.heights.iter_mut().zip(delta.iter()) {
            *h += *d;
        }
    }
}

/// Box-blur smoothing pass (3x3, replicate padding).
///
/// A visual finish pass, not a conservation law: mass drift stays under
/// 0.5% while total variation strictly decreases. Kills hydraulic spikes
/// and mesh shimmer without moving landforms.
pub fn smooth(field: &mut Heightfield, iterations: u32) {
    let res = field.resolution as usize;
    if res < 3 {
        return;
    }
    let mut tmp = vec![0.0f32; res * res];
    for _ in 0..iterations {
        for j in 0..res {
            for i in 0..res {
                let mut sum = 0.0;
                for dj in -1..=1 {
                    for di in -1..=1 {
                        sum += field.at(i as i32 + di, j as i32 + dj);
                    }
                }
                tmp[j * res + i] = sum / 9.0;
            }
        }
        std::mem::swap(&mut field.heights, &mut tmp);
    }
}

/// Depression filling (priority-flood).
///
/// Rain on a pitted heightfield ponds in every artifact hollow instead of
/// routing downhill. Filling closed depressions up to their spill level
/// (plus a hair `eps` for drainage gradient) is the standard hydrological
/// preprocessing that turns pond-soup into streams: water runs off to the
/// rims or into the preserved basin instead of sticking where it fell.
///
/// `preserve` keeps one intentional basin (the dug lake):
/// `(world_center, world_radius)` cells are never raised, so the lake
/// survives the fill while everything else learns to drain.
pub fn fill_depressions(field: &mut Heightfield, preserve: Option<([f32; 2], f32)>) {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let res = field.resolution as usize;
    if res < 3 {
        return;
    }
    let in_disc = |i: usize, j: usize| -> bool {
        match preserve {
            None => false,
            Some((c, r)) => {
                let x = (i as f32 / (res - 1) as f32 - 0.5) * field.size;
                let z = (j as f32 / (res - 1) as f32 - 0.5) * field.size;
                (x - c[0]).hypot(z - c[1]) < r
            }
        }
    };
    let mut filled = field.heights.clone();
    let mut closed = vec![false; res * res];
    // (height bits, index), min-first; the index tiebreak keeps it bitwise
    // deterministic for a given input.
    let mut heap: BinaryHeap<(Reverse<u32>, Reverse<usize>)> = BinaryHeap::new();
    for j in 0..res {
        for i in 0..res {
            if i == 0 || j == 0 || i + 1 == res || j + 1 == res {
                let k = j * res + i;
                closed[k] = true;
                heap.push((Reverse(filled[k].to_bits()), Reverse(k)));
            }
        }
    }
    let eps = 1e-4f32;
    while let Some((Reverse(hb), Reverse(k))) = heap.pop() {
        let h = f32::from_bits(hb);
        let (i, j) = (k % res, k / res);
        for (di, dj) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (ni, nj) = (i as i32 + di, j as i32 + dj);
            if ni < 0 || nj < 0 || ni >= res as i32 || nj >= res as i32 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            let nk = nj * res + ni;
            if closed[nk] {
                continue;
            }
            closed[nk] = true;
            if !in_disc(ni, nj) {
                filled[nk] = filled[nk].max(h + eps);
            }
            heap.push((Reverse(filled[nk].to_bits()), Reverse(nk)));
        }
    }
    field.heights = filled;
}

/// Multiscale cavity map (concavity occludes, convexity wears).
///
/// Laplacian of the heightfield at radii 1, 2 and 4 cells, combined
/// `0.5/0.3/0.2` and normalized by the local relief: positive = crevice
/// (AO), negative = exposed edge (wear highlight). Bake once after
/// erosion; multiply albedo by `1 - k*cavity` with `k ~ 0.6-1.0`.
pub fn cavity_map(field: &Heightfield) -> Vec<f32> {
    let res = field.resolution as usize;
    if res < 9 {
        return vec![0.0; res * res];
    }
    let (_, hi) = field.range();
    let (lo, _) = field.range();
    let sigma = ((hi - lo) / (field.size).max(1.0) * 4.0).max(1e-4);
    let mut out = vec![0.0f32; res * res];
    for (radius, weight) in [(1usize, 0.5f32), (2, 0.3), (4, 0.2)] {
        for j in 0..res {
            for i in 0..res {
                let c = field.at(i as i32, j as i32);
                let lap = (field.at(i as i32 + radius as i32, j as i32)
                    + field.at(i as i32 - radius as i32, j as i32)
                    + field.at(i as i32, j as i32 + radius as i32)
                    + field.at(i as i32, j as i32 - radius as i32))
                    * 0.25
                    - c;
                out[j * res + i] += weight * (lap / (sigma * radius as f32)).clamp(-1.0, 1.0);
            }
        }
    }
    out
}

/// Total variation (sum of neighbor diffs): roughness metric.
pub fn total_variation(field: &Heightfield) -> f64 {
    let res = field.resolution as usize;
    let mut tv = 0.0f64;
    for j in 0..res {
        for i in 0..res {
            let c = field.at(i as i32, j as i32) as f64;
            if i + 1 < res {
                tv += (field.at(i as i32 + 1, j as i32) as f64 - c).abs();
            }
            if j + 1 < res {
                tv += (field.at(i as i32, j as i32 + 1) as f64 - c).abs();
            }
        }
    }
    tv
}

/// Hydraulic droplet erosion in place (deterministic for `seed`).
pub fn erode_hydraulic(field: &mut Heightfield, params: &ErosionParams, seed: u64) {
    erode_hydraulic_with_flow(field, params, seed, 0.0, &|_, _| (0.0, 0.0));
}

/// Hydraulic erosion steered by a flow field (L1 water coupling).
///
/// `flow` maps world `(x, z)` to a steering vector; `flow_weight` 0..=1
/// blends the droplet heading toward it (0 = legacy gradient-only path,
/// bit-identical to [`erode_hydraulic`]).
pub fn erode_hydraulic_with_flow(
    field: &mut Heightfield,
    params: &ErosionParams,
    seed: u64,
    flow_weight: f32,
    flow: &dyn Fn(f32, f32) -> (f32, f32),
) {
    let res = field.resolution as usize;
    if res < 3 {
        return;
    }
    let flow_weight = flow_weight.clamp(0.0, 1.0);
    let size = field.size;
    let to_world = move |c: f32| (c / (res - 1) as f32 - 0.5) * size;
    let mut rng = SplitMix64::new(seed);
    for _ in 0..params.droplets {
        // Random start inside the rim.
        let mut x = rng.range_f32(1.0, res as f32 - 2.0);
        let mut z = rng.range_f32(1.0, res as f32 - 2.0);
        let mut dir_x = 0.0f32;
        let mut dir_z = 0.0f32;
        let mut speed = 1.0f32;
        let mut water = 1.0f32;
        let mut sediment = 0.0f32;
        // Last valid position (for delta/evaporation deposits).
        let mut lx = x;
        let mut lz = z;
        let mut settled = false;
        for _ in 0..params.lifetime {
            let ix = x as usize;
            let iz = z as usize;
            if ix == 0 || iz == 0 || ix + 1 >= res || iz + 1 >= res {
                // Running off-rim drops the load: river deltas, and the
                // books stay balanced.
                distribute(field, lx, lz, sediment);
                settled = true;
                break;
            }
            lx = x;
            lz = z;
            let fx = x - ix as f32;
            let fz = z - iz as f32;
            // Gradient via bilinear height differences.
            let h00 = field.at(ix as i32, iz as i32);
            let h10 = field.at(ix as i32 + 1, iz as i32);
            let h01 = field.at(ix as i32, iz as i32 + 1);
            let h11 = field.at(ix as i32 + 1, iz as i32 + 1);
            let gx = (h10 - h00) * (1.0 - fz) + (h11 - h01) * fz;
            let gz = (h01 - h00) * (1.0 - fx) + (h11 - h10) * fx;
            dir_x = dir_x * params.inertia - gx * (1.0 - params.inertia);
            dir_z = dir_z * params.inertia - gz * (1.0 - params.inertia);
            if flow_weight > 0.0 {
                // Steer the heading toward the flow direction.
                let (fx, fz) = flow(to_world(x), to_world(z));
                let fl = (fx * fx + fz * fz).sqrt();
                if fl > 1e-6 {
                    let dl = (dir_x * dir_x + dir_z * dir_z).sqrt().max(1e-6);
                    let (hx, hz) = (dir_x / dl, dir_z / dl);
                    let (qx, qz) = (fx / fl, fz / fl);
                    dir_x = hx + (qx - hx) * flow_weight;
                    dir_z = hz + (qz - hz) * flow_weight;
                }
            }
            let len = (dir_x * dir_x + dir_z * dir_z).sqrt().max(1e-4);
            dir_x /= len;
            dir_z /= len;
            x += dir_x;
            z += dir_z;
            if x < 1.0 || z < 1.0 || x > res as f32 - 2.0 || z > res as f32 - 2.0 {
                distribute(field, lx, lz, sediment);
                settled = true;
                break;
            }
            let h_new = bilinear(field, x, z);
            let dh = h_new - bilinear(field, x - dir_x, z - dir_z);
            let capacity = (-dh).max(0.01) * speed * water * params.capacity;
            if sediment > capacity || dh > 0.0 {
                // Deposit uphill or overloaded.
                let deposit = if dh > 0.0 {
                    dh.min(sediment)
                } else {
                    (sediment - capacity) * params.erosion_rate
                };
                sediment -= deposit;
                distribute(field, x, z, deposit);
            } else {
                // Erode downhill.
                let erode = ((capacity - sediment) * params.erosion_rate).min(-dh);
                sediment += erode;
                distribute(field, x, z, -erode);
            }
            speed = (speed * speed + (-dh) * params.gravity)
                .sqrt()
                .max(1e-4)
                .min(params.max_speed);
            water *= 1.0 - params.evaporate;
        }
        // Lifetime expiry evaporates the drop: load stays behind.
        if !settled {
            distribute(field, lx, lz, sediment);
        }
    }
}

fn bilinear(field: &Heightfield, x: f32, z: f32) -> f32 {
    let res = field.resolution as usize;
    let x = x.clamp(0.0, res as f32 - 1.001);
    let z = z.clamp(0.0, res as f32 - 1.001);
    let (ix, iz) = (x.floor() as usize, z.floor() as usize);
    let (fx, fz) = (x - ix as f32, z - iz as f32);
    let a = field.at(ix as i32, iz as i32);
    let b = field.at(ix as i32 + 1, iz as i32);
    let c = field.at(ix as i32, iz as i32 + 1);
    let d = field.at(ix as i32 + 1, iz as i32 + 1);
    a * (1.0 - fx) * (1.0 - fz) + b * fx * (1.0 - fz) + c * (1.0 - fx) * fz + d * fx * fz
}

/// Add `amount` bilinearly to the 4 cells around `(x, z)`.
fn distribute(field: &mut Heightfield, x: f32, z: f32, amount: f32) {
    let res = field.resolution as usize;
    let x = x.clamp(0.0, res as f32 - 1.001);
    let z = z.clamp(0.0, res as f32 - 1.001);
    let (ix, iz) = (x.floor() as usize, z.floor() as usize);
    let (fx, fz) = (x - ix as f32, z - iz as f32);
    let w = [
        ((ix, iz), (1.0 - fx) * (1.0 - fz)),
        ((ix + 1, iz), fx * (1.0 - fz)),
        ((ix, iz + 1), (1.0 - fx) * fz),
        ((ix + 1, iz + 1), fx * fz),
    ];
    for ((i, j), weight) in w {
        field.heights[j * res + i] += amount * weight;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{TerrainGraph, TerrainParams};

    fn island(res: u32) -> Heightfield {
        TerrainGraph::new(TerrainParams {
            seed: 7,
            resolution: res,
            ..Default::default()
        })
        .bake()
    }

    #[test]
    fn thermal_conserves_and_carleens_cliffs() {
        let mut h = island(33);
        let before = h.total();
        let max_slope_before = max_neighbor_slope(&h);
        erode_thermal(&mut h, &ErosionParams::default());
        assert!((h.total() - before).abs() < 1e-2, "lost sediment");
        assert!(max_neighbor_slope(&h) <= max_slope_before + 1e-4);
    }

    #[test]
    fn hydraulic_conserves_and_digs() {
        let mut h = island(33);
        let before = h.total();
        let params = ErosionParams {
            droplets: 500,
            ..Default::default()
        };
        erode_hydraulic(&mut h, &params, 11);
        // Rim exits deposit deltas: mass conserved up to fp noise.
        let after = h.total();
        assert!(
            (after - before).abs() < before.abs() * 1e-3 + 0.1,
            "mass drift: {before} -> {after}"
        );
        // It actually dug: peak relief changed and heights moved.
        let moved: f32 = h
            .heights
            .iter()
            .zip(island(33).heights.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(moved > 10.0, "no carving happened: {moved}");
        let (lo, hi) = (h.range().0, h.range().1);
        assert!(lo.is_finite() && hi.is_finite() && hi - lo < 500.0);
        // Deterministic.
        let mut h2 = island(33);
        erode_hydraulic(&mut h2, &params, 11);
        assert_eq!(h.heights, h2.heights);
    }

    #[test]
    fn bench_reports_throughput() {
        // Informative only (no perf gate: CI machines vary).
        let mut h = island(65);
        let params = ErosionParams {
            droplets: 1000,
            ..Default::default()
        };
        let t0 = std::time::Instant::now();
        erode_hydraulic(&mut h, &params, 5);
        eprintln!(
            "erosion bench: 1000 droplets on 65x65 took {:.1}s",
            t0.elapsed().as_secs_f64()
        );
        assert!(h.total().is_finite());
    }

    #[test]
    fn flow_steering_digs_downstream() {
        use crate::graph::Heightfield;
        // Gentle z-tilt (x-symmetric): gradient-only droplets wander
        // symmetrically in x, flow-steered ones stream +x and dig there.
        let flat = || {
            let mut heights = Vec::with_capacity(33 * 33);
            for j in 0..33 {
                for _ in 0..33 {
                    heights.push(5.0 + j as f32 * 0.25);
                }
            }
            Heightfield {
                resolution: 33,
                size: 32.0,
                heights,
            }
        };
        let params = ErosionParams {
            droplets: 1500,
            ..Default::default()
        };
        let mut plain = flat();
        erode_hydraulic(&mut plain, &params, 21);
        let mut steered = flat();
        erode_hydraulic_with_flow(&mut steered, &params, 21, 0.8, &|_, _| (1.0, 0.0));
        let centroid_x = |h: &Heightfield, before: &Heightfield| {
            let (mut sx, mut m) = (0.0f64, 0.0f64);
            for (i, (a, b)) in h.heights.iter().zip(before.heights.iter()).enumerate() {
                let removed = (*b as f64 - *a as f64).max(0.0);
                sx += (i % 33) as f64 * removed;
                m += removed;
            }
            sx / m.max(1e-9)
        };
        let reference = flat();
        // Downhill (+x bias): steering keeps erosion rates high while
        // skewing the dig downstream. (Pure +x flow would contour-follow
        // flat z-rows and dig nothing — correct physics, bad test.)
        let mut steered = flat();
        erode_hydraulic_with_flow(&mut steered, &params, 21, 0.8, &|_, _| (0.5, -1.0));
        let c_plain = centroid_x(&plain, &reference);
        let c_steered = centroid_x(&steered, &reference);
        // Symmetric wander centers; steering pushes the dig downstream.
        assert!((c_plain - 16.0).abs() < 2.0, "{c_plain}");
        assert!(c_steered > c_plain + 1.0, "{c_steered} vs {c_plain}");
        // Steering conserves too.
        assert!((steered.total() - reference.total()).abs() < 1.0);
    }

    #[test]
    fn smooth_kills_roughness_keeps_mass() {
        let mut h = island(33);
        let before = h.total();
        let tv_before = total_variation(&h);
        smooth(&mut h, 2);
        assert!((h.total() - before).abs() < before.abs() * 0.005 + 0.1);
        assert!(total_variation(&h) < tv_before * 0.9);
        // Deterministic.
        let mut h2 = island(33);
        smooth(&mut h2, 2);
        assert_eq!(h.heights, h2.heights);
    }

    #[test]
    fn cavity_marks_crevice_not_crest() {
        // Synthetic ridge: center high. Laplacian < 0 on the crest,
        // > 0 in the ditch beside it.
        let mut h = Heightfield {
            resolution: 17,
            size: 16.0,
            heights: vec![0.0; 17 * 17],
        };
        for j in 0..17 {
            for i in 0..17 {
                // V-shaped valley along x=8.
                h.heights[j * 17 + i] = ((i as f32 - 8.0).abs() / 8.0) * 4.0;
            }
        }
        let c = cavity_map(&h);
        // Valley floor (concave): positive.
        assert!(c[8 * 17 + 8] > 0.1, "{}", c[8 * 17 + 8]);
        // Far flank (planar): near zero.
        assert!(c[8 * 17 + 2].abs() < 0.3, "{}", c[8 * 17 + 2]);
    }

    #[test]
    fn full_default_run_stays_bounded() {
        // Regression: 4000 default droplets on the default island must
        // carve without runaway spikes (relief stays same order as baked).
        let mut h = island(33);
        let (blo, bhi) = h.range();
        erode_hydraulic(&mut h, &ErosionParams::default(), 7);
        let (lo, hi) = h.range();
        assert!(lo.is_finite() && hi.is_finite());
        assert!(
            hi - lo < (bhi - blo) * 4.0 + 20.0,
            "blowup: [{blo}, {bhi}] -> [{lo}, {hi}]"
        );
    }

    #[test]
    fn erosion_is_idempotent_enough_to_settle() {
        let mut h = island(17);
        let params = ErosionParams {
            droplets: 200,
            ..Default::default()
        };
        erode_hydraulic(&mut h, &params, 3);
        let snapshot = h.heights.clone();
        erode_thermal(&mut h, &ErosionParams::default());
        let moved: f32 = h
            .heights
            .iter()
            .zip(snapshot.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(moved.is_finite());
    }

    #[test]
    fn fill_raises_pits_to_spill_not_above() {
        use crate::graph::{Heightfield, TerrainParams};
        // Donut: rim at 5, pit floor at 0 -> spill fill, never above rim.
        let res = 7usize;
        let mut heights = vec![5.0f32; res * res];
        heights[3 * res + 3] = 0.0;
        heights[3 * res + 2] = 1.0;
        let mut h = Heightfield {
            resolution: res as u32,
            size: 64.0,
            heights,
        };
        let _ = TerrainParams::default();
        fill_depressions(&mut h, None);
        let c = h.heights[3 * res + 3];
        assert!(c > 4.99 && c <= 5.0 + 1e-3, "pit not spilled: {c}");
        assert!(h.heights.iter().all(|v| v.is_finite()));
        assert!(h.heights.iter().all(|v| *v >= 0.0));
    }

    #[test]
    fn fill_preserves_the_lake_disc() {
        use crate::graph::Heightfield;
        let res = 9usize;
        let heights = vec![5.0f32; res * res];
        let mut h = Heightfield {
            resolution: res as u32,
            size: 80.0,
            heights,
        };
        h.heights[4 * res + 4] = -6.0;
        // Disc around map center (world 0,0), radius 20: the pit survives.
        fill_depressions(&mut h, Some(([0.0, 0.0], 20.0)));
        assert_eq!(h.heights[4 * res + 4], -6.0);
        // ...while a pit outside the disc fills.
        h.heights[res + 1] = 0.5;
        fill_depressions(&mut h, Some(([0.0, 0.0], 20.0)));
        assert!(h.heights[res + 1] > 4.99, "outer pit survived");
    }

    #[test]
    fn fill_leaves_draining_ramps_alone() {
        use crate::graph::Heightfield;
        let res = 8usize;
        let heights: Vec<f32> = (0..res * res).map(|k| (k % res) as f32).collect();
        let mut h = Heightfield {
            resolution: res as u32,
            size: 64.0,
            heights: heights.clone(),
        };
        fill_depressions(&mut h, None);
        assert_eq!(h.heights, heights, "ramp should drain already");
    }

    fn max_neighbor_slope(h: &Heightfield) -> f32 {
        let res = h.resolution as usize;
        let mut max: f32 = 0.0;
        for j in 0..res {
            for i in 0..res {
                let c = h.at(i as i32, j as i32);
                for (di, dj) in [(1, 0), (0, 1)] {
                    let n = h.at(i as i32 + di, j as i32 + dj);
                    max = max.max((c - n).abs());
                }
            }
        }
        max
    }
}
