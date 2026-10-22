//! La guía de la misión seguida, como un GPS: adónde ir aquí (el sitio del paso, o la puerta hacia
//! la zona donde está), el camino hasta allí por el suelo que se puede pisar (A* sobre una rejilla de
//! cada lugar, con las esquinas recortadas), y cómo se ve: flechas que corren por el suelo delante de
//! la Reina y una columna de luz en el destino. La línea del minimapa, el rombo en pantalla y el
//! panel de la misión los dibuja el overlay (`overlay/mission_overlay.rs`).
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::missions;
use super::*;

/// No guide at all (the presentation video's footage: its running arrows and light column read
/// as flicker there).
pub(crate) static GUIDE_HIDDEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// How near the target counts as there.
const ARRIVED: f32 = 2.0;

// ------------------------------------------------------------------ the world's zones
/// A mission's zone by the name the catalog gives it.
pub(super) fn zone_named(name: &str) -> Option<ZoneId> {
    Some(match name {
        "castillo" => ZoneId::Castle,
        "bosque" => ZoneId::Forest,
        "villa" => ZoneId::Village,
        "penascal" | "peñascal" => ZoneId::Penascal,
        "termas" => ZoneId::Baths,
        "burdel" => ZoneId::Brothel,
        _ => return None,
    })
}

/// A zone as the panel names it.
pub(super) fn zone_title(zone: ZoneId) -> &'static str {
    match zone {
        ZoneId::Castle => "el castillo",
        ZoneId::Forest => "el bosque",
        ZoneId::Village => "la villa",
        ZoneId::Penascal => "el Peñascal",
        ZoneId::Baths => "las termas",
        ZoneId::Brothel => "el burdel",
        ZoneId::Dungeon => "la mina",
        ZoneId::Boss => "el combate",
        ZoneId::War => "la mesa de guerra",
    }
}

/// The zones a door leads to from each.
fn neighbours(zone: ZoneId) -> &'static [ZoneId] {
    match zone {
        ZoneId::Castle => &[ZoneId::Forest],
        ZoneId::Forest => &[ZoneId::Castle, ZoneId::Village, ZoneId::Penascal],
        ZoneId::Village => &[ZoneId::Forest, ZoneId::Baths, ZoneId::Brothel],
        ZoneId::Penascal => &[ZoneId::Forest],
        ZoneId::Baths | ZoneId::Brothel => &[ZoneId::Village],
        ZoneId::Dungeon => &[ZoneId::Forest],
        ZoneId::Boss | ZoneId::War => &[],
    }
}

/// The next zone on the way from `from` to `to` (the doors walked breadth first).
pub(super) fn next_zone(from: ZoneId, to: ZoneId) -> Option<ZoneId> {
    if from == to {
        return Some(to);
    }
    let mut seen = vec![from];
    let mut queue = VecDeque::from([(from, None::<ZoneId>)]);
    while let Some((zone, first)) = queue.pop_front() {
        for &n in neighbours(zone) {
            if seen.contains(&n) {
                continue;
            }
            seen.push(n);
            let first = first.or(Some(n));
            if n == to {
                return first;
            }
            queue.push_back((n, first));
        }
    }
    None
}

// ------------------------------------------------------------------ the grids
/// Where one can stand in a place, on a grid: each cell free or not (for a body of the Queen's
/// size), made once per place and floor.
pub(super) struct Grid {
    x0: f32,
    z0: f32,
    step: f32,
    nx: usize,
    nz: usize,
    free: Vec<bool>,
}

impl Grid {
    fn build(bounds: [f32; 4], step: f32, blocked: impl Fn(f32, f32) -> bool) -> Self {
        let [x0, x1, z0, z1] = bounds;
        let nx = ((x1 - x0) / step).ceil().max(1.0) as usize + 1;
        let nz = ((z1 - z0) / step).ceil().max(1.0) as usize + 1;
        let mut free = Vec::with_capacity(nx * nz);
        for j in 0..nz {
            for i in 0..nx {
                free.push(!blocked(x0 + i as f32 * step, z0 + j as f32 * step));
            }
        }
        Self {
            x0,
            z0,
            step,
            nx,
            nz,
            free,
        }
    }

    fn centre(&self, i: usize, j: usize) -> (f32, f32) {
        (
            self.x0 + i as f32 * self.step,
            self.z0 + j as f32 * self.step,
        )
    }

