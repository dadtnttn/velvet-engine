//! Cellular Arena (**ALPHA** sand / cellular demo).
//! Run: pradera → cueva → descenso con bifurcaciones.
//!
//! A/D move · Space jump · LMB dig · RMB place · F cast · 1/2/3 spells · R restart

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use softbuffer::{Context, Surface};
use velvet_cellular::prelude::*;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

// Internal pixel-art buffer (nearest-neighbor scaled to window).
const CELL_PX: i32 = 4;
const VIEW_W: i32 = 160;
const VIEW_H: i32 = 90;
const GW: i32 = VIEW_W * CELL_PX; // 640
const GH: i32 = VIEW_H * CELL_PX; // 360
const SCALE: u32 = 2; // window 1280×720
const WIN_W: u32 = GW as u32 * SCALE;
const WIN_H: u32 = GH as u32 * SCALE;

// Level bounds (focused run, not infinite sandbox).
const MAP_X0: i32 = -160;
const MAP_X1: i32 = 200;
const MAP_Y0: i32 = -5;
const MAP_Y1: i32 = 195;
/// Grass top of the prairie (world +Y is up).
const SURFACE_Y: i32 = 148;
/// Cave mouth center on the hillside.
const CAVE_MOUTH_X: i32 = 38;
const CAVE_MOUTH_Y: i32 = SURFACE_Y - 2;
/// Treasure / heart chamber depth (win if you reach this).
const GOAL_Y: f32 = 22.0;
const SPAWN_X: f32 = -40.0;
const SPAWN_Y: f32 = (SURFACE_Y + 3) as f32;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Play,
    Win,
    Lose,
}

struct Bit {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    c: u32,
}

#[derive(Default, Clone, Copy)]
struct Keys {
    left: bool,
    right: bool,
    up: bool,
    cast: bool,
    restart: bool,
    s1: bool,
    s2: bool,
    s3: bool,
}

struct Game {
    session: CellularSession,
    player: u32,
    phase: Phase,
    keys: Keys,
    mx: f32,
    my: f32,
    lmb: bool,
    rmb: bool,
    cast_cd: f32,
    spell: u8,
    walk: f32,
    time: f32,
    fb: Vec<u8>,
    bits: Vec<Bit>,
    window: Option<Arc<Window>>,
    context: Option<Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    status: String,
    shake: f32,
    /// Precomputed vignette multipliers (len = GW*GH), avoids full-screen math every frame.
    vignette: Vec<u8>,
    title_cd: f32,
    frame_ms: f32,
    /// Player has walked into the cave mouth.
    entered_cave: bool,
    /// Lowest Y reached (depth; world +Y is up).
    deepest_y: f32,
}

#[inline]
fn pack(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

#[inline]
fn hash2(x: i32, y: i32) -> u32 {
    let mut n = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

/// Material look: saturated Noita-ish base + cell noise (like material grain).
fn mat_rgb(key: &str, x: i32, y: i32) -> (u8, u8, u8) {
    let h = hash2(x, y);
    let d = |c: u8, amp: i16| -> u8 {
        let o = ((h >> 8) % (amp as u32 * 2 + 1)) as i16 - amp;
        (c as i16 + o).clamp(0, 255) as u8
    };
    // checker micro-variation (Noita materials are never flat)
    let chk = ((x ^ y) & 1) as i16;
    let d2 = |c: u8, amp: i16| d(((c as i16 + chk).clamp(0, 255)) as u8, amp);

    match key {
        "air" | "" => (14, 12, 22),
        "bedrock" => (d2(22, 3), d2(20, 3), d2(28, 3)),
        "stone" => (d2(98, 8), d2(96, 8), d2(104, 7)),
        "dirt" => (d2(110, 10), d2(72, 8), d2(42, 6)),
        "grass" => (d2(42, 6), d2(140, 12), d2(48, 6)),
        "sand" => (d2(210, 12), d2(185, 10), d2(110, 8)),
        "water" => {
            // subtle horizontal bands
            let band = ((y.wrapping_mul(3) + (h as i32 & 3)) & 7) as u8;
            (d(40 + band, 4), d(100 + band * 2, 6), d(220 - band, 8))
        }
        "oil" => (d2(38, 4), d2(30, 3), d2(24, 3)),
        "lava" => {
            let pulse = ((h >> 20) & 31) as u8;
            (255, 40 + pulse * 3, 8 + pulse)
        }
        "wood" => (d2(118, 8), d2(72, 6), d2(38, 5)),
        "fire" => {
            let pulse = ((h >> 18) & 63) as u8;
            (255, 90 + pulse, 20 + pulse / 2)
        }
        "smoke" => (d2(55, 6), d2(54, 6), d2(58, 6)),
        "blood" => (d2(150, 12), d2(12, 4), d2(22, 4)),
        "dried_blood" => (d2(70, 6), d2(14, 3), d2(18, 3)),
        "acid" => (d2(70, 8), d2(230, 12), d2(50, 8)),
        "steam" => (d2(175, 8), d2(185, 8), d2(200, 8)),
        "ice" | "ice_block" => (d2(175, 8), d2(215, 8), d2(245, 6)),
        "copper" | "metal" | "steel" | "iron" => (d2(155, 10), d2(150, 10), d2(160, 10)),
        "gold" => {
            let spark = if (h & 0xff) > 220 { 40u8 } else { 0 };
            (
                (230 + spark).min(255) as u8,
                (190 + spark / 2).min(255) as u8,
                40,
            )
        }
        "gunpowder" => (d2(45, 5), d2(42, 5), d2(40, 5)),
        "flesh" => (d2(170, 10), d2(70, 8), d2(75, 8)),
        "bone" => (d2(225, 8), d2(215, 8), d2(195, 8)),
        "poison" => (d2(100, 8), d2(220, 10), d2(60, 8)),
        "slime_trail" => (d2(50, 8), d2(190, 10), d2(70, 8)),
        "salt" => (d2(235, 6), d2(235, 6), d2(240, 6)),
        "glass" => (d2(160, 8), d2(200, 8), d2(210, 8)),
        "ash" => (d2(85, 6), d2(85, 6), d2(88, 6)),
        "basalt" => (d2(42, 5), d2(40, 5), d2(48, 5)),
        "obsidian" => (d2(18, 3), d2(10, 2), d2(28, 4)),
        "granite" => (d2(108, 10), d2(105, 10), d2(112, 8)),
        "limestone" => (d2(188, 8), d2(182, 8), d2(168, 8)),
        "coal_ore" => (d2(28, 5), d2(28, 5), d2(30, 5)),
        "iron_ore" => (d2(100, 8), d2(55, 6), d2(45, 5)),
        "crystal" => (d2(170, 12), d2(210, 10), d2(255, 6)),
        "snow" => (d2(235, 6), d2(240, 6), d2(250, 4)),
        "moss" => (d2(40, 6), d2(115, 10), d2(48, 6)),
        "leaf" => (d2(50, 8), d2(145, 12), d2(40, 6)),
        "vine" => (d2(30, 5), d2(95, 8), d2(38, 5)),
        "brick" => (d2(155, 8), d2(68, 6), d2(48, 5)),
        "concrete" => (d2(125, 6), d2(125, 6), d2(128, 6)),
        "wood_plank" => (d2(145, 8), d2(95, 7), d2(48, 5)),
        "mana" => (d2(70, 8), d2(110, 10), d2(255, 8)),
        "magic_dust" => (d2(200, 12), d2(90, 10), d2(255, 8)),
        "tar" => (d2(18, 3), d2(14, 2), d2(10, 2)),
        "honey" => (d2(210, 8), d2(150, 8), d2(30, 5)),
        "napalm" => (d2(255, 4), d2(130, 10), d2(20, 6)),
        "mushroom" => (d2(155, 10), d2(90, 8), d2(135, 10)),
        _ => (d2(95, 8), d2(90, 8), d2(100, 8)),
    }
}

/// Seeded LCG for map decoration.
fn rng_next(state: &mut u64) -> u32 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
}

fn rng_range(state: &mut u64, lo: i32, hi: i32) -> i32 {
    let span = (hi - lo).max(1) as u32;
    lo + (rng_next(state) % span) as i32
}

/// Carve a thick tunnel segment (Bresenham-ish disks).
fn carve_tunnel(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, radius: i32) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let steps = dx.max(dy).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = x0 as f32 + (x1 - x0) as f32 * t;
        let y = y0 as f32 + (y1 - y0) as f32 * t;
        world.erase_circle(x as i32, y as i32, radius);
    }
}

