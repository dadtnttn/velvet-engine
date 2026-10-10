//! The QA walker: plays the game the way a player would and writes down what goes wrong.
//!
//! For every zone (the castle, the Dark Forest, the villa, the springs and their rooms, the
//! brothel's ground floor) it builds a navmesh from the zone's own collision ([`super::nav`]), so
//! it knows everywhere one ought to be able to get to. Then it drives the Queen there with the
//! real controls (W toward each corner of the way, the real `tick`: the same collisions, doors,
//! maids and girls a player meets), and keeps a log:
//!
//! - **stuck**: the navmesh says there is a way, but she cannot get along it (a snag in the
//!   collision a player would hit); with a screenshot.
//! - **islands**: walkable ground nobody can reach from where the zone starts.
//! - **frames**: how long frames take (worst, 95th percentile) and where the slow ones were.
//! - **prompts**: every F prompt met, and the known interactables (doors, hotspots) that showed
//!   none when she stood at them.
//! - **broken**: a position that is not a number, or out of the zone's bounds; a panic.
//!
//! `demon-queen-3d --capture-toon <out> --qa-bot` writes `qa-report.md`, `qa-report.json` and the
//! screenshots to `<out>`.
//!
//! What any game's walker does (surveying a navmesh, spreading targets, following a way and
//! telling when it is stuck, frame times, the report) is velvet-qa's; this is the game's side: its
//! zones, how to enter each, where she can stand, how to hold the keys, who is in the way.

use super::nav::NavMesh;
use super::*;
use velvet_qa::{
    eight_way_keys, spread_targets, write_report, zone_map, FrameTimes, Incident, PathFollower,
    Progress, Survey, ZoneReport,
};

/// One zone to walk: how to get there, where one can stand, where it starts, what to try.
struct Zone {
    name: &'static str,
    enter: fn(&mut App),
    bounds: [f32; 4],
    /// Where the player can stand (the zone's own collision).
    walkable: Box<dyn Fn(&App, f32, f32) -> bool>,
    /// Points where an F prompt should show.
    interactables: Vec<(String, [f32; 2])>,
}

fn castle_walkable(app: &App, x: f32, z: f32) -> bool {
    // The castle from its prototype, when it is the one in use.
    app.castle.is_some() && !castle3d::plan().blocked(x, z, 0.3, 0)
}

