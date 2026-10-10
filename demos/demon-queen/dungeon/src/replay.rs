//! Deterministic end-to-end playthrough. Uses live run actions and kinematic
//! movement, never teleports, edits objective flags, disables AI or grants HP.
use super::*;
#[derive(serde::Serialize, Debug)]
pub struct Report {
    pub seed: u64,
    pub frames: usize,
    pub distance: f32,
    pub gold: u32,
    pub health: f32,
    pub seal: bool,
    pub crest: bool,
    pub boss_defeated: bool,
    pub extracted: bool,
}
/// Counts simulated frames and shows each one to an observer (a renderer
/// timing the walk, or nothing).
struct Tick<'a> {
    n: usize,
    observe: &'a mut dyn FnMut(&Dungeon),
}
impl Tick<'_> {
    fn step(&mut self, run: &Dungeon) {
        self.n += 1;
        (self.observe)(run);
    }
}
pub fn complete(run: &mut Dungeon) -> Result<Report, String> {
    complete_observed(run, &mut |_| {})
}
/// Play the run to the end, handing every frame to `observe`.
pub fn complete_observed(
    run: &mut Dungeon,
    observe: &mut dyn FnMut(&Dungeon),
) -> Result<Report, String> {
    let mut frames = Tick { n: 0, observe };
    fn frame(run: &mut Dungeon, target: Vec3, frames: &mut Tick<'_>) {
        let delta = horizontal(target - run.controller.position);
        if run.health < 60.0 {
            run.heal();
        }
        run.controller.yaw = (-delta.x).atan2(-delta.z);
        run.controller.pitch = 0.0;
        run.update(
            Input {
                forward: if delta.length() < 0.1 { 0.0 } else { 1.0 },
                sprint: true,
                ..Default::default()
            },
            1.0 / 60.0,
        );
        frames.step(run);
    }
    /// Steer round the first solid dressing circle across the straight
    /// line to `target` (themed centrepieces, heaps, stalagmites).
    fn detour(run: &Dungeon, target: Vec3) -> Vec3 {
        let p = run.controller.position;
        let room = run.map.region_at(p);
        let d = horizontal(target - p);
        let len = d.length();
        if len < 1.0e-3 {
            return target;
        }
        let dir = d * (1.0 / len);
        let blocker = run.decor[room]
            .props
            .iter()
            .filter(|prop| prop.radius > 0.0 && (prop.position.y - p.y).abs() < 1.2)
            .filter_map(|prop| {
                let t = horizontal(prop.position - p).dot(dir);
                let off = horizontal(prop.position - (p + dir * t)).length();
                (t > 0.0 && t < len + prop.radius && off < prop.radius + PLAYER_RADIUS + 0.05)
                    .then_some((t, prop.position, prop.radius))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, c, r)) = blocker else {
            return target;
        };
        let side = Vec3::new(-dir.z, 0.0, dir.x);
        let sign = if horizontal(p - c).dot(side) >= 0.0 {
            1.0
        } else {
            -1.0
        };
        c + side * (sign * (r + PLAYER_RADIUS + 0.4)) + Vec3::Y * (p.y - c.y)
    }
    fn walk(run: &mut Dungeon, target: Vec3, frames: &mut Tick<'_>) -> Result<(), String> {
        // Room centres can hold solid dressing (a themed centrepiece): aim
        // at the nearest point the player can actually stand on.
        let room = run.map.region_at(target);
        let target = decor::resolve(&run.decor[room], target, PLAYER_RADIUS + 0.05);
        let max = (((target - run.controller.position).length() / 2.0 + 10.0) * 60.0) as usize;
        for _ in 0..max {
            if horizontal(target - run.controller.position).length() < 0.18
                && (target.y - run.controller.position.y).abs() < 0.35
            {
                return Ok(());
            }
            let aim = detour(run, target);
            frame(run, aim, frames);
        }
        let current_room = run.map.region_at(run.controller.position);
        let nearby = run
            .actors
            .iter()
            .filter(|actor| {
                actor.health > 0.0
                    && run.map.spawns[actor.spawn].room == current_room
                    && run.map.spawns[actor.spawn].health > 0.0
            })
            .map(|actor| {
                (
                    actor.spawn,
                    run.map.spawns[actor.spawn].definition.as_str(),
                    horizontal(actor.position - run.controller.position).length(),
                    actor.position,
                )
            })
            .collect::<Vec<_>>();
        Err(format!(
            "seed {}: live run stuck at {:?}, target {:?}, room {} {:?}, yaw {:.2}, nearby {:?}, HP {}, seal={}, crest={}",
            run.seed(),
            run.controller.position,
            target,
            current_room,
            run.map.rooms[current_room].role,
            run.controller.yaw,
            nearby,
            run.health,
            run.seal,
            run.crest
        ))
    }
    for index in 1..run.map.critical_path.len() {
        let from = run.map.critical_path[index - 1];
        let to = run.map.critical_path[index];
        let points = run.map.route_points(from, to);
        for p in points.iter().skip(1) {
            walk(run, *p, &mut frames)?;
        }
        let targets = run
            .actors
            .iter()
            .filter(|a| run.map.spawns[a.spawn].room == to && a.health > 0.0)
            .map(|a| a.spawn)
            .collect::<Vec<_>>();
        for id in targets {
            // Champions take several times the cuts of an ordinary foe.
            let budget =
                (1800.0 * super::combat::toughness(run.map.spawns[id].kind) * 1.5) as usize;
            for _ in 0..budget {
                if run.actors[id].health <= 0.0 {
                    break;
                }
                let room = run.map.spawns[id].room;
                let current = run.map.region_at(run.controller.position);
                if current != room {
                    let route = run.map.route_points(current, room);
                    if let Some(next) = route
                        .iter()
                        .skip(1)
                        .find(|point| horizontal(**point - run.controller.position).length() > 0.35)
                    {
                        walk(run, *next, &mut frames)?;
                    }
                    continue;
                }
                let p = run.actors[id].position;
                let offset = horizontal(run.controller.position - p);
                let distance = offset.length();
                // The authored sword reaches 1.85 m plus the target capsule.
                // Waiting for an arbitrary 2.05 m center distance made the
                // replay keep chasing a scout even when it could hit already.
                if distance <= SWORD_REACH + 0.43 {
                    let aim = p + Vec3::Y
                        * if run.map.spawns[id].kind == SpawnKind::Boss {
                            1.65
                        } else {
                            0.95
                        };
                    let d = aim - run.controller.eye();
                    run.controller.yaw = (-d.x).atan2(-d.z);
                    run.controller.pitch = (d.y / d.length().max(0.001)).asin();
                    run.swing_sword();
                    run.update(Input::default(), 1.0 / 60.0);
                    frames.step(run);
                    continue;
                }
                // Navigate around authored cover instead of charging straight
                // at a retreating crossbow scout or stepping off a ledge.
                let goal = p + offset.normalize_or_zero() * 1.35;
                let path =
                    navigation::room_path(&run.map, room, run.controller.position, goal, 0.30);
                let waypoint = path
                    .into_iter()
                    .find(|point| horizontal(*point - run.controller.position).length() > 0.35)
                    .unwrap_or(goal);
                frame(run, waypoint, &mut frames);
            }
            if run.actors[id].health > 0.0 {
                return Err(format!(
                    "seed {}: unresolved encounter {id} ({:?} {} room {}): HP {:.1}, player {:?} room {}, enemy {:?}",
                    run.seed(),
                    run.map.spawns[id].kind,
                    run.map.spawns[id].definition,
                    run.map.spawns[id].room,
                    run.actors[id].health,
                    run.controller.position,
                    run.map.region_at(run.controller.position),
                    run.actors[id].position,
                ));
            }
        }
        let actors = run
            .actors
            .iter()
            .filter(|a| run.map.spawns[a.spawn].room == to && run.map.spawns[a.spawn].health == 0.0)
            .map(|a| a.spawn)
            .collect::<Vec<_>>();
        for id in actors {
            let s = run.map.spawns[id].clone();
            if s.kind == SpawnKind::Trap {
                continue;
            }
            let center = run.map.rooms[to].center;
            walk(run, center, &mut frames)?;
            let toward = horizontal(center - s.position).normalize_or_zero();
            walk(run, s.position + toward * 1.2, &mut frames)?;
            let presses = if s.kind == SpawnKind::Seal {
                s.definition
                    .trim_start_matches("sigil_")
                    .parse::<usize>()
                    .unwrap()
                    + 1
            } else {
                1
            };
            for _ in 0..presses {
                if run.nearby_actor() != Some(id) {
                    return Err(format!("wrong interaction target {id}"));
                }
                run.interact();
            }
        }
        if !run.pending_exit {
            walk(run, run.map.rooms[to].center, &mut frames)?;
        }
    }
    if !(run.seal && run.crest && run.won && run.pending_exit) {
        return Err("playthrough did not complete every objective".into());
    }
    Ok(Report {
        seed: run.seed(),
        frames: frames.n,
        distance: run.controller.distance,
        gold: run.gold,
        health: run.health,
        seal: run.seal,
        crest: run.crest,
        boss_defeated: run.won,
        extracted: run.pending_exit,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_run_can_be_completed() {
        for seed in [0, 1, 2, 42, 4815] {
            let mut run = Dungeon::generate(seed);
            println!("{:?}", complete(&mut run).unwrap());
        }
    }
}
