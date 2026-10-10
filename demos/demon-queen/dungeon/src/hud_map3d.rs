//! The expedition chart in 3D: every known room is a slab lifted to its real
//! altitude (exaggerated so a two-metre step reads), seen through an
//! orthographic camera the player can turn, tilt and zoom. Floors are tinted
//! by altitude, corridors climb between them, drop lines and shadows on the
//! base plane say how high each room floats, and a ruler gives the metres.
//! Drawn back to front (painter's algorithm) into the HUD's vertex batch.
use super::glass::{self, alpha, altitude_color, lift, mix, Rgba, Tone, CRIMSON, IVORY};
use super::map::{medallion, role_wash, room_resolved, Knowledge};
use super::{prompt_icons, Dungeon, Hud, Vertex};
use velvet_dungeon::{Connection, ConnectionKind, Role, Room, Shape};

/// Vertical exaggeration of the chart.
pub(super) const EXAGGERATION: f32 = 1.5;
/// Thickness of a floor slab and height of its low wall, in metres.
const SLAB: f32 = 1.8;
const WALL: f32 = 1.0;
/// How far above its floor a room's badge floats, in metres.
const BADGE: f32 = 3.4;

#[derive(Clone, Copy)]
pub(super) struct View {
    c: f32,
    s: f32,
    sp: f32,
    cp: f32,
    scale: f32,
    origin: [f32; 2],
    focus: [f32; 3],
}

impl View {
    /// Distance toward the viewer along the ground (before tilt).
    fn zr(&self, x: f32, z: f32) -> f32 {
        (x - self.focus[0]) * self.s + (z - self.focus[2]) * self.c
    }
    fn at(&self, x: f32, y: f32, z: f32) -> [f32; 2] {
        let (dx, dy, dz) = (x - self.focus[0], y - self.focus[1], z - self.focus[2]);
        let xr = dx * self.c - dz * self.s;
        let zr = dx * self.s + dz * self.c;
        [
            self.origin[0] + xr * self.scale,
            self.origin[1] + (zr * self.sp - dy * EXAGGERATION * self.cp) * self.scale,
        ]
    }
    /// Larger means nearer the camera; drawn later.
    fn depth(&self, x: f32, y: f32, z: f32) -> f32 {
        self.zr(x, z) * self.cp + (y - self.focus[1]) * EXAGGERATION * self.sp
    }
    /// Screen y of altitude `y` on the vertical axis through the focus.
    fn altitude_y(&self, y: f32) -> f32 {
        self.origin[1] - (y - self.focus[1]) * EXAGGERATION * self.cp * self.scale
    }
}

struct Ctx<'a> {
    run: &'a Dungeon,
    known: &'a Knowledge,
    tone: &'a Tone,
    view: View,
    low: f32,
    span: f32,
    base: f32,
    centre: [f32; 2],
    radius: f32,
    current: usize,
}

impl Ctx<'_> {
    fn alt(&self, y: f32) -> Rgba {
        altitude_color((y - self.low) / self.span)
    }
    fn top_color(&self, room: &Room) -> Rgba {
        let mut c = mix(self.alt(room.center.y), role_wash(room.role), 0.26);
        c = lift(c, 0.92);
        if room.id == self.current {
            c = lift(mix(c, [1.0, 0.9, 0.6, 1.0], 0.2), 1.12);
        }
        c
    }
}

