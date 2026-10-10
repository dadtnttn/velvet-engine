//! terraflow: terrain + lake showcase, headless or windowed 3D.
//!
//! `--headless` runs the deterministic pipeline (bake, erosion, shallow
//! water + rain, flow-coupled re-erosion, SPH dam break) and writes PNGs +
//! `.scene3d.json` + stats. Without flags it opens a wgpu window: orbit
//! with arrows/WASD, zoom with Q/E, `R` regenerates, Esc quits. Water is a
//! still plane at lake level; the live fluids live in the crates + headless
//! stats (see `velvet-liquid`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use velvet_liquid::{ShallowParams, ShallowWater, SphParams, SphSim, WaveSpectrum};
use velvet_math::{Mat4, Vec3};
use velvet_render::{
    scene_draws, upload_scene_meshes, Camera3D, Lighting3D, Mesh3D, MeshVertex3D, SceneMeshCache,
    Wgpu3DRenderer,
};
use velvet_scene3d::{flatten_scene, FlatNode3D};
use velvet_terrain::{
    build_mesh, erode_hydraulic, erode_hydraulic_with_flow, erode_thermal, paint_biomes,
    ErosionParams, Heightfield, TerrainGraph, TerrainParams,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

fn arg_value(args: &[String], name: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.to_string())
}

fn write_png(path: &Path, res: u32, pixels: Vec<u8>) {
    let img = velvet_image::RgbaImage {
        width: res,
        height: res,
        pixels,
    };
    let bytes =
        velvet_image::encode(&img, velvet_image::ImageEncode::default()).expect("png encode");
    std::fs::write(path, bytes).expect("write png");
}

/// Water-surface tint (blues) with the same sun as the terrain preview.
fn water_preview(field: &Heightfield) -> Vec<u8> {
    let res = field.resolution as usize;
    let (lo, hi) = field.range();
    let span = (hi - lo).max(1e-6);
    let mut out = vec![0u8; res * res * 4];
    for j in 0..res {
        for i in 0..res {
            let t = (field.at(i as i32, j as i32) - lo) / span;
            let step = field.step();
            let dx = field.at(i as i32 + 1, j as i32) - field.at(i as i32 - 1, j as i32);
            let dz = field.at(i as i32, j as i32 + 1) - field.at(i as i32, j as i32 - 1);
            let shade = (1.0 - (dx * dx + dz * dz).sqrt() / (2.0 * step) * 0.35).clamp(0.3, 1.0);
            let (r, g, b) = (0.12 + 0.1 * t, 0.35 + 0.25 * t, 0.65 + 0.25 * t);
            let k = (j * res + i) * 4;
            out[k] = (r * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 1] = (g * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 2] = (b * shade * 255.0).clamp(0.0, 255.0) as u8;
            out[k + 3] = 255;
        }
    }
    out
}

/// Bake + erode a preset composition (no water yet).
fn bake_preset(preset: &str, seed: u64, resolution: u32) -> (TerrainParams, Heightfield) {
    let mut params = if preset == "island" {
        TerrainParams {
            seed,
            resolution,
            size: 128.0,
            ..Default::default()
        }
    } else {
        let mut p = TerrainParams::cordillera(seed);
        p.resolution = resolution;
        p
    };
    params.seed = seed;
    params.resolution = resolution;
    let mut field = TerrainGraph::new(params.clone()).bake();
    erode_thermal(&mut field, &ErosionParams::default());
    erode_hydraulic(&mut field, &ErosionParams::default(), seed ^ 0xE0DE);
    velvet_terrain::smooth(&mut field, 2);
    // Hydrology first: artifact pits learn to drain (streams, not pond
    // soup); the dug lake disc is preserved as the terminal basin.
    velvet_terrain::fill_depressions(&mut field, lake_preserve(&params));
    (params, field)
}

/// Shallow water over a bedrock field, pre-rolled with rain.
///
/// The sim runs at its own (lower) resolution: bedrock is downsampled
/// from the render field, which cuts per-frame CPU ~4x with no visible
/// difference on a smooth surface. Open rims let rivers leave the map;
/// without them any rain becomes a global swamp on flat land.
/// Lake preserve disc for depression filling (world center + radius).
fn lake_preserve(params: &TerrainParams) -> Option<([f32; 2], f32)> {
    if params.lake_radius > 0.0 {
        Some((
            [
                params.lake_center[0] * params.size,
                params.lake_center[1] * params.size,
            ],
            params.lake_radius * params.size * 1.3,
        ))
    } else {
        None
    }
}

fn make_water(
    field: &Heightfield,
    preroll: u32,
    water_res: u32,
    rain: f32,
    preserve: Option<([f32; 2], f32)>,
) -> ShallowWater {
    let wres = water_res.clamp(9, 513) as usize;
    let mut bedrock = Vec::with_capacity(wres * wres);
    for j in 0..wres {
        for i in 0..wres {
            let x = (i as f32 / (wres - 1) as f32 - 0.5) * field.size;
            let z = (j as f32 / (wres - 1) as f32 - 0.5) * field.size;
            bedrock.push(field.sample_world(x, z));
        }
    }
    // Downsampling re-quantizes micro-terraces that trap wall films which
    // never drain; fill the sim bed too (same lake preserve).
    let mut bed = Heightfield {
        resolution: wres as u32,
        size: field.size,
        heights: bedrock,
    };
    velvet_terrain::fill_depressions(&mut bed, preserve);
    let bedrock = bed.heights;
    let wparams = ShallowParams {
        resolution: wres as u32,
        size: field.size,
        open_rim: true,
        ..Default::default()
    };
    let mut water = ShallowWater::new(wparams, bedrock).expect("bedrock");
    // Optional initial rain (viewer passes 0: lake-only, no stray ponds).
    if rain > 0.0 {
        water.rain(rain);
    }
    for _ in 0..preroll {
        water.step();
    }
    water
}

/// Swash run-up field over the water grid: the long-wave part of the sea,
/// evaluated per node. Added to the sim depth before contouring, the
/// shoreline breathes in and out with the same waves that move the surface
/// (still-water contour = a stuck-on decal; this is the run-up).
///
/// `gain` converts offshore swell height to beach run-up (~1.4x).
fn runup_grid(
    water: &ShallowWater,
    swash: &WaveSpectrum,
    size: f32,
    t: f32,
    gain: f32,
) -> Vec<f32> {
    let n = water.resolution();
    let mut lift = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 / (n - 1) as f32 - 0.5) * size;
            let z = (j as f32 / (n - 1) as f32 - 0.5) * size;
            lift.push(swash.height(x, z, t) * gain);
        }
    }
    lift
}