    fn at(&self, i: isize, j: isize) -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < self.nx
            && (j as usize) < self.nz
            && self.free[j as usize * self.nx + i as usize]
    }

    fn cell(&self, x: f32, z: f32) -> (isize, isize) {
        (
            ((x - self.x0) / self.step).round() as isize,
            ((z - self.z0) / self.step).round() as isize,
        )
    }

    /// The free cell nearest (x, z), looked for in growing rings up to `reach` cells.
    fn nearest_free(&self, x: f32, z: f32, reach: isize) -> Option<(usize, usize)> {
        let (ci, cj) = self.cell(x, z);
        let mut best: Option<((usize, usize), f32)> = None;
        for r in 0..=reach {
            for j in cj - r..=cj + r {
                for i in ci - r..=ci + r {
                    if (i - ci).abs() != r && (j - cj).abs() != r {
                        continue;
                    }
                    if self.at(i, j) {
                        let (px, pz) = self.centre(i as usize, j as usize);
                        let d = (px - x).hypot(pz - z);
                        if best.map_or(true, |(_, b)| d < b) {
                            best = Some(((i as usize, j as usize), d));
                        }
                    }
                }
            }
            if best.is_some() {
                return best.map(|(c, _)| c);
            }
        }
        None
    }

    /// Whether the straight line a → b crosses only free cells.
    fn clear(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let n = (len / (self.step * 0.5)).ceil().max(1.0) as usize;
        (0..=n).all(|k| {
            let t = k as f32 / n as f32;
            let (i, j) = self.cell(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            self.at(i, j)
        })
    }

    /// The way from `from` to `to`: A* on the cells (diagonals only where both sides are free),
    /// then pulled straight wherever the ground between two corners is clear.
    fn path(&self, from: (f32, f32), to: (f32, f32)) -> Option<Vec<(f32, f32)>> {
        let start = self.nearest_free(from.0, from.1, 14)?;
        let goal = self.nearest_free(to.0, to.1, 24)?;
        let index = |(i, j): (usize, usize)| j * self.nx + i;
        let (gi, gj) = (goal.0 as i64, goal.1 as i64);
        let guess = |i: usize, j: usize| {
            let (dx, dz) = (
                (i as i64 - gi).unsigned_abs(),
                (j as i64 - gj).unsigned_abs(),
            );
            (10 * dx.max(dz) + 4 * dx.min(dz)) as u32
        };
        let n = self.nx * self.nz;
        let mut cost = vec![u32::MAX; n];
        let mut came = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        cost[index(start)] = 0;
        heap.push(Reverse((guess(start.0, start.1), index(start) as u32)));
        let mut reached = false;
        while let Some(Reverse((_, at))) = heap.pop() {
            let at = at as usize;
            if at == index(goal) {
                reached = true;
                break;
            }
            let (i, j) = ((at % self.nx) as isize, (at / self.nx) as isize);
            let here = cost[at];
            for (di, dj) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (a, b) = (i + di, j + dj);
                if !self.at(a, b) {
                    continue;
                }
                let diagonal = di != 0 && dj != 0;
                if diagonal && !(self.at(i + di, j) && self.at(i, j + dj)) {
                    continue;
                }
                let next = b as usize * self.nx + a as usize;
                let c = here + if diagonal { 14 } else { 10 };
                if c < cost[next] {
                    cost[next] = c;
                    came[next] = at as u32;
                    heap.push(Reverse((c + guess(a as usize, b as usize), next as u32)));
                }
            }
        }
        if !reached {
            return None;
        }
        let mut cells = Vec::new();
        let mut at = index(goal);
        loop {
            cells.push(self.centre(at % self.nx, at / self.nx));
            if at == index(start) {
                break;
            }
            at = came[at] as usize;
        }
        cells.reverse();
        // Pulled straight: from each corner, on to the furthest cell still in clear sight.
        let mut out = vec![from];
        let mut i = 0;
        let mut last = from;
        while i + 1 < cells.len() {
            let mut j = i + 1;
            while j + 1 < cells.len() && self.clear(last, cells[j + 1]) {
                j += 1;
            }
            out.push(cells[j]);
            last = cells[j];
            i = j;
        }
        if (last.0 - to.0).hypot(last.1 - to.1) > 0.05 && self.clear(last, to) {
            out.push(to);
        }
        Some(out)
    }
}

