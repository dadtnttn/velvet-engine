//! Shared staging for the six galleries (web prototype `galleries/common.js`):
//! where the doorways' walkways run, which walls, bays and chamfered corners
//! are free, and helpers that place solids, props, lights and scattered
//! clutter without ever blocking a doorway. Generic dressing (stains,
//! cracks, puddles, pebbles, candles, webs, chains, loads) lives here too.
use super::super::architecture::{layout, Pilaster};
use super::kit::*;
use super::plan::*;
use velvet_dungeon::{Blueprint, Room};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

pub const IRON: C3 = [0.13, 0.115, 0.11];
pub const WOOD: C3 = [0.34, 0.2, 0.11];
pub const DARK_WOOD: C3 = [0.2, 0.12, 0.07];
pub const BONE: C3 = [0.84, 0.78, 0.66];
pub const WAX: C3 = [0.95, 0.88, 0.7];
pub const GOLD: C3 = [0.9, 0.58, 0.16];

/// A sort key rounded to a millimetre (web `byKey`, JS `Math.round`), so
/// mirror-image ties stay ties whatever the float noise.
pub fn mm(key: f32) -> i64 {
    (key as f64 * 1000.0 + 0.5).floor() as i64
}

pub fn yaw_toward(from: Vec3, to: Vec3) -> f32 {
    yaw_of(hz(to - from))
}
/// Forward `f` and side `s` of a yaw (web `frameOf`).
pub fn frame_of(yaw: f32) -> (Vec3, Vec3) {
    let f = yaw_dir(yaw);
    (f, v(f.z, 0.0, -f.x))
}