enum Item {
    Room(usize),
    Corridor(usize),
    Ghost(usize),
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Frames everything known at the current turn/tilt/zoom.
fn make_view(run: &Dungeon, known: &Knowledge, vp: [f32; 4]) -> (View, f32, f32, [f32; 2], f32) {
    let player = run.controller.position;
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    let (mut low, mut high) = (player.y, player.y);
    for room in &run.map.rooms {
        if known.visited[room.id] || known.glimpsed[room.id] {
            low = low.min(room.center.y);
            high = high.max(room.center.y);
            for p in &room.outline {
                let (x, z) = (room.center.x + p[0], room.center.z + p[1]);
                min = [min[0].min(x), min[1].min(z)];
                max = [max[0].max(x), max[1].max(z)];
            }
        }
    }
    if !min[0].is_finite() {
        min = [player.x - 12.0, player.z - 12.0];
        max = [player.x + 12.0, player.z + 12.0];
    }
    let centre = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let mut radius = 12.0f32;
    for room in &run.map.rooms {
        if known.visited[room.id] || known.glimpsed[room.id] {
            for p in &room.outline {
                let (dx, dz) = (
                    room.center.x + p[0] - centre[0],
                    room.center.z + p[1] - centre[1],
                );
                radius = radius.max((dx * dx + dz * dz).sqrt());
            }
        }
    }
    let pitch = run.map_pitch.clamp(-1.45, 1.52);
    let zoom = run.map_zoom.clamp(0.6, 3.2);
    let (sp, cp) = pitch.sin_cos();
    let (s, c) = run.map_yaw.sin_cos();
    let ymid = (low + high) * 0.5;
    // Frame everything known: measure it through the camera at unit scale,
    // then scale it to fill the viewport and centre it.
    let probe = View {
        c,
        s,
        sp,
        cp,
        scale: 1.0,
        origin: [0.0, 0.0],
        focus: [centre[0], ymid, centre[1]],
    };
    let mut bounds = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut grow = |p: [f32; 2]| {
        bounds = [
            bounds[0].min(p[0]),
            bounds[1].max(p[0]),
            bounds[2].min(p[1]),
            bounds[3].max(p[1]),
        ];
    };
    for room in &run.map.rooms {
        if known.visited[room.id] || known.glimpsed[room.id] {
            for p in &room.outline {
                let (x, z) = (room.center.x + p[0], room.center.z + p[1]);
                grow(probe.at(x, room.center.y - SLAB, z));
                grow(probe.at(x, room.center.y + WALL, z));
            }
            grow(probe.at(room.center.x, room.center.y + BADGE + 2.0, room.center.z));
        }
    }
    grow(probe.at(player.x, player.y + BADGE + 2.5, player.z));
    let (bw, bh) = (
        (bounds[1] - bounds[0]).max(1.0),
        (bounds[3] - bounds[2]).max(1.0),
    );
    let fit = ((vp[2] - 130.0) / bw).min((vp[3] - 170.0) / bh);
    let scale = fit * zoom;
    let k = smooth((zoom - 1.0) / 1.2);
    let focus = [
        centre[0] + (player.x - centre[0]) * k,
        ymid + (player.y - ymid) * k,
        centre[1] + (player.z - centre[1]) * k,
    ];
    let middle = [vp[0] + vp[2] * 0.5, vp[1] + vp[3] * 0.5 + 24.0];
    let framed = [
        middle[0] - (bounds[0] + bounds[1]) * 0.5 * scale,
        middle[1] - (bounds[2] + bounds[3]) * 0.5 * scale,
    ];
    let view = View {
        c,
        s,
        sp,
        cp,
        scale,
        origin: [
            framed[0] + (middle[0] - framed[0]) * k + run.map_pan[0],
            framed[1] + (middle[1] - framed[1]) * k + run.map_pan[1],
        ],
        focus,
    };
    (view, low, high, centre, radius)
}

/// Outward unit normals (world x, z) of a closed outline.
fn edge_normals(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let n = points.len();
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let e = [b[0] - a[0], b[1] - a[1]];
            let l = (e[0] * e[0] + e[1] * e[1]).sqrt().max(1e-4);
            if area > 0.0 {
                [e[1] / l, -e[0] / l]
            } else {
                [-e[1] / l, e[0] / l]
            }
        })
        .collect()
}

fn shade(color: Rgba, k: f32) -> Rgba {
    lift(color, k)
}

fn fill_polygon(v: &mut Vec<Vertex>, points: &[[f32; 2]], color: Rgba) {
    for [a, b, c] in prompt_icons::triangulate(points) {
        super::triangle(v, points[a], points[b], points[c], color);
    }
}