fn zones() -> Vec<Zone> {
    let mut list = Vec::new();
    let castle_spots = |level: u8| -> Vec<(String, [f32; 2])> {
        castle3d::plan()
            .hotspots
            .iter()
            .filter(|h| h.id != "lira" && h.id != "nemi" && h.level == level)
            .map(|h| (h.id.clone(), [h.x, h.z]))
            .collect()
    };
    list.push(Zone {
        name: "castillo",
        enter: |app| {
            app.nature_mode = false;
            app.village_entered = false;
            app.baths = None;
            app.brothel = None;
            app.castle = None;
            app.player = Vec3::new(-17.6, 0.0, -12.6);
        },
        bounds: [-40.0, -40.0, 40.0, 40.0],
        walkable: Box::new(castle_walkable),
        interactables: castle_spots(0),
    });
    // The castle's upper floor (the stair between the two is walked in castle3d's own tests).
    list.push(Zone {
        name: "castillo-alto",
        enter: |app| {
            app.nature_mode = false;
            app.village_entered = false;
            app.baths = None;
            app.brothel = None;
            app.castle = None;
            app.player = Vec3::new(1.6, castle3d::plan().level_h, 15.0);
        },
        bounds: [-40.0, -40.0, 40.0, 40.0],
        walkable: Box::new(|_, x, z| !castle3d::plan().blocked(x, z, 0.3, 1)),
        interactables: castle_spots(1),
    });
    list.push(Zone {
        name: "bosque",
        enter: |app| {
            app.baths = None;
            app.brothel = None;
            app.enter_nature_from_castle();
        },
        bounds: [
            -NATURE_PLAYABLE_LIMIT,
            -NATURE_PLAYABLE_LIMIT,
            NATURE_PLAYABLE_LIMIT,
            NATURE_PLAYABLE_LIMIT,
        ],
        walkable: Box::new(|_, x, z| !nature_position_blocked(Vec3::new(x, 0.0, z))),
        interactables: Vec::new(),
    });
    let c = fantasy_village_center();
    let doors: Vec<(String, [f32; 2])> = village::building_anchors()
        .iter()
        .filter_map(|a| {
            village::door_anchor(a.footprint_index).map(|(door, out)| {
                (
                    a.id.to_string(),
                    [door.x + out.x * 1.2, door.z + out.z * 1.2],
                )
            })
        })
        .collect();
    list.push(Zone {
        name: "villa",
        enter: |app| {
            app.baths = None;
            app.brothel = None;
            app.nature_mode = true;
            app.village_entered = true;
            let (spawn, _) = village::spawn();
            app.player = spawn;
        },
        bounds: [
            c.x - village::RADIUS,
            c.z - village::RADIUS,
            c.x + village::RADIUS,
            c.z + village::RADIUS,
        ],
        walkable: Box::new(|_, x, z| !fantasy_village_position_blocked(Vec3::new(x, 0.0, z))),
        interactables: doors,
    });
    let p = baths::plan();
    list.push(Zone {
        name: "termas",
        enter: |app| {
            app.brothel = None;
            app.nature_mode = true;
            app.village_entered = true;
            let state = baths::State::enter(Vec3::ZERO, 0.0);
            app.player = state.position();
            app.baths = Some(state);
        },
        bounds: [-24.0, -22.0, 24.0, 22.0],
        walkable: Box::new(|_, x, z| !baths::plan().blocked_outdoor(x, z, 0.3)),
        interactables: p
            .hotspots
            .iter()
            .map(|h| (h.label.clone(), [h.x, h.z]))
            .collect(),
    });
    for room in ["recepcion", "privada"] {
        let Some(r) = p.rooms.get(room) else { continue };
        list.push(Zone {
            name: if room == "recepcion" {
                "termas-recepcion"
            } else {
                "termas-privada"
            },
            enter: if room == "recepcion" {
                |app| enter_baths_room(app, "recepcion")
            } else {
                |app| enter_baths_room(app, "privada")
            },
            bounds: [
                r.center.x - r.w / 2.0,
                r.center.z - r.d / 2.0,
                r.center.x + r.w / 2.0,
                r.center.z + r.d / 2.0,
            ],
            walkable: Box::new(move |_, x, z| {
                !baths::plan()
                    .rooms
                    .get(room)
                    .is_some_and(|r| r.blocked_public(x, z, 0.3))
            }),
            interactables: r
                .hotspots
                .iter()
                .map(|h| (h.label.clone(), [h.x, h.z]))
                .collect(),
        });
    }
    list.push(Zone {
        name: "burdel",
        enter: |app| {
            app.baths = None;
            app.nature_mode = true;
            app.village_entered = true;
            app.player = village::entrance();
            app.enter_brothel();
            app.queen_thought = None;
        },
        bounds: brothel::plan().bounds(),
        walkable: Box::new(|_, x, z| !brothel::plan().blocked(x, z, 0)),
        interactables: brothel::plan()
            .hotspots
            .iter()
            .filter(|h| h.level == 0)
            .map(|h| (h.label.clone(), [h.x, h.z]))
            .collect(),
    });
    list
}

fn enter_baths_room(app: &mut App, room: &str) {
    app.brothel = None;
    app.nature_mode = true;
    app.village_entered = true;
    let mut state = baths::State::enter(Vec3::ZERO, 0.0);
    state.room = Some(room.to_owned());
    if let Some(r) = baths::plan().rooms.get(room) {
        (state.x, state.z) = (r.spawn.x, r.spawn.z);
    }
    app.player = state.position();
    app.baths = Some(state);
}

/// Where she is, by whichever walker the zone uses.
fn here(app: &App) -> [f32; 2] {
    if let Some(b) = &app.baths {
        return [b.x, b.z];
    }
    if let Some(b) = &app.brothel {
        return [b.walker.x, b.walker.z];
    }
    [app.player.x, app.player.z]
}

/// Holds the keys that walk her toward `dir`: the camera turned so W points there when the zone
/// lets it turn, else the best of the eight directions.
fn steer(app: &mut App, dir: [f32; 2]) {
    let len = dir[0].hypot(dir[1]);
    app.move_forward = false;
    app.move_back = false;
    app.move_left = false;
    app.move_right = false;
    if len < 1e-4 {
        return;
    }
    let d = [dir[0] / len, dir[1] / len];
    let fixed_view = app.baths.as_ref().is_some_and(|b| b.room.is_some()) || app.brothel.is_some();
    if !fixed_view {
        // forward = (-sin yaw, -cos yaw) = d.
        let yaw = (-d[0]).atan2(-d[1]);
        app.iso.yaw_target = yaw;
        app.iso.yaw = yaw;
        app.move_forward = true;
        return;
    }
    let (forward, right) = app.iso.walk_axes();
    (
        app.move_forward,
        app.move_back,
        app.move_left,
        app.move_right,
    ) = eight_way_keys([forward.x, forward.z], [right.x, right.z], d);
}