/// Plant a simple tree (trunk + leaf canopy) on the prairie.
fn plant_tree(world: &mut World, tx: i32, floor_y: i32, wood: MaterialId, leaf: MaterialId) {
    let h = 5 + ((tx as u32).wrapping_mul(17) % 4) as i32;
    if !wood.is_air() {
        for y in 1..=h {
            world.set(tx, floor_y + y, Cell::of(wood));
        }
    }
    if !leaf.is_air() {
        world.paint_circle(tx, floor_y + h + 1, 3, leaf);
        world.paint_circle(tx - 1, floor_y + h, 2, leaf);
        world.paint_circle(tx + 1, floor_y + h, 2, leaf);
    }
}

fn new_world() -> (CellularSession, u32) {
    let mut s = CellularSession::with_builtins(WorldConfig {
        max_loaded_chunks: 512,
        seed: 0xCAFE_0001,
        ambient_temp: 14.0,
        ..WorldConfig::default()
    });
    s.use_hot = true;
    s.sim_hz = 45.0;
    s.sim.pressure = false;
    s.sim.temperature = false;
    s.particles.config.max_particles = 2048;

    let bed = s.mat("bedrock");
    let stone = s.mat("stone");
    let dirt = s.mat("dirt");
    let grass = s.mat("grass");
    let sand = s.mat("sand");
    let water = s.mat("water");
    let oil = s.mat("oil");
    let wood = s.mat("wood");
    let lava = s.mat("lava");
    let gold = s.mat("gold");
    let acid = s.mat("acid");
    let coal = s.mat("coal_ore");
    let iron = s.mat("iron_ore");
    let crystal = s.mat("crystal");
    let moss = s.mat("moss");
    let leaf = s.mat("leaf");
    let mushroom = s.mat("mushroom");
    let basalt = s.mat("basalt");
    let granite = s.mat("granite");

    let rock = if granite.is_air() { stone } else { granite };
    let deep_rock = if basalt.is_air() { stone } else { basalt };
    // ═══════════════════════════════════════════════════════════════════
    // LEVEL: Pradera → boca de cueva en la colina → descenso con ramas
    // ═══════════════════════════════════════════════════════════════════

    // Solid fill underground + mountain body (everything is rock first)
    s.world
        .paint_rect(MAP_X0, MAP_Y0, MAP_X1, SURFACE_Y + 2, rock);
    // deeper basalt band
    s.world.paint_rect(MAP_X0, MAP_Y0, MAP_X1, 55, deep_rock);
    // bedrock shell
    s.world.paint_rect(MAP_X0, MAP_Y0, MAP_X1, MAP_Y0 + 4, bed);
    s.world.paint_rect(MAP_X0, MAP_Y0, MAP_X0 + 4, MAP_Y1, bed);
    s.world.paint_rect(MAP_X1 - 4, MAP_Y0, MAP_X1, MAP_Y1, bed);
    s.world.paint_rect(MAP_X0, MAP_Y1 - 4, MAP_X1, MAP_Y1, bed);

    // ── PRADERA (left open surface) ────────────────────────────────────
    // Clear sky over prairie (left of hillside)
    let hill_start = 22; // rock face begins here
    for y in SURFACE_Y + 1..MAP_Y1 - 4 {
        for x in MAP_X0 + 4..hill_start + 8 {
            s.world.set(x, y, Cell::air());
        }
    }
    // rolling dirt + grass floor on prairie
    for x in MAP_X0 + 4..hill_start + 5 {
        let und = ((x as f32 * 0.11).sin() * 1.5) as i32;
        let floor = SURFACE_Y + und;
        // clear above floor
        for y in floor + 1..MAP_Y1 - 4 {
            s.world.set(x, y, Cell::air());
        }
        if !dirt.is_air() {
            s.world.paint_rect(x, floor - 5, x + 1, floor, dirt);
        } else {
            s.world.paint_rect(x, floor - 5, x + 1, floor, stone);
        }
        if !grass.is_air() {
            s.world.set(x, floor, Cell::of(grass));
        }
    }
    // trees on prairie (not blocking cave mouth)
    for tx in [-120, -100, -80, -65, -50, -30, -15, 0, 10] {
        plant_tree(&mut s.world, tx, SURFACE_Y, wood, leaf);
    }
    // little pond on prairie
    s.world.paint_circle(-90, SURFACE_Y - 1, 4, water);
    s.world.paint_circle(-88, SURFACE_Y - 2, 3, water);

    // ── COLINA / MONTAÑA (right side — cave is inside) ─────────────────
    // Raise hill silhouette above surface so the mouth is visible
    for x in hill_start..MAP_X1 - 4 {
        let t = (x - hill_start) as f32 / 80.0;
        let peak = (8.0 + t * 28.0 + (x as f32 * 0.09).sin() * 3.0) as i32;
        let top = (SURFACE_Y + peak).min(MAP_Y1 - 6);
        s.world.paint_rect(x, SURFACE_Y, x + 1, top, rock);
        // dirt cap on slope
        if !dirt.is_air() && x < hill_start + 40 {
            s.world.set(x, top, Cell::of(dirt));
            if !grass.is_air() {
                s.world.set(x, top + 1, Cell::of(grass));
            }
        }
        // sky above hill peak
        for y in top + 2..MAP_Y1 - 4 {
            s.world.set(x, y, Cell::air());
        }
    }

    // ── BOCA DE CUEVA (clear dark opening in the hillside) ─────────────
    // Wide mouth you can walk into from the prairie
    s.world.erase_circle(CAVE_MOUTH_X, CAVE_MOUTH_Y, 7);
    s.world.erase_circle(CAVE_MOUTH_X + 4, CAVE_MOUTH_Y - 2, 6);
    s.world.erase_circle(CAVE_MOUTH_X + 8, CAVE_MOUTH_Y - 4, 5);
    // short ramp into the dark
    carve_tunnel(
        &mut s.world,
        CAVE_MOUTH_X,
        CAVE_MOUTH_Y,
        CAVE_MOUTH_X + 12,
        CAVE_MOUTH_Y - 10,
        5,
    );

    // ── MAIN DESCENT: expands + bifurcates ─────────────────────────────
    // Path nodes: (x, y, radius) — y decreases = deeper
    // radius grows as we go down
    let spine: &[(i32, i32, i32)] = &[
        (CAVE_MOUTH_X + 12, CAVE_MOUTH_Y - 10, 5), // just inside
        (48, 125, 5),
        (42, 110, 6),
        (55, 95, 7), // widens
        (38, 80, 8),
        (60, 65, 9),
        (45, 50, 10),
        (52, 35, 12), // large mid chamber
        (48, 22, 14), // heart chamber (goal)
    ];
    for w in spine.windows(2) {
        let (x0, y0, r0) = w[0];
        let (x1, y1, r1) = w[1];
        let r = ((r0 + r1) / 2).max(4);
        carve_tunnel(&mut s.world, x0, y0, x1, y1, r);
        s.world.erase_circle(x1, y1, r1);
    }

    // Bifurcations (side branches) — deeper = longer/wider
    let branches: &[(i32, i32, i32, i32, i32)] = &[
        // (from_x, from_y, to_x, to_y, radius)
        (42, 110, 18, 108, 4), // left spur early
        (42, 110, 68, 112, 4), // right spur early
        (55, 95, 80, 90, 5),   // right mid
        (55, 95, 28, 88, 5),   // left mid
        (38, 80, 10, 75, 6),   // left lower
        (38, 80, 72, 78, 5),
        (60, 65, 90, 60, 6), // right deep
        (60, 65, 25, 58, 6),
        (45, 50, 15, 45, 7), // left deep chamber
        (45, 50, 85, 48, 7), // right deep chamber
        (52, 35, 20, 30, 8),
        (52, 35, 90, 32, 8),
    ];
    for &(x0, y0, x1, y1, r) in branches {
        carve_tunnel(&mut s.world, x0, y0, x1, y1, r);
        s.world.erase_circle(x1, y1, r + 2); // room at end of branch
                                             // small ledge / floor at branch tips so enemies stand
        s.world.paint_rect(x1 - 4, y1 - 5, x1 + 4, y1 - 3, stone);
    }

    // Platforms along main spine for vertical play
    for &(x, y, _) in spine.iter().skip(1) {
        s.world.paint_rect(x - 6, y - 4, x + 6, y - 2, stone);
        if !dirt.is_air() && y > 90 {
            s.world.paint_rect(x - 5, y - 2, x + 5, y - 1, dirt);
        }
    }

    // ── Loot / hazards by depth ────────────────────────────────────────
    // shallow: sand drip, moss, mushrooms
    if !moss.is_air() {
        scatter_blobs(&mut s.world, 20, 100, 80, 130, moss, 20, 2, 11);
    }
    if !mushroom.is_air() {
        scatter_blobs(&mut s.world, 15, 90, 85, 120, mushroom, 12, 1, 12);
    }
    s.world.paint_circle(70, 92, 4, water);
    s.world.paint_circle(25, 85, 3, sand);

    // mid: ores, oil, water pockets
    if !coal.is_air() {
        scatter_blobs(&mut s.world, 15, 55, 95, 95, coal, 25, 2, 21);
    }
    if !iron.is_air() {
        scatter_blobs(&mut s.world, 20, 50, 90, 90, iron, 18, 2, 22);
    }
    s.world.paint_circle(88, 60, 5, water);
    if !oil.is_air() {
        s.world.paint_circle(18, 58, 4, oil);
    }

    // deep: gold, crystal, lava, acid
    if !gold.is_air() {
        s.world.paint_circle(48, 22, 3, gold); // heart treasure
        s.world.paint_circle(20, 30, 2, gold);
        s.world.paint_circle(90, 32, 2, gold);
        scatter_blobs(&mut s.world, 25, 15, 80, 40, gold, 12, 1, 31);
    }
    if !crystal.is_air() {
        scatter_blobs(&mut s.world, 30, 18, 75, 45, crystal, 15, 2, 32);
    }
    if !lava.is_air() {
        s.world.paint_rect(30, MAP_Y0 + 5, 70, MAP_Y0 + 10, lava);
        s.world.paint_circle(55, 18, 4, lava);
    }
    if !acid.is_air() {
        s.world.paint_circle(85, 48, 3, acid);
    }

    // wooden props near entrance (campsite feel on prairie)
    s.world
        .paint_rect(-55, SURFACE_Y + 1, -52, SURFACE_Y + 4, wood);
    s.world
        .paint_rect(-25, SURFACE_Y + 1, -22, SURFACE_Y + 3, wood);

    // emitters only near active play (perf)
    let _ = s.add_emitter(70.0, 94.0, "water", 6.0);
    let _ = s.add_emitter(25.0, 87.0, "sand", 5.0);
    let _ = s.add_emitter(55.0, 20.0, "fire", 3.0);

    // ── Player: pradera, mirando hacia la cueva ────────────────────────
    let player = s.spawn_agent(SPAWN_X, SPAWN_Y);
    if let Some(a) = s.agents.get_mut(player) {
        a.hp = 120.0;
        a.max_hp = 120.0;
        a.speed = 40.0;
        a.jump_speed = 46.0;
        a.hw = 1.0;
        a.hh = 1.8;
        a.dig_radius = 2;
        a.place_radius = 1;
        a.place_material = stone;
    }

    // ── Enemies: none on open prairie; denser deeper ───────────────────
    // just inside mouth (tutorial)
    let _ = s.spawn_enemy("slime", 50.0, 128.0);
    let _ = s.spawn_enemy("slime", 45.0, 120.0);
    // mid
    for (x, y) in [
        (55.0, 100.0),
        (35.0, 95.0),
        (70.0, 90.0),
        (25.0, 85.0),
        (50.0, 78.0),
        (65.0, 70.0),
        (30.0, 68.0),
    ] {
        let _ = s.spawn_enemy("slime", x, y);
    }
    // deep + branch rooms
    for (x, y) in [
        (15.0, 45.0),
        (85.0, 48.0),
        (90.0, 60.0),
        (20.0, 58.0),
        (50.0, 40.0),
        (40.0, 32.0),
    ] {
        let _ = s.spawn_enemy("slime", x, y);
    }
    // guardians near goal
    let _ = s.spawn_enemy("brute", 48.0, 26.0);
    let _ = s.spawn_enemy("brute", 40.0, 24.0);
    let _ = s.spawn_enemy("brute", 58.0, 25.0);

    // Sleep static rock for FPS
    s.world.sleep_all_chunks();
    s.hot.clear_hot();
    for (x, y) in [
        (SPAWN_X as i32, SPAWN_Y as i32),
        (CAVE_MOUTH_X, CAVE_MOUTH_Y),
        (48, 125),
        (55, 20),
        (70, 94),
    ] {
        s.world.wake_neighbors(x, y);
        s.hot.touch(x, y);
    }

    println!(
        "RUN: pradera → cueva  mouth=({},{})  goal_y={}  enemies={}  occupied={}",
        CAVE_MOUTH_X,
        CAVE_MOUTH_Y,
        GOAL_Y,
        s.enemies.alive_count(),
        s.world.occupied_cells()
    );

    (s, player)
}