impl Hud {
    pub(super) fn draw_chart(
        &self,
        run: &Dungeon,
        known: &Knowledge,
        tone: &Tone,
        vp: [f32; 4],
        v: &mut Vec<Vertex>,
    ) {
        let (view, low, high, centre, radius) = make_view(run, known, vp);
        let cx = Ctx {
            run,
            known,
            tone,
            view,
            low,
            span: (high - low).max(4.0),
            base: low - 3.5,
            centre,
            radius,
            current: run.map.region_at(run.controller.position),
        };
        // Seen from underneath the base plane is the nearest thing: it is
        // drawn last, as a pane of glass in front of the rooms.
        let below = view.sp < 0.0;
        if !below {
            self.chart_stage(&cx, v);
            self.chart_shadows(&cx, v);
        }

        let mut items: Vec<(f32, Item)> = Vec::new();
        for room in &run.map.rooms {
            let d = view.depth(room.center.x, room.center.y, room.center.z);
            if known.visited[room.id] {
                items.push((d, Item::Room(room.id)));
            } else if known.glimpsed[room.id] {
                items.push((d, Item::Ghost(room.id)));
            }
        }
        for conn in &run.map.connections {
            if known.corridor[conn.id] {
                let mid = (conn.start + conn.end) * 0.5;
                items.push((
                    view.depth(mid.x, mid.y, mid.z) + 0.001,
                    Item::Corridor(conn.id),
                ));
            }
        }
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, item) in &items {
            match item {
                Item::Room(id) => self.chart_room(&cx, &run.map.rooms[*id], v),
                Item::Corridor(id) => self.chart_corridor(&cx, &run.map.connections[*id], v),
                Item::Ghost(id) => self.chart_ghost(&cx, &run.map.rooms[*id], v),
            }
        }
        if below {
            self.chart_stage(&cx, v);
            self.chart_shadows(&cx, v);
        }
        self.chart_badges(&cx, v);
        self.chart_player(&cx, v);
        self.chart_ruler(&cx, vp, v);
    }

    /// The floor the complex stands on: a world grid that only appears near
    /// what is charted, fading out with distance.
    fn chart_stage(&self, cx: &Ctx, v: &mut Vec<Vertex>) {
        let (view, base, run) = (cx.view, cx.base, cx.run);
        let cell = 6.0f32;
        let reach = 11.0f32;
        let mut anchors: Vec<[f32; 2]> = Vec::new();
        for room in run
            .map
            .rooms
            .iter()
            .filter(|r| cx.known.visited[r.id] || cx.known.glimpsed[r.id])
        {
            anchors.push([room.center.x, room.center.z]);
        }
        for conn in run
            .map
            .connections
            .iter()
            .filter(|c| cx.known.corridor[c.id])
        {
            anchors.push([
                (conn.start.x + conn.end.x) * 0.5,
                (conn.start.z + conn.end.z) * 0.5,
            ]);
        }
        if anchors.is_empty() {
            return;
        }
        let strength = |gx: i32, gz: i32| -> f32 {
            let p = [(gx as f32 + 0.5) * cell, (gz as f32 + 0.5) * cell];
            let d = anchors
                .iter()
                .map(|a| (a[0] - p[0]).hypot(a[1] - p[1]))
                .fold(f32::INFINITY, f32::min);
            (1.0 - d / reach).clamp(0.0, 1.0)
        };
        let (lo, hi) = (
            [
                ((cx.centre[0] - cx.radius - reach) / cell).floor() as i32,
                ((cx.centre[1] - cx.radius - reach) / cell).floor() as i32,
            ],
            [
                ((cx.centre[0] + cx.radius + reach) / cell).ceil() as i32,
                ((cx.centre[1] + cx.radius + reach) / cell).ceil() as i32,
            ],
        );
        let mut floor: Vec<[f32; 2]> = Vec::new();
        for gx in lo[0]..hi[0] {
            for gz in lo[1]..hi[1] {
                let k = strength(gx, gz);
                if k <= 0.02 {
                    continue;
                }
                let (x0, z0, x1, z1) = (
                    gx as f32 * cell,
                    gz as f32 * cell,
                    (gx + 1) as f32 * cell,
                    (gz + 1) as f32 * cell,
                );
                let quad = [
                    view.at(x0, base, z0),
                    view.at(x1, base, z0),
                    view.at(x1, base, z1),
                    view.at(x0, base, z1),
                ];
                for i in [0usize, 1, 2, 0, 2, 3] {
                    v.push(Vertex {
                        p: quad[i],
                        uv: [0.0015; 2],
                        color: [0.03, 0.10, 0.13, 0.30 * k],
                    });
                }
                let edge = alpha(cx.tone.accent, 0.16 * k);
                glass::segment(v, quad[1], quad[2], 1.0, edge);
                glass::segment(v, quad[2], quad[3], 1.0, edge);
                if strength(gx - 1, gz) <= 0.02 {
                    glass::segment(v, quad[0], quad[3], 1.0, edge);
                }
                if strength(gx, gz - 1) <= 0.02 {
                    glass::segment(v, quad[0], quad[1], 1.0, edge);
                }
                floor.push(quad[0]);
            }
        }
    }

    /// Shadows on the base plane and the drop lines that anchor each
    /// floating room to it.
    fn chart_shadows(&self, cx: &Ctx, v: &mut Vec<Vertex>) {
        let (view, run) = (cx.view, cx.run);
        for conn in
            run.map.connections.iter().filter(|c| {
                cx.known.corridor[c.id] && cx.known.visited[c.a] && cx.known.visited[c.b]
            })
        {
            let quad = ribbon(conn, 0.0);
            let pts: Vec<[f32; 2]> = quad.iter().map(|p| view.at(p[0], cx.base, p[1])).collect();
            fill_polygon(v, &pts, [0.0, 0.02, 0.03, 0.22]);
        }
        for room in run.map.rooms.iter().filter(|r| cx.known.visited[r.id]) {
            let pts: Vec<[f32; 2]> = room
                .outline
                .iter()
                .map(|p| view.at(room.center.x + p[0], cx.base, room.center.z + p[1]))
                .collect();
            fill_polygon(v, &pts, [0.0, 0.02, 0.03, 0.34]);
            let lift_m = room.center.y - SLAB - cx.base;
            if lift_m > 1.2 {
                let top = view.at(room.center.x, room.center.y - SLAB, room.center.z);
                let foot = view.at(room.center.x, cx.base, room.center.z);
                let color = alpha(mix(cx.alt(room.center.y), [1.0, 1.0, 1.0, 1.0], 0.3), 0.32);
                glass::dashes(v, top, foot, 1.2, 5.0, 5.0, color);
                glass::disc(v, foot, 2.6, color);
            }
        }
    }

    fn chart_room(&self, cx: &Ctx, room: &Room, v: &mut Vec<Vertex>) {
        let view = cx.view;
        let y = room.center.y;
        let world: Vec<[f32; 2]> = room
            .outline
            .iter()
            .map(|p| [room.center.x + p[0], room.center.z + p[1]])
            .collect();
        let normals = edge_normals(&world);
        let front = |i: usize| normals[i][0] * view.s + normals[i][1] * view.c > 0.02;
        let top = cx.top_color(room);
        let side = shade(cx.alt(y), 0.30);
        let wall = shade(mix(top, [0.5, 0.5, 0.5, 1.0], 0.1), 0.52);
        let doors: Vec<([f32; 2], f32)> = cx
            .run
            .map
            .connections
            .iter()
            .filter(|c| c.a == room.id || c.b == room.id)
            .map(|c| {
                let p = if c.a == room.id { c.start } else { c.end };
                ([p.x, p.z], c.width * 0.5 + 0.7)
            })
            .collect();
        let _ = front;

        if view.sp < 0.0 {
            // Seen from below: what stands on the floor is behind it, then the
            // slab's sides, and its underside in front.
            self.wall_pieces(cx, &world, &normals, y, &doors, false, wall, v);
            self.wall_pieces(cx, &world, &normals, y, &doors, true, wall, v);
            for o in cx.run.map.obstacles.iter().filter(|o| o.room == room.id) {
                let h = o.half[0].max(0.4);
                let block = [
                    [o.center.x - h, o.center.z - h],
                    [o.center.x + h, o.center.z - h],
                    [o.center.x + h, o.center.z + h],
                    [o.center.x - h, o.center.z + h],
                ];
                self.prism(
                    cx,
                    &block,
                    y,
                    y + o.height.clamp(0.8, 2.6),
                    shade(top, 0.5),
                    shade(top, 0.3),
                    v,
                );
            }
            self.slab_sides(cx, &world, &normals, y, side, v);
            let bottom: Vec<[f32; 2]> = world
                .iter()
                .map(|p| view.at(p[0], y - SLAB, p[1]))
                .collect();
            fill_polygon(v, &bottom, lift(mix(side, top, 0.25), 1.5));
            glass::stroke(
                v,
                &bottom,
                true,
                1.3,
                alpha(mix(top, [1.0, 1.0, 1.0, 1.0], 0.4), 0.8),
            );
            if room.id == cx.current {
                glass::stroke(v, &bottom, true, 2.4, cx.tone.bright);
            }
            return;
        }

        // Walls at the back, seen from inside.
        self.wall_pieces(cx, &world, &normals, y, &doors, false, wall, v);
        // The slab under the floor: its sides facing the viewer.
        self.slab_sides(cx, &world, &normals, y, side, v);
        // The floor.
        let floor: Vec<[f32; 2]> = world.iter().map(|p| view.at(p[0], y, p[1])).collect();
        fill_polygon(v, &floor, top);
        if room.shape == Shape::Bridge {
            // The chasm: dark wells in the four corners of the cross deck.
            for (sx, sz) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let (x0, z0) = (room.center.x + sx * 1.8, room.center.z + sz * 1.8);
                let (x1, z1) = (
                    room.center.x + sx * room.half[0] * 0.92,
                    room.center.z + sz * room.half[1] * 0.92,
                );
                let well = [
                    view.at(x0, y, z0),
                    view.at(x1, y, z0),
                    view.at(x1, y, z1),
                    view.at(x0, y, z1),
                ];
                fill_polygon(v, &well, [0.0, 0.01, 0.02, 0.8]);
            }
        }
        // Pillars and blocks standing on it.
        for o in cx.run.map.obstacles.iter().filter(|o| o.room == room.id) {
            let h = o.half[0].max(0.4);
            let block = [
                [o.center.x - h, o.center.z - h],
                [o.center.x + h, o.center.z - h],
                [o.center.x + h, o.center.z + h],
                [o.center.x - h, o.center.z + h],
            ];
            let height = o.height.clamp(0.8, 2.6);
            self.prism(
                cx,
                &block,
                y,
                y + height,
                shade(top, 0.62),
                shade(top, 0.38),
                v,
            );
        }
        // Walls at the front, with the doorways left open.
        self.wall_pieces(cx, &world, &normals, y, &doors, true, wall, v);
        glass::stroke(
            v,
            &floor,
            true,
            1.3,
            alpha(mix(top, [1.0, 1.0, 1.0, 1.0], 0.45), 0.85),
        );
        if room.id == cx.current {
            let pulse = 0.5 + 0.5 * (cx.run.map_time * 2.4).sin();
            glass::stroke(
                v,
                &floor,
                true,
                8.0,
                alpha(cx.tone.bright, 0.14 + 0.12 * pulse),
            );
            glass::stroke(v, &floor, true, 2.4, cx.tone.bright);
        }
    }

    /// The sides of a floor slab that face the viewer.
    fn slab_sides(
        &self,
        cx: &Ctx,
        world: &[[f32; 2]],
        normals: &[[f32; 2]],
        y: f32,
        side: Rgba,
        v: &mut Vec<Vertex>,
    ) {
        let view = cx.view;
        let n = world.len();
        for i in 0..n {
            if normals[i][0] * view.s + normals[i][1] * view.c <= 0.02 {
                continue;
            }
            let (a, b) = (world[i], world[(i + 1) % n]);
            let k = 0.85 - 0.15 * (normals[i][0] * view.c - normals[i][1] * view.s);
            glass::quad(
                v,
                (view.at(a[0], y, a[1]), shade(side, k)),
                (view.at(b[0], y, b[1]), shade(side, k)),
                (view.at(b[0], y - SLAB, b[1]), shade(side, k * 0.7)),
                (view.at(a[0], y - SLAB, a[1]), shade(side, k * 0.7)),
            );
        }
    }

    /// A vertical wall around the outline, in short pieces so the doorways
    /// can stay open. `front` picks the faces toward the viewer.
    #[allow(clippy::too_many_arguments)]
    fn wall_pieces(
        &self,
        cx: &Ctx,
        world: &[[f32; 2]],
        normals: &[[f32; 2]],
        y: f32,
        doors: &[([f32; 2], f32)],
        front: bool,
        color: Rgba,
        v: &mut Vec<Vertex>,
    ) {
        let view = cx.view;
        let n = world.len();
        for i in 0..n {
            let facing = normals[i][0] * view.s + normals[i][1] * view.c > 0.02;
            if facing != front {
                continue;
            }
            let (a, b) = (world[i], world[(i + 1) % n]);
            let e = [b[0] - a[0], b[1] - a[1]];
            let len = (e[0] * e[0] + e[1] * e[1]).sqrt();
            let pieces = (len / 1.6).ceil().max(1.0) as usize;
            // Lit from the upper left of the screen.
            let lit = 0.78 - 0.30 * (normals[i][0] * view.c - normals[i][1] * view.s);
            let color = shade(color, lit.clamp(0.4, 1.1));
            for k in 0..pieces {
                let (t0, t1) = (k as f32 / pieces as f32, (k + 1) as f32 / pieces as f32);
                let mid = [a[0] + e[0] * (t0 + t1) * 0.5, a[1] + e[1] * (t0 + t1) * 0.5];
                if doors
                    .iter()
                    .any(|(d, half)| (d[0] - mid[0]).hypot(d[1] - mid[1]) < *half)
                {
                    continue;
                }
                let p0 = [a[0] + e[0] * t0, a[1] + e[1] * t0];
                let p1 = [a[0] + e[0] * t1, a[1] + e[1] * t1];
                glass::quad(
                    v,
                    (view.at(p0[0], y, p0[1]), shade(color, 0.85)),
                    (view.at(p1[0], y, p1[1]), shade(color, 0.85)),
                    (view.at(p1[0], y + WALL, p1[1]), color),
                    (view.at(p0[0], y + WALL, p0[1]), color),
                );
                // A lit lip along the wall's top.
                let lip = lift(color, 1.9);
                glass::quad(
                    v,
                    (view.at(p0[0], y + WALL - 0.16, p0[1]), lip),
                    (view.at(p1[0], y + WALL - 0.16, p1[1]), lip),
                    (view.at(p1[0], y + WALL, p1[1]), lip),
                    (view.at(p0[0], y + WALL, p0[1]), lip),
                );
            }
        }
    }

    /// A box from `y0` to `y1`: front sides then the lid.
    #[allow(clippy::too_many_arguments)]
    fn prism(
        &self,
        cx: &Ctx,
        world: &[[f32; 2]],
        y0: f32,
        y1: f32,
        lid: Rgba,
        side: Rgba,
        v: &mut Vec<Vertex>,
    ) {
        let view = cx.view;
        let normals = edge_normals(world);
        let n = world.len();
        for i in 0..n {
            if normals[i][0] * view.s + normals[i][1] * view.c <= 0.02 {
                continue;
            }
            let (a, b) = (world[i], world[(i + 1) % n]);
            glass::quad(
                v,
                (view.at(a[0], y0, a[1]), side),
                (view.at(b[0], y0, b[1]), side),
                (view.at(b[0], y1, b[1]), shade(side, 1.2)),
                (view.at(a[0], y1, a[1]), shade(side, 1.2)),
            );
        }
        let top: Vec<[f32; 2]> = world.iter().map(|p| view.at(p[0], y1, p[1])).collect();
        fill_polygon(v, &top, lid);
    }

    fn chart_corridor(&self, cx: &Ctx, conn: &Connection, v: &mut Vec<Vertex>) {
        let view = cx.view;
        let both = cx.known.visited[conn.a] && cx.known.visited[conn.b];
        let corners = ribbon(conn, 0.4);
        let (ya, yb) = (conn.start.y, conn.end.y);
        let ys = [ya, yb, yb, ya];
        let pts: Vec<[f32; 2]> = corners
            .iter()
            .zip(ys)
            .map(|(p, y)| view.at(p[0], y, p[1]))
            .collect();
        let mid_y = (ya + yb) * 0.5;
        let color = mix(lift(cx.alt(mid_y), 0.8), [0.62, 0.56, 0.44, 1.0], 0.3);
        if !both {
            // Glimpsed: a dashed sketch.
            fill_polygon(v, &pts, alpha(color, 0.12));
            glass::dashes(v, pts[0], pts[1], 1.2, 6.0, 5.0, alpha(color, 0.55));
            glass::dashes(v, pts[3], pts[2], 1.2, 6.0, 5.0, alpha(color, 0.55));
            return;
        }
        if view.sp < 0.0 {
            // From below: the underside of the ribbon.
            let thickness = 0.9;
            let under: Vec<[f32; 2]> = corners
                .iter()
                .zip(ys)
                .map(|(p, y)| view.at(p[0], y - thickness, p[1]))
                .collect();
            let skirt = shade(cx.alt(mid_y), 0.28);
            fill_polygon(v, &under, lift(mix(skirt, color, 0.3), 1.5));
            glass::segment(
                v,
                under[0],
                under[1],
                1.2,
                alpha(mix(color, [1.0, 1.0, 1.0, 1.0], 0.4), 0.7),
            );
            glass::segment(
                v,
                under[3],
                under[2],
                1.2,
                alpha(mix(color, [1.0, 1.0, 1.0, 1.0], 0.4), 0.7),
            );
            return;
        }
        // The skirt on the side facing the viewer.
        let side = if view.zr(corners[0][0], corners[0][1]) > view.zr(corners[3][0], corners[3][1])
        {
            (0, 1)
        } else {
            (3, 2)
        };
        let skirt = shade(cx.alt(mid_y), 0.28);
        let (p, q) = (corners[side.0], corners[side.1]);
        let thickness = 0.9;
        glass::quad(
            v,
            (view.at(p[0], ya, p[1]), skirt),
            (view.at(q[0], yb, q[1]), skirt),
            (view.at(q[0], yb - thickness, q[1]), shade(skirt, 0.7)),
            (view.at(p[0], ya - thickness, p[1]), shade(skirt, 0.7)),
        );
        fill_polygon(v, &pts, color);
        let edge = alpha(mix(color, [1.0, 1.0, 1.0, 1.0], 0.5), 0.75);
        glass::segment(v, pts[0], pts[1], 1.2, edge);
        glass::segment(v, pts[3], pts[2], 1.2, edge);
        let len = conn.length().max(0.1);
        let lerp = |t: f32, side: f32| {
            let a = [
                corners[0][0] + (corners[1][0] - corners[0][0]) * t,
                corners[0][1] + (corners[1][1] - corners[0][1]) * t,
            ];
            let b = [
                corners[3][0] + (corners[2][0] - corners[3][0]) * t,
                corners[3][1] + (corners[2][1] - corners[3][1]) * t,
            ];
            let y = ya + (yb - ya) * t;
            let _ = side;
            (view.at(a[0], y, a[1]), view.at(b[0], y, b[1]))
        };
        match conn.kind {
            ConnectionKind::Stairs => {
                let steps = (len / 0.9).floor().max(3.0) as usize;
                for k in 1..steps {
                    let (l, r) = lerp(k as f32 / steps as f32, 0.0);
                    glass::segment(v, l, r, 1.0, [0.0, 0.0, 0.0, 0.32]);
                }
            }
            ConnectionKind::Ramp => {
                let up = if yb > ya { 1.0 } else { -1.0 };
                for t in [0.3, 0.5, 0.7] {
                    let (l, r) = lerp(t, 0.0);
                    let (l2, r2) = lerp((t + 0.08 * up).clamp(0.0, 1.0), 0.0);
                    let m = [(l2[0] + r2[0]) * 0.5, (l2[1] + r2[1]) * 0.5];
                    glass::segment(v, l, m, 1.2, [0.0, 0.0, 0.0, 0.35]);
                    glass::segment(v, r, m, 1.2, [0.0, 0.0, 0.0, 0.35]);
                }
            }
            ConnectionKind::Secret => {
                let (l, r) = lerp(0.0, 0.0);
                let (l2, r2) = lerp(1.0, 0.0);
                glass::dashes(
                    v,
                    [(l[0] + r[0]) * 0.5, (l[1] + r[1]) * 0.5],
                    [(l2[0] + r2[0]) * 0.5, (l2[1] + r2[1]) * 0.5],
                    2.0,
                    7.0,
                    5.0,
                    [0.62, 0.42, 0.95, 0.95],
                );
            }
            _ => {}
        }
        if conn.critical {
            let dots = (len / 1.6).floor().max(2.0) as usize;
            for k in 1..dots {
                let (l, r) = lerp(k as f32 / dots as f32, 0.0);
                glass::disc(
                    v,
                    [(l[0] + r[0]) * 0.5, (l[1] + r[1]) * 0.5],
                    2.4,
                    [1.0, 0.72, 0.22, 0.95],
                );
            }
        }
    }

    /// A room only glimpsed through a doorway: its footprint, dashed.
    fn chart_ghost(&self, cx: &Ctx, room: &Room, v: &mut Vec<Vertex>) {
        let view = cx.view;
        let pts: Vec<[f32; 2]> = room
            .outline
            .iter()
            .map(|p| view.at(room.center.x + p[0], room.center.y, room.center.z + p[1]))
            .collect();
        fill_polygon(
            v,
            &pts,
            alpha(mix(cx.alt(room.center.y), [0.4, 0.5, 0.55, 1.0], 0.6), 0.10),
        );
        for i in 0..pts.len() {
            glass::dashes(
                v,
                pts[i],
                pts[(i + 1) % pts.len()],
                1.3,
                6.0,
                5.0,
                alpha(mix(cx.alt(room.center.y), [0.7, 0.8, 0.85, 1.0], 0.5), 0.6),
            );
        }
        let c = view.at(room.center.x, room.center.y, room.center.z);
        self.centered(v, "?", c[0], c[1], 22.0, [0.75, 0.85, 0.9, 0.7]);
    }

    /// Room badges float above their floors on a thin stem, nearest last.
    fn chart_badges(&self, cx: &Ctx, v: &mut Vec<Vertex>) {
        let (view, run) = (cx.view, cx.run);
        let player_y = run.controller.position.y;
        let mut rooms: Vec<&Room> = run
            .map
            .rooms
            .iter()
            .filter(|r| cx.known.visited[r.id])
            .collect();
        rooms.sort_by(|a, b| {
            view.depth(a.center.x, a.center.y, a.center.z)
                .total_cmp(&view.depth(b.center.x, b.center.y, b.center.z))
        });
        for room in rooms {
            let floor = view.at(room.center.x, room.center.y, room.center.z);
            let up = view.at(room.center.x, room.center.y + BADGE, room.center.z);
            let color = alpha(mix(cx.alt(room.center.y), [1.0, 1.0, 1.0, 1.0], 0.35), 0.75);
            glass::segment(v, floor, up, 1.3, color);
            glass::disc(v, floor, 2.8, color);
            let radius = match room.role {
                Role::Boss | Role::Treasure | Role::Secret | Role::Exit | Role::Entrance => 16.0,
                Role::Normal => 10.5,
                _ => 14.0,
            };
            medallion(v, up, radius, room.role, room_resolved(run, room));
            let dy = room.center.y - player_y;
            let label = if run.debug {
                Some(format!("{} · {:+.0}", room.id, room.center.y))
            } else if dy.abs() > 0.5 {
                Some(format!("{:+.0} m", dy))
            } else {
                None
            };
            if let Some(label) = label {
                let w = self.width(&label, 13.0) + 14.0;
                let (x, y) = (up[0] - w * 0.5, up[1] + radius + 4.0);
                glass::round(
                    v,
                    [x, y, w, 20.0],
                    10.0,
                    [0.0, 0.01, 0.02, 0.86],
                    [0.0, 0.01, 0.02, 0.86],
                );
                glass::round_outline(
                    v,
                    [x, y, w, 20.0],
                    10.0,
                    1.0,
                    alpha(cx.alt(room.center.y), 0.85),
                );
                self.centered(
                    v,
                    &label,
                    up[0],
                    y + 10.0,
                    13.0,
                    mix(cx.alt(room.center.y), [1.0, 1.0, 1.0, 1.0], 0.6),
                );
            }
        }
    }

    /// The Queen: a heading arrow on her floor, a ring, and a pin that stays
    /// readable over everything, with a line down to the base plane.
    fn chart_player(&self, cx: &Ctx, v: &mut Vec<Vertex>) {
        let (view, run) = (cx.view, cx.run);
        let p = run.controller.position;
        let time = run.map_time;
        let foot = view.at(p.x, p.y, p.z);
        let ground = view.at(p.x, cx.base, p.z);
        glass::dashes(v, foot, ground, 1.4, 5.0, 4.0, [1.0, 0.72, 0.3, 0.6]);
        // Ring on her own floor and on the base plane.
        let circle = |y: f32, r: f32| -> Vec<[f32; 2]> {
            (0..28)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 28.0;
                    view.at(p.x + a.cos() * r, y, p.z + a.sin() * r)
                })
                .collect()
        };
        glass::stroke(v, &circle(cx.base, 1.6), true, 1.2, [1.0, 0.72, 0.3, 0.45]);
        let pulse = (time * 3.0).sin() * 0.5 + 0.5;
        glass::stroke(
            v,
            &circle(p.y, 1.7 + pulse * 0.9),
            true,
            1.6,
            [CRIMSON[0] + 0.2, 0.12, 0.08, 0.35 + 0.4 * (1.0 - pulse)],
        );
        // Heading arrow lying on the floor.
        let f = run.controller.forward();
        let l = (f.x * f.x + f.z * f.z).sqrt().max(0.001);
        let d = [f.x / l, f.z / l];
        let s = [-d[1], d[0]];
        let at = |a: f32, b: f32| {
            view.at(
                p.x + d[0] * a + s[0] * b,
                p.y + 0.05,
                p.z + d[1] * a + s[1] * b,
            )
        };
        let arrow = [at(2.6, 0.0), at(-1.2, 1.5), at(-0.3, 0.0), at(-1.2, -1.5)];
        fill_polygon(v, &arrow, [0.95, 0.16, 0.10, 1.0]);
        glass::stroke(v, &arrow, true, 1.4, [1.0, 0.92, 0.75, 0.95]);
        // The pin.
        let pin = view.at(p.x, p.y + BADGE + 1.2, p.z);
        glass::segment(v, foot, pin, 1.8, [1.0, 0.86, 0.5, 0.9]);
        glass::disc(v, [pin[0], pin[1] + 1.5], 11.0, [0.0, 0.0, 0.0, 0.45]);
        glass::disc(v, pin, 10.0, [0.95, 0.16, 0.10, 1.0]);
        glass::ring(v, pin, 10.0, 2.0, [1.0, 0.92, 0.75, 1.0]);
        glass::disc(v, pin, 3.6, [1.0, 0.95, 0.85, 1.0]);
        self.centered(v, "TÚ", pin[0], pin[1] - 25.0, 14.0, [1.0, 0.92, 0.75, 1.0]);
    }

    /// A metre ruler along the left edge of the chart.
    fn chart_ruler(&self, cx: &Ctx, vp: [f32; 4], v: &mut Vec<Vertex>) {
        let view = cx.view;
        if view.cp < 0.22 {
            self.text(
                v,
                "Vista cenital · el color marca la altura",
                vp[0] + 34.0,
                vp[1] + vp[3] - 84.0,
                13.0,
                alpha(cx.tone.accent, 0.8),
            );
            return;
        }
        let (a0, a1) = (cx.low - 1.0, cx.low + cx.span + 1.0);
        let (y0, y1) = (view.altitude_y(a0), view.altitude_y(a1));
        let x = vp[0] + 46.0;
        if (y0 - y1).abs() < 70.0 {
            return;
        }
        let slices = 16;
        for i in 0..slices {
            let (ta, tb) = (i as f32 / slices as f32, (i + 1) as f32 / slices as f32);
            let (ya, yb) = (y0 + (y1 - y0) * ta, y0 + (y1 - y0) * tb);
            let (ca, cb) = (altitude_color(ta), altitude_color(tb));
            glass::quad(
                v,
                ([x - 3.0, ya], alpha(ca, 0.9)),
                ([x + 3.0, ya], alpha(ca, 0.9)),
                ([x + 3.0, yb], alpha(cb, 0.9)),
                ([x - 3.0, yb], alpha(cb, 0.9)),
            );
        }
        let px_per_m = (y0 - y1).abs() / (a1 - a0);
        let step = [1.0f32, 2.0, 4.0, 5.0, 10.0, 20.0]
            .into_iter()
            .find(|s| s * px_per_m >= 30.0)
            .unwrap_or(20.0);
        let mut m = (a0 / step).ceil() * step;
        while m <= a1 {
            let y = view.altitude_y(m);
            glass::segment(v, [x + 4.0, y], [x + 13.0, y], 1.2, [1.0, 1.0, 1.0, 0.7]);
            self.mid(
                v,
                &format!("{:+.0}", m),
                x + 18.0,
                y,
                13.0,
                alpha(IVORY, 0.85),
            );
            m += step;
        }
        self.text(v, "m", x - 3.0, y1 - 24.0, 13.0, alpha(cx.tone.accent, 0.9));
    }
}