pub fn seg_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let d = hz(b - a);
    let t = if d.length_squared() > 1e-6 {
        (hz(p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    hz(p - (a + d * t)).length()
}

#[derive(Clone, Copy, Debug)]
pub struct Door {
    pub p: Vec3,
    pub out: Vec3,
    pub width: f32,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub a: Vec3,
    pub b: Vec3,
    pub mid: Vec3,
    pub tangent: Vec3,
    pub inward: Vec3,
    pub length: f32,
    pub index: usize,
    pub doors: Vec<Door>,
}
#[derive(Clone, Debug)]
pub struct Slot {
    pub center: Vec3,
    pub inward: Vec3,
    pub tangent: Vec3,
    pub width: f32,
    pub wall: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Scatter {
    pub walkway: f32,
    pub ring: [f32; 2],
    pub tries: usize,
    pub near: Option<Vec3>,
    pub spread: f32,
}
impl Default for Scatter {
    fn default() -> Self {
        Self {
            walkway: 0.45,
            ring: [0.0, 1.0],
            tries: 12,
            near: None,
            spread: 1.0,
        }
    }
}

/// The staging of one gallery: its frame (entry → far wall), walls, bays,
/// corners, doorways and walkways, plus placement helpers over the plan.
pub struct Stage<'a> {
    pub map: &'a Blueprint,
    pub room: &'a Room,
    pub plan: &'a mut Plan,
    pub placer: Placer<'a>,
    pub rng: ArtRng,
    pub c: Vec3,
    pub focus: Vec3,
    pub height: f32,
    pub inward: Vec3,
    pub right: Vec3,
    pub depth: f32,
    pub width: f32,
    pub entry: Vec3,
    pub spring: f32,
    pub doors: Vec<Door>,
    pub walls: Vec<Edge>,
    pub corners: Vec<Edge>,
    pub pilasters: Vec<Pilaster>,
    pub ribs: Vec<(Vec3, Vec3)>,
    pub bays: Vec<Slot>,
    used: Vec<bool>,
}

impl<'a> Stage<'a> {
    pub fn new(
        map: &'a Blueprint,
        room: &'a Room,
        plan: &'a mut Plan,
        placer: Placer<'a>,
        rng: ArtRng,
        entry: Vec3,
    ) -> Self {
        let (inward, right, depth, width) = frame(room);
        let c = room.center;
        let arch = layout(room);
        let doors: Vec<Door> = room
            .sockets
            .iter()
            .map(|s| Door {
                p: s.position,
                out: norm(v(s.outward.x, 0.0, s.outward.z)),
                width: s.width,
            })
            .collect();
        let mut walls = vec![];
        let mut corners = vec![];
        let n = room.outline.len();
        for i in 0..n {
            let a = room.outline[i];
            let b = room.outline[(i + 1) % n];
            let pa = v(c.x + a[0], c.y, c.z + a[1]);
            let pb = v(c.x + b[0], c.y, c.z + b[1]);
            let tangent = norm(hz(pb - pa));
            let mid = (pa + pb) * 0.5;
            let mut inw = v(-tangent.z, 0.0, tangent.x);
            if inw.dot(c - mid) < 0.0 {
                inw = -inw;
            }
            let edge = Edge {
                a: pa,
                b: pb,
                mid,
                tangent,
                inward: inw,
                length: hz(pb - pa).length(),
                index: i,
                doors: doors
                    .iter()
                    .filter(|d| seg_distance(d.p, pa, pb) < 0.6)
                    .copied()
                    .collect(),
            };
            if i % 2 == 0 {
                walls.push(edge);
            } else {
                corners.push(edge);
            }
        }
        let pilasters = arch.pilasters.clone();
        // Wall slots: the free stretches of every straight wall between
        // doorways and pilasters, cut into lengths of about three metres.
        let mut bays = vec![];
        for (wi, wall) in walls.iter().enumerate() {
            let mut cuts: Vec<(f32, f32)> = vec![];
            for d in &wall.doors {
                let t = (d.p - wall.a).dot(wall.tangent);
                cuts.push((t - d.width * 0.5 - 0.7, t + d.width * 0.5 + 0.7));
            }
            for p in &pilasters {
                if seg_distance(p.base, wall.a, wall.b) > 0.3 {
                    continue;
                }
                let t = (p.base - wall.a).dot(wall.tangent);
                cuts.push((t - 0.42, t + 0.42));
            }
            cuts.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
            let end = wall.length - 0.55;
            let mut start = 0.55;
            let mut free = vec![];
            for &(x, y) in &cuts {
                if x > start {
                    free.push((start, x.min(end)));
                }
                start = start.max(y);
            }
            if end > start {
                free.push((start, end));
            }
            for (x, y) in free {
                let len = y - x;
                if len < 1.2 {
                    continue;
                }
                let count = ((len / 3.0).round() as usize).max(1);
                let w = len / count as f32;
                for k in 0..count {
                    bays.push(Slot {
                        center: wall.a + wall.tangent * (x + w * (k as f32 + 0.5)),
                        inward: wall.inward,
                        tangent: wall.tangent,
                        width: w,
                        wall: wi,
                    });
                }
            }
        }
        let mut used = vec![false; bays.len()];
        // Slots another planner already dressed stay taken.
        for &uc in &plan.used_slots {
            for (k, bay) in bays.iter().enumerate() {
                if hdist(bay.center, uc) < 0.6 {
                    used[k] = true;
                }
            }
        }
        let focus = plan.focus;
        Self {
            map,
            room,
            plan,
            placer,
            rng,
            c,
            focus,
            height: room.height,
            inward,
            right,
            depth,
            width,
            entry,
            spring: arch.spring,
            doors,
            walls,
            corners,
            pilasters,
            ribs: arch.ribs.clone(),
            bays,
            used,
        }
    }

    pub fn floor(&self, p: Vec3) -> Vec3 {
        with_y(p, self.room.floor_y(p.x, p.z))
    }
    /// Point in the gallery frame: `f` toward the far wall, `r` to the right.
    pub fn at(&self, f: f32, r: f32) -> Vec3 {
        self.c + self.inward * f + self.right * r
    }
    pub fn top(&self, p: Vec3) -> Vec3 {
        with_y(p, self.c.y + self.height)
    }
    pub fn inside(&self, p: Vec3, r: f32) -> bool {
        self.room.contains(p.x, p.z, r)
    }
    /// Distance to the nearest walkway (doorway → focus).
    pub fn walkway(&self, p: Vec3) -> f32 {
        self.doors
            .iter()
            .map(|d| seg_distance(p, d.p, self.focus))
            .fold(f32::INFINITY, f32::min)
    }
    /// Distance to the nearest doorway opening.
    pub fn doorway(&self, p: Vec3) -> f32 {
        self.doors
            .iter()
            .map(|d| hdist(p, d.p) - d.width * 0.5)
            .fold(f32::INFINITY, f32::min)
    }
    /// Whether a circle overlaps anything solid already placed.
    pub fn crowded(&self, p: Vec3, r: f32, gap: f32) -> bool {
        self.placer
            .taken
            .iter()
            .any(|&(q, qr)| hdist(q, p) < r + qr + gap)
    }
    /// A solid circle placed through the placer: doors and walkways stay clear.
    pub fn place(
        &mut self,
        cands: &[Vec3],
        r: f32,
        focus_gap: f32,
        obstacle_gap: f32,
    ) -> Option<Vec3> {
        self.placer.try_place(cands, r, focus_gap, obstacle_gap)
    }
    /// Long furniture as a row of circles along `yaw`: every circle must be clear.
    pub fn place_row(
        &mut self,
        cands: &[Vec3],
        yaw: f32,
        half_len: f32,
        half_wid: f32,
        focus_gap: f32,
        obstacle_gap: f32,
    ) -> Option<Vec3> {
        let f = yaw_dir(yaw);
        let count = ((half_len / half_wid).round() as usize).max(1);
        let offsets: Vec<f32> = if count == 1 {
            vec![0.0]
        } else {
            (0..count)
                .map(|k| {
                    -half_len
                        + half_wid
                        + (k as f32 * (2.0 * (half_len - half_wid))) / (count - 1) as f32
                })
                .collect()
        };
        for &cand in cands {
            let circles: Vec<Vec3> = offsets.iter().map(|&o| cand + f * o).collect();
            if !circles.iter().all(|&p| self.inside(p, half_wid + 0.12)) {
                continue;
            }
            let floored: Vec<Vec3> = circles.iter().map(|&p| self.floor(p)).collect();
            if !floored
                .iter()
                .all(|&p| self.placer.clear(p, half_wid, focus_gap, obstacle_gap))
            {
                continue;
            }
            for &p in &floored {
                self.placer.taken.push((p, half_wid));
                self.plan.solids.push((p, half_wid));
            }
            return Some(self.floor(cand));
        }
        None
    }
    pub fn solid(&mut self, p: Vec3, r: f32, wall: bool) {
        self.plan.solids.push((p, r));
        self.placer.taken.push((p, r));
        if wall {
            self.plan.wall_solids.push((p, r));
        }
    }
    pub fn piece(
        &mut self,
        build: impl Fn(&mut Mesh3D, &mut Mesh3D, &mut Ctx) + Send + Sync + 'static,
    ) {
        self.plan.piece(build);
    }
    pub fn prop(&mut self, asset: &'static str, p: Vec3, yaw: f32, height: f32, radius: f32) {
        self.plan.props.push(Prop {
            asset,
            position: p,
            yaw,
            height,
            radius,
        });
    }
    pub fn light(&mut self, p: Vec3, color: C3, intensity: f32, radius: f32) {
        self.plan.light(p, color, intensity, radius);
    }
    pub fn flame(&mut self, p: Vec3, color: C3, size: f32) {
        self.plan.fixtures.push(Fixture { p, color, size });
    }
    pub fn mount(&mut self, asset: &'static str, anchor: Vec3, yaw: f32, height: f32) {
        self.plan.mounts.push(Mount {
            asset,
            anchor,
            yaw,
            height,
        });
    }

    /// A free floor spot for non-solid clutter.
    pub fn spot(&self, p: Vec3, r: f32, walkway: f32) -> Option<Vec3> {
        if !self.inside(p, r + 0.12) {
            return None;
        }
        if self.doorway(p) < r + 0.55 {
            return None;
        }
        if walkway > 0.0 && self.walkway(p) < r + walkway {
            return None;
        }
        if self.crowded(p, r, 0.04) {
            return None;
        }
        Some(self.floor(p))
    }

    /// Random floor points (rejection sampled) handed to `f(stage, p, k)`.
    pub fn scatter(
        &mut self,
        count: usize,
        r: f32,
        opts: Scatter,
        f: &mut dyn FnMut(&mut Stage<'a>, Vec3, usize),
    ) -> usize {
        let mut made = 0;
        let mut k = 0;
        while k < count * opts.tries && made < count {
            k += 1;
            let p;
            if let Some(near) = opts.near {
                let a = self.rng.range(0.0, TAU);
                let d = opts.spread * self.rng.unit64().sqrt() as f32;
                p = near + v(a.cos() * d, 0.0, a.sin() * d);
            } else {
                let u = self.rng.range(-1.0, 1.0);
                let w = self.rng.range(-1.0, 1.0);
                let rr = u.abs().max(w.abs());
                if rr < opts.ring[0] || rr > opts.ring[1] {
                    continue;
                }
                p = self.c + v(u * self.room.half[0], 0.0, w * self.room.half[1]);
            }
            let Some(q) = self.spot(p, r, opts.walkway) else {
                continue;
            };
            f(self, q, made);
            made += 1;
        }
        made
    }

    /// Free wall bays (indices), nearest to `toward` first (or shuffled).
    pub fn free_bays(&mut self, toward: Option<Vec3>, min_width: f32) -> Vec<Slot> {
        let mut bays: Vec<Slot> = self
            .bays
            .iter()
            .enumerate()
            .filter(|(k, b)| !self.used[*k] && b.width >= min_width)
            .map(|(_, b)| b.clone())
            .collect();
        match toward {
            Some(t) => {
                bays.sort_by_key(|b| mm(hdist(b.center, t)));
                bays
            }
            None => {
                self.rng.shuffle(&mut bays);
                bays
            }
        }
    }
    pub fn take_bay(&mut self, bay: &Slot) -> Slot {
        if let Some(k) = self
            .bays
            .iter()
            .position(|b| hdist(b.center, bay.center) < 1e-4 && b.wall == bay.wall)
        {
            self.used[k] = true;
        }
        self.plan.used_slots.push(bay.center);
        bay.clone()
    }

    pub fn sparkles(
        &mut self,
        count: usize,
        lo: f32,
        hi: f32,
        center: Option<Vec3>,
        spread: Option<f32>,
    ) {
        for _ in 0..count {
            let c = center.unwrap_or(self.c);
            let sx = spread.unwrap_or(self.room.half[0] * 0.8);
            let sz = spread.unwrap_or(self.room.half[1] * 0.8);
            let x = self.rng.range(-sx, sx);
            let y = self.rng.range(lo, hi);
            let z = self.rng.range(-sz, sz);
            self.plan.sparkles.push(with_y(c, self.c.y) + v(x, y, z));
        }
    }
}

// ------------------------------------------------------------ floor dressing

/// Irregular flat blob just above the floor (stains, dust, soot, moss).
pub fn blob(
    m: &mut Mesh3D,
    center: Vec3,
    r: f32,
    color: C3,
    rng: &mut ArtRng,
    y: f32,
    points: usize,
    rough: f32,
) {
    let radii: Vec<f32> = (0..points)
        .map(|_| r * (1.0 - rough * 0.5 + rough * rng.unit()))
        .collect();
    let c = center + Y(y);
    for k in 0..points {
        let a0 = (k as f32 / points as f32) * TAU;
        let a1 = ((k + 1) as f32 / points as f32) * TAU;
        let p0 = c + v(a0.cos() * radii[k], 0.0, a0.sin() * radii[k]);
        let p1 = c + v(
            a1.cos() * radii[(k + 1) % points],
            0.0,
            a1.sin() * radii[(k + 1) % points],
        );
        tri(
            m,
            c,
            p0,
            p1,
            VY,
            tint(color, 0.96 + 0.08 * (k as f32 * 2.3).sin()),
        );
    }
}

/// A branching crack scratched into the floor.
pub fn crack(
    m: &mut Mesh3D,
    start: Vec3,
    dir: Vec3,
    length: f32,
    rng: &mut ArtRng,
    color: C3,
    y: f32,
) {
    let mut p = start + Y(y);
    let mut d = norm(hz(dir));
    let steps = ((length / 0.18).round() as usize).max(3);
    for k in 0..steps {
        let turn = rng.range(-0.55, 0.55);
        d = norm(d + v(-d.z, 0.0, d.x) * turn);
        let q = p + d * (length / steps as f32);
        let w = 0.016 * (1.0 - k as f32 / steps as f32) + 0.004;
        let n = v(-d.z, 0.0, d.x) * w;
        quad(m, [p - n, q - n * 0.7, q + n * 0.7, p + n], VY, color);
        if k > 1 && rng.chance(0.25) {
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            crack(
                m,
                q - Y(y),
                v(-d.z, 0.0, d.x) * side + d,
                length * 0.35,
                rng,
                color,
                y,
            );
        }
        p = q;
    }
}

/// What still water mirrors at grazing angles when nothing tints it.
pub const CAVE_REFLECTION: C3 = [0.16, 0.15, 0.14];

pub fn pond(
    wet: &mut Vec<Pond>,
    outline: &[Vec3],
    level: f32,
    color: C3,
    reflect: C3,
    depth: f32,
    halo: f32,
    drip: Option<f32>,
) {
    let ring: Vec<Vec3> = outline.iter().map(|&p| with_y(p, level)).collect();
    let mut center = Vec3::ZERO;
    for &p in &ring {
        center = center + p;
    }
    center = with_y(center * (1.0 / ring.len() as f32), level);
    wet.push(Pond {
        center,
        outline: ring,
        color,
        reflect,
        depth,
        halo,
        drip,
    });
}

/// An irregular round shoreline: `points` radii about `center`.
pub fn shoreline(center: Vec3, r: f32, rng: &mut ArtRng, points: usize, rough: f32) -> Vec<Vec3> {
    (0..points)
        .map(|k| {
            let a = (k as f32 / points as f32) * TAU;
            let d = r * (1.0 - rough * 0.5 + rough * rng.unit());
            center + v(a.cos() * d, 0.0, a.sin() * d)
        })
        .collect()
}

pub fn ellipse(center: Vec3, rx: f32, rz: f32, points: usize) -> Vec<Vec3> {
    (0..points)
        .map(|k| {
            let a = (k as f32 / points as f32) * TAU;
            center + v(a.cos() * rx, 0.0, a.sin() * rz)
        })
        .collect()
}

/// The mirrored tint of water whose old flat `sheen` glowed in this hue.
pub fn reflection_of(sheen: Option<C3>) -> C3 {
    match sheen {
        Some(s) => mix_color(
            CAVE_REFLECTION,
            tint(s, 0.2 / s[0].max(s[1]).max(s[2]).max(0.05)),
            0.6,
        ),
        None => CAVE_REFLECTION,
    }
}

/// Standing water: with the water list it is real water; `sheen` tints
/// what it mirrors.
pub fn puddle(
    wet: &mut Vec<Pond>,
    center: Vec3,
    r: f32,
    rng: &mut ArtRng,
    water: C3,
    sheen: Option<C3>,
    depth: f32,
) {
    let outline = shoreline(center, r, rng, 16, 0.3);
    pond(
        wet,
        &outline,
        center.y + 0.012,
        water,
        reflection_of(sheen),
        depth,
        1.22,
        None,
    );
}

/// Small scattered stones.
pub fn pebbles(m: &mut Mesh3D, center: Vec3, r: f32, count: usize, rng: &mut ArtRng, color: C3) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let d = r * rng.unit64().sqrt() as f32;
        let s = rng.range(0.025, 0.07);
        let sy = rng.range(0.5, 0.9);
        let shade = rng.range(0.75, 1.2);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            center + v(a.cos() * d, 0.0, a.sin() * d),
            v(s * 1.2, s * sy, s),
            tint(color, shade),
            phase,
        );
    }
}