/// Dynamic water paint over pristine terrain colors, every water tick:
/// beach wet envelope + wash-foam tongue at the run-up edge, plus damp
/// rock wherever the sim holds a film (streams, storm runoff). Thin films
/// darken the rock instead of turning it blue: that is what kills the
/// stuck-on layer look on slopes.
fn paint_shore(
    verts: &mut [MeshVertex3D],
    base: &[[f32; 4]],
    water: &ShallowWater,
    size: f32,
    water_level: f32,
    edge_lift: f32,
    seed: u64,
) {
    let n = water.resolution();
    let film_at = |x: f32, z: f32| -> f32 {
        let fx = ((x / size + 0.5) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let fz = ((z / size + 0.5) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
        let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(n - 1), (j0 + 1).min(n - 1));
        let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
        let d = &water.depth;
        let a = d[j0 * n + i0];
        let b = d[j0 * n + i1];
        let c = d[j1 * n + i0];
        let e = d[j1 * n + i1];
        a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + e * tx * tz
    };
    let edge = water_level + edge_lift;
    for (idx, v) in verts.iter_mut().enumerate() {
        let h = v.position[1];
        let film = film_at(v.position[0], v.position[2]);
        let near_shore = h < water_level + 3.0 && h > water_level - 4.0;
        if !near_shore && film < 0.004 {
            continue;
        }
        let mut col = [base[idx][0], base[idx][1], base[idx][2]];
        if near_shore {
            // Wet envelope: everything the swash can reach stays damped dark.
            let wet = 1.0 - smoothstep01((h - (water_level - 0.6)) / 2.8);
            col[0] *= 1.0 - 0.38 * wet;
            col[1] *= 1.0 - 0.38 * wet;
            col[2] *= 1.0 - 0.34 * wet;
            // Wash tongue: foam band around the moving edge, lace breakup.
            if h > water_level - 1.5 && h < edge + 0.6 {
                let band = 1.0 - ((h - edge).abs() / 0.55).clamp(0.0, 1.0);
                let breakup = smoothstep01(
                    (hash01((idx as u32).wrapping_mul(2654435761) ^ (seed as u32)) - 0.32) / 0.45,
                );
                let foam = (band * breakup).clamp(0.0, 1.0) * 0.9;
                col[0] += (0.93 - col[0]) * foam;
                col[1] += (0.95 - col[1]) * foam;
                col[2] += (0.95 - col[2]) * foam;
            }
        }
        // Damp rock under any sim film (fades where pools go deep: the
        // water mesh itself covers those).
        if film >= 0.004 {
            let damp = (1.0 - smoothstep01(film / 0.4)) * 0.35;
            col[0] *= 1.0 - damp;
            col[1] *= 1.0 - damp;
            col[2] *= 1.0 - damp * 0.9;
        }
        v.color = [
            col[0].clamp(0.0, 1.2),
            col[1].clamp(0.0, 1.2),
            col[2].clamp(0.0, 1.2),
            1.0,
        ];
    }
}

/// Viewer water-cycle rates (per 30 Hz sim step). Storms (rain well above
/// evaporation) grow visible streams; sunny lulls clear films. Evaporation
/// anti-correlates with rain, like sun after a storm; open water carries a
/// small potential-ET edge over soil. Calibrated in
/// `water_cycle_holds_lake_and_spares_the_plain`.
const CYCLE_RAIN: f32 = 0.0003;
const CYCLE_EVAP: f32 = 0.0004;
const CYCLE_OPEN: f32 = 0.0001;
const CYCLE_DEEP: f32 = 0.4;
const STORM_PERIOD: f32 = 120.0;

/// Storm factor 0.15 (lull) .. 1.0 (peak), period 120 s. Shared by the
/// live loop and the calibration test (t in seconds).
fn storm(t: f32) -> f32 {
    0.15 + 0.85 * (0.5 + 0.5 * (std::f32::consts::TAU * t / STORM_PERIOD).sin())
}

/// Fill ONLY the dug lake: cells near `center` (world units) whose surface
/// sits below `level` are topped up to it. Global `fill_to_level` also
/// floods every erosion pit below the water table (satellite ponds).
fn fill_lake(water: &mut ShallowWater, size: f32, center: [f32; 2], radius: f32, level: f32) {
    let n = water.resolution();
    let surf = water.surface();
    for j in 0..n {
        for i in 0..n {
            let x = (i as f32 / (n - 1) as f32 - 0.5) * size;
            let z = (j as f32 / (n - 1) as f32 - 0.5) * size;
            let d = ((x - center[0]).powi(2) + (z - center[1]).powi(2)).sqrt();
            if d < radius {
                let k = j * n + i;
                if surf[k] < level {
                    water.depth[k] += level - surf[k];
                }
            }
        }
    }
}

/// Full headless pipeline: artifacts + stats (CI-friendly).
fn run_headless(
    preset: &str,
    seed: u64,
    resolution: u32,
    water_steps: u32,
    out: &Path,
    no_sph: bool,
) {
    let t0 = Instant::now();
    std::fs::create_dir_all(out).expect("out dir");
    let (params, mut field) = bake_preset(preset, seed, resolution);
    let (tlo, thi) = field.range();
    println!("terrain [{preset}]: res={resolution} range=[{tlo:.1}, {thi:.1}]");

    let preserve = lake_preserve(&params);
    let mut water = make_water(&field, water_steps, resolution / 2, 0.05, preserve);
    // Lakes fill instantly to the water table; rain + rivers do the rest.
    if params.lake_radius > 0.0 {
        water.fill_to_level(params.water_level);
    }
    println!(
        "water: volume={:.1} maxdepth={:.2}",
        water.volume(),
        water.depth.iter().cloned().fold(0.0f32, f32::max)
    );
    // Couple back: steer droplets with the simulated flow field.
    erode_hydraulic_with_flow(
        &mut field,
        &ErosionParams {
            droplets: 800,
            ..Default::default()
        },
        seed ^ 0xF10E,
        0.5,
        &|x, z| water.velocity_world(x, z),
    );

    let sph_tris = if no_sph {
        0
    } else {
        let mut sph = SphSim::dam_break(SphParams::default(), 8, 8, 6, 0.5);
        for _ in 0..60 {
            sph.step();
        }
        let mesh = sph.surface_mesh(0.5, 0.5);
        println!(
            "sph: {} particles com=[{:.1}, {:.1}, {:.1}] surface_tris={}",
            sph.len(),
            sph.center_of_mass()[0],
            sph.center_of_mass()[1],
            sph.center_of_mass()[2],
            mesh.triangle_count()
        );
        mesh.triangle_count()
    };

    let mut terrain_mesh = build_mesh(&field);
    paint_biomes(&mut terrain_mesh, &field, 0.0, seed ^ 0xB10);
    let mut seen = [false; velvet_terrain::BIOME_COUNT];
    for b in &terrain_mesh.biomes {
        seen[*b as usize] = true;
    }
    println!(
        "mesh: {} verts {} tris biomes={:?}",
        terrain_mesh.vertex_count(),
        terrain_mesh.triangle_count(),
        seen
    );
    write_png(
        &out.join("terrain.png"),
        field.resolution,
        velvet_terrain::preview_rgba(&field),
    );
    let surface = water.surface_field();
    write_png(
        &out.join("water.png"),
        surface.resolution,
        water_preview(&surface),
    );

    let scene = terrain_scene_doc("terraflow", &field, params.water_level);
    velvet_scene3d::validate_scene(&scene).expect("scene validates");
    std::fs::write(
        out.join("terraflow.scene3d.json"),
        serde_json::to_string_pretty(&scene).expect("json"),
    )
    .expect("write scene");
    println!(
        "done in {:.1}s -> {} (sph_tris={sph_tris})",
        t0.elapsed().as_secs_f64(),
        out.display()
    );
}

/// Two-node scene document: terrain heightfield + a flat still-water
/// plane at the lake level (simple, honest, no shimmer tricks).
fn terrain_scene_doc(name: &str, field: &Heightfield, water_level: f32) -> velvet_scene3d::Scene3D {
    let terrain = velvet_scene3d::SceneNode3D {
        id: "terrain".to_string(),
        mesh: velvet_scene3d::MeshKind3D::Heightfield {
            resolution: field.resolution,
            size: field.size,
            heights: field.heights.clone(),
        },
        transform: Default::default(),
        material: velvet_scene3d::MaterialDoc {
            base_color: [0.35, 0.5, 0.28, 1.0],
            ..Default::default()
        },
        cast_shadow: true,
        collider: true,
        disabled: false,
        children: Vec::new(),
    };
    let water = velvet_scene3d::SceneNode3D {
        id: "water".to_string(),
        mesh: velvet_scene3d::MeshKind3D::Plane,
        transform: velvet_scene3d::TransformDoc {
            position: [0.0, water_level + 0.05, 0.0],
            scale: [field.size, 1.0, field.size],
            ..Default::default()
        },
        material: velvet_scene3d::MaterialDoc {
            base_color: [0.16, 0.42, 0.72, 1.0],
            ..Default::default()
        },
        cast_shadow: false,
        collider: false,
        disabled: false,
        children: Vec::new(),
    };
    velvet_scene3d::Scene3D {
        name: name.to_string(),
        nodes: vec![terrain, water],
        camera: Default::default(),
        lights: Vec::new(),
        environment: Default::default(),
    }
}

/// Fixed sun for baked CPU lighting (matches the preview rasterizer).
const SUN_DIR: [f32; 3] = [-0.42, 0.68, 0.26];

/// Deterministic 0..1 hash (strata tones, snow sparkle).
fn hash01(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x as f32) / (u32::MAX as f32)
}