impl Game {
    fn new(headless: bool) -> Self {
        let (session, player) = new_world();
        Self {
            session,
            player,
            phase: Phase::Play,
            keys: Keys::default(),
            mx: WIN_W as f32 * 0.5,
            my: WIN_H as f32 * 0.5,
            lmb: false,
            rmb: false,
            cast_cd: 0.0,
            spell: 1,
            walk: 0.0,
            time: 0.0,
            fb: vec![0; (GW * GH * 4) as usize],
            bits: Vec::with_capacity(6000),
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            status: "Pradera — ve a la CUEVA a la derecha →".into(),
            shake: 0.0,
            vignette: build_vignette(),
            title_cd: 0.0,
            frame_ms: 16.0,
            entered_cave: false,
            deepest_y: SPAWN_Y,
        }
    }

    fn restart(&mut self) {
        let (s, p) = new_world();
        self.session = s;
        self.player = p;
        self.phase = Phase::Play;
        self.bits.clear();
        self.shake = 0.0;
        self.entered_cave = false;
        self.deepest_y = SPAWN_Y;
        self.status = "Pradera — ve a la CUEVA a la derecha →".into();
    }

    fn pos(&self) -> (f32, f32) {
        self.session
            .agents
            .get(self.player)
            .map(|a| (a.x, a.y))
            .unwrap_or((SPAWN_X, SPAWN_Y))
    }