/// A plain wax candle with drips; its flame is emissive. Returns the flame.
pub fn wax_candle(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    radius: f32,
    height: f32,
    rng: &mut ArtRng,
    wax: C3,
    lit: bool,
) -> Vec3 {
    let w = tint(wax, rng.range(0.9, 1.04));
    lathe(
        m,
        base,
        VY,
        &[
            (0.0, radius * 1.12),
            (height * 0.08, radius),
            (height, radius * 0.98),
            (height + 0.004, radius * 0.72),
        ],
        8,
        w,
        true,
    );
    for k in 0..3 {
        let a = rng.range(0.0, TAU);
        let d = v(a.cos(), 0.0, a.sin());
        let len = rng.range(0.02, height * 0.55);
        kit_block(
            m,
            base + d * (radius * 0.97) + Y(height - len * 0.5),
            VX,
            VY,
            VZ,
            v(0.007 + k as f32 * 0.002, len * 0.5, 0.007),
            tint(w, 1.05),
        );
    }
    kit_block(
        m,
        base + Y(height + 0.012),
        VX,
        VY,
        VZ,
        v(0.003, 0.012, 0.003),
        [0.05, 0.04, 0.03],
    );
    let tip = base + Y(height + 0.016);
    if lit {
        let k = (radius / 0.04).clamp(0.75, 1.4);
        lathe(
            glow,
            tip,
            VY,
            &[
                (0.0, 0.0),
                (0.012 * k, 0.02 * k),
                (0.04 * k, 0.026 * k),
                (0.08 * k, 0.012 * k),
                (0.105 * k, 0.0),
            ],
            6,
            [1.0, 0.42, 0.07],
            false,
        );
        lathe(
            glow,
            tip + Y(0.006 * k),
            VY,
            &[(0.0, 0.0), (0.02 * k, 0.012 * k), (0.048 * k, 0.0)],
            5,
            [1.0, 0.86, 0.5],
            false,
        );
    }
    tip + Y(0.05)
}