/// The grid of a zone (and the castle's floor), made the first time it is wanted.
fn grid(zone: ZoneId, level: u8) -> Option<&'static Grid> {
    static CASTLE: [OnceLock<Option<Grid>>; 2] = [OnceLock::new(), OnceLock::new()];
    static FOREST: OnceLock<Option<Grid>> = OnceLock::new();
    static VILLAGE: OnceLock<Option<Grid>> = OnceLock::new();
    static PENASCAL: OnceLock<Option<Grid>> = OnceLock::new();
    static BATHS: OnceLock<Option<Grid>> = OnceLock::new();
    let made = |cell: &'static OnceLock<Option<Grid>>, make: &dyn Fn() -> Option<Grid>| {
        cell.get_or_init(|| {
            let started = Instant::now();
            let g = make();
            if let Some(g) = &g {
                eprintln!(
                    "[guía] rejilla {zone:?}/{level}: {}×{} en {:.0} ms",
                    g.nx,
                    g.nz,
                    started.elapsed().as_secs_f64() * 1000.0
                );
            }
            g
        })
        .as_ref()
    };
    match zone {
        ZoneId::Castle => made(&CASTLE[usize::from(level.min(1))], &|| {
            let p = castle3d::plan();
            let rooms: Vec<_> = p.rooms.iter().filter(|r| r.level == level).collect();
            if rooms.is_empty() {
                return None;
            }
            let b = rooms
                .iter()
                .fold([f32::MAX, f32::MIN, f32::MAX, f32::MIN], |b, r| {
                    [
                        b[0].min(r.x0),
                        b[1].max(r.x1),
                        b[2].min(r.z0),
                        b[3].max(r.z1),
                    ]
                });
            Some(Grid::build(b, 0.25, |x, z| p.blocked(x, z, 0.3, level)))
        }),
        ZoneId::Forest => made(&FOREST, &|| {
            Some(Grid::build(dark_forest::walk_bounds(), 0.4, |x, z| {
                dark_forest::blocked(Vec3::new(x, 0.0, z))
            }))
        }),
        ZoneId::Village => made(&VILLAGE, &|| {
            Some(Grid::build(village::walk_bounds(), 0.35, |x, z| {
                village::blocked(Vec3::new(x, 0.0, z))
            }))
        }),
        ZoneId::Penascal => made(&PENASCAL, &|| {
            if !penascal3d::available() {
                return None;
            }
            let p = penascal3d::plan();
            Some(Grid::build(p.bounds(), 0.3, |x, z| p.blocked(x, z, 0.3)))
        }),
        // the springs' valley (its rooms are small: the way across them is straight)
        ZoneId::Baths => made(&BATHS, &|| {
            let p = baths::plan();
            let h = p.half;
            Some(Grid::build([-h, h, -h, h], 0.35, |x, z| {
                p.blocked_outdoor(x, z, 0.3)
            }))
        }),
        _ => None,
    }
}

// ------------------------------------------------------------------ what the guide shows
/// The followed mission's guide this step: what it says, where to go here and the way there.
#[derive(Clone, Debug, Default)]
pub(crate) struct Guide {
    pub title: String,
    pub step: String,
    pub detail: String,
    pub main: bool,
    /// How many missions are open (J goes through them) and the others' titles.
    pub open: usize,
    pub others: Vec<String>,
    /// The zone the step is in, when it is not this one.
    pub elsewhere: Option<&'static str>,
    /// Where to walk to here (the step's spot, or the door on the way), and its name.
    pub target: Option<Vec3>,
    pub target_label: String,
    /// The way there from her feet (on the ground), and how long it is.
    pub route: Vec<Vec3>,
    pub distance: f32,
    /// She is at the step's own spot.
    pub arrived: bool,
}

/// The way last found: kept while she follows it.
#[derive(Clone, Debug)]
pub(crate) struct Route {
    zone: ZoneId,
    level: u8,
    goal: Vec3,
    from: Vec3,
    at: f32,
    points: Vec<Vec3>,
}

/// The castle's stair ends: its foot on the ground floor and its top upstairs.
fn stair_ends() -> Option<(Vec3, Vec3)> {
    let p = castle3d::plan();
    let s = p.stair.as_ref()?;
    let cx = (s.x0 + s.x1) * 0.5;
    Some((
        Vec3::new(cx, 0.0, s.z_bottom + 0.55),
        Vec3::new(cx, p.level_h, s.z_top - 0.55),
    ))
}