    /// 0 = surface, 100 = goal depth.
    fn depth_pct(&self) -> i32 {
        let span = (SURFACE_Y as f32 - GOAL_Y).max(1.0);
        let d = (SURFACE_Y as f32 - self.deepest_y).clamp(0.0, span);
        ((d / span) * 100.0) as i32
    }

    fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        let (px, py) = self.pos();
        let cam_x = px - VIEW_W as f32 * 0.5;
        let cam_y = py - VIEW_H as f32 * 0.55;
        // window → low-res → cell
        let gx = sx / SCALE as f32;
        let gy = sy / SCALE as f32;
        let cell_x = gx / CELL_PX as f32;
        let cell_y = (GH as f32 - gy) / CELL_PX as f32;
        (cam_x + cell_x, cam_y + cell_y)
    }

    fn bit(&mut self, x: f32, y: f32, vx: f32, vy: f32, life: f32, c: u32) {
        if self.bits.len() > 1800 {
            self.bits.drain(0..400);
        }
        self.bits.push(Bit {
            x,
            y,
            vx,
            vy,
            life,
            c,
        });
    }

    fn burst(&mut self, x: f32, y: f32, n: u32, c: u32, sp: f32) {
        let n = n.min(28); // hard cap VFX cost
        for i in 0..n {
            let a = i as f32 * 2.399 + self.time * 3.0;
            let s = sp * (0.4 + (i % 5) as f32 * 0.15);
            self.bit(
                x,
                y,
                a.cos() * s,
                a.sin() * s + 4.0,
                0.25 + (i % 4) as f32 * 0.12,
                c,
            );
        }
    }

    fn tick_bits(&mut self, dt: f32) {
        for b in &mut self.bits {
            b.vy -= 55.0 * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            b.life -= dt;
            b.vx *= 0.96;
        }
        self.bits.retain(|b| b.life > 0.0);
    }

    fn tick(&mut self, dt: f32) {
        // EMA of frame time for title FPS
        self.frame_ms = self.frame_ms * 0.9 + (dt * 1000.0) * 0.1;
        self.time += dt;
        self.shake = (self.shake - dt * 10.0).max(0.0);
        if self.keys.restart {
            self.keys.restart = false;
            self.restart();
            return;
        }
        if self.phase != Phase::Play {
            self.tick_bits(dt);
            return;
        }

        let (px, py) = self.pos();
        self.session.set_enemy_target(None, px, py);
        let (ax, ay) = self.screen_to_world(self.mx, self.my);
        let aim = (ay - py).atan2(ax - px);

        if self.keys.s1 {
            self.spell = 1;
            self.keys.s1 = false;
        }
        if self.keys.s2 {
            self.spell = 2;
            self.keys.s2 = false;
        }
        if self.keys.s3 {
            self.spell = 3;
            self.keys.s3 = false;
        }

        if self.keys.left || self.keys.right {
            self.walk += dt * 16.0;
            if (self.time * 20.0) as i32 % 2 == 0 {
                self.bit(
                    px,
                    py - 1.5,
                    self.time.sin() * 5.0,
                    3.0,
                    0.25,
                    pack(160, 130, 80),
                );
            }
        }

        if self.lmb {
            self.burst(ax, ay, 4, pack(170, 145, 95), 10.0);
            let _ = self.session.particle_burst(ax, ay, "sand", 2);
        }
        if self.rmb {
            self.burst(ax, ay, 2, pack(100, 100, 110), 5.0);
        }

        self.session.agent_input(
            self.player,
            AgentInput {
                move_x: (self.keys.right as i32 - self.keys.left as i32) as f32,
                move_y: 0.0,
                jump: self.keys.up,
                dig: self.lmb,
                place: self.rmb,
                cast: false,
                aim,
            },
        );

        self.cast_cd = (self.cast_cd - dt).max(0.0);
        if self.keys.cast && self.cast_cd <= 0.0 {
            self.keys.cast = false;
            self.cast_cd = 0.28;
            let key = match self.spell {
                2 => "water_ball",
                3 => "digging_blast",
                _ => "spark_bolt",
            };
            let ox = px + aim.cos() * 2.8;
            let oy = py + aim.sin() * 2.8;
            if self.session.cast_spell(key, ox, oy) {
                self.shake = 0.22;
                let col = match self.spell {
                    2 => pack(50, 130, 255),
                    3 => pack(170, 150, 95),
                    _ => pack(255, 160, 40),
                };
                self.burst(ox, oy, 50, col, 24.0);
            }
        }

        self.session.tick(dt.min(0.05));
        self.tick_bits(dt);

        // progress: cave entry + deepest point
        if px > (CAVE_MOUTH_X - 4) as f32 && py < (SURFACE_Y + 6) as f32 {
            self.entered_cave = true;
        }
        if py < self.deepest_y {
            self.deepest_y = py;
        }

        // contact damage
        let mut dmg = 0.0;
        let mut hit = false;
        if let Some(p) = self.session.agents.get(self.player).cloned() {
            for e in &self.session.enemies.enemies {
                if e.alive {
                    let dx = e.x - p.x;
                    let dy = e.y - p.y;
                    if dx * dx + dy * dy < (e.hw + p.hw + 0.4).powi(2) {
                        dmg += e.contact_damage * dt * 7.0;
                        hit = true;
                    }
                }
            }
        }
        if dmg > 0.0 {
            let _ = self.session.agents.damage(
                self.player,
                dmg,
                &mut self.session.world,
                &mut self.session.particles,
                &mut self.session.physics,
            );
            if hit {
                self.burst(px, py, 10, pack(210, 25, 35), 14.0);
                self.shake = 0.18;
            }
        }

        let enemies = self.session.enemies.alive_count();
        let hp = self
            .session
            .agents
            .get(self.player)
            .map(|a| a.hp)
            .unwrap_or(0.0);
        let depth = self.depth_pct();

        if hp <= 0.0 {
            self.phase = Phase::Lose;
            self.burst(px, py, 80, pack(190, 15, 25), 20.0);
            self.status = "MUERTO — R reinicia".into();
        } else if self.deepest_y <= GOAL_Y + 4.0 {
            // reached heart chamber (gold + brutes)
            self.phase = Phase::Win;
            self.burst(px, py + 3.0, 80, pack(255, 210, 50), 22.0);
            self.status = "¡LLEGASTE AL FONDO! Oro de la montaña — R otra".into();
        } else if !self.entered_cave {
            self.status = format!(
                "Pradera  HP {:.0}  → camina a la CUEVA (derecha)  hechizo {}",
                hp, self.spell
            );
        } else if depth < 35 {
            self.status = format!(
                "Cueva  HP {:.0}  profundidad {}%  baja y explora ramas  enemigos {}",
                hp, depth, enemies
            );
        } else if depth < 70 {
            self.status = format!(
                "Minas  HP {:.0}  profundidad {}%  bifurcaciones + botín  enemigos {}",
                hp, depth, enemies
            );
        } else {
            self.status = format!(
                "Profundidades  HP {:.0}  {}% → busca la cámara de oro  enemigos {}",
                hp, depth, enemies
            );
        }
        self.title_cd -= dt;
        if self.title_cd <= 0.0 {
            self.title_cd = 0.35;
            let fps = (1000.0 / self.frame_ms.max(1.0)) as i32;
            if let Some(w) = &self.window {
                w.set_title(&format!("Cave Run — {} | {} fps", self.status, fps));
            }
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn compose_fb(&mut self) {
        let (px, py) = self.pos();
        let shx = if self.shake > 0.0 {
            (self.time * 55.0).sin() * self.shake * 2.5
        } else {
            0.0
        };
        let shy = if self.shake > 0.0 {
            (self.time * 49.0).cos() * self.shake * 2.5
        } else {
            0.0
        };
        let origin_x = (px - VIEW_W as f32 * 0.5 + shx).floor() as i32;
        let origin_y = (py - VIEW_H as f32 * 0.55 + shy).floor() as i32;

        // ── world cells: one color per cell, fill 4×4 without subpixel hash ──
        // Integer light (no sqrt): light = clamp(1.2 - dist/62) ≈ using dist2 tiers
        for cy in 0..VIEW_H {
            for cx in 0..VIEW_W {
                let wx = origin_x + cx;
                let wy = origin_y + cy;
                let cell = self.session.world.get(wx, wy);
                let key = if cell.is_air() {
                    "air"
                } else {
                    self.session.world.materials.get(cell.material).key.as_str()
                };
                let (mut r, mut g, mut b) = mat_rgb(key, wx, wy);

                if cell.flags.contains(velvet_cellular::CellFlags::BURNING) {
                    let flicker =
                        ((hash2(wx, wy.wrapping_add((self.time * 12.0) as i32)) >> 24) & 40) as u8;
                    r = 255;
                    g = 100 + flicker;
                    b = 15;
                }

                // fast torch light (avoid sqrt): approx with dist² bands
                let dx = wx - px as i32;
                let dy = wy - py as i32;
                let dist2 = (dx * dx + dy * dy) as u32;
                // light in 0..256 fixed point
                let mut light = if dist2 > 62 * 62 {
                    28u32 // ~0.11
                } else if dist2 > 45 * 45 {
                    70
                } else if dist2 > 30 * 30 {
                    120
                } else if dist2 > 18 * 18 {
                    180
                } else if dist2 > 8 * 8 {
                    230
                } else {
                    256
                };
                // emissive materials
                match key {
                    "lava" | "fire" => light = light.saturating_add(70).min(280),
                    "gold" | "acid" => light = light.saturating_add(20).min(270),
                    _ => {}
                }
                r = ((r as u32 * light) >> 8).min(255) as u8;
                g = ((g as u32 * light) >> 8).min(255) as u8;
                b = ((b as u32 * light) >> 8).min(255) as u8;
                // warm near center
                if dist2 < 20 * 20 {
                    r = r.saturating_add(12);
                    g = g.saturating_add(6);
                }

                let gx0 = cx * CELL_PX;
                let gy0 = (VIEW_H - 1 - cy) * CELL_PX;
                // solid fill + 1px darker edge (cheap cell border)
                let (er, eg, eb) = if key != "air" && key != "" {
                    (
                        (r as u16 * 210 / 255) as u8,
                        (g as u16 * 210 / 255) as u8,
                        (b as u16 * 210 / 255) as u8,
                    )
                } else {
                    (r, g, b)
                };
                for oy in 0..CELL_PX {
                    for ox in 0..CELL_PX {
                        let edge = ox == 0 || oy == 0;
                        if edge && key != "air" {
                            put(&mut self.fb, gx0 + ox, gy0 + oy, er, eg, eb);
                        } else {
                            put(&mut self.fb, gx0 + ox, gy0 + oy, r, g, b);
                        }
                    }
                }
            }
        }

        // ── free particles ──────────────────────────────────────────────
        let mut drawn = 0u32;
        for p in self.session.particles.particles.iter().filter(|p| p.alive) {
            if drawn > 1200 {
                break;
            }
            let (sx, sy) = world_to_fb(p.x, p.y, origin_x, origin_y);
            if sx < -2 || sy < -2 || sx >= GW + 2 || sy >= GH + 2 {
                continue;
            }
            let (r, g, b) = if !p.material.is_air() {
                let c = self.session.world.materials.get(p.material).color;
                (c[0], c[1], c[2])
            } else {
                (255, 200, 50)
            };
            put(&mut self.fb, sx, sy, r, g, b);
            put(&mut self.fb, sx + 1, sy, r, g, b);
            drawn += 1;
        }
        for b in &self.bits {
            let (sx, sy) = world_to_fb(b.x, b.y, origin_x, origin_y);
            if sx < 0 || sy < 0 || sx >= GW || sy >= GH {
                continue;
            }
            let r = ((b.c >> 16) & 0xff) as u8;
            let g = ((b.c >> 8) & 0xff) as u8;
            let bl = (b.c & 0xff) as u8;
            put(&mut self.fb, sx, sy, r, g, bl);
        }

        // ── enemies (only those near the camera) ────────────────────────
        let margin = 12.0;
        for e in &self.session.enemies.enemies {
            if !e.alive {
                continue;
            }
            if e.x < origin_x as f32 - margin
                || e.y < origin_y as f32 - margin
                || e.x > (origin_x + VIEW_W) as f32 + margin
                || e.y > (origin_y + VIEW_H) as f32 + margin
            {
                continue;
            }
            let (sx, sy) = world_to_fb(e.x, e.y, origin_x, origin_y);
            if e.def_key == "brute" {
                draw_brute(&mut self.fb, sx, sy, self.time);
            } else {
                draw_slime(&mut self.fb, sx, sy, self.time + e.x * 0.3);
            }
        }

        // ── player (wizard) ─────────────────────────────────────────────
        if let Some(a) = self.session.agents.get(self.player) {
            let (sx, sy) = world_to_fb(a.x, a.y, origin_x, origin_y);
            let (ax, ay) = self.screen_to_world(self.mx, self.my);
            let aim = (ay - a.y).atan2(ax - a.x);
            let face = if aim.cos() >= 0.0 { 1 } else { -1 };
            draw_wizard(&mut self.fb, sx, sy, face, aim, self.walk, a.grounded);

            // aim reticle (pixel cross, no AA)
            let (cx, cy) = world_to_fb(ax, ay, origin_x, origin_y);
            for i in 3..=6 {
                put(&mut self.fb, cx + i, cy, 255, 220, 60);
                put(&mut self.fb, cx - i, cy, 255, 220, 60);
                put(&mut self.fb, cx, cy + i, 255, 220, 60);
                put(&mut self.fb, cx, cy - i, 255, 220, 60);
            }
            put(&mut self.fb, cx, cy, 255, 255, 180);
        }

        // ── HUD ─────────────────────────────────────────────────────────
        let hp = self
            .session
            .agents
            .get(self.player)
            .map(|a| a.hp / a.max_hp)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let depth_t = (self.depth_pct() as f32 / 100.0).clamp(0.0, 1.0);
        // framed bars
        hud_panel(&mut self.fb, 4, 4, 100, 28);
        hud_bar(&mut self.fb, 8, 8, 90, 8, hp, 40, 200, 70);
        // depth progress toward heart chamber (gold bar)
        hud_bar(&mut self.fb, 8, 20, 90, 6, depth_t, 180, 140, 50);
        // spell slots
        for i in 0..3 {
            let sel = self.spell == (i + 1) as u8;
            let (r, g, b) = match i {
                0 => (255, 140, 30),
                1 => (40, 110, 255),
                _ => (180, 150, 80),
            };
            let x = 8 + i * 22;
            let y = GH - 22;
            for oy in 0..14 {
                for ox in 0..18 {
                    let border = ox == 0 || oy == 0 || ox == 17 || oy == 13;
                    let c = if border {
                        if sel {
                            (255, 240, 120)
                        } else {
                            (50, 48, 60)
                        }
                    } else if sel {
                        (r, g, b)
                    } else {
                        (r / 3, g / 3, b / 3)
                    };
                    put(&mut self.fb, x + ox, y + oy, c.0, c.1, c.2);
                }
            }
        }

        // precomputed vignette (8-bit scale, 255 = full)
        let vig = &self.vignette;
        for i in 0..(GW * GH) as usize {
            let m = vig[i] as u32;
            if m < 250 {
                let j = i * 4;
                self.fb[j] = ((self.fb[j] as u32 * m) >> 8) as u8;
                self.fb[j + 1] = ((self.fb[j + 1] as u32 * m) >> 8) as u8;
                self.fb[j + 2] = ((self.fb[j + 2] as u32 * m) >> 8) as u8;
            }
        }

        if self.phase != Phase::Play {
            let (r, g, b) = match self.phase {
                Phase::Win => (35u8, 70, 25),
                Phase::Lose => (70, 12, 12),
                _ => (0, 0, 0),
            };
            for i in (0..self.fb.len()).step_by(4) {
                self.fb[i] = self.fb[i] / 2 + r / 2;
                self.fb[i + 1] = self.fb[i + 1] / 2 + g / 2;
                self.fb[i + 2] = self.fb[i + 2] / 2 + b / 2;
            }
        }
    }

    fn present(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let ww = size.width.max(1);
        let wh = size.height.max(1);
        self.compose_fb();
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap());
        let mut buf = surface.buffer_mut().unwrap();
        // nearest-neighbor upscale (keeps hard pixels)
        for y in 0..wh {
            let sy = y * GH as u32 / wh;
            for x in 0..ww {
                let sx = x * GW as u32 / ww;
                let i = ((sy * GW as u32 + sx) * 4) as usize;
                let (r, g, b) = if i + 2 < self.fb.len() {
                    (self.fb[i], self.fb[i + 1], self.fb[i + 2])
                } else {
                    (0, 0, 0)
                };
                buf[(y * ww + x) as usize] = pack(r, g, b);
            }
        }
        let _ = buf.present();
    }
}