/// A cluster of candles standing in a pool of melted wax.
pub fn candle_cluster(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    spread: f32,
    rng: &mut ArtRng,
    wax: C3,
) {
    blob(
        m,
        base,
        spread * 1.25,
        tint(wax, 0.82),
        rng,
        0.008,
        12,
        0.45,
    );
    let mut placed: Vec<Vec3> = vec![];
    let mut k = 0;
    while k < count * 5 && placed.len() < count {
        k += 1;
        let a = rng.range(0.0, TAU);
        let d = spread * rng.unit64().sqrt() as f32;
        let p = base + v(a.cos() * d, 0.0, a.sin() * d);
        if placed.iter().any(|&q| hdist(q, p) < 0.085) {
            continue;
        }
        placed.push(p);
        let r = rng.range(0.028, 0.052);
        let h = rng.range(0.07, 0.36);
        let lit = rng.chance(0.85);
        wax_candle(m, glow, p + Y(0.008), r, h, rng, wax, lit);
    }
}

/// Iron chain from `top` straight down `length`.
pub fn chain(m: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng, color: C3) -> Vec3 {
    let mut y = 0.0;
    let mut alt = rng.chance(0.5);
    while y < length {
        bar(
            m,
            top - Y(y),
            top - Y(y + 0.09),
            0.022,
            0.008,
            if alt { VX } else { VZ },
            tint(color, 1.3),
        );
        y += 0.078;
        alt = !alt;
    }
    top - Y(length)
}