/// Plan corners of a corridor: start-left, end-left, end-right, start-right,
/// pushed `extend` metres into the rooms at both ends.
fn ribbon(conn: &Connection, extend: f32) -> [[f32; 2]; 4] {
    let d = [conn.end.x - conn.start.x, conn.end.z - conn.start.z];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.01);
    let u = [d[0] / len, d[1] / len];
    let n = [-u[1], u[0]];
    let half = (conn.width * 0.5).max(0.8);
    let a = [conn.start.x - u[0] * extend, conn.start.z - u[1] * extend];
    let b = [conn.end.x + u[0] * extend, conn.end.z + u[1] * extend];
    [
        [a[0] + n[0] * half, a[1] + n[1] * half],
        [b[0] + n[0] * half, b[1] + n[1] * half],
        [b[0] - n[0] * half, b[1] - n[1] * half],
        [a[0] - n[0] * half, a[1] - n[1] * half],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(yaw: f32, pitch: f32) -> View {
        let (s, c) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        View {
            c,
            s,
            sp,
            cp,
            scale: 10.0,
            origin: [500.0, 400.0],
            focus: [0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn higher_floors_draw_higher_on_screen_and_later() {
        for yaw in [0.0, 0.7, 2.0, 4.0] {
            let v = view(yaw, 0.9);
            let low = v.at(3.0, 0.0, -2.0);
            let high = v.at(3.0, 6.0, -2.0);
            assert!(
                high[1] < low[1],
                "altitude must lift the point up the screen"
            );
            assert!((high[0] - low[0]).abs() < 1e-3, "altitude must not shift x");
            assert!(
                v.depth(3.0, 6.0, -2.0) > v.depth(3.0, 0.0, -2.0),
                "the upper floor is nearer the camera"
            );
        }
    }

    #[test]
    fn top_down_view_ignores_altitude() {
        let v = view(0.3, std::f32::consts::FRAC_PI_2);
        let a = v.at(4.0, 0.0, 5.0);
        let b = v.at(4.0, 9.0, 5.0);
        assert!((a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3);
    }

    #[test]
    fn rotation_keeps_distances_on_the_ground() {
        let base = view(0.0, std::f32::consts::FRAC_PI_2);
        for yaw in [0.5, 1.7, 3.1] {
            let turned = view(yaw, std::f32::consts::FRAC_PI_2);
            let d0 = |v: View| {
                let (a, b) = (v.at(0.0, 0.0, 0.0), v.at(3.0, 0.0, 4.0));
                (a[0] - b[0]).hypot(a[1] - b[1])
            };
            assert!((d0(base) - d0(turned)).abs() < 1e-3);
        }
    }

    #[test]
    fn edge_normals_point_away_from_the_centre_either_winding() {
        let square = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let mut reversed = square;
        reversed.reverse();
        for outline in [square, reversed] {
            for (i, n) in edge_normals(&outline).into_iter().enumerate() {
                let (a, b) = (outline[i], outline[(i + 1) % 4]);
                let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
                assert!(n[0] * mid[0] + n[1] * mid[1] > 0.5);
            }
        }
    }
}