fn build_vignette() -> Vec<u8> {
    let mut v = vec![255u8; (GW * GH) as usize];
    for y in 0..GH {
        for x in 0..GW {
            let dx = (x as f32 / GW as f32 - 0.5) * 2.0;
            let dy = (y as f32 / GH as f32 - 0.5) * 2.0;
            let t = (1.0 - (dx * dx * 0.28 + dy * dy * 0.32)).clamp(0.35, 1.0);
            v[(y * GW + x) as usize] = (t * 255.0) as u8;
        }
    }
    v
}

fn world_to_fb(wx: f32, wy: f32, origin_x: i32, origin_y: i32) -> (i32, i32) {
    let cx = ((wx - origin_x as f32) * CELL_PX as f32) as i32;
    let cy_world = ((wy - origin_y as f32) * CELL_PX as f32) as i32;
    let cy = (GH - 1) - cy_world;
    (cx, cy)
}

#[inline]
fn put(fb: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8) {
    if x < 0 || y < 0 || x >= GW || y >= GH {
        return;
    }
    let i = ((y * GW + x) * 4) as usize;
    fb[i] = r;
    fb[i + 1] = g;
    fb[i + 2] = b;
    fb[i + 3] = 255;
}

fn hud_panel(fb: &mut [u8], x: i32, y: i32, w: i32, h: i32) {
    for oy in 0..h {
        for ox in 0..w {
            let border = ox == 0 || oy == 0 || ox == w - 1 || oy == h - 1;
            if border {
                put(fb, x + ox, y + oy, 70, 65, 90);
            } else {
                put(fb, x + ox, y + oy, 18, 16, 28);
            }
        }
    }
}