impl App {
    /// A step's spot in its zone: where it is (a maid where she walks now), its name, its floor.
    fn spot_in(&self, zone: ZoneId, id: &str) -> Option<(Vec3, String, u8)> {
        if id.is_empty() {
            return None;
        }
        match zone {
            ZoneId::Castle => {
                let p = castle3d::plan();
                let h = p.hotspots.iter().find(|h| h.id == id)?;
                let at = match (id, self.castle.as_ref()) {
                    ("lira", Some(c)) => c.maid(0).0,
                    ("nemi", Some(c)) => c.maid(1).0,
                    _ => Vec3::new(h.x, 0.0, h.z),
                };
                let y = h.level as f32 * p.level_h;
                Some((Vec3::new(at.x, y, at.z), h.label.clone(), h.level))
            }
            ZoneId::Forest => dark_forest::spot(id).map(|(at, title)| (at, title.to_owned(), 0)),
            ZoneId::Village => village::spot(id).map(|(at, title)| (at, title.to_owned(), 0)),
            ZoneId::Penascal => {
                let h = penascal3d::plan().hotspot(id)?;
                Some((Vec3::new(h.x, 0.0, h.z), h.label.clone(), 0))
            }
            ZoneId::Baths => self.baths_spot(id),
            _ => None,
        }
    }

    /// A spot of the springs, out in the valley or in one of its rooms: there when she is where it
    /// is, else the door on the way (the room's way out, or the valley's door into its room).
    fn baths_spot(&self, id: &str) -> Option<(Vec3, String, u8)> {
        let inside = self.baths.as_ref()?;
        let p = baths::plan();
        let valley =
            |h: &baths::Hotspot| Some((Vec3::new(h.x, p.stand(h.x, h.z), h.z), h.label.clone(), 0));
        let in_room = |h: &baths::Hotspot| Some((Vec3::new(h.x, 0.0, h.z), h.label.clone(), 0));
        // out of the room she is in, to the valley
        let way_out = || {
            let room = p.rooms.get(inside.room.as_deref()?)?;
            let h = room.hotspots.iter().find(|h| h.exit)?;
            in_room(h)
        };
        if let Some(h) = p.hotspots.iter().find(|h| h.id == id) {
            return match inside.room {
                None => valley(h),
                Some(_) => way_out(),
            };
        }
        let (room_id, h) = p.rooms.iter().find_map(|(rid, r)| {
            r.hotspots
                .iter()
                .find(|h| h.id == id && !h.exit)
                .map(|h| (rid.as_str(), h))
        })?;
        match inside.room.as_deref() {
            Some(here) if here == room_id => in_room(h),
            Some(_) => way_out(),
            None => valley(
                p.hotspots
                    .iter()
                    .find(|d| d.enter.as_deref() == Some(room_id))?,
            ),
        }
    }

    /// The door in `here` that leads to `next`: where it is, its name.
    fn door_to(&self, here: ZoneId, next: ZoneId) -> Option<(Vec3, String, u8)> {
        let (at, name) = match (here, next) {
            (ZoneId::Castle, ZoneId::Forest) => {
                let h = castle3d::plan()
                    .hotspots
                    .iter()
                    .find(|h| h.id == castle3d::GATE)?;
                (Vec3::new(h.x, 0.0, h.z), "La puerta del castillo")
            }
            (ZoneId::Forest, ZoneId::Castle) => (castle_entry_position(), "El castillo"),
            (ZoneId::Forest, ZoneId::Village) => (village_entry_position(), "La villa"),
            (ZoneId::Forest, ZoneId::Penascal) => {
                (dark_forest::exit("penascal")?.0, "El camino del Peñascal")
            }
            (ZoneId::Village, ZoneId::Forest) => (village::entrance(), "El camino del bosque"),
            (ZoneId::Village, ZoneId::Baths) => {
                let (at, _) = village::spot("villa_banos_termales")?;
                (at, "Las termas")
            }
            (ZoneId::Baths, ZoneId::Village) => return self.baths_spot("salida"),
            (ZoneId::Penascal, ZoneId::Forest) => {
                let h = penascal3d::plan().hotspot("salida")?;
                (Vec3::new(h.x, 0.0, h.z), "La bajada al bosque")
            }
            _ => return None,
        };
        Some((at, name.to_owned(), 0))
    }

    /// The ground's height under (x, z) in a zone (for the arrows on it).
    fn guide_ground(zone: ZoneId, x: f32, z: f32, level_y: f32) -> f32 {
        match zone {
            ZoneId::Forest => nature_walk_surface_height(x, z),
            ZoneId::Village => village::surface_height(x, z),
            ZoneId::Castle => level_y,
            // (the rooms stand far off, on their own floor)
            ZoneId::Baths if x < 200.0 => baths::plan().stand(x, z),
            _ => 0.0,
        }
    }

