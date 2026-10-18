//! The battlefield as data: heights on a 2 m grid built from authored features (ridges, hills,
//! rock walls, the river with its ford and bridge, marsh and causeways, roads, woods, set pieces),
//! and the queries the simulation needs: walk height, blocked ground, speed, cover, a 4 m
//! passability grid with clearance (how wide a formation fits) and A* paths round obstacles.
use std::sync::Arc;

use crate::baked::{BattleMap, DECK, NO_WATER, SOLID, WORK};
use crate::maps::{MapSpec, Rect};
use crate::noise::{clamp, fbm, line_near, noise, resample, smooth, LinePoint};

/// The river's and the marsh's surface.
pub const WATER_Y: f32 = -0.55;
/// Water deeper than this cannot be waded.
const DEEP: f32 = 0.72;
/// On the war map 2.0's battle maps (their wide shallow rivers are waded waist-deep; the deep
/// ones, the harbours and the sea are well past it).
const DEEP_BAKED: f32 = 1.2;

/// How wide each set piece stands in the way (0: walk through or round by its walls).
pub fn prop_footprint(kind: &str) -> f32 {
    match kind {
        "watchtower" => 6.5,
        "market_stone" => 3.2,
        "lodge" | "farmhouse" => 6.0,
        "windmill" => 4.2,
        "hut" => 3.5,
        "standing_stone" => 1.2,
        "well" => 1.8,
        "totem" => 0.8,
        "shrine" => 3.0,
        "tent" => 3.2,
        "cart" => 1.8,
        _ => 0.0,
    }
}

fn in_poly(x: f32, z: f32, pts: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ([xi, zi], [xj, zj]) = (pts[i], pts[j]);
        if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Distance to a polygon's edge (negative inside).
fn poly_edge(x: f32, z: f32, pts: &[[f32; 2]]) -> f32 {
    let mut best = 1e9f32;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ([ax, az], [bx, bz]) = (pts[j], pts[i]);
        let (dx, dz) = (bx - ax, bz - az);
        let l2 = dx * dx + dz * dz;
        let t = clamp(
            ((x - ax) * dx + (z - az) * dz) / if l2 > 0.0 { l2 } else { 1.0 },
            0.0,
            1.0,
        );
        best = best.min((x - ax - dx * t).hypot(z - az - dz * t));
        j = i;
    }
    if in_poly(x, z, pts) {
        -best
    } else {
        best
    }
}

pub struct RiverLine {
    pub line: Vec<LinePoint>,
    pub total: f32,
    pub w2: f32,
    pub bank: f32,
    pub ford: Option<(f32, f32)>,
    pub bridge_at: Option<f32>,
    pub bed_deep: f32,
    pub bed_ford: f32,
}

/// The trolls' bridge: an arched stone deck across the river.
#[derive(Clone, Copy, Debug)]
pub struct Bridge {
    pub x: f32,
    pub z: f32,
    /// Across the river (along the deck).
    pub ux: f32,
    pub uz: f32,
    /// Along the river (across the deck).
    pub vx: f32,
    pub vz: f32,
    pub half: f32,
    pub width: f32,
    pub rise: f32,
    pub base: f32,
}

#[derive(Clone, Debug)]
pub struct Crossing {
    pub ford: bool,
    pub name: &'static str,
    pub x: f32,
    pub z: f32,
    pub width: f32,
}

pub struct CliffLine {
    pub line: Vec<LinePoint>,
    pub thick: f32,
    pub height: f32,
}

pub struct WallLine {
    pub line: Vec<LinePoint>,
    pub thick: f32,
    pub height: f32,
    pub low: bool,
    pub gaps: Vec<(f32, f32)>,
    pub gatehouse: Option<f32>,
}

struct RidgeLine {
    line: Vec<LinePoint>,
    width: f32,
    height: f32,
    steep: i32,
    taper: f32,
}

/// The 4 m grid for orders: free or not, and how far to the nearest obstacle.
pub struct Passability {
    pub pass: Vec<u8>,
    pub clear: Vec<f32>,
    pub pw: usize,
    pub ph: usize,
    pub px0: f32,
    pub pz0: f32,
    pub pc: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub z: f32,
}

pub struct Field {
    pub spec: MapSpec,
    pub res: f32,
    pub nx: usize,
    pub nz: usize,
    pub x0: f32,
    pub z0: f32,
    pub w: f32,
    pub d: f32,
    pub play: Rect,
    pub h: Vec<f32>,
    pub forest: Vec<f32>,
    pub river_d: Vec<f32>,
    pub road: Vec<f32>,
    pub marsh: Vec<f32>,
    pub cliff: Vec<f32>,
    pub river: Option<RiverLine>,
    pub bridge: Option<Bridge>,
    pub crossings: Vec<Crossing>,
    pub cliffs: Vec<CliffLine>,
    pub roads: Vec<Vec<LinePoint>>,
    pub walls: Vec<WallLine>,
    solids: Vec<(f32, f32, f32)>,
    pub grid: Passability,
    /// A battle map from the war map prototype (`baked.rs`), when the field is one: its land,
    /// what stands on it and the works that can be broken.
    pub baked: Option<Arc<BattleMap>>,
    /// The works broken open so far (their ground passable, rubble), by index.
    pub open: Vec<bool>,
    /// What armies build or bring that stands in the way: palisades and engines (`None` once
    /// gone), and where a siege tower's bridge is down on a wall (circles the wall is crossed in).
    pub barriers: Vec<Option<Barrier>>,
    pub gaps: Vec<Option<(f32, f32, f32)>>,
}