/// Cobweb sheet across an inner corner.
pub fn corner_web(
    m: &mut Mesh3D,
    apex: Vec3,
    dir_a: Vec3,
    dir_b: Vec3,
    size: f32,
    rng: &mut ArtRng,
    color: C3,
) {
    let spokes = 7;
    let rays: Vec<Vec3> = (0..spokes)
        .map(|k| norm(dir_a.lerp(dir_b, k as f32 / (spokes - 1) as f32)))
        .collect();
    let n = norm(dir_a.cross(dir_b));
    let thread = |m: &mut Mesh3D, a: Vec3, b: Vec3, w: f32| {
        let d = norm(b - a);
        let side = norm(d.cross(n)) * w;
        quad(m, [a + side, b + side, b - side, a - side], n, color);
        quad(
            m,
            [a + side, b + side, b - side, a - side],
            -n,
            tint(color, 0.8),
        );
    };
    for &r in &rays {
        let len = size * rng.range(0.85, 1.05);
        thread(m, apex, apex + r * len, 0.006);
    }
    for ring in 1..=5 {
        let d = (ring as f32 / 5.2) * size;
        for k in 0..spokes - 1 {
            let a = apex + rays[k] * d;
            let b = apex + rays[k + 1] * d;
            let sag = (a + b) * 0.5 - Y(d * 0.06);
            thread(m, a, sag, 0.004);
            thread(m, sag, b, 0.004);
        }
    }
}