    /// Each step: the followed mission's guide (its way found again when she has moved off it, the
    /// target has moved, or it has grown old).
    pub(super) fn tick_guide(&mut self) {
        let hidden = GUIDE_HIDDEN.load(std::sync::atomic::Ordering::Relaxed);
        let Some((mission, step)) = self.followed_mission().filter(|_| !hidden) else {
            self.guide = None;
            self.route = None;
            return;
        };
        let open = missions::open(&self.story);
        let s = &mission.steps[step];
        let mut guide = Guide {
            title: mission.title.clone(),
            step: s.text.clone(),
            detail: s.detail.clone(),
            main: mission.main(),
            open: open.len(),
            others: open
                .iter()
                .filter(|(m, _)| m.id != mission.id)
                .take(2)
                .map(|(m, _)| m.title.clone())
                .collect(),
            ..Guide::default()
        };
        let here = self.zone_id();
        if here == ZoneId::Boss {
            self.route = None;
            self.guide = None;
            return;
        }
        let goal_zone = zone_named(&s.zone);
        let found = match goal_zone {
            Some(z) if z == here => self.spot_in(here, &s.target),
            Some(z) => {
                guide.elsewhere = Some(zone_title(z));
                next_zone(here, z).and_then(|n| self.door_to(here, n))
            }
            None => None,
        };
        let Some((target, label, target_level)) = found else {
            self.route = None;
            self.guide = Some(guide);
            return;
        };
        guide.target = Some(target);
        guide.target_label = label;
        guide.route = self.route_to(here, target, target_level);
        guide.distance = guide
            .route
            .windows(2)
            .map(|w| Vec3::new(w[1].x - w[0].x, 0.0, w[1].z - w[0].z).length())
            .sum();
        if guide.route.is_empty() {
            guide.distance =
                Vec3::new(target.x - self.player.x, 0.0, target.z - self.player.z).length();
        }
        guide.arrived = guide.elsewhere.is_none() && guide.distance < ARRIVED;
        self.guide = Some(guide);
    }