/// Biome palette: lakebed, sand, grass, rock, snow.
const TERRAIN_PALETTE: [[f32; 3]; 5] = [
    [0.52, 0.47, 0.33],
    [0.76, 0.63, 0.42],
    [0.29, 0.49, 0.23],
    [0.42, 0.39, 0.35],
    [0.93, 0.94, 0.95],
];

/// Terrain mesh with baked photoreal-ish detail (same layout as the bridge
/// upload, so `update_mesh_vertices` accepts it).
///
/// Per vertex: jittered biome weights (no banding), macro variation,
/// multiscale cavity AO + edge wear, blue-shadowed snow by sun-facing,
/// rock strata bands, and a wet + foam shoreline band. All CPU-baked.
fn colored_terrain_mesh(
    field: &Heightfield,
    water_level: f32,
    seed: u64,
    apply_shore: bool,
) -> Mesh3D {
    use velvet_terrain::{classify, Fbm};
    let mut mesh = Mesh3D::heightfield(
        field.resolution as usize,
        field.size,
        &field.heights,
        Default::default(),
    );
    let res = field.resolution as usize;
    let (_, hi_raw) = field.range();
    let (lo_raw, _) = field.range();
    let relief = (hi_raw - lo_raw).max(1.0);
    let cavity = velvet_terrain::cavity_map(field);
    let detail = Fbm::new(seed ^ 0xDE7A11, 2, 2.0, 0.5);
    let macro_n = Fbm::new(seed ^ 0xAC207, 3, 2.0, 0.5);
    let moist = Fbm::new(seed ^ 0xB0157, 3, 2.0, 0.5);
    let sun = normalized(SUN_DIR);
    for (v_idx, v) in mesh.vertices.iter_mut().enumerate() {
        let i = v_idx % res;
        let j = v_idx / res;
        let x = (i as f32 / (res - 1) as f32 - 0.5) * field.size;
        let z = (j as f32 / (res - 1) as f32 - 0.5) * field.size;
        let h = v.position[1];
        let slope = ((v.normal[0] * v.normal[0] + v.normal[2] * v.normal[2]).sqrt()
            / v.normal[1].max(0.05))
        .min(3.0);
        // Jittered classification kills banding.
        let jn = detail.sample(x * 0.06, z * 0.06);
        let m = moist.sample(x * 0.02 + 9.1, z * 0.02 - 4.7) * 0.5 + 0.5;
        let w = classify(h + jn * 3.0, slope, m, 0.0, relief);
        let mut col = [0.0f32; 3];
        for (b, weight) in w.weights.iter().enumerate() {
            let c = TERRAIN_PALETTE[b.min(4)];
            col[0] += c[0] * weight;
            col[1] += c[1] * weight;
            col[2] += c[2] * weight;
        }
        // Macro variation patches (multiply, never overlay).
        let mv = macro_n.sample(x * 0.008, z * 0.008) * 0.5 + 0.5;
        let mm = 0.82 + 0.36 * mv;
        col[0] *= mm;
        col[1] *= mm;
        col[2] *= mm;
        // Cavity AO + edge wear.
        let cav = cavity[v_idx].clamp(-1.0, 1.0);
        let ao = (1.0 - 0.55 * cav.max(0.0)).clamp(0.45, 1.0);
        let wear = 1.0 + 0.18 * (-cav).clamp(0.0, 1.0);
        // Snow: blue shadow by sun-facing + sparkle speckle.
        let snow_w = w.weights[4];
        if snow_w > 0.01 {
            let sun_face = (v.normal[0] * sun[0] + v.normal[1] * sun[1] + v.normal[2] * sun[2])
                .clamp(0.0, 1.0);
            let lit = [1.0, 0.98, 0.94];
            let shadow = [0.62, 0.72, 0.90];
            let sparkle = if hash01((v_idx as u32).wrapping_mul(2654435761)) > 0.97 {
                1.07
            } else {
                1.0
            };
            col[0] = col[0] * (1.0 - snow_w)
                + (shadow[0] + (lit[0] - shadow[0]) * sun_face) * snow_w * sparkle;
            col[1] = col[1] * (1.0 - snow_w)
                + (shadow[1] + (lit[1] - shadow[1]) * sun_face) * snow_w * sparkle;
            col[2] = col[2] * (1.0 - snow_w)
                + (shadow[2] + (lit[2] - shadow[2]) * sun_face) * snow_w * sparkle;
        }
        // Rock strata bands + wet shoreline + foam waterline.
        let rock_w = w.weights[3];
        if rock_w > 0.01 {
            let band = ((h + jn * 4.0) / 14.0).floor() as u32;
            let tone = 0.85 + 0.3 * hash01(band.wrapping_mul(2246822519));
            let edge = {
                let f = ((h + jn * 4.0) / 14.0).fract().abs();
                (0.08 - f.min(1.0 - f)).max(0.0) / 0.08
            };
            let strata = (tone * (1.0 - 0.25 * edge)).clamp(0.7, 1.2);
            col[0] = col[0] * (1.0 - rock_w) + col[0] * strata * rock_w;
            col[1] = col[1] * (1.0 - rock_w) + col[1] * strata * rock_w;
            col[2] = col[2] * (1.0 - rock_w) + col[2] * strata * rock_w;
        }
        // Static shore paint lives in `paint_shore` (dynamic, per water
        // tick); the pristine path skips it so the beach can answer waves.
        if apply_shore {
            let above = h - water_level;
            if above < 1.2 {
                // Wet darkening + foam waterline with breakup.
                let wet = (1.0 - (above / 1.2).clamp(0.0, 1.0)) * 0.45;
                col[0] *= 1.0 - wet;
                col[1] *= 1.0 - wet;
                col[2] *= 1.0 - wet * 0.9;
                let foam_band = 1.0 - (above.abs() / 0.45).clamp(0.0, 1.0);
                let breakup = smoothstep01((jn * 0.5 + 0.5 - 0.35) / 0.4);
                let foam = (foam_band * breakup).clamp(0.0, 1.0) * 0.85;
                col[0] += (0.92 - col[0]) * foam;
                col[1] += (0.94 - col[1]) * foam;
                col[2] += (0.94 - col[2]) * foam;
            }
        }
        v.color = [
            (col[0] * ao * wear).clamp(0.0, 1.2),
            (col[1] * ao * wear).clamp(0.0, 1.2),
            (col[2] * ao * wear).clamp(0.0, 1.2),
            1.0,
        ];
    }
    mesh
}