fn hud_bar(fb: &mut [u8], x: i32, y: i32, w: i32, h: i32, t: f32, r: u8, g: u8, b: u8) {
    for oy in 0..h {
        for ox in 0..w {
            put(fb, x + ox, y + oy, 28, 24, 36);
        }
    }
    let fw = ((w - 2) as f32 * t.clamp(0.0, 1.0)) as i32;
    for oy in 1..h - 1 {
        for ox in 1..=fw {
            // gradient shine
            let shine = if oy == 1 { 30u8 } else { 0 };
            put(
                fb,
                x + ox,
                y + oy,
                r.saturating_add(shine),
                g.saturating_add(shine),
                b,
            );
        }
    }
}

/// Green slime blob — bouncy silhouette.
fn draw_slime(fb: &mut [u8], sx: i32, sy: i32, t: f32) {
    let bounce = (t * 6.0).sin() * 1.5;
    let body = (50u8, 190, 70);
    let dark = (30u8, 120, 45);
    let light = (90u8, 230, 110);
    for oy in -5..=4 {
        for ox in -6..=6 {
            let yy = oy as f32 - bounce * 0.3;
            if (ox * ox) as f32 + yy * yy * 1.4 < 28.0 {
                let c = if oy < -2 {
                    light
                } else if oy > 2 {
                    dark
                } else {
                    body
                };
                put(fb, sx + ox, sy + oy + bounce as i32, c.0, c.1, c.2);
            }
        }
    }
    // eyes
    put(fb, sx - 2, sy - 1 + bounce as i32, 250, 250, 230);
    put(fb, sx + 2, sy - 1 + bounce as i32, 250, 250, 230);
    put(fb, sx - 2, sy + bounce as i32, 20, 30, 40);
    put(fb, sx + 2, sy + bounce as i32, 20, 30, 40);
}