/// One frame of play: the tick, and every `render_every` frames a render (timed).
fn frame(app: &mut App, frames: &mut FrameTimes, n: usize, render_every: usize) {
    let dt = 1.0 / 60.0;
    app.tick(dt);
    if n % render_every == 0 {
        let t0 = Instant::now();
        app.render_frame();
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        if frames.record(ms, here(app)) {
            let full = app
                .state
                .as_ref()
                .is_some_and(|s| s.renderer.last_shadow_full);
            let tris = app
                .state
                .as_ref()
                .map_or(0, |s| s.renderer.last_drawn_triangles);
            eprintln!(
                "[qa] slow frame {ms:.0} ms at {:?}: full shadow redraw {full}, {tris} triangles",
                here(app)
            );
        }
    }
}

fn shot(app: &mut App, out: &Path, name: &str) -> Option<String> {
    app.render_frame();
    let file = format!("{name}.png");
    app.capture(&out.join(&file)).ok().map(|_| file)
}

/// Walks every zone; writes the report into `out`.
pub(crate) fn run(app: &mut App, out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    app.qa_start = app.player;
    app.environment_cycle.set_phase(0.45);
    let mut reports = Vec::new();
    let mut incidents: Vec<Incident> = Vec::new();
    let only: Option<String> = std::env::args().skip_while(|a| a != "--qa-zone").nth(1);
    let targets_per_zone: usize = std::env::args()
        .skip_while(|a| a != "--qa-targets")
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);
    for zone in zones() {
        if only.as_deref().is_some_and(|o| o != zone.name) {
            continue;
        }
        let started = Instant::now();
        (zone.enter)(app);
        // Let the zone load and settle.
        for _ in 0..30 {
            app.tick(1.0 / 60.0);
            app.render_frame();
        }
        let start = here(app);
        let walk = &zone.walkable;
        // The castle's real extent is its rooms' bounds.
        let bounds = match (zone.name, app.state.as_ref()) {
            ("castillo" | "castillo-alto", _) if app.castle.is_some() => {
                let [x0, x1, z0, z1] = castle3d::plan().bounds();
                [x0, z0, x1, z1]
            }
            _ => zone.bounds,
        };
        let nav = NavMesh::build(bounds, 0.25, 10.0, |x, z| walk(app, x, z).then_some(0.0));
        let survey = Survey::of(&nav, start);
        let mut report = ZoneReport {
            zone: zone.name.to_owned(),
            walkable_m2: survey.walkable_m2,
            reachable_m2: survey.reachable_m2,
            outside_m2: survey.outside_m2,
            ..Default::default()
        };
        // Islands: connected walkable pieces she cannot get to, over a square metre.
        for &(c, area) in &survey.islands {
            report.islands += 1;
            // A look at it: the Queen put there for a moment (where the zone lets us).
            let picture = if app.baths.is_none() && app.brothel.is_none() {
                let keep = app.player;
                app.player = Vec3::new(c[0], keep.y, c[1]);
                app.camera_focus = app.player;
                let shot = shot(app, out, &format!("{}-isla-{}", zone.name, report.islands));
                app.player = keep;
                app.camera_focus = keep;
                shot
            } else {
                None
            };
            incidents.push(Incident {
                zone: zone.name.into(),
                kind: "island",
                at: c,
                detail: format!("{area:.1} m² of walkable ground cut off from the start"),
                shot: picture,
            });
        }
        if !nav.walkable_at(start[0], start[1]) {
            incidents.push(Incident {
                zone: zone.name.into(),
                kind: "broken",
                at: start,
                detail: "the zone starts her on ground her own collision rejects".into(),
                shot: shot(app, out, &format!("{}-start", zone.name)),
            });
        }
        // Targets: spread over the reachable ground (farthest point first).
        let mut targets: Vec<(String, [f32; 2])> =
            spread_targets(&nav, &survey.reach, start, targets_per_zone, 0.25)
                .into_iter()
                .enumerate()
                .map(|(k, c)| (format!("t{k}"), c))
                .collect();
        // Interactables after the sweep, each walked to.
        for (label, at) in &zone.interactables {
            if let Some(p) = nav.nearest(at[0], at[1], 1.5) {
                targets.push((format!("F:{label}"), p));
            }
        }
        report.targets = targets.len();
        report.interactables = zone.interactables.len();
        let mut frames = FrameTimes::default();
        let mut prompts = BTreeSet::new();
        let mut n = 0usize;
        for (label, target) in &targets {
            let from = here(app);
            let Some(mut follow) = PathFollower::new(&nav, from, *target) else {
                continue;
            };
            let mut ok = false;
            let mut last = from;
            while follow.in_time() {
                let p = here(app);
                if !p[0].is_finite() || !p[1].is_finite() {
                    incidents.push(Incident {
                        zone: zone.name.into(),
                        kind: "broken",
                        at: last,
                        detail: format!("her position became {p:?} walking to {label}"),
                        shot: None,
                    });
                    break;
                }
                if p[0] < zone.bounds[0] - 1.0
                    || p[0] > zone.bounds[2] + 1.0
                    || p[1] < zone.bounds[1] - 1.0
                    || p[1] > zone.bounds[3] + 1.0
                {
                    incidents.push(Incident {
                        zone: zone.name.into(),
                        kind: "broken",
                        at: p,
                        detail: format!("out of the zone's bounds walking to {label}"),
                        shot: shot(
                            app,
                            out,
                            &format!("{}-{}-bounds", zone.name, label.replace([':', ' '], "_")),
                        ),
                    });
                    break;
                }
                report.walked_m += (p[0] - last[0]).hypot(p[1] - last[1]);
                last = p;
                let Some(c) = follow.corner(&nav, p) else {
                    ok = true;
                    break;
                };
                steer(app, [c[0] - p[0], c[1] - p[1]]);
                frame(app, &mut frames, n, 5);
                n += 1;
                if n % 10 == 0 {
                    if let Some(prompt) = app.interaction_prompt() {
                        prompts.insert(match &prompt.detail {
                            Some(d) => format!("{} · {}", prompt.verb, d),
                            None => prompt.verb.clone(),
                        });
                    }
                }
                match follow.progress(&nav, p, c, 1.0 / 60.0) {
                    Progress::Going => {}
                    Progress::Replanned => continue,
                    Progress::Stuck => break,
                }
            }
            app.move_forward = false;
            app.move_back = false;
            app.move_left = false;
            app.move_right = false;
            if ok {
                report.reached += 1;
                if let Some(name) = label.strip_prefix("F:") {
                    // Stand a moment: does a prompt show?
                    let mut seen = None;
                    for _ in 0..20 {
                        frame(app, &mut frames, n, 5);
                        n += 1;
                        if let Some(prompt) = app.interaction_prompt() {
                            seen = Some(prompt.verb.clone());
                        }
                    }
                    if seen.is_some() {
                        report.interactables_ok += 1;
                    } else {
                        let p = here(app);
                        incidents.push(Incident {
                            zone: zone.name.into(),
                            kind: "no-prompt",
                            at: p,
                            detail: format!("standing at «{name}» shows no F prompt"),
                            shot: shot(
                                app,
                                out,
                                &format!("{}-noprompt-{}", zone.name, incidents.len()),
                            ),
                        });
                    }
                }
            } else {
                report.stuck += 1;
                let p = here(app);
                // What stops her: the maids (castle) or the girls (springs) in the way, if any.
                let mut blockers = Vec::new();
                if let (Some(inside), false) = (&app.castle, app.nature_mode) {
                    for (i, name) in ["Lira", "Nemi"].iter().enumerate() {
                        let (at, _) = inside.maid(i);
                        let d = (at.x - p[0]).hypot(at.z - p[1]);
                        if d < 1.6 {
                            blockers.push(format!("{name} a {d:.1} m"));
                        }
                    }
                }
                if let Some(b) = &app.baths {
                    for o in &b.others {
                        let d = (o[0] - p[0]).hypot(o[1] - p[1]);
                        if d < 1.6 {
                            blockers.push(format!("una chica a {d:.1} m"));
                        }
                    }
                }
                incidents.push(Incident {
                    zone: zone.name.into(),
                    kind: "stuck",
                    at: p,
                    detail: format!(
                        "could not get to {label} at ({:.1}, {:.1}); {:.1} m short{}",
                        target[0],
                        target[1],
                        follow.short_by(),
                        if blockers.is_empty() {
                            String::new()
                        } else {
                            format!(" — en el camino: {}", blockers.join(", "))
                        }
                    ),
                    shot: shot(
                        app,
                        out,
                        &format!("{}-stuck-{}", zone.name, incidents.len()),
                    ),
                });
                // Start the next one from somewhere she can reach.
                (zone.enter)(app);
                for _ in 0..10 {
                    app.tick(1.0 / 60.0);
                }
            }
        }
        // The zone's map: reachable ground green, islands red, where she got stuck marked.
        let snags: Vec<[f32; 2]> = incidents
            .iter()
            .filter(|i| i.zone == zone.name && i.kind == "stuck")
            .map(|i| i.at)
            .collect();
        let (w, h, px) = zone_map(&nav, &survey.reach, start, &snags, 4);
        let _ = image::save_buffer(
            out.join(format!("{}-mapa.png", zone.name)),
            &px,
            w,
            h,
            image::ColorType::Rgba8,
        );
        // Frame times (the first frames of loading left out).
        if let Some((mean, p95, worst)) = frames.stats() {
            report.frame_worst_ms = worst;
            report.frame_p95_ms = p95;
            report.frame_mean_ms = mean;
        }
        for (ms, at) in frames.slowest(3) {
            incidents.push(Incident {
                zone: zone.name.into(),
                kind: "slow-frame",
                at,
                detail: format!("{ms:.0} ms frame"),
                shot: None,
            });
        }
        report.prompts = prompts.into_iter().collect();
        report.seconds = started.elapsed().as_secs_f32();
        eprintln!("[qa] {}", report.summary());
        reports.push(report);
    }
    app.baths = None;
    app.brothel = None;
    write_report(out, &reports, &incidents)?;
    eprintln!(
        "[qa] {} incidents; report in {}",
        incidents.len(),
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The brothel's own walker, driven along the navmesh's ways with the eight directions the
    /// fixed view allows (the QA walker's steering): every reachable target must be reached.
    #[test]
    fn the_brothel_ground_floor_can_be_walked_everywhere() {
        let p = brothel::plan();
        let nav = NavMesh::build(p.bounds(), 0.25, 10.0, |x, z| {
            (!p.blocked(x, z, 0)).then_some(0.0)
        });
        let start = brothel::Walker {
            x: p.spawn.x,
            z: p.spawn.z,
            level: 0,
            on_stair: false,
        };
        let reach = nav.reachable_from(start.x, start.z);
        let yaw = std::f32::consts::FRAC_PI_4;
        let (s, c) = yaw.sin_cos();
        let dirs: Vec<[f32; 2]> = [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, -1.0),
            (-1.0, 1.0),
        ]
        .iter()
        .map(|&(fw, rt): &(f32, f32)| {
            let v = [-s * fw + c * rt, -c * fw - s * rt];
            let l = v[0].hypot(v[1]);
            [v[0] / l, v[1] / l]
        })
        .collect();
        let mut failures = Vec::new();
        for r in (0..nav.rects.len() as u32).filter(|&r| reach[r as usize]) {
            let target = nav.rect_center(r);
            let mut w = start;
            let mut follow = PathFollower::new(&nav, [w.x, w.z], target).unwrap();
            let mut ok = false;
            for _ in 0..(60 * 30) {
                let here = [w.x, w.z];
                let Some(cn) = follow.corner(&nav, here) else {
                    ok = true;
                    break;
                };
                let d = [cn[0] - here[0], cn[1] - here[1]];
                let l = d[0].hypot(d[1]).max(1e-6);
                let best = dirs
                    .iter()
                    .copied()
                    .max_by(|a, b| {
                        (a[0] * d[0] + a[1] * d[1])
                            .partial_cmp(&(b[0] * d[0] + b[1] * d[1]))
                            .unwrap()
                    })
                    .unwrap();
                let _ = l;
                let step = 2.2 / 60.0 / 4.0;
                for _ in 0..4 {
                    w = p.slide(w, best[0] * step, best[1] * step, false);
                }
            }
            if !ok {
                failures.push((target, [w.x, w.z]));
            }
        }
        assert!(
            failures.is_empty(),
            "{} unreachable: {:?}",
            failures.len(),
            &failures[..failures.len().min(8)]
        );
    }
}