/// Something built on the field that feet cannot pass: a line (a palisade) or a disc (an engine).
#[derive(Clone, Copy, Debug)]
pub struct Barrier {
    pub ax: f32,
    pub az: f32,
    pub bx: f32,
    pub bz: f32,
    /// Half its thickness (a disc: its radius, a = b).
    pub half: f32,
}

impl Barrier {
    pub fn dist(&self, x: f32, z: f32) -> f32 {
        let (dx, dz) = (self.bx - self.ax, self.bz - self.az);
        let l2 = dx * dx + dz * dz;
        let t = if l2 > 1e-6 {
            (((x - self.ax) * dx + (z - self.az) * dz) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (x - self.ax - dx * t).hypot(z - self.az - dz * t)
    }
}

impl Field {
    pub fn build(spec: MapSpec) -> Field {
        let res = 2.0f32;
        let (w, d) = (spec.w, spec.d);
        let nx = (w / res).round() as usize + 1;
        let nz = (d / res).round() as usize + 1;
        let (x0, z0) = (-w / 2.0, -d / 2.0);
        let play = spec.play;
        let n = nx * nz;
        let mut h = vec![0.0f32; n];
        let mut forest = vec![0.0f32; n];
        let mut river_d = vec![1e3f32; n];
        let mut road = vec![0.0f32; n];
        let mut marsh_g = vec![0.0f32; n];
        let mut cliff_g = vec![0.0f32; n];
        let seed = spec.seed;
        let amp = spec.base;

        let river = spec.river.as_ref().map(|r| {
            let line = resample(&r.pts, 2.0, 3);
            let total = line.last().unwrap().s;
            RiverLine {
                total,
                w2: r.width / 2.0,
                bank: if r.shallow { 4.0 } else { 7.0 },
                ford: r
                    .ford
                    .map(|(at, len)| (at * total - len / 2.0, at * total + len / 2.0)),
                bridge_at: r.bridge.map(|at| at * total),
                bed_deep: if r.shallow { 0.95 } else { 2.9 },
                bed_ford: 0.98,
                line,
            }
        });
        let ridges: Vec<RidgeLine> = spec
            .ridges
            .iter()
            .map(|r| RidgeLine {
                line: resample(&r.pts, 2.0, 2),
                width: r.width,
                height: r.height,
                steep: r.steep,
                taper: r.taper,
            })
            .collect();
        let cliffs: Vec<CliffLine> = spec
            .cliffs
            .iter()
            .map(|c| CliffLine {
                line: resample(&c.pts, 2.0, 1),
                thick: c.thick,
                height: c.height,
            })
            .collect();
        let roads: Vec<Vec<LinePoint>> = spec.roads.iter().map(|p| resample(p, 2.0, 3)).collect();
        let causeways: Vec<(Vec<LinePoint>, f32)> = spec
            .causeways
            .iter()
            .map(|c| (resample(&c.pts, 2.0, 3), c.width))
            .collect();
        let walls: Vec<WallLine> = spec
            .walls
            .iter()
            .map(|w| WallLine {
                line: resample(&w.pts, 1.5, 0),
                thick: w.thick,
                height: w.height,
                low: w.low,
                gaps: w.gaps.iter().map(|g| (g.at, g.w)).collect(),
                gatehouse: w.gatehouse,
            })
            .collect();

        let outside = |x: f32, z: f32| {
            (play.x0 - x)
                .max(0.0)
                .max(x - play.x1)
                .hypot((play.z0 - z).max(0.0).max(z - play.z1))
        };

        for j in 0..nz {
            for i in 0..nx {
                let k = j * nx + i;
                let (x, z) = (x0 + i as f32 * res, z0 + j as f32 * res);
                // gentle rolling ground, never below the water
                let mut y = (fbm(x / 90.0, z / 90.0, seed, 3) - 0.5) * 2.0 * amp
                    + (fbm(x / 24.0, z / 24.0, seed + 9, 2) - 0.5) * 0.6;
                y = y.max(-0.2) + 0.3;
                for hl in &spec.hills {
                    let dd = (x - hl.x).hypot(z - hl.z) / hl.r;
                    if dd < 2.2 {
                        y += hl.h
                            * (-2.4 * dd * dd).exp()
                            * (0.92 + 0.16 * noise(x / 14.0, z / 14.0, seed + 3));
                    }
                }
                // ridges: a flat crest along a line, slopes either side (one side may be steep)
                for r in &ridges {
                    let near = line_near(x, z, &r.line);
                    let steep_side = r.steep != 0 && near.side.signum() as i32 == r.steep;
                    let wd = if steep_side { r.width * 0.42 } else { r.width };
                    let crest = r.width * 0.22;
                    if near.d < wd + 4.0 {
                        let t = smooth(wd, crest, near.d);
                        let end = r.line.last().unwrap().s;
                        let along = if r.taper > 0.0 {
                            smooth(0.0, r.taper, near.s).min(smooth(end, end - r.taper, near.s))
                        } else {
                            1.0
                        };
                        y += r.height
                            * t.powf(if steep_side { 0.8 } else { 1.4 })
                            * along
                            * (0.94 + 0.12 * noise(x / 9.0, z / 9.0, seed + 17));
                    }
                }
                let out = outside(x, z);
                if out > 0.0 {
                    y += 15.0
                        * smooth(0.0, 70.0, out)
                        * (0.7 + 0.6 * fbm(x / 40.0, z / 40.0, seed + 21, 2));
                }
                // marsh hollows and the causeways across them
                let mut marsh = 0.0f32;
                for m in &spec.marsh {
                    let dd = (x - m.x).hypot(z - m.z) / m.r;
                    marsh = marsh.max(smooth(
                        1.0,
                        0.55,
                        dd * (0.8 + 0.4 * noise(x / 9.0, z / 9.0, seed + 5)),
                    ));
                }
                if marsh > 0.0 {
                    y = y * (1.0 - marsh) + (WATER_Y - 0.32) * marsh;
                }
                for (line, width) in &causeways {
                    let near = line_near(x, z, line);
                    let t = smooth(width / 2.0 + 2.5, width / 2.0, near.d);
                    if t > 0.0 {
                        y = y.max(y * (1.0 - t) + (WATER_Y + 0.45) * t);
                    }
                }
                marsh_g[k] = marsh;
                if let Some(rv) = &river {
                    let near = line_near(x, z, &rv.line);
                    river_d[k] = near.d;
                    y *= smooth(rv.w2, rv.w2 + 46.0, near.d) * 0.85
                        + 0.15 * smooth(rv.w2 + 8.0, rv.w2 + 20.0, near.d);
                    let mut bed = rv.bed_deep;
                    if let Some((s0, s1)) = rv.ford {
                        let f = smooth(s0 - 10.0, s0 + 4.0, near.s)
                            * (1.0 - smooth(s1 - 4.0, s1 + 10.0, near.s));
                        bed += (rv.bed_ford - bed) * f;
                    }
                    y -= bed * (1.0 - smooth(rv.w2 * 0.25, rv.w2 + rv.bank, near.d));
                }
                // rock walls: the ground rises into a cliff band nobody can climb
                let mut cliff = 0.0f32;
                for c in &cliffs {
                    let near = line_near(x, z, &c.line);
                    let half = c.thick / 2.0;
                    if near.d < half + 6.0 {
                        let t = smooth(half + 5.0, half * 0.4, near.d);
                        let top = c.height * (0.85 + 0.3 * noise(x / 6.0, z / 6.0, seed + 31));
                        y = y.max(y + top * t.powf(0.6));
                        if near.d < half + 0.5 {
                            cliff = 1.0;
                        }
                    }
                }
                cliff_g[k] = cliff;
                h[k] = y;
                // woods: circles and outlined stands (with ragged edges), and the border's forest
                let mut f = 0.0f32;
                for p in &spec.forests {
                    match p {
                        crate::maps::Forest::Poly { pts, density } => {
                            let e = poly_edge(x, z, pts)
                                + (noise(x / 8.0, z / 8.0, seed + 7) - 0.5) * 9.0;
                            f = f.max(smooth(2.0, -6.0, e) * density);
                        }
                        crate::maps::Forest::Circle(c) => {
                            let dd = (x - c.x).hypot(z - c.z) / c.r;
                            f = f.max(smooth(
                                1.0,
                                0.62,
                                dd * (0.82 + 0.36 * noise(x / 11.0, z / 11.0, seed + 7)),
                            ));
                        }
                    }
                }
                if out > 4.0 {
                    f = f.max(
                        smooth(4.0, 26.0, out)
                            * smooth(0.34, 0.5, fbm(x / 30.0, z / 30.0, seed + 13, 2)),
                    );
                }
                if let Some(rv) = &river {
                    f *= smooth(rv.w2 + 3.0, rv.w2 + 10.0, river_d[k]);
                }
                for c in &spec.clearings {
                    f *= smooth(c.r * 0.7, c.r * 1.25, (x - c.x).hypot(z - c.z));
                }
                f *= (1.0 - marsh) * (1.0 - cliff);
                forest[k] = f;
            }
        }
        // roads: dirt tracks kept clear of trees
        if !roads.is_empty() {
            for j in 0..nz {
                for i in 0..nx {
                    let k = j * nx + i;
                    let (x, z) = (x0 + i as f32 * res, z0 + j as f32 * res);
                    let mut r = 0.0f32;
                    for line in &roads {
                        let dd = line_near(x, z, line).d;
                        r = r.max(smooth(
                            4.4,
                            2.3,
                            dd * (0.85 + 0.3 * noise(x / 5.0, z / 5.0, seed + 2)),
                        ));
                        forest[k] *= smooth(2.0, 8.0, dd);
                    }
                    road[k] = match &river {
                        Some(rv) if river_d[k] < rv.w2 + 1.0 => 0.0,
                        _ => r,
                    };
                }
            }
        }
        // the bridge
        let bridge = river.as_ref().and_then(|rv| {
            let at = rv.bridge_at?;
            let c = rv
                .line
                .iter()
                .min_by(|a, b| (a.s - at).abs().total_cmp(&(b.s - at).abs()))
                .copied()?;
            Some(Bridge {
                x: c.x,
                z: c.z,
                ux: -c.tz,
                uz: c.tx,
                vx: c.tx,
                vz: c.tz,
                half: rv.w2 + rv.bank + 3.0,
                width: 8.0,
                rise: 1.6,
                base: 0.35,
            })
        });
        let solids = spec
            .props
            .iter()
            .filter(|p| prop_footprint(p.kind) > 0.0)
            .map(|p| (p.x, p.z, prop_footprint(p.kind)))
            .collect();
        let mut field = Field {
            spec,
            res,
            nx,
            nz,
            x0,
            z0,
            w,
            d,
            play,
            h,
            forest,
            river_d,
            road,
            marsh: marsh_g,
            cliff: cliff_g,
            river,
            bridge,
            crossings: Vec::new(),
            cliffs,
            roads,
            walls,
            solids,
            grid: Passability {
                pass: Vec::new(),
                clear: Vec::new(),
                pw: 0,
                ph: 0,
                px0: 0.0,
                pz0: 0.0,
                pc: 4.0,
            },
            baked: None,
            open: Vec::new(),
            barriers: Vec::new(),
            gaps: Vec::new(),
        };
        field.build_grid();
        // crossings (for the minimap's labels and the AI's sense of the river)
        if let Some(rv) = &field.river {
            let at = |s: f32| {
                *rv.line
                    .iter()
                    .min_by(|a, b| (a.s - s).abs().total_cmp(&(b.s - s).abs()))
                    .unwrap()
            };
            if let Some((s0, s1)) = rv.ford {
                let c = at((s0 + s1) / 2.0);
                field.crossings.push(Crossing {
                    ford: true,
                    name: "el vado",
                    x: c.x,
                    z: c.z,
                    width: (s1 - s0) * 0.7,
                });
            }
        }
        if let Some(b) = field.bridge {
            field.crossings.push(Crossing {
                ford: false,
                name: "el puente de los trolls",
                x: b.x,
                z: b.z,
                width: b.width - 1.6,
            });
        }
        field
    }

    /// The field of a battle map from the prototype: its land and its structures as baked.
    pub fn from_baked(map: Arc<BattleMap>) -> Field {
        let l = &map.land;
        let (kind, title, blurb) = names(&map);
        let mut spec = crate::maps::empty(kind, title, blurb, 3, 0.0);
        spec.deploy_player = None;
        spec.deploy_enemy = None;
        let (nx, nz, res, x0, z0) = (l.nx, l.nz, l.res, l.x0, l.z0);
        let w = (nx - 1) as f32 * res;
        let d = (nz - 1) as f32 * res;
        let play = spec.play;
        let works = map.works.len();
        let mut field = Field {
            spec,
            res,
            nx,
            nz,
            x0,
            z0,
            w,
            d,
            play,
            h: l.h.clone(),
            forest: l.forest.clone(),
            river_d: vec![1e3; nx * nz],
            road: l.road.clone(),
            marsh: l.marsh.clone(),
            cliff: l.cliff.clone(),
            river: None,
            bridge: None,
            crossings: Vec::new(),
            cliffs: Vec::new(),
            roads: Vec::new(),
            walls: Vec::new(),
            solids: Vec::new(),
            grid: Passability {
                pass: Vec::new(),
                clear: Vec::new(),
                pw: 0,
                ph: 0,
                px0: 0.0,
                pz0: 0.0,
                pc: 2.0,
            },
            baked: Some(map),
            open: vec![false; works],
            barriers: Vec::new(),
            gaps: Vec::new(),
        };
        field.build_grid();
        field
    }

    /// A work has been broken (or rebuilt): its ground opens (closes) to feet and to paths.
    pub fn set_open(&mut self, work: usize, open: bool) {
        if work < self.open.len() && self.open[work] != open {
            self.open[work] = open;
            self.build_grid();
        }
    }

    /// Something built that stands in the way (paths go round it from now on); its index.
    pub fn add_barrier(&mut self, b: Barrier) -> usize {
        self.barriers.push(Some(b));
        self.build_grid();
        self.barriers.len() - 1
    }

    pub fn remove_barrier(&mut self, k: usize) {
        if let Some(slot) = self.barriers.get_mut(k) {
            if slot.take().is_some() {
                self.build_grid();
            }
        }
    }

    /// A siege tower's bridge down on a wall: the wall is crossed there; its index.
    pub fn add_gap(&mut self, x: f32, z: f32, r: f32) -> usize {
        self.gaps.push(Some((x, z, r)));
        self.build_grid();
        self.gaps.len() - 1
    }

    pub fn remove_gap(&mut self, k: usize) {
        if let Some(slot) = self.gaps.get_mut(k) {
            if slot.take().is_some() {
                self.build_grid();
            }
        }
    }

    fn in_gap(&self, x: f32, z: f32) -> bool {
        self.gaps
            .iter()
            .flatten()
            .any(|&(gx, gz, r)| (x - gx) * (x - gx) + (z - gz) * (z - gz) < r * r)
    }

    /// The top of whatever is here (ground, a deck, a wall's walk, a roof) and the work it is, if
    /// one: where a stone coming down lands.
    pub fn top_at(&self, x: f32, z: f32) -> (f32, Option<usize>) {
        let g = self.height_at(x, z);
        match self.structure(x, z) {
            Some((WORK, id, top)) if id > 0 && !self.open.get(id - 1).copied().unwrap_or(false) => {
                (top.max(g), Some(id - 1))
            }
            Some((SOLID, _, top)) | Some((DECK, _, top)) => (top.max(g), None),
            _ => (g, None),
        }
    }

    /// What stands on the baked map's ground here: (kind, work, top).
    fn structure(&self, x: f32, z: f32) -> Option<(u8, usize, f32)> {
        let b = &self.baked.as_ref()?.blocks;
        let k = b.cell(x, z)?;
        Some((b.kind[k], b.work[k] as usize, b.top[k]))
    }

    /// The work standing here (unbroken), if any.
    pub fn work_at(&self, x: f32, z: f32) -> Option<usize> {
        match self.structure(x, z)? {
            (WORK, id, _) if id > 0 && !self.open.get(id - 1).copied().unwrap_or(false) => {
                Some(id - 1)
            }
            _ => None,
        }
    }

    /// The water's surface here (`None` on dry land).
    fn water_surface(&self, x: f32, z: f32) -> Option<f32> {
        let m = self.baked.as_ref()?;
        let l = &m.land;
        let i = clamp(((x - l.x0) / l.res).round(), 0.0, l.nx as f32 - 1.0) as usize;
        let j = clamp(((z - l.z0) / l.res).round(), 0.0, l.nz as f32 - 1.0) as usize;
        let s = l.water[j * l.nx + i];
        (s > NO_WATER).then_some(s)
    }

    fn sample(&self, a: &[f32], x: f32, z: f32) -> f32 {
        let fx = clamp((x - self.x0) / self.res, 0.0, self.nx as f32 - 1.001);
        let fz = clamp((z - self.z0) / self.res, 0.0, self.nz as f32 - 1.001);
        let (i, j) = (fx as usize, fz as usize);
        let (u, v) = (fx - i as f32, fz - j as f32);
        let k = j * self.nx + i;
        let (p, q, r, s) = (a[k], a[k + 1], a[k + self.nx], a[k + self.nx + 1]);
        p + (q - p) * u + (r - p) * v + (p - q - r + s) * u * v
    }

    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.sample(&self.h, x, z)
    }

    /// The bridge's deck height here, if on it.
    pub fn deck_at(&self, x: f32, z: f32) -> Option<f32> {
        if self.baked.is_some() {
            return match self.structure(x, z) {
                Some((DECK, _, top)) => Some(top),
                _ => None,
            };
        }
        let b = self.bridge?;
        let (dx, dz) = (x - b.x, z - b.z);
        let u = dx * b.ux + dz * b.uz;
        let v = dx * b.vx + dz * b.vz;
        if u.abs() > b.half || v.abs() > b.width / 2.0 {
            return None;
        }
        let t = u / b.half;
        Some(b.base + b.rise * (1.0 - t * t))
    }

    pub fn walk_height(&self, x: f32, z: f32) -> f32 {
        let g = self.height_at(x, z);
        match self.deck_at(x, z) {
            Some(deck) if deck > g => deck,
            _ => g,
        }
    }

    pub fn depth_at(&self, x: f32, z: f32) -> f32 {
        if self.baked.is_some() {
            return match self.water_surface(x, z) {
                Some(s) => s - self.height_at(x, z),
                None => -10.0,
            };
        }
        WATER_Y - self.height_at(x, z)
    }

    /// The water's surface here, for what floats or splashes (the field's line, or the baked map's
    /// own level; `None` on a baked map's dry land).
    pub fn water_y(&self, x: f32, z: f32) -> Option<f32> {
        if self.baked.is_some() {
            self.water_surface(x, z)
        } else {
            Some(WATER_Y)
        }
    }

    pub fn forest_at(&self, x: f32, z: f32) -> f32 {
        self.sample(&self.forest, x, z)
    }

    pub fn marsh_at(&self, x: f32, z: f32) -> f32 {
        self.sample(&self.marsh, x, z)
    }

    pub fn road_at(&self, x: f32, z: f32) -> f32 {
        self.sample(&self.road, x, z)
    }

    pub fn cliff_at(&self, x: f32, z: f32) -> bool {
        self.sample(&self.cliff, x, z) > 0.5
    }

    pub fn wall_at(&self, x: f32, z: f32) -> bool {
        if self.baked.is_some() {
            return self.work_at(x, z).is_some();
        }
        for w in &self.walls {
            let near = line_near(x, z, &w.line);
            if near.d < w.thick / 2.0 + 0.6 {
                let gap = w
                    .gaps
                    .iter()
                    .any(|&(at, gw)| (near.s - at).abs() < gw / 2.0);
                if !gap {
                    return true;
                }
            }
        }
        false
    }

    pub fn blocked(&self, x: f32, z: f32) -> bool {
        if x < self.x0 + 4.0 || x > -self.x0 - 4.0 || z < self.z0 + 4.0 || z > -self.z0 - 4.0 {
            return true;
        }
        if self
            .barriers
            .iter()
            .flatten()
            .any(|b| b.dist(x, z) < b.half)
        {
            return true;
        }
        if self.baked.is_some() {
            match self.structure(x, z) {
                Some((SOLID, _, _)) => return true,
                Some((DECK, _, _)) => return false,
                Some((WORK, id, _)) if id > 0 && !self.open.get(id - 1).copied().unwrap_or(false) => {
                    if !self.in_gap(x, z) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        let deep = if self.baked.is_some() { DEEP_BAKED } else { DEEP };
        if self.depth_at(x, z) > deep && self.deck_at(x, z).is_none() {
            return true;
        }
        if self.cliff_at(x, z) {
            return true;
        }
        if self
            .solids
            .iter()
            .any(|&(sx, sz, r)| (x - sx) * (x - sx) + (z - sz) * (z - sz) < r * r)
        {
            return true;
        }
        !self.walls.is_empty() && self.wall_at(x, z)
    }

    /// How fast feet go here (woods and water slow them, roads help).
    pub fn speed_at(&self, x: f32, z: f32) -> f32 {
        let mut k = 1.0;
        let f = self.forest_at(x, z);
        if f > 0.3 {
            k *= 1.0 - 0.32 * f;
        }
        if self.deck_at(x, z).is_none() && self.depth_at(x, z) > 0.04 {
            k *= 0.55;
        }
        if self.road_at(x, z) > 0.5 {
            k *= 1.08;
        }
        k
    }

    pub fn grad_at(&self, x: f32, z: f32) -> (f32, f32) {
        (
            (self.height_at(x + 1.0, z) - self.height_at(x - 1.0, z)) / 2.0,
            (self.height_at(x, z + 1.0) - self.height_at(x, z - 1.0)) / 2.0,
        )
    }

    /// The side of the river a point is on (±1; 0 without a river).
    pub fn river_side(&self, x: f32, z: f32) -> f32 {
        self.river.as_ref().map_or(0.0, |rv| {
            let s = line_near(x, z, &rv.line).side.signum();
            if s == 0.0 {
                1.0
            } else {
                s
            }
        })
    }

    // ---------------------------------------------------------- passability
    /// Works out again where feet can go (after something was built or broke).
    pub fn rebuild_grid(&mut self) {
        self.build_grid();
    }

    fn build_grid(&mut self) {
        // (the baked maps' gates and lanes are narrow: a finer grid keeps them open to paths)
        let pc = if self.baked.is_some() { 2.0f32 } else { 4.0 };
        let probe = pc * 0.35;
        let (px0, pz0) = (self.x0 + 6.0, self.z0 + 6.0);
        let pw = ((self.w - 12.0) / pc).floor() as usize;
        let ph = ((self.d - 12.0) / pc).floor() as usize;
        let mut pass = vec![0u8; pw * ph];
        for j in 0..ph {
            for i in 0..pw {
                let cx = px0 + (i as f32 + 0.5) * pc;
                let cz = pz0 + (j as f32 + 0.5) * pc;
                let mut free = !self.blocked(cx, cz);
                if free {
                    for (ox, oz) in [(-probe, -probe), (probe, -probe), (-probe, probe), (probe, probe)] {
                        if self.blocked(cx + ox, cz + oz) {
                            free = false;
                        }
                    }
                }
                pass[j * pw + i] = free as u8;
            }
        }
        // clearance: multi-source BFS from blocked cells (8-neighbour chamfer)
        let mut clear = vec![1e9f32; pw * ph];
        let mut q: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for k in 0..pw * ph {
            if pass[k] == 0 {
                clear[k] = 0.0;
                q.push_back(k);
            }
        }
        // the field's edge counts as an obstacle too
        for i in 0..pw {
            for j in [0, ph - 1] {
                let k = j * pw + i;
                if pass[k] == 1 {
                    clear[k] = clear[k].min(1.0);
                    q.push_back(k);
                }
            }
        }
        while let Some(k) = q.pop_front() {
            let (i, j) = ((k % pw) as i32, (k / pw) as i32);
            for dj in -1..=1i32 {
                for di in -1..=1i32 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let (ii, jj) = (i + di, j + dj);
                    if ii < 0 || jj < 0 || ii >= pw as i32 || jj >= ph as i32 {
                        continue;
                    }
                    let kk = jj as usize * pw + ii as usize;
                    let nd = clear[k] + if di != 0 && dj != 0 { 1.414 } else { 1.0 };
                    if nd < clear[kk] {
                        clear[kk] = nd;
                        q.push_back(kk);
                    }
                }
            }
        }
        self.grid = Passability {
            pass,
            clear,
            pw,
            ph,
            px0,
            pz0,
            pc,
        };
    }

    fn cell_of(&self, x: f32, z: f32) -> (usize, usize) {
        let g = &self.grid;
        (
            clamp(((x - g.px0) / g.pc).floor(), 0.0, g.pw as f32 - 1.0) as usize,
            clamp(((z - g.pz0) / g.pc).floor(), 0.0, g.ph as f32 - 1.0) as usize,
        )
    }

    /// Metres from (x, z) to the nearest thing in the way.
    pub fn clearance_at(&self, x: f32, z: f32) -> f32 {
        let (i, j) = self.cell_of(x, z);
        self.grid.clear[j * self.grid.pw + i] * self.grid.pc
    }

    pub fn pass_at(&self, x: f32, z: f32) -> bool {
        let (i, j) = self.cell_of(x, z);
        self.grid.pass[j * self.grid.pw + i] == 1
    }

    pub fn line_clear(&self, ax: f32, az: f32, bx: f32, bz: f32) -> bool {
        let n = ((bx - ax).hypot(bz - az) / 2.5).ceil() as usize;
        (1..=n).all(|s| {
            let t = s as f32 / n as f32;
            self.pass_at(ax + (bx - ax) * t, az + (bz - az) * t)
        })
    }

    pub fn nearest_free(&self, x: f32, z: f32) -> Point {
        let (ci, cj) = self.cell_of(x, z);
        let g = &self.grid;
        for r in 0..30i32 {
            for dj in -r..=r {
                for di in -r..=r {
                    if di.abs().max(dj.abs()) != r {
                        continue;
                    }
                    let (i, j) = (ci as i32 + di, cj as i32 + dj);
                    if i < 0 || j < 0 || i >= g.pw as i32 || j >= g.ph as i32 {
                        continue;
                    }
                    if g.pass[j as usize * g.pw + i as usize] == 1 {
                        return Point {
                            x: g.px0 + (i as f32 + 0.5) * g.pc,
                            z: g.pz0 + (j as f32 + 0.5) * g.pc,
                        };
                    }
                }
            }
        }
        Point { x, z }
    }

    /// A* from a to b round obstacles, preferring roads and open ground; smoothed by line of sight.
    pub fn find_path(&self, ax: f32, az: f32, bx: f32, bz: f32) -> Vec<Point> {
        if self.line_clear(ax, az, bx, bz) {
            return vec![Point { x: bx, z: bz }];
        }
        let goal = if self.pass_at(bx, bz) {
            Point { x: bx, z: bz }
        } else {
            self.nearest_free(bx, bz)
        };
        let start = if self.pass_at(ax, az) {
            Point { x: ax, z: az }
        } else {
            self.nearest_free(ax, az)
        };
        let g = &self.grid;
        let (si, sj) = self.cell_of(start.x, start.z);
        let (gi, gj) = self.cell_of(goal.x, goal.z);
        let (s_k, g_k) = (sj * g.pw + si, gj * g.pw + gi);
        let mut cost = vec![f32::INFINITY; g.pw * g.ph];
        let mut came = vec![usize::MAX; g.pw * g.ph];
        let mut open = std::collections::BinaryHeap::new();
        cost[s_k] = 0.0;
        open.push(Open(0.0, s_k));
        let mut found = false;
        let mut iter = 0;
        while let Some(Open(_, k)) = open.pop() {
            iter += 1;
            if iter > 60_000 {
                break;
            }
            if k == g_k {
                found = true;
                break;
            }
            let (i, j) = ((k % g.pw) as i32, (k / g.pw) as i32);
            for dj in -1..=1i32 {
                for di in -1..=1i32 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let (ii, jj) = (i + di, j + dj);
                    if ii < 0 || jj < 0 || ii >= g.pw as i32 || jj >= g.ph as i32 {
                        continue;
                    }
                    let kk = jj as usize * g.pw + ii as usize;
                    if g.pass[kk] == 0 {
                        continue;
                    }
                    if di != 0
                        && dj != 0
                        && (g.pass[j as usize * g.pw + ii as usize] == 0
                            || g.pass[jj as usize * g.pw + i as usize] == 0)
                    {
                        continue;
                    }
                    let cx = g.px0 + (ii as f32 + 0.5) * g.pc;
                    let cz = g.pz0 + (jj as f32 + 0.5) * g.pc;
                    let step = (if di != 0 && dj != 0 { 1.414 } else { 1.0 })
                        * (1.0
                            + 0.5 * self.forest_at(cx, cz)
                            + if self.depth_at(cx, cz) > 0.05 {
                                0.6
                            } else {
                                0.0
                            }
                            - 0.15 * self.road_at(cx, cz)
                            + if g.clear[kk] < 2.0 { 0.4 } else { 0.0 });
                    let ng = cost[k] + step;
                    if ng < cost[kk] {
                        cost[kk] = ng;
                        came[kk] = k;
                        let hh = ((ii - gi as i32) as f32).hypot((jj - gj as i32) as f32);
                        open.push(Open(ng + hh, kk));
                    }
                }
            }
        }
        if !found {
            return vec![Point { x: bx, z: bz }];
        }
        let mut cells = Vec::new();
        let mut k = g_k;
        while k != usize::MAX && k != s_k {
            cells.push(Point {
                x: g.px0 + ((k % g.pw) as f32 + 0.5) * g.pc,
                z: g.pz0 + ((k / g.pw) as f32 + 0.5) * g.pc,
            });
            k = came[k];
        }
        cells.reverse();
        // string-pulling: keep only the turns line of sight needs
        let mut out = Vec::new();
        let mut from = start;
        let mut idx = 0;
        while idx < cells.len() {
            let mut far = idx;
            for t in (idx + 1..cells.len()).rev() {
                if self.line_clear(from.x, from.z, cells[t].x, cells[t].z) {
                    far = t;
                    break;
                }
            }
            out.push(cells[far]);
            from = cells[far];
            idx = far + 1;
        }
        if let Some(last) = out.last_mut() {
            *last = goal;
        } else {
            out.push(goal);
        }
        out
    }

    pub fn route(&self, ax: f32, az: f32, bx: f32, bz: f32) -> Vec<Point> {
        self.find_path(ax, az, bx, bz)
    }

    /// A rough length of the way (straight, or round something).
    pub fn route_length(&self, ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
        let d = (bx - ax).hypot(bz - az);
        if self.line_clear(ax, az, bx, bz) {
            d
        } else {
            d * 1.5 + 30.0
        }
    }
}

/// The battle map's names, as the field's spec wants them (static: one copy per map, kept).
fn names(map: &BattleMap) -> (&'static str, &'static str, &'static str) {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<HashMap<String, (&'static str, &'static str, &'static str)>>> =
        OnceLock::new();
    let mut m = NAMES.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap();
    *m.entry(map.id.clone()).or_insert_with(|| {
        (
            Box::leak(map.id.clone().into_boxed_str()),
            Box::leak(map.name.clone().into_boxed_str()),
            Box::leak(map.blurb.clone().into_boxed_str()),
        )
    })
}

/// An open A* node, smallest first.
struct Open(f32, usize);

impl PartialEq for Open {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for Open {}
impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Open {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.total_cmp(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::{map_spec, MAP_IDS};

    #[test]
    fn every_field_builds_with_open_ground_and_a_way_across() {
        for id in MAP_IDS {
            let f = Field::build(map_spec(id, None));
            let free = f.grid.pass.iter().filter(|&&p| p == 1).count();
            assert!(free > f.grid.pass.len() / 3, "{id}: too little open ground");
            // from the player's side to the enemy's, there is always a way
            let path = f.find_path(0.0, 120.0, 0.0, -120.0);
            let end = path.last().unwrap();
            assert!(end.z < -60.0, "{id}: no way north ({end:?})");
        }
    }

    #[test]
    fn the_ford_is_shallow_and_the_river_deep() {
        let f = Field::build(map_spec("vado", None));
        let rv = f.river.as_ref().unwrap();
        let c = f.crossings.iter().find(|c| c.ford).unwrap();
        assert!(!f.blocked(c.x, c.z), "the ford can be waded");
        let deep = rv.line[rv.line.len() / 8];
        assert!(f.blocked(deep.x, deep.z), "the river cannot");
        let b = f.bridge.unwrap();
        assert!(!f.blocked(b.x, b.z) && f.deck_at(b.x, b.z).is_some());
    }
}