/// Red brute — chunky armored blob.
fn draw_brute(fb: &mut [u8], sx: i32, sy: i32, t: f32) {
    let wobble = ((t * 4.0).sin() * 0.8) as i32;
    let body = (160u8, 45, 40);
    let dark = (100u8, 25, 22);
    let horn = (200u8, 180, 140);
    for oy in -7i32..=5 {
        for ox in -7i32..=7 {
            if ox * ox + oy * oy < 42 {
                let c = if ox.abs() > 4 { dark } else { body };
                put(fb, sx + ox + wobble, sy + oy, c.0, c.1, c.2);
            }
        }
    }
    // horns
    put(fb, sx - 5 + wobble, sy - 8, horn.0, horn.1, horn.2);
    put(fb, sx - 4 + wobble, sy - 9, horn.0, horn.1, horn.2);
    put(fb, sx + 5 + wobble, sy - 8, horn.0, horn.1, horn.2);
    put(fb, sx + 4 + wobble, sy - 9, horn.0, horn.1, horn.2);
    // eyes (glowing)
    put(fb, sx - 2 + wobble, sy - 2, 255, 80, 40);
    put(fb, sx + 2 + wobble, sy - 2, 255, 80, 40);
    put(fb, sx - 2 + wobble, sy - 1, 255, 200, 80);
    put(fb, sx + 2 + wobble, sy - 1, 255, 200, 80);
}