/// Loose bones and the odd skull, lying on the floor.
pub fn loose_bones(
    m: &mut Mesh3D,
    center: Vec3,
    r: f32,
    count: usize,
    rng: &mut ArtRng,
    skulls: bool,
    color: C3,
) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let d = r * rng.unit64().sqrt() as f32;
        let p = center + v(a.cos() * d, 0.022, a.sin() * d);
        if skulls && rng.chance(0.18) {
            let s = rng.range(0.75, 1.0);
            let yaw = rng.range(0.0, TAU);
            let tilt = rng.range(-0.4, 0.4);
            let shade = rng.range(0.82, 1.02);
            super::themekit::skull(m, p + Y(0.06), s, yaw, tilt, tint(color, shade));
            continue;
        }
        let d1 = rng.range(0.0, TAU);
        let d2 = rng.range(0.0, TAU);
        let dir = v(d1.cos(), 0.0, d2.sin());
        let len = rng.range(0.14, 0.4);
        let b0 = p - dir * (len / 2.0);
        let b1 = p + dir * (len / 2.0);
        let c = tint(color, rng.range(0.62, 0.8));
        let side = v(-dir.z, 0.0, dir.x);
        bar(m, b0, b1, 0.013, 0.013, VY, c);
        for q in [b0, b1] {
            lathe(
                m,
                q + side * 0.01 - Y(0.012),
                VY,
                &[(0.0, 0.016), (0.012, 0.018), (0.026, 0.0)],
                5,
                tint(c, 1.04),
                false,
            );
            lathe(
                m,
                q - side * 0.01 - Y(0.012),
                VY,
                &[(0.0, 0.014), (0.01, 0.016), (0.022, 0.0)],
                5,
                tint(c, 0.98),
                false,
            );
        }
    }
}