fn smoothstep01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blur a grid copy (presentation only: smooths the contour isoline).
fn blur_grid(src: &[f32], n: usize) -> Vec<f32> {
    let mut out = src.to_vec();
    for j in 1..n - 1 {
        for i in 1..n - 1 {
            let k = j * n + i;
            out[k] = (src[k] * 4.0 + src[k - 1] + src[k + 1] + src[k - n] + src[k + n]) / 8.0;
        }
    }
    out
}

/// Living lake mesh: marching-squares shoreline from the sim depth field,
/// spectral wave displacement (visual only), depth tint + Fresnel sky
/// reflection + sun glitter + foam. Same winding as the terrain mesher.
fn colored_water_mesh(
    water: &ShallowWater,
    field: &Heightfield,
    spectrum: &WaveSpectrum,
    lift: &[f32],
    time: f32,
    cam: [f32; 3],
) -> Mesh3D {
    use velvet_liquid::{contour_triangles, foam_mask};
    use velvet_render::MeshVertex3D;
    let n = water.resolution();
    if n < 2 {
        return Mesh3D {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: Default::default(),
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        };
    }
    // Visible films start here; thinner sheets only darken the rock
    // (damp in `paint_shore`). A 0.06 film on a high bench reads as a
    // sticker; real puddles need a little more body to go blue.
    const FILM: f32 = 0.09;
    // The shoreline is the sim's own doing: rain falls, water flows
    // downhill, pools in the lake, drains at the rims. No presentation lift
    // in the iso field (that reads as a sticker flood on slopes).
    let smooth = blur_grid(&water.depth, n);
    let tris = contour_triangles(&smooth, n, FILM);
    if tris.is_empty() {
        return Mesh3D {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: Default::default(),
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        };
    }
    let size = field.size;
    let surf = water.surface();
    let foam = foam_mask(water);
    let grid_at = |vals: &[f32], gx: f32, gz: f32| -> f32 {
        let fx = gx.clamp(0.0, (n - 1) as f32);
        let fz = gz.clamp(0.0, (n - 1) as f32);
        let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(n - 1), (j0 + 1).min(n - 1));
        let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
        let a = vals[j0 * n + i0];
        let b = vals[j0 * n + i1];
        let c = vals[j1 * n + i0];
        let d = vals[j1 * n + i1];
        a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + d * tx * tz
    };
    let sun = normalized(SUN_DIR);
    let deep = [0.03, 0.25, 0.50];
    let shallow = [0.30, 0.60, 0.70];
    let sky = [0.60, 0.74, 0.88];
    let mut vertices = Vec::with_capacity(tris.len() * 3);
    let mut indices = Vec::with_capacity(tris.len() * 3);
    // Bedrock slope sampling (finer terrain field).
    let fres = (field.resolution as f32 - 1.0).max(1.0);
    let st = field.size / fres;
    let slope_at = |x: f32, z: f32| -> f32 {
        let fi = ((x / field.size + 0.5) * fres).round() as i32;
        let fj = ((z / field.size + 0.5) * fres).round() as i32;
        let bx = (field.at(fi + 1, fj) - field.at(fi - 1, fj)) / (2.0 * st);
        let bz = (field.at(fi, fj + 1) - field.at(fi, fj - 1)) / (2.0 * st);
        (bx * bx + bz * bz).sqrt()
    };
    for tri in &tris {
        // Cliff films never drain in the sim's eye, but drawing them pastes
        // stickers on walls: a triangle that is ALL cliff + thin is water
        // that should have fallen already. Damp rock below tells the story.
        let clinging: Vec<(f32, f32)> = tri
            .iter()
            .map(|corner| {
                let x = (corner[0] / (n - 1) as f32 - 0.5) * size;
                let z = (corner[1] / (n - 1) as f32 - 0.5) * size;
                (
                    slope_at(x, z),
                    grid_at(&water.depth, corner[0], corner[1]).max(0.0),
                )
            })
            .collect();
        if clinging.iter().all(|(s, d)| *s > 0.9 && *d < 1.0) {
            continue;
        }
        for (corner, (slope, depth)) in tri.iter().zip(clinging.iter()) {
            let (gx, gz) = (corner[0], corner[1]);
            let (slope, depth) = (*slope, *depth);
            let x = (gx / (n - 1) as f32 - 0.5) * size;
            let z = (gz / (n - 1) as f32 - 0.5) * size;
            let base_y = grid_at(&surf, gx, gz);
            // Shoaling remnant: full waves offshore, ~30% wash survives at
            // the waterline (pinning it to zero is what looked stuck-on).
            let chop_scale = 0.30 + 0.70 * smoothstep01(depth / 1.5);
            // Up-rush foam: swash lift pushes white water up the beach.
            let uprush = grid_at(lift, gx, gz).clamp(0.0, 1.0);
            // Mirror film: thin sheets on gentle ground mirror the sky
            // instead of going saturated blue (puddle, not sticker).
            let mirror =
                (1.0 - smoothstep01(depth / 0.30)) * (1.0 - smoothstep01((slope - 0.15) / 0.25));
            let g = spectrum.displace(x, z, time);
            let px = x + g[0] * chop_scale;
            let pz = z + g[2] * chop_scale;
            let py = base_y + g[1] * chop_scale;
            // Analytic spectral gradient -> normal (exact, no FD shimmer),
            // flattened where the film mirrors.
            let sg = spectrum.gradient(x, z, time);
            let wobble = 1.0 - mirror * 0.9;
            let normal = normalized([
                -sg[0] * chop_scale * wobble,
                1.0,
                -sg[1] * chop_scale * wobble,
            ]);
            // Depth tint -> mirror-sky -> Fresnel sky -> sun glitter.
            let dt = smoothstep01(depth / 6.0);
            let mut col = [
                shallow[0] + (deep[0] - shallow[0]) * dt,
                shallow[1] + (deep[1] - shallow[1]) * dt,
                shallow[2] + (deep[2] - shallow[2]) * dt,
            ];
            col[0] += (sky[0] - col[0]) * mirror * 0.75;
            col[1] += (sky[1] - col[1]) * mirror * 0.75;
            col[2] += (sky[2] - col[2]) * mirror * 0.75;
            let vx = cam[0] - px;
            let vy = cam[1] - py;
            let vz = cam[2] - pz;
            let vl = (vx * vx + vy * vy + vz * vz).sqrt().max(1e-6);
            let ndv = ((normal[0] * vx + normal[1] * vy + normal[2] * vz) / vl).clamp(0.0, 1.0);
            let fres = 0.10 + 0.90 * (1.0 - ndv).powi(5);
            col[0] += (sky[0] - col[0]) * fres;
            col[1] += (sky[1] - col[1]) * fres;
            col[2] += (sky[2] - col[2]) * fres;
            let hx2 = sun[0] + vx / vl;
            let hy2 = sun[1] + vy / vl;
            let hz2 = sun[2] + vz / vl;
            let hl = (hx2 * hx2 + hy2 * hy2 + hz2 * hz2).sqrt().max(1e-6);
            let ndh = ((normal[0] * hx2 + normal[1] * hy2 + normal[2] * hz2) / hl).max(0.0);
            let spec = ndh.powi(90) * 1.6 + ndh.powi(8) * 0.12;
            // Whitewater: films on steep bedrock are torrents, not ponds.
            // Only truly deep pools stay blue on slopes (a gorge lake is
            // still a lake); mirror films are already sky, not blue.
            let torrent = smoothstep01((slope - 0.30) / 0.5) * (1.0 - smoothstep01(depth / 3.0));
            // Foam: sim mask + shoreline lap + up-rush, broken up by hash.
            let fm = grid_at(&foam, gx, gz).clamp(0.0, 1.0);
            let shore = 1.0 - smoothstep01(depth / 0.5);
            // World-space lace (`as u32` saturates negatives, hence offset).
            let breakup = hash01(
                (((px + 4096.0) * 1.7) as u32)
                    .wrapping_mul(374761393)
                    .wrapping_add((((pz + 4096.0) * 1.7) as u32).wrapping_mul(668265263)),
            );
            // Lace breakup gates shore/sim foam; whitewater is continuous
            // and bypasses it (a torrent has no blue holes).
            let lace = ((fm * 0.9 + shore * (0.7 + 0.6 * uprush))
                * smoothstep01((breakup - 0.25) / 0.55))
            .clamp(0.0, 1.0);
            let foam_w = (lace + torrent * 0.95).clamp(0.0, 1.0) * 0.9;
            col[0] = (col[0] + spec + (0.92 - col[0]) * foam_w).clamp(0.0, 1.2);
            col[1] = (col[1] + spec + (0.94 - col[1]) * foam_w).clamp(0.0, 1.2);
            col[2] = (col[2] + spec * 0.9 + (0.94 - col[2]) * foam_w).clamp(0.0, 1.2);
            indices.push(vertices.len() as u32);
            vertices.push(MeshVertex3D {
                position: [px, py, pz],
                normal,
                uv: [0.0, 0.0],
                color: [col[0], col[1], col[2], 1.0],
            });
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Default::default(),
        metallic_factor: 0.0,
        roughness_factor: 1.0,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

fn normalized(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / len, v[1] / len, v[2] / len]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = arg_value(&args, "--seed", "7").parse().unwrap_or(7);
    let resolution: u32 = arg_value(&args, "--resolution", "257")
        .parse()
        .unwrap_or(257);
    let preset = arg_value(&args, "--preset", "cordillera");
    if args.iter().any(|a| a == "--headless") {
        let water_steps: u32 = arg_value(&args, "--steps-water", "120")
            .parse()
            .unwrap_or(120);
        let no_sph = args.iter().any(|a| a == "--no-sph");
        let out = std::path::PathBuf::from(arg_value(&args, "--out", "terraflow_out"));
        run_headless(&preset, seed, resolution, water_steps, &out, no_sph);
        return;
    }
    if let Err(e) = run_windowed(&preset, seed, resolution) {
        eprintln!("viewer error: {e:#}");
        std::process::exit(1);
    }
}

// ─── Windowed 3D viewer ───────────────────────────────────────────────────

struct Viewer {
    preset: String,
    seed: u64,
    res: u32,
    water_level: f32,
    field: Heightfield,
    water: ShallowWater,
    spectrum: WaveSpectrum,
    swash: WaveSpectrum,
    terrain_base: Vec<MeshVertex3D>,
    flat: Vec<FlatNode3D>,
    camera: Camera3D,
    lighting: Lighting3D,
    yaw: f32,
    pitch: f32,
    dist: f32,
    target_x: f32,
    target_y: f32,
    target_z: f32,
    keys: HashSet<KeyCode>,
    time: f32,
    water_tris: usize,
    paused: bool,
    water_accum: f32,
    mesh_cooldown: f32,
    shore_cooldown: f32,
    frames: u64,
    fps_time: Instant,
    fps: u32,
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    renderer: Option<Wgpu3DRenderer>,
    cache: SceneMeshCache,
    root: PathBuf,
    last: Instant,
}

fn run_windowed(preset: &str, seed: u64, resolution: u32) -> anyhow::Result<()> {
    let (params, field) = bake_preset(preset, seed, resolution.clamp(17, 257));
    // Rain falls live in the loop below; the bed starts drained.
    let mut water = make_water(
        &field,
        150,
        field.resolution / 2,
        0.0,
        lake_preserve(&params),
    );
    if params.lake_radius > 0.0 {
        fill_lake(
            &mut water,
            field.size,
            [
                params.lake_center[0] * params.size,
                params.lake_center[1] * params.size,
            ],
            params.lake_radius * params.size * 1.15,
            params.water_level,
        );
    }
    // Spectral wind-sea: longest carrier fits the lake, crest reads at
    // 100+ units of camera distance. The swash twin keeps only the long
    // carriers: smooth run-up lift, no chop jitter on the waterline.
    let spectrum = WaveSpectrum::wind_sea(seed ^ 0x5EA7, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
    let swash = WaveSpectrum::wind_sea(seed ^ 0x5A57, [1.0, 0.35], 7.0, 30.0, 0.55, 8);
    let scene = terrain_scene_doc("terraflow", &field, params.water_level);
    velvet_scene3d::validate_scene(&scene).map_err(|e| anyhow::anyhow!("{e}"))?;
    let flat = flatten_scene(&scene);
    // Default framing: look at the lake, mountains behind.
    let dist = field.size * 0.45;
    let mut app = Viewer {
        preset: preset.to_string(),
        seed,
        res: field.resolution,
        water_level: params.water_level,
        field,
        water,
        spectrum,
        swash,
        terrain_base: Vec::new(),
        flat,
        camera: Camera3D::default(),
        lighting: Lighting3D::default(),
        yaw: 0.7,
        pitch: 0.75,
        dist,
        target_x: params.lake_center[0] * params.size,
        target_y: params.water_level + 4.0,
        target_z: params.lake_center[1] * params.size,
        keys: HashSet::new(),
        time: 0.0,
        water_tris: 0,
        paused: false,
        water_accum: 0.0,
        mesh_cooldown: 0.0,
        shore_cooldown: 0.0,
        frames: 0,
        fps_time: Instant::now(),
        fps: 0,
        window: None,
        device: None,
        queue: None,
        surface: None,
        config: None,
        renderer: None,
        cache: SceneMeshCache::default(),
        root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        last: Instant::now(),
    };
    app.place_camera();
    let event_loop = EventLoop::new().map_err(|e| anyhow::anyhow!("{e}"))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    use winit::error::EventLoopError;
    match event_loop.run_app(&mut app) {
        Ok(()) => Ok(()),
        Err(EventLoopError::ExitFailure(code)) => Err(anyhow::anyhow!("exit {code}")),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

impl Viewer {
    fn place_camera(&mut self) {
        // Depth range follows world scale (default far=200 clips islands).
        let size = self.field.size;
        self.camera.near = (size * 0.002).max(0.05);
        self.camera.far = size * 8.0;
        self.dist = self.dist.clamp(20.0, size * 3.0);
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.camera.target = Vec3::new(self.target_x, self.target_y, self.target_z);
        self.camera.position = Vec3::new(
            self.target_x + self.dist * cp * sy,
            self.target_y + self.dist * sp,
            self.target_z + self.dist * cp * cy,
        );
    }

    fn regenerate(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        let preset = self.preset.clone();
        let (params, field) = bake_preset(&preset, self.seed, self.res);
        self.water_level = params.water_level;
        self.target_x = params.lake_center[0] * params.size;
        self.target_y = params.water_level + 4.0;
        self.target_z = params.lake_center[1] * params.size;
        self.field = field;
        let wres = self.field.resolution / 2;
        let preserve = lake_preserve(&params);
        self.water = make_water(&self.field, 150, wres, 0.0, preserve);
        self.spectrum = WaveSpectrum::wind_sea(self.seed ^ 0x5EA7, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
        self.swash = WaveSpectrum::wind_sea(self.seed ^ 0x5A57, [1.0, 0.35], 7.0, 30.0, 0.55, 8);
        if params.lake_radius > 0.0 {
            let size = self.field.size;
            fill_lake(
                &mut self.water,
                size,
                [
                    params.lake_center[0] * params.size,
                    params.lake_center[1] * params.size,
                ],
                params.lake_radius * params.size * 1.15,
                params.water_level,
            );
        }
        self.push_meshes();
    }

    /// Swash edge lift at the camera focus (drives the terrain wash tongue
    /// with the same phase family as the moving waterline).
    fn edge_lift(&self) -> f32 {
        self.swash.height(self.target_x, self.target_z, self.time) * 1.4
    }

    /// Refresh the terrain mesh from CPU state.
    fn push_meshes(&mut self) {
        let water_level = self.water_level;
        let seed = self.seed;
        let time = self.time;
        let cam = self.camera.position;
        let cam_pos = [cam.x, cam.y, cam.z];
        let size = self.field.size;
        let lift = runup_grid(&self.water, &self.swash, size, time, 1.4);
        let edge = self.edge_lift();
        // Pristine terrain (no baked shore); wash paints a working copy so
        // the stored base never accumulates stale foam.
        let terrain = colored_terrain_mesh(&self.field, water_level, seed, false);
        self.terrain_base = terrain.vertices.clone();
        let mut verts = terrain.vertices;
        let base: Vec<[f32; 4]> = self.terrain_base.iter().map(|v| v.color).collect();
        paint_shore(
            &mut verts,
            &base,
            &self.water,
            size,
            water_level,
            edge,
            seed,
        );
        let terrain_verts = verts;
        if let (Some(queue), Some(renderer), Some(device)) =
            (&self.queue, &mut self.renderer, &self.device)
        {
            if let Some((handle, _)) = self.cache.get("terrain") {
                let _ = renderer.update_mesh_vertices(queue, handle, &terrain_verts);
            }
            let water_mesh = colored_water_mesh(
                &self.water,
                &self.field,
                &self.spectrum,
                &lift,
                time,
                cam_pos,
            );
            if !water_mesh.vertices.is_empty() && !water_mesh.indices.is_empty() {
                if let Some((handle, _)) = self.cache.get("water") {
                    let _ = renderer.replace_mesh(device, handle, &water_mesh);
                }
            }
        }
    }

    /// Water-only refresh (per mesh tick) + shore wash repaint.
    fn push_meshes_water(&mut self) {
        let time = self.time;
        let cam = self.camera.position;
        let water_level = self.water_level;
        let seed = self.seed;
        let size = self.field.size;
        let lift = runup_grid(&self.water, &self.swash, size, time, 1.4);
        let edge = self.edge_lift();
        if let (Some(queue), Some(renderer), Some(device)) =
            (&self.queue, &mut self.renderer, &self.device)
        {
            let mesh = colored_water_mesh(
                &self.water,
                &self.field,
                &self.spectrum,
                &lift,
                time,
                [cam.x, cam.y, cam.z],
            );
            if !mesh.vertices.is_empty() && !mesh.indices.is_empty() {
                self.water_tris = mesh.indices.len() / 3;
                if let Some((handle, _)) = self.cache.get("water") {
                    let _ = renderer.replace_mesh(device, handle, &mesh);
                }
            }
            // Shore repaint rides along only while the base exists and the
            // shore tick fired (see step_and_render).
            if self.shore_cooldown <= 0.0 && !self.terrain_base.is_empty() {
                self.shore_cooldown = 0.5;
                let mut verts = self.terrain_base.clone();
                let base: Vec<[f32; 4]> = verts.iter().map(|v| v.color).collect();
                paint_shore(
                    &mut verts,
                    &base,
                    &self.water,
                    size,
                    water_level,
                    edge,
                    seed,
                );
                if let Some((handle, _)) = self.cache.get("terrain") {
                    let _ = renderer.update_mesh_vertices(queue, handle, &verts);
                }
            }
        }
    }

    fn step_and_render(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.time += dt;
        // Orbit / zoom.
        let rot = 1.6 * dt;
        if self.key_down(KeyCode::ArrowLeft) || self.key_down(KeyCode::KeyA) {
            self.yaw -= rot;
        }
        if self.key_down(KeyCode::ArrowRight) || self.key_down(KeyCode::KeyD) {
            self.yaw += rot;
        }
        if self.key_down(KeyCode::ArrowUp) || self.key_down(KeyCode::KeyW) {
            self.pitch = (self.pitch + rot * 0.7).min(1.45);
        }
        if self.key_down(KeyCode::ArrowDown) || self.key_down(KeyCode::KeyS) {
            self.pitch = (self.pitch - rot * 0.7).max(0.08);
        }
        if self.key_down(KeyCode::KeyQ) {
            self.dist *= 1.0 + dt;
        }
        if self.key_down(KeyCode::KeyE) {
            self.dist = (self.dist * (1.0 - dt)).max(20.0);
        }
        self.place_camera();
        // Live water: rain/evaporation cycle + sim at 30 Hz, contour mesh
        // at most every 0.12 s. Storms grow streams, lulls clear them.
        if !self.paused {
            self.water_accum += dt;
            while self.water_accum >= 1.0 / 30.0 {
                self.water_accum -= 1.0 / 30.0;
                let s = storm(self.time);
                self.water.rain(CYCLE_RAIN * s);
                self.water.evaporate(CYCLE_EVAP * (1.6 - s));
                self.water
                    .evaporate_open_water(CYCLE_OPEN * (1.6 - s), CYCLE_DEEP);
                self.water.step();
            }
            self.mesh_cooldown -= dt;
            self.shore_cooldown -= dt;
            if self.mesh_cooldown <= 0.0 {
                self.mesh_cooldown = 0.12;
                self.push_meshes_water();
            }
        }
        self.frames += 1;
        if self.fps_time.elapsed().as_secs_f32() >= 0.5 {
            self.fps = (self.frames as f32 / self.fps_time.elapsed().as_secs_f32()) as u32;
            self.frames = 0;
            self.fps_time = Instant::now();
        }
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "terraflow — seed={} {}fps {} wtris={} (R regen, Space water, Q/E zoom)",
                self.seed,
                self.fps,
                if self.paused { "paused" } else { "live" },
                self.water_tris,
            ));
            window.request_redraw();
        }
    }

    fn key_down(&self, code: KeyCode) -> bool {
        self.keys.contains(&code)
    }

    fn redraw(&mut self) {
        let (device, queue, surface, config, renderer) = match (
            &self.device,
            &self.queue,
            &self.surface,
            &self.config,
            &mut self.renderer,
        ) {
            (Some(d), Some(q), Some(s), Some(c), Some(r)) => (d, q, s, c, r),
            _ => return,
        };
        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(device, config);
                return;
            }
            Err(_) => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // The water mesh is built in world space, but the scene node it
        // replaces is a unit Plane with a scale baked into its world
        // matrix: neutralize it so the shoreline lands on the terrain.
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("water".to_string(), Mat4::IDENTITY);
        let draws = scene_draws(&self.cache, &self.flat, &overrides);
        let size = (config.width, config.height);
        renderer.render(
            device,
            queue,
            &view,
            size,
            self.camera,
            self.lighting,
            &draws,
            wgpu::Color {
                r: 0.35,
                g: 0.55,
                b: 0.75,
                a: 1.0,
            },
        );
        frame.present();
    }
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("terraflow")
                        .with_inner_size(LogicalSize::new(1280, 720)),
                )
                .expect("window"),
        );
        let (device, queue, surface, config) = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            let surface = instance.create_surface(window.clone()).expect("surface");
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .expect("adapter");
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("terraflow-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
                .expect("device");
            let size = window.inner_size();
            let capabilities = surface.get_capabilities(&adapter);
            let format = capabilities
                .formats
                .iter()
                .copied()
                .find(wgpu::TextureFormat::is_srgb)
                .unwrap_or(capabilities.formats[0]);
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: capabilities.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(&device, &config);
            (device, queue, surface, config)
        });
        let mut renderer = Wgpu3DRenderer::new(
            &device,
            &queue,
            config.format,
            (config.width, config.height),
        );
        self.cache = upload_scene_meshes(&mut renderer, &device, &self.root, &self.flat)
            .expect("upload terrain meshes");
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);
        self.config = Some(config);
        self.renderer = Some(renderer);
        self.window = Some(window);
        // Paint biome/water vertex colors over the plain bridge upload.
        self.push_meshes();
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape {
                        event_loop.exit();
                        return;
                    }
                    match event.state {
                        ElementState::Pressed => {
                            if code == KeyCode::KeyR {
                                self.regenerate();
                            }
                            if code == KeyCode::Space {
                                self.paused = !self.paused;
                            }
                            self.keys.insert(code);
                        }
                        ElementState::Released => {
                            self.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let (Some(device), Some(surface), Some(config)) =
                    (&self.device, &self.surface, &mut self.config)
                {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(device, config);
                }
            }
            WindowEvent::RedrawRequested => {
                self.step_and_render();
                self.redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_runs_small_and_writes_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("t");
        std::fs::create_dir_all(&out).unwrap();
        // Small deterministic run through the same stages as main().
        let params = TerrainParams {
            seed: 11,
            resolution: 17,
            size: 32.0,
            ..Default::default()
        };
        let mut field = TerrainGraph::new(params).bake();
        erode_thermal(&mut field, &ErosionParams::default());
        let wparams = ShallowParams {
            resolution: 17,
            size: 32.0,
            ..Default::default()
        };
        let mut water = ShallowWater::new(wparams, field.heights.clone()).unwrap();
        water.rain(0.4);
        for _ in 0..10 {
            water.step();
        }
        assert!(water.volume() > 0.0);
        write_png(
            &out.join("terrain.png"),
            field.resolution,
            velvet_terrain::preview_rgba(&field),
        );
        write_png(
            &out.join("water.png"),
            field.resolution,
            water_preview(&water.surface_field()),
        );
        assert!(out.join("terrain.png").exists());
        assert!(out.join("water.png").exists());
        // water.png decodes.
        let bytes = std::fs::read(out.join("water.png")).unwrap();
        let info = velvet_image::probe(&bytes).unwrap();
        assert_eq!((info.width, info.height), (17, 17));
    }

    #[test]
    fn viewer_lake_holds_water_and_contours() {
        // Same conditions as the windowed viewer (no rain, lake filled).
        let (params, field) = bake_preset("cordillera", 7, 129);
        let mut water = make_water(
            &field,
            150,
            field.resolution / 2,
            0.0,
            lake_preserve(&params),
        );
        assert!(params.lake_radius > 0.0);
        fill_lake(
            &mut water,
            field.size,
            [
                params.lake_center[0] * params.size,
                params.lake_center[1] * params.size,
            ],
            params.lake_radius * params.size * 1.15,
            params.water_level,
        );
        let v0 = water.volume();
        assert!(v0 > 100.0, "fill failed, vol={v0}");
        // ~1 min of live 30 Hz stepping: the lake must not drain away.
        for _ in 0..2000 {
            water.step();
        }
        let v1 = water.volume();
        eprintln!("lake volume after fill={v0:.0} after 2000 steps={v1:.0}");
        assert!(v1 > v0 * 0.5, "lake drained: {v0:.0} -> {v1:.0}");
        let spectrum = WaveSpectrum::wind_sea(7 ^ 0x5EA7, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
        let swash = WaveSpectrum::wind_sea(7 ^ 0x5A57, [1.0, 0.35], 7.0, 30.0, 0.55, 8);
        let lift = runup_grid(&water, &swash, field.size, 3.0, 1.4);
        let mesh = colored_water_mesh(&water, &field, &spectrum, &lift, 3.0, [0.0, 200.0, 200.0]);
        eprintln!(
            "water mesh verts={} tris={}",
            mesh.vertices.len(),
            mesh.indices.len() / 3
        );
        assert!(!mesh.vertices.is_empty(), "contour mesh is empty");
    }

    #[test]
    fn shoreline_breathes_with_swash() {
        // The waterline must move: lift fields at different phases differ,
        // and the wet-cell count changes with them.
        let (params, field) = bake_preset("cordillera", 7, 65);
        let mut water = make_water(
            &field,
            60,
            field.resolution / 2,
            0.0,
            lake_preserve(&params),
        );
        fill_lake(
            &mut water,
            field.size,
            [
                params.lake_center[0] * params.size,
                params.lake_center[1] * params.size,
            ],
            params.lake_radius * params.size * 1.15,
            params.water_level,
        );
        let swash = WaveSpectrum::wind_sea(7 ^ 0x5A57, [1.0, 0.35], 7.0, 30.0, 0.55, 8);
        let l0 = runup_grid(&water, &swash, field.size, 0.0, 1.4);
        let l1 = runup_grid(&water, &swash, field.size, 9.3, 1.4);
        let drift: f32 = l0
            .iter()
            .zip(l1.iter())
            .map(|(a, b)| (a - b).abs())
            .sum::<f32>()
            / l0.len() as f32;
        eprintln!("swash mean lift change={drift:.3}");
        assert!(drift > 0.05, "static run-up field");
        let wet_cells = |lift: &[f32]| {
            blur_grid(&water.depth, water.resolution())
                .iter()
                .zip(lift.iter())
                .filter(|(d, l)| *d + *l >= 0.06)
                .count()
        };
        let (c0, c1) = (wet_cells(&l0), wet_cells(&l1));
        eprintln!("wet cells {c0} -> {c1}");
        assert!(c0 != c1, "waterline does not move");
    }

    #[test]
    fn water_cycle_holds_lake_and_spares_the_plain() {
        // Two storm cycles: the lake must survive while the open plain
        // (measured at the end of a lull) stays below the visible film.
        let (params, field) = bake_preset("cordillera", 7, 65);
        let mut water = make_water(
            &field,
            60,
            field.resolution / 2,
            0.0,
            lake_preserve(&params),
        );
        let center = [
            params.lake_center[0] * params.size,
            params.lake_center[1] * params.size,
        ];
        let radius = params.lake_radius * params.size * 1.15;
        fill_lake(&mut water, field.size, center, radius, params.water_level);
        let v0 = water.volume();
        // End at the bottom of a lull (1.75 periods): films must have
        // cleared by the time the sky is at its driest.
        let steps = (1.75 * STORM_PERIOD * 30.0) as usize;
        for k in 0..steps {
            let s = storm(k as f32 / 30.0);
            water.rain(CYCLE_RAIN * s);
            water.evaporate(CYCLE_EVAP * (1.6 - s));
            water.evaporate_open_water(CYCLE_OPEN * (1.6 - s), CYCLE_DEEP);
            water.step();
        }
        let v1 = water.volume();
        eprintln!("cycle lake volume {v0:.0} -> {v1:.0}");
        assert!(
            v1 > v0 * 0.4 && v1 < v0 * 2.5,
            "lake did not hold: {v0:.0} -> {v1:.0}"
        );
        // Wet cells outside the lake disc (fraction of the grid).
        let n = water.resolution();
        let mut outside = 0;
        for j in 0..n {
            for i in 0..n {
                if water.depth[j * n + i] < 0.06 {
                    continue;
                }
                let x = (i as f32 / (n - 1) as f32 - 0.5) * field.size;
                let z = (j as f32 / (n - 1) as f32 - 0.5) * field.size;
                if ((x - center[0]).powi(2) + (z - center[1]).powi(2)).sqrt() > radius {
                    outside += 1;
                }
            }
        }
        let frac = outside as f32 / (n * n) as f32;
        eprintln!("cycle wet outside lake: {frac:.4}");
        assert!(frac < 0.01, "plain turned to swamp: {frac:.4}");
    }

    #[test]
    fn steep_storm_films_are_whitewater_not_blue_stickers() {
        // A storm peak at live rates: runoff threads the slopes. High above
        // the lake, blue may only mark pooled depth (tarns); thin films
        // must render as white torrents, and at least one must exist.
        let (params, field) = bake_preset("cordillera", 7, 33);
        let mut water = make_water(
            &field,
            30,
            field.resolution / 2,
            0.0,
            lake_preserve(&params),
        );
        for k in 0..900 {
            let s = storm(k as f32 / 30.0);
            water.rain(CYCLE_RAIN * s);
            water.evaporate(CYCLE_EVAP * (1.6 - s));
            water.evaporate_open_water(CYCLE_OPEN * (1.6 - s), CYCLE_DEEP);
            water.step();
        }
        let spectrum = WaveSpectrum::wind_sea(7 ^ 0x5EA7, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
        let n = water.resolution();
        let lift = vec![0.0f32; n * n];
        let mesh = colored_water_mesh(&water, &field, &spectrum, &lift, 3.0, [0.0, 200.0, 200.0]);
        assert!(!mesh.vertices.is_empty());
        // Same blurred field the mesh contours: a vertex beside a real
        // pool reads as pool, not as a sticker.
        let smooth = blur_grid(&water.depth, n);
        let depth_at = |x: f32, z: f32| -> f32 {
            let fx = ((x / field.size + 0.5) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
            let fz = ((z / field.size + 0.5) * (n - 1) as f32).clamp(0.0, (n - 1) as f32);
            let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
            let (i1, j1) = ((i0 + 1).min(n - 1), (j0 + 1).min(n - 1));
            let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
            let a = smooth[j0 * n + i0];
            let b = smooth[j0 * n + i1];
            let c = smooth[j1 * n + i0];
            let d = smooth[j1 * n + i1];
            a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + d * tx * tz
        };
        let mut blue_thin_high = 0;
        let mut white = 0;
        for v in &mesh.vertices {
            // Saturated blue sticker (mirror films run bright red channel).
            let blue = v.color[2] - v.color[0] > 0.12 && v.color[0] < 0.45;
            if v.color.iter().take(3).all(|c| *c > 0.75) {
                white += 1;
            }
            if blue
                && v.position[1] > params.water_level + 5.0
                && depth_at(v.position[0], v.position[2]) < 0.35
            {
                blue_thin_high += 1;
            }
        }
        eprintln!("high thin blue verts={blue_thin_high} white verts={white}");
        assert_eq!(blue_thin_high, 0, "blue stickers on high slopes");
        assert!(white > 0, "no whitewater rendered on soaked slopes");
        // Cliff-cull: no emitted triangle may sit all-cliff + thin (water
        // that should have fallen, not a sticker on the wall).
        let fres = (field.resolution as f32 - 1.0).max(1.0);
        let st = field.size / fres;
        let slope_of = |x: f32, z: f32| -> f32 {
            let fi = ((x / field.size + 0.5) * fres).round() as i32;
            let fj = ((z / field.size + 0.5) * fres).round() as i32;
            let bx = (field.at(fi + 1, fj) - field.at(fi - 1, fj)) / (2.0 * st);
            let bz = (field.at(fi, fj + 1) - field.at(fi, fj - 1)) / (2.0 * st);
            (bx * bx + bz * bz).sqrt()
        };
        let mut clinging_tris = 0;
        for tri in mesh.vertices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }
            let all = tri.iter().all(|v| {
                slope_of(v.position[0], v.position[2]) > 0.9
                    && depth_at(v.position[0], v.position[2]) < 1.0
            });
            if all {
                clinging_tris += 1;
            }
        }
        eprintln!("clinging cliff tris={clinging_tris}");
        assert_eq!(clinging_tris, 0, "wall stickers emitted");
    }

    #[test]
    fn full_res_rebuild_stays_interactive() {
        // What push_meshes_water does every 0.12 s at viewer resolution:
        // time it, so a slowdown never hides behind the fps counter.
        let (params, field) = bake_preset("cordillera", 7, 129);
        let mut water = make_water(
            &field,
            30,
            field.resolution / 2,
            0.0,
            lake_preserve(&params),
        );
        fill_lake(
            &mut water,
            field.size,
            [
                params.lake_center[0] * params.size,
                params.lake_center[1] * params.size,
            ],
            params.lake_radius * params.size * 1.15,
            params.water_level,
        );
        for k in 0..600 {
            let s = storm(k as f32 / 30.0);
            water.rain(CYCLE_RAIN * s);
            water.evaporate(CYCLE_EVAP * (1.6 - s));
            water.step();
        }
        let spectrum = WaveSpectrum::wind_sea(7 ^ 0x5EA7, [1.0, 0.35], 7.0, 34.0, 0.9, 48);
        let swash = WaveSpectrum::wind_sea(7 ^ 0x5A57, [1.0, 0.35], 7.0, 30.0, 0.55, 8);
        let t0 = std::time::Instant::now();
        let lift = runup_grid(&water, &swash, field.size, 20.0, 1.4);
        let mesh = colored_water_mesh(&water, &field, &spectrum, &lift, 20.0, [0.0, 200.0, 200.0]);
        let terrain = colored_terrain_mesh(&field, params.water_level, 7, false);
        let mut verts = terrain.vertices.clone();
        let base: Vec<[f32; 4]> = verts.iter().map(|v| v.color).collect();
        paint_shore(
            &mut verts,
            &base,
            &water,
            field.size,
            params.water_level,
            0.3,
            7,
        );
        let dt = t0.elapsed();
        eprintln!(
            "rebuild: water_tris={} terrain_verts={} took={dt:?}",
            mesh.indices.len() / 3,
            verts.len()
        );
        assert!(dt.as_millis() < 120, "rebuild hitch: {dt:?}");
    }

    #[test]
    fn scene_doc_validates_with_water_node() {
        let (params, field) = bake_preset("cordillera", 5, 17);
        let scene = terrain_scene_doc("t", &field, params.water_level);
        velvet_scene3d::validate_scene(&scene).unwrap();
        assert_eq!(scene.nodes.len(), 2);
        assert!(matches!(
            scene.nodes[1].mesh,
            velvet_scene3d::MeshKind3D::Plane
        ));
    }
}