/// Noita-inspired wizard: robe, skin head, wand arm.
fn draw_wizard(fb: &mut [u8], sx: i32, sy: i32, face: i32, aim: f32, walk: f32, grounded: bool) {
    // palette (Mina-ish cool blue robe)
    let robe = (48u8, 72, 140);
    let robe_d = (32u8, 48, 100);
    let robe_l = (70u8, 100, 170);
    let skin = (235u8, 195, 155);
    let skin_d = (200u8, 155, 120);
    let boot = (35u8, 30, 40);
    let belt = (210u8, 170, 45);
    let wand_wood = (140u8, 90, 40);
    let wand_tip = (255u8, 230, 90);

    let stride = if grounded {
        (walk.sin() * 2.5) as i32
    } else {
        1
    };

    // shadow under feet
    for ox in -4..=4 {
        put(fb, sx + ox, sy + 9, 10, 8, 16);
    }

    // legs + boots
    for y in 0..6 {
        // left
        put(fb, sx - 2 + stride, sy + 3 + y, boot.0, boot.1, boot.2);
        put(
            fb,
            sx - 1 + stride,
            sy + 3 + y,
            robe_d.0,
            robe_d.1,
            robe_d.2,
        );
        // right
        put(fb, sx + 1 - stride, sy + 3 + y, boot.0, boot.1, boot.2);
        put(
            fb,
            sx + 2 - stride,
            sy + 3 + y,
            robe_d.0,
            robe_d.1,
            robe_d.2,
        );
    }

    // robe body (taller silhouette)
    for y in -3i32..=4 {
        for x in -4i32..=4 {
            let wide = if y > 1 { 4 } else { 3 };
            if x.abs() <= wide {
                let c = if x * face > 1 {
                    robe_l
                } else if x * face < -1 {
                    robe_d
                } else {
                    robe
                };
                put(fb, sx + x, sy + y, c.0, c.1, c.2);
            }
        }
    }
    // robe fold line
    for y in -2..=3 {
        put(fb, sx + face, sy + y, robe_d.0, robe_d.1, robe_d.2);
    }
    // gold belt
    for x in -4..=4 {
        put(fb, sx + x, sy + 2, belt.0, belt.1, belt.2);
    }
    put(fb, sx, sy + 2, 255, 220, 80);

    // head (skin)
    for y in -9..=-4 {
        for x in -3..=3 {
            if x * x + (y + 6) * (y + 6) <= 12 {
                let c = if x * face > 0 { skin } else { skin_d };
                put(fb, sx + x, sy + y, c.0, c.1, c.2);
            }
        }
    }
    // simple hair / scalp shadow
    for x in -2..=2 {
        put(fb, sx + x, sy - 9, 60, 50, 45);
    }
    // eye (facing)
    put(fb, sx + face * 2, sy - 6, 30, 35, 50);
    put(fb, sx + face * 2, sy - 6, 250, 250, 240);
    put(fb, sx + face * 2 + face, sy - 6, 20, 25, 40);

    // back arm hang
    for y in 0..5 {
        put(fb, sx - face * 4, sy - 1 + y, skin.0, skin.1, skin.2);
    }

    // wand arm toward aim (world up → screen down)
    let arm_len = 14.0;
    let hx = sx as f32 + aim.cos() * arm_len;
    let hy = sy as f32 - aim.sin() * arm_len;
    let shoulder_x = sx + face * 3;
    let shoulder_y = sy - 1;
    let steps = 16;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ax = shoulder_x as f32 * (1.0 - t) + hx * t;
        let ay = shoulder_y as f32 * (1.0 - t) + hy * t;
        if i < steps / 2 {
            put(fb, ax as i32, ay as i32, skin.0, skin.1, skin.2);
            put(fb, ax as i32, ay as i32 + 1, skin_d.0, skin_d.1, skin_d.2);
        } else {
            put(
                fb,
                ax as i32,
                ay as i32,
                wand_wood.0,
                wand_wood.1,
                wand_wood.2,
            );
            put(
                fb,
                ax as i32,
                ay as i32 - 1,
                wand_wood.0.saturating_sub(20),
                wand_wood.1.saturating_sub(15),
                wand_wood.2.saturating_sub(10),
            );
        }
    }
    // glowing tip
    let tx = hx as i32;
    let ty = hy as i32;
    put(fb, tx, ty, wand_tip.0, wand_tip.1, wand_tip.2);
    put(fb, tx + 1, ty, 255, 180, 40);
    put(fb, tx, ty - 1, 255, 160, 30);
    put(fb, tx - 1, ty, 255, 200, 60);
    put(fb, tx, ty + 1, 255, 140, 20);
}

impl ApplicationHandler for Game {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Cave Run — pradera → cueva")
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let context = Context::new(window.clone()).expect("ctx");
        let surface = Surface::new(&context, window.clone()).expect("surface");
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        self.last = Instant::now();
        self.burst(0.0, 30.0, 40, pack(255, 200, 50), 14.0);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                let d = event.state == ElementState::Pressed;
                let PhysicalKey::Code(c) = event.physical_key else {
                    return;
                };
                match c {
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.keys.left = d,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.keys.right = d,
                    KeyCode::KeyW | KeyCode::ArrowUp | KeyCode::Space => self.keys.up = d,
                    KeyCode::KeyF | KeyCode::Enter => {
                        if d {
                            self.keys.cast = true;
                        }
                    }
                    KeyCode::Digit1 => {
                        if d {
                            self.keys.s1 = true;
                        }
                    }
                    KeyCode::Digit2 => {
                        if d {
                            self.keys.s2 = true;
                        }
                    }
                    KeyCode::Digit3 => {
                        if d {
                            self.keys.s3 = true;
                        }
                    }
                    KeyCode::KeyR => {
                        if d {
                            self.keys.restart = true;
                        }
                    }
                    KeyCode::Escape => {
                        if d {
                            el.exit();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mx = position.x as f32;
                self.my = position.y as f32;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let d = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => self.lmb = d,
                    MouseButton::Right => self.rmb = d,
                    MouseButton::Middle if d => self.keys.cast = true,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last).as_secs_f32().min(0.05);
                self.last = now;
                self.tick(dt);
                self.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.headless {
            self.hframes += 1;
            self.tick(1.0 / 60.0);
            if self.hframes >= 90 {
                println!(
                    "headless occupied={} enemies={} particles={}",
                    self.session.world.occupied_cells(),
                    self.session.enemies.alive_count(),
                    self.session.particle_count()
                );
                println!("ASSERT_OK cellular_arena_demo");
                el.exit();
            }
            return;
        }
        el.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("cellular_arena=info,info");
    let headless = std::env::args().any(|a| a == "--headless");
    println!("=== Cave Run: pradera → boca de cueva → descenso con ramas ===");
    println!("Objetivo: entra a la CUEVA (derecha) y baja hasta el oro del fondo");
    println!("A/D move | Space jump | LMB dig | RMB place | F cast | 1/2/3 spells | R restart");
    println!(
        "Window {}x{}  internal {}x{}  cell {}px",
        WIN_W, WIN_H, GW, GH, CELL_PX
    );

    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut game = Game::new(headless);
    el.run_app(&mut game)?;
    Ok(())
}