/// Broken pottery shards around a point.
pub fn shards(m: &mut Mesh3D, center: Vec3, r: f32, count: usize, rng: &mut ArtRng, clay: C3) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let d = r * rng.unit64().sqrt() as f32;
        let p = center + v(a.cos() * d, 0.01, a.sin() * d);
        let s = rng.range(0.04, 0.09);
        let t = rng.range(0.0, TAU);
        let u = norm(v(t.cos(), rng.range(0.0, 0.4), t.sin())) * s;
        let w = norm(v(-t.sin(), rng.range(0.0, 0.4), t.cos())) * (s * 0.7);
        let shade = rng.range(0.8, 1.1);
        tri(
            m,
            p - u,
            p + w,
            p + u * 0.8 - w * 0.5,
            VY,
            tint(clay, shade),
        );
    }
}

/// A wobbling path from `a` to `b` (roots, veins and mycelium).
pub fn wander(a: Vec3, b: Vec3, steps: usize, amp: f32, rng: &mut ArtRng, lift: f32) -> Vec<Vec3> {
    let d = b - a;
    let side = norm(v(-d.z, 0.0, d.x));
    let phase = rng.range(0.0, TAU);
    let mut pts = vec![];
    for k in 0..=steps {
        let t = k as f32 / steps as f32;
        let jitter = rng.range(-0.3, 0.3);
        let w = (t * std::f32::consts::PI).sin() * amp * ((t * 7.0 + phase).sin() * 0.7 + jitter);
        pts.push(a + d * t + side * w + Y((t * std::f32::consts::PI).sin() * lift));
    }
    pts
}

// ---------------------------------------------------------------- loads

/// Horizontal distance from `p` to the segment a–b.
fn edge_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let d = hz(b - a);
    let t = (hz(p - a).dot(d) / d.length_squared().max(1e-9)).clamp(0.0, 1.0);
    hz(p - (a + d * t)).length()
}

pub struct Load {
    pub crown: f32,
    pub rocks: usize,
    pub size: [f32; 2],
    pub bed: C3,
    pub stone: C3,
    pub ore: Option<C3>,
    pub clear: f32,
}

/// A load heaped in an open container. `rim` is the container's inner
/// outline where the load meets its walls (convex, in order); rocks are
/// bedded in it, all clear of the walls, some with glowing nuggets.
pub fn heap_load(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    rim: &[Vec3],
    load: &Load,
    rng: &mut ArtRng,
) -> Vec3 {
    let n = rim.len();
    let mut center = Vec3::ZERO;
    for &p in rim {
        center = center + p;
    }
    center = center * (1.0 / n as f32);
    let lengths: Vec<f32> = (0..n)
        .map(|k| (rim[k] - rim[(k + 1) % n]).length())
        .collect();
    let perimeter: f32 = lengths.iter().sum();
    let boundary = |t: f32| {
        let mut d = t.rem_euclid(1.0) * perimeter;
        for k in 0..n {
            if d <= lengths[k] {
                return rim[k].lerp(rim[(k + 1) % n], d / lengths[k].max(1e-9));
            }
            d -= lengths[k];
        }
        rim[0]
    };
    let lump = |p: Vec3| 0.35 * (p.x * 23.0 + p.z * 7.0).sin() * (p.z * 19.0 - p.x * 5.0).sin();
    let crown = load.crown;
    let surface = |rho: f32, t: f32| {
        let p = center + (boundary(t) - center) * rho;
        p + Y(crown * (1.0 - rho * rho) + crown * lump(p) * rho * (1.0 - rho))
    };
    let rings = 6;
    let around = (n * 4).max(16);
    let bed = load.bed;
    grid_surface(
        m,
        rings,
        around,
        &|i, j| surface(i as f32 / rings as f32, j as f32 / around as f32),
        &|i, j| {
            tint(
                bed,
                0.8 + 0.25 * (i as f32 * 2.1 + j as f32 * 1.7).sin() * (j as f32 * 0.9).sin(),
            )
        },
        true,
        Some(&|_, _, _| VY),
        None,
    );
    let mut placed = 0;
    let mut k = 0;
    while k < load.rocks * 8 && placed < load.rocks {
        k += 1;
        let rho = 0.82 * rng.unit64().sqrt() as f32;
        let t = rng.unit();
        let p = surface(rho, t);
        let sx = rng.range(load.size[0], load.size[1]);
        let sy = rng.range(load.size[0], load.size[1]) * 0.75;
        let sz = rng.range(load.size[0], load.size[1]);
        let reach = sx.max(sz);
        let clearance = (0..n)
            .map(|e| edge_distance(p, rim[e], rim[(e + 1) % n]))
            .fold(f32::INFINITY, f32::min);
        if clearance < reach + load.clear {
            continue;
        }
        let shade = rng.range(0.7, 1.15);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            p - Y(sy * 0.35),
            v(sx, sy, sz),
            tint(load.stone, shade),
            phase,
        );
        if let Some(ore) = load.ore {
            if rng.chance(0.4) {
                let r = rng.range(0.02, 0.035);
                let h = rng.range(0.06, 0.12);
                let shade = rng.range(0.45, 0.62);
                crystal(glow, p + Y(sy * 0.4), r, h, tint(ore, shade), 5);
            }
        }
        placed += 1;
    }
    center
}