    /// The way from her feet to `target` in this zone (in the castle by the stair when the target
    /// is on the other floor), kept from the last search while she follows it.
    fn route_to(&mut self, zone: ZoneId, target: Vec3, target_level: u8) -> Vec<Vec3> {
        let me = self.player;
        let (level, on_stair) = match (zone, self.castle.as_ref()) {
            (ZoneId::Castle, Some(c)) => (c.level, c.on_stair),
            _ => (0, false),
        };
        let level_h = castle3d::plan().level_h;
        let stale = self.route.as_ref().map_or(true, |r| {
            r.zone != zone
                || r.level != level
                || (r.goal - target).length() > 0.6
                || Vec3::new(me.x - r.from.x, 0.0, me.z - r.from.z).length() > 4.0
                || self.elapsed - r.at > 2.5
                || off_route(&r.points, me) > 1.4
        });
        if stale {
            let ground =
                |p: (f32, f32), y: f32| Vec3::new(p.0, Self::guide_ground(zone, p.0, p.1, y), p.1);
            let leg = |from: Vec3, to: Vec3, floor: u8| -> Vec<Vec3> {
                let y = floor as f32 * level_h;
                match grid(zone, floor).and_then(|g| g.path((from.x, from.z), (to.x, to.z))) {
                    Some(points) => points.into_iter().map(|p| ground(p, y)).collect(),
                    None => vec![ground((from.x, from.z), y), ground((to.x, to.z), y)],
                }
            };
            let points = if zone == ZoneId::Castle && (on_stair || level != target_level) {
                match stair_ends() {
                    Some((foot, top)) => {
                        let (near, far) = if target_level == 1 {
                            (foot, top)
                        } else {
                            (top, foot)
                        };
                        let mut way = if on_stair {
                            vec![me]
                        } else {
                            leg(me, near, level)
                        };
                        way.push(near);
                        way.extend(leg(far, target, target_level));
                        way
                    }
                    None => leg(me, target, level),
                }
            } else {
                leg(me, target, level)
            };
            self.route = Some(Route {
                zone,
                level,
                goal: target,
                from: me,
                at: self.elapsed,
                points,
            });
        }
        // From her feet, on from the point of the way nearest her.
        let points = &self
            .route
            .as_ref()
            .map(|r| r.points.clone())
            .unwrap_or_default();
        if points.len() < 2 {
            return Vec::new();
        }
        let nearest = (0..points.len() - 1)
            .min_by(|&a, &b| {
                segment_distance(me, points[a], points[a + 1]).total_cmp(&segment_distance(
                    me,
                    points[b],
                    points[b + 1],
                ))
            })
            .unwrap_or(0);
        let y = Self::guide_ground(
            zone,
            me.x,
            me.z,
            if on_stair {
                me.y
            } else {
                level as f32 * level_h
            },
        );
        let mut way = vec![Vec3::new(me.x, y, me.z)];
        way.extend_from_slice(&points[nearest + 1..]);
        way
    }
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (abx, abz) = (b.x - a.x, b.z - a.z);
    let len2 = abx * abx + abz * abz;
    let t = if len2 > 1e-6 {
        (((p.x - a.x) * abx + (p.z - a.z) * abz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a.x + abx * t - p.x).hypot(a.z + abz * t - p.z)
}

/// How far she is from the way.
fn off_route(points: &[Vec3], p: Vec3) -> f32 {
    points
        .windows(2)
        .map(|w| segment_distance(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

/// The target on the 1080p screen: where (kept inside the edges when it is out of view) and
/// whether it is in view.
pub(super) fn screen_marker(camera: Camera3D, at: Vec3) -> ([f32; 2], bool) {
    let (w, h) = (WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
    // Behind her only with a perspective camera: the isometric one sees along parallel lines, so a
    // point nearer than its near plane is still where the projection puts it.
    let view = camera.view_matrix().transform_point3(at);
    let behind = camera.orthographic_height.is_none() && view.z >= -camera.near;
    let ndc = camera.view_projection(w / h).transform_point3(at);
    let (mut x, mut y) = (ndc.x, ndc.y);
    if behind {
        x = -x;
        y = -y;
    }
    let inside = !behind && x.abs() <= 0.92 && y.abs() <= 0.86;
    if !inside {
        // Pushed out to the frame along the line from the middle of the screen.
        let k = (x.abs() / 0.92).max(y.abs() / 0.80).max(1e-3);
        x /= k;
        y /= k;
    }
    ([(x * 0.5 + 0.5) * w, (1.0 - (y * 0.5 + 0.5)) * h], inside)
}

// ------------------------------------------------------------------ the guide in 3D
/// The guide's meshes and pictures: a flat quad for the arrows and the ring, crossed quads for the
/// column of light.
pub(super) struct GuideMeshes {
    quad: MeshHandle,
    column: MeshHandle,
    chevron: TextureHandle,
    ring: TextureHandle,
    beam: TextureHandle,
}

fn texture(n: usize, alpha: impl Fn(f32, f32) -> f32) -> TextureData3D {
    let mut rgba = Vec::with_capacity(n * n * 4);
    for j in 0..n {
        for i in 0..n {
            let a = alpha((i as f32 + 0.5) / n as f32, (j as f32 + 0.5) / n as f32);
            rgba.extend_from_slice(&[255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8]);
        }
    }
    TextureData3D {
        width: n as u32,
        height: n as u32,
        rgba,
    }
}

/// Distance from p to the segment a-b (in a texture's square).
fn seg(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (abx, aby) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / (abx * abx + aby * aby)).clamp(0.0, 1.0);
    (a.0 + abx * t - p.0).hypot(a.1 + aby * t - p.1)
}

impl GuideMeshes {
    fn new(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self> {
        let quad =
            renderer.upload_mesh(device, &Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0)))?;
        // Two crossed upright quads, each both ways round: a column of light seen from anywhere.
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (dx, dz) in [(1.0f32, 0.0f32), (0.0, 1.0)] {
            let base = vertices.len() as u32;
            for (s, y) in [(-1.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)] {
                vertices.push(MeshVertex3D::new(
                    [dx * s * 0.5, y, dz * s * 0.5],
                    [dz, 0.0, -dx],
                    [(s + 1.0) * 0.5, 1.0 - y],
                ));
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        }
        let column = renderer.upload_mesh(
            device,
            &Mesh3D {
                vertices,
                indices,
                ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
            },
        )?;
        // An arrowhead pointing down the texture (+v, the quad's +z): two soft strokes.
        let chevron = renderer.upload_texture(
            device,
            queue,
            &texture(64, |u, v| {
                let d = seg((u, v), (0.14, 0.30), (0.5, 0.78)).min(seg(
                    (u, v),
                    (0.86, 0.30),
                    (0.5, 0.78),
                ));
                let core = 1.0 - ((d - 0.075) / 0.05).clamp(0.0, 1.0);
                let glow = (1.0 - (d / 0.24).min(1.0)).powi(2) * 0.35;
                core.max(glow)
            }),
        )?;
        let ring = renderer.upload_texture(
            device,
            queue,
            &texture(96, |u, v| {
                let r = (u - 0.5).hypot(v - 0.5) * 2.0;
                let band = 1.0 - ((r - 0.82).abs() / 0.09).min(1.0);
                let inner = (1.0 - r).max(0.0).powi(2) * 0.25;
                band.powf(1.5).max(inner)
            }),
        )?;
        let beam = renderer.upload_texture(
            device,
            queue,
            &texture(64, |u, v| {
                // v = 0 at the top of the column: it fades upward, and to its sides.
                let up = v.powf(1.6);
                let across = 1.0 - ((u - 0.5).abs() * 2.0).powi(2);
                up * across
            }),
        )?;
        Ok(Self {
            quad,
            column,
            chevron,
            ring,
            beam,
        })
    }
}

impl RenderState {
    /// The guide in the world: arrows running along the ground ahead of her and a column of light
    /// over the target (a pulse of rings at its foot).
    pub(super) fn mission_draws(
        &mut self,
        guide: &Guide,
        elapsed: f32,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        if self.guide_meshes.is_none() {
            match GuideMeshes::new(&mut self.renderer, &self.device, &self.queue) {
                Ok(m) => self.guide_meshes = Some(m),
                Err(error) => {
                    eprintln!("[guía] {error:#}");
                    return;
                }
            }
        }
        let Some(m) = self.guide_meshes.as_ref() else {
            return;
        };
        let gold = |a: f32| Material3D::additive([1.15, 0.70, 0.18, a]);
        // The arrows: every metre and a bit along the way, from a step ahead of her to the target
        // (or a dozen metres on), sliding forward.
        if !guide.arrived && guide.route.len() >= 2 {
            const SPACING: f32 = 1.35;
            const REACH: f32 = 15.0;
            let slide = (elapsed * 1.7).rem_euclid(SPACING);
            let mut walked = 0.0;
            let mut next = 1.1 + slide;
            for w in guide.route.windows(2) {
                let (a, b) = (w[0], w[1]);
                let len = Vec3::new(b.x - a.x, 0.0, b.z - a.z).length();
                if len < 1e-3 {
                    continue;
                }
                let dir = Vec3::new(b.x - a.x, 0.0, b.z - a.z) * (1.0 / len);
                let yaw = dir.x.atan2(dir.z);
                while next <= walked + len && next < REACH.min(guide.distance - 0.8) {
                    let t = (next - walked) / len;
                    let at = a + (b - a) * t;
                    // Fading in at her feet and out far ahead.
                    let fade = ((next - 1.1) / 1.2).clamp(0.0, 1.0)
                        * (1.0 - ((next - REACH * 0.7) / (REACH * 0.3)).clamp(0.0, 1.0));
                    draws.push(MeshDraw3D {
                        mesh: m.quad,
                        transform: Mat4::from_translation(Vec3::new(at.x, at.y + 0.06, at.z))
                            * Mat4::from_rotation_y(yaw)
                            * Mat4::from_scale(Vec3::new(0.85, 1.0, 0.85)),
                        material: gold(0.85 * fade),
                        texture: Some(m.chevron),
                    });
                    next += SPACING;
                }
                walked += len;
                if next >= REACH {
                    break;
                }
            }
        }
        // The column of light and its rings, over the step's own spot or the door on the way.
        if let Some(t) = guide.target {
            let pulse = 0.82 + 0.18 * (elapsed * 2.4).sin();
            draws.push(MeshDraw3D {
                mesh: m.column,
                transform: Mat4::from_translation(Vec3::new(t.x, t.y + 0.02, t.z))
                    * Mat4::from_rotation_y(elapsed * 0.4)
                    * Mat4::from_scale(Vec3::new(1.5, 7.5, 1.5)),
                material: gold(0.55 * pulse),
                texture: Some(m.beam),
            });
            for k in 0..2 {
                let phase = (elapsed * 0.55 + k as f32 * 0.5).rem_euclid(1.0);
                let size = 1.2 + phase * 2.4;
                draws.push(MeshDraw3D {
                    mesh: m.quad,
                    transform: Mat4::from_translation(Vec3::new(t.x, t.y + 0.05, t.z))
                        * Mat4::from_scale(Vec3::new(size, 1.0, size)),
                    material: gold(0.9 * (1.0 - phase)),
                    texture: Some(m.ring),
                });
            }
        }
    }
}

impl App {
    /// The mission panel, the way on the maps, the mark on the screen and the notice, for the
    /// overlay (`None`: no mission open and no notice).
    pub(super) fn mission_view(&self, camera: Camera3D) -> Option<map_overlay::MissionView> {
        let notice = self.missions.shown.as_ref().map(|(heading, title, at)| {
            (
                (*heading).to_owned(),
                title.clone(),
                (self.elapsed - at).max(0.0),
            )
        });
        let Some(g) = self.guide.as_ref() else {
            return notice.map(|notice| map_overlay::MissionView {
                notice: Some(notice),
                ..Default::default()
            });
        };
        let here_dungeon = self.zone_id() == ZoneId::Dungeon;
        let distance = match (g.elsewhere, g.target) {
            (Some(zone), _) => format!("En {zone}"),
            (None, Some(_)) if g.arrived => "Aquí".to_owned(),
            (None, Some(_)) => format!("{:.0} m", g.distance.max(1.0)),
            (None, None) => String::new(),
        };
        let marker = g
            .target
            .filter(|_| !here_dungeon)
            .map(|t| screen_marker(camera, t + Vec3::new(0.0, 2.6, 0.0)));
        Some(map_overlay::MissionView {
            title: g.title.clone(),
            step: g.step.clone(),
            detail: g.detail.clone(),
            main: g.main,
            distance,
            others: g.others.clone(),
            can_cycle: g.open > 1,
            route: g.route.iter().map(|p| [p.x, p.z]).collect(),
            target: g.target.map(|t| [t.x, t.z]),
            marker,
            marker_label: g.target_label.clone(),
            marker_distance: if g.arrived {
                String::new()
            } else {
                format!("{:.0} m", g.distance.max(1.0))
            },
            notice,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_way_between_zones_goes_through_the_wood() {
        assert_eq!(
            next_zone(ZoneId::Castle, ZoneId::Village),
            Some(ZoneId::Forest)
        );
        assert_eq!(
            next_zone(ZoneId::Castle, ZoneId::Penascal),
            Some(ZoneId::Forest)
        );
        assert_eq!(
            next_zone(ZoneId::Penascal, ZoneId::Castle),
            Some(ZoneId::Forest)
        );
        assert_eq!(
            next_zone(ZoneId::Forest, ZoneId::Penascal),
            Some(ZoneId::Penascal)
        );
        assert_eq!(
            next_zone(ZoneId::Baths, ZoneId::Castle),
            Some(ZoneId::Village)
        );
        assert_eq!(
            next_zone(ZoneId::Village, ZoneId::Village),
            Some(ZoneId::Village)
        );
    }

    /// The way found on a zone's grid: from `from` to near `to`, each leg over free ground.
    fn walkable(zone: ZoneId, level: u8, from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
        let g = grid(zone, level).expect("grid");
        let way = g
            .path(from, to)
            .unwrap_or_else(|| panic!("{zone:?}: no way from {from:?} to {to:?}"));
        let end = way.last().copied().unwrap();
        // Within reach of it (a big table's middle is a few metres in).
        assert!(
            (end.0 - to.0).hypot(end.1 - to.1) < 4.0,
            "{zone:?}: ends at {end:?}, not near {to:?}"
        );
        for w in way.windows(2).skip(1) {
            assert!(
                g.clear(w[0], w[1]),
                "{zone:?}: leg {:?} → {:?} crosses a wall",
                w[0],
                w[1]
            );
        }
        way
    }

    #[test]
    fn the_guide_finds_the_way_in_every_zone() {
        // The castle: from the bedroom to the war table, and to the patio's gate.
        let p = castle3d::plan();
        let spot = |id: &str| {
            p.hotspots
                .iter()
                .find(|h| h.id == id)
                .map(|h| (h.x, h.z))
                .unwrap()
        };
        let bed = castle3d::bedroom_spawn();
        walkable(ZoneId::Castle, 0, bed, spot("mesa-guerra"));
        walkable(ZoneId::Castle, 0, bed, spot(castle3d::GATE));
        walkable(ZoneId::Castle, 0, spot("mesa-guerra"), spot("balcon"));
        // The wood: from the castle's door to the villa's gate and to the Peñascal's way.
        let castle = castle_entry_position();
        let villa = village_entry_position();
        walkable(ZoneId::Forest, 0, (castle.x, castle.z), (villa.x, villa.z));
        if let Some((way, _)) = dark_forest::exit("penascal") {
            walkable(ZoneId::Forest, 0, (castle.x, castle.z), (way.x, way.z));
        }
        // The villa: from its gate to Grizna's corral.
        let (gate, _) = village::spawn();
        if let Some((corral, _)) = village::spot("corral") {
            walkable(ZoneId::Village, 0, (gate.x, gate.z), (corral.x, corral.z));
        }
        // El Peñascal: from the way in to the chief and to every spot.
        if penascal3d::available() {
            let pp = penascal3d::plan();
            for h in &pp.hotspots {
                walkable(ZoneId::Penascal, 0, (pp.spawn.x, pp.spawn.z), (h.x, h.z));
            }
        }
    }
}