// ------------------------------------------------------------ common pieces

pub fn dust_of(stone: C3) -> C3 {
    mix_color(tint(stone, 0.9), [0.44, 0.42, 0.46], 0.3)
}

/// The shared piece builders every gallery plans (web `COMMON_PIECES`).
impl Stage<'_> {
    pub fn dust(&mut self, center: Vec3, radius: f32, color: Option<C3>) {
        self.piece(move |m, _g, ctx| {
            let c = color.unwrap_or(dust_of(ctx.stone));
            blob(m, center, radius, c, &mut ctx.rng, 0.005, 14, 0.45);
        });
    }
    pub fn stain(
        &mut self,
        center: Vec3,
        radius: f32,
        color: C3,
        y: Option<f32>,
        rough: Option<f32>,
    ) {
        self.piece(move |m, _g, ctx| {
            blob(
                m,
                center,
                radius,
                color,
                &mut ctx.rng,
                y.unwrap_or(0.006),
                14,
                rough.unwrap_or(0.4),
            )
        });
    }
    pub fn crack(&mut self, start: Vec3, dir: Vec3, length: f32, color: Option<C3>) {
        self.piece(move |m, _g, ctx| {
            let c = color.unwrap_or(tint(ctx.stone, 0.3));
            crack(m, start, dir, length, &mut ctx.rng, c, 0.007);
        });
    }
    pub fn puddle(&mut self, center: Vec3, radius: f32, water: Option<C3>, sheen: Option<C3>) {
        self.piece(move |_m, _g, ctx| {
            let Ctx { rng, wet, .. } = ctx;
            puddle(
                wet,
                center,
                radius,
                rng,
                water.unwrap_or([0.05, 0.07, 0.09]),
                sheen,
                0.3,
            );
        });
    }
    pub fn pebbles(&mut self, center: Vec3, radius: f32, count: usize, color: Option<C3>) {
        self.piece(move |m, _g, ctx| {
            let c = color.unwrap_or(tint(ctx.stone, 0.9));
            pebbles(m, center, radius, count, &mut ctx.rng, c);
        });
    }
    pub fn candle_cluster(&mut self, base: Vec3, count: usize, spread: f32) {
        self.piece(move |m, g, ctx| candle_cluster(m, g, base, count, spread, &mut ctx.rng, WAX));
    }
    pub fn chain_hang(&mut self, top: Vec3, length: f32) {
        self.piece(move |m, _g, ctx| {
            chain(m, top, length, &mut ctx.rng, IRON);
        });
    }
    pub fn corner_web(&mut self, apex: Vec3, a: Vec3, b: Vec3, size: f32) {
        self.piece(move |m, _g, ctx| {
            corner_web(m, apex, a, b, size, &mut ctx.rng, [0.78, 0.78, 0.8])
        });
    }
    pub fn loose_bones(&mut self, center: Vec3, radius: f32, count: usize) {
        self.piece(move |m, _g, ctx| {
            loose_bones(m, center, radius, count, &mut ctx.rng, true, BONE)
        });
    }
    pub fn shards(&mut self, center: Vec3, radius: f32, count: usize, color: Option<C3>) {
        self.piece(move |m, _g, ctx| {
            shards(
                m,
                center,
                radius,
                count,
                &mut ctx.rng,
                color.unwrap_or([0.46, 0.24, 0.13]),
            )
        });
    }
}
