//! Contextual placement and budgets, intentionally independent of layout RNG.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Difficulty {
    pub depth: u32,
    pub group_size: u32,
    pub build_power: f32,
    pub run_modifier: f32,
}
impl Default for Difficulty {
    fn default() -> Self {
        Self {
            depth: 1,
            group_size: 1,
            build_power: 1.0,
            run_modifier: 1.0,
        }
    }
}
impl Difficulty {
    pub fn validate(&self) -> Result<(), String> {
        if self.group_size == 0
            || self.group_size > 8
            || !self.build_power.is_finite()
            || !(0.1..=10.0).contains(&self.build_power)
            || !self.run_modifier.is_finite()
            || !(0.1..=10.0).contains(&self.run_modifier)
        {
            Err("invalid difficulty inputs".into())
        } else {
            Ok(())
        }
    }
    pub fn budget(&self, room: &Room, biome: &Biome, config: &Config) -> f32 {
        (1.0 + room.progress * 2.2 + self.depth.min(100) as f32 * 0.1)
            * biome.danger
            * config.danger_level
            * (1.0 + (self.group_size - 1) as f32 * 0.5)
            * self.build_power.sqrt()
            * self.run_modifier
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpawnKind {
    Enemy,
    Elite,
    Miniboss,
    Boss,
    Chest,
    Seal,
    Rest,
    Memory,
    Exit,
    Entrance,
    Trap,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spawn {
    pub id: usize,
    pub room: usize,
    pub kind: SpawnKind,
    pub definition: String,
    pub position: Vec3,
    pub health: f32,
    pub damage: f32,
    pub speed: f32,
    pub gold: u32,
    pub budget: f32,
}
pub fn place(map: &Blueprint, catalog: &Catalog, difficulty: &Difficulty) -> Vec<Spawn> {
    let mut out = Vec::new();
    // Give one mid-run stalker encounter a readable miniboss beat. The choice
    // uses gameplay data only; layout and decoration streams stay untouched.
    let miniboss_encounter = map.rooms.iter()
        .filter(|r|r.role==Role::Encounter&&r.critical)
        // Keep the champion in the mid-run combat beat: early enough to be
        // discoverable, but after players have met the ordinary ambush roles.
        .min_by(|a,b|(a.progress-0.48).abs().total_cmp(&(b.progress-0.48).abs()))
        .or_else(||map.rooms.iter().find(|r|r.role==Role::Encounter))
        .map(|r|r.id);
    for r in &map.rooms {
        let biome = &catalog.biomes[r.biome];
        let budget = difficulty.budget(r, biome, &map.config);
        let mut rng = rng::Rng::stream(map.seed, "gameplay_v2", r.id as u64);
        let mut loot = rng::Rng::stream(map.seed, "loot_v2", r.id as u64);
        let kind = match r.role {
            Role::Entrance => Some(SpawnKind::Entrance),
            Role::Exit => Some(SpawnKind::Exit),
            Role::Treasure | Role::Secret => Some(SpawnKind::Chest),
            Role::Puzzle => Some(SpawnKind::Seal),
            Role::Rest => Some(SpawnKind::Rest),
            Role::Event => Some(SpawnKind::Memory),
            _ => None,
        };
        if kind == Some(SpawnKind::Seal) {
            for i in 0..3 {
                out.push(Spawn {
                    id: out.len(),
                    room: r.id,
                    kind: SpawnKind::Seal,
                    definition: format!("sigil_{i}"),
                    position: r.center + Vec3::new((i as f32 - 1.0) * 2.1, 0.0, -r.half[1] * 0.52),
                    health: 0.0,
                    damage: 0.0,
                    speed: 0.0,
                    gold: 0,
                    budget,
                });
            }
        } else if let Some(kind) = kind {
            let def = &catalog.loot[catalog
                .loot
                .iter()
                .position(|l| l.id == biome.loot_pool[0])
                .unwrap()];
            let position = if r.shape == Shape::Bridge {
                r.center + Vec3::new(0.0, 0.0, r.half[1] * 0.62)
            } else if kind == SpawnKind::Chest && (r.is_grand_treasury() || r.role == Role::Secret)
            {
                grand_chest_position(r)
            } else if kind == SpawnKind::Chest {
                chest_alcove(map, r).unwrap_or(r.center + Vec3::new(r.half[0] * 0.38, 0.0, 0.0))
            } else if kind == SpawnKind::Rest {
                shrine_font(map, r).unwrap_or(r.center + Vec3::new(r.half[0] * 0.38, 0.0, 0.0))
            } else {
                r.center + Vec3::new(r.half[0] * 0.38, 0.0, 0.0)
            };
            out.push(Spawn {
                id: out.len(),
                room: r.id,
                kind,
                definition: def.id.clone(),
                position,
                health: 0.0,
                damage: 0.0,
                speed: 0.0,
                gold: def.gold_min + loot.index((def.gold_max - def.gold_min + 1) as usize) as u32,
                budget,
            });
        }
        if r.role == Role::Encounter {
            // Place flankers first so ordinary enemies preserve clear lanes
            // around their bays instead of randomly occupying them.
            if let Some(def) = catalog
                .enemies
                .iter()
                .find(|enemy| enemy.id == "bone_scout")
            {
                for side in [-1.0, 1.0] {
                    let Some(position) = flank_position(map, r, side, &out) else {
                        continue;
                    };
                    out.push(Spawn {
                        id: out.len(),
                        room: r.id,
                        kind: SpawnKind::Enemy,
                        definition: def.id.clone(),
                        position,
                        health: def.health * (0.82 + budget * 0.025),
                        damage: def.damage * 0.78,
                        speed: def.speed,
                        gold: 6 + loot.index(8) as u32,
                        budget,
                    });
                }
            }
            if miniboss_encounter == Some(r.id) {
                if let Some(def) = catalog.enemies.iter().find(|enemy| enemy.id == "sentinel") {
                    if let Some(position) = miniboss_position(map, r, &out) {
                        out.push(Spawn {
                            id: out.len(),
                            room: r.id,
                            kind: SpawnKind::Miniboss,
                            definition: def.id.clone(),
                            position,
                            health: def.health * 2.1 * (0.8 + budget * 0.055),
                            damage: def.damage * 1.35,
                            speed: def.speed * 1.08,
                            gold: 18 + loot.index(12) as u32,
                            budget,
                        });
                    }
                }
            }
        }
        if r.role.combat() {
            let special = match r.role {
                Role::Boss => SpawnKind::Boss,
                Role::Miniboss => SpawnKind::Miniboss,
                Role::Elite => SpawnKind::Elite,
                _ => SpawnKind::Enemy,
            };
            // Reserve new specialist silhouettes for optional side rooms so
            // the authored difficulty curve on the critical spine stays fair.
            let pool = biome
                .enemy_pool
                .iter()
                .filter_map(|id| catalog.enemies.iter().find(|e| &e.id == id))
                .filter(|enemy| !r.critical || !enemy.optional_only)
                .collect::<Vec<_>>();
            let minimum = pool.iter().map(|e| e.cost).fold(f32::INFINITY, f32::min);
            let mut remaining = budget.max(minimum);
            // The main route keeps its tested four-enemy pacing; side rooms
            // reward exploration with denser encounters, scaled by danger.
            let count = if special != SpawnKind::Enemy {
                1
            } else if r.critical {
                4
            } else {
                4 + usize::from(budget >= 3.0)
            };
            for i in 0..count {
                let affordable = pool
                    .iter()
                    .copied()
                    .filter(|e| special != SpawnKind::Enemy || e.cost <= remaining + 0.001)
                    .collect::<Vec<_>>();
                if affordable.is_empty() {
                    break;
                }
                let def = affordable[rng.index(affordable.len())];
                remaining -= def.cost;
                let mut candidates = Vec::new();
                for _ in 0..24 {
                    let p = r.center
                        + Vec3::new(
                            rng.range(-0.58, 0.58) * r.half[0],
                            0.0,
                            rng.range(-0.58, 0.58) * r.half[1],
                        );
                    if !r.contains(p.x, p.z, 0.8)
                        || r.floor_y(p.x, p.z) < r.center.y - 0.1
                        || map.capsule_floor(p, 0.6).is_none()
                    {
                        continue;
                    }
                    let door_distance = r
                        .sockets
                        .iter()
                        .map(|s| horizontal(s.position - p).length())
                        .fold(f32::INFINITY, f32::min);
                    let separation = out
                        .iter()
                        .filter(|s| s.room == r.id)
                        .map(|s| horizontal(s.position - p).length())
                        .fold(8.0, f32::min);
                    if door_distance < 3.0 || separation < 1.5 {
                        continue;
                    }
                    // Prefer space around the actor, off the entry sightline;
                    // a boss holds the centre, regulars flank the approach.
                    let off_axis = (p.x - r.center.x).abs().min((p.z - r.center.z).abs());
                    let score = door_distance
                        + separation * 0.4
                        + if special == SpawnKind::Boss {
                            -horizontal(p - r.center).length()
                        } else {
                            off_axis
                        };
                    candidates.push((score, p));
                }
                candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
                let Some((_, p)) = candidates.first() else {
                    continue;
                };
                let multiplier = match special {
                    SpawnKind::Boss => 4.2,
                    SpawnKind::Miniboss => 2.5,
                    SpawnKind::Elite => 1.6,
                    _ => 1.0,
                };
                out.push(Spawn {
                    id: out.len(),
                    room: r.id,
                    kind: special,
                    definition: def.id.clone(),
                    position: *p,
                    health: def.health * multiplier * (0.8 + budget * 0.08),
                    damage: def.damage * multiplier.sqrt(),
                    speed: def.speed,
                    gold: 5 + loot.index(10) as u32,
                    budget,
                });
                let _ = i;
            }
        }
        if r.role == Role::Elite {
            // A visible, avoidable floor hazard; never on a socket or route axis.
            let p = r.center + Vec3::new(r.half[0] * 0.40, 0.0, -r.half[1] * 0.40);
            if r.floor_y(p.x, p.z) == r.center.y {
                out.push(Spawn {
                    id: out.len(),
                    room: r.id,
                    kind: SpawnKind::Trap,
                    definition: "ember_vent".into(),
                    position: p,
                    health: 0.0,
                    damage: 9.0,
                    speed: 0.0,
                    gold: 0,
                    budget,
                });
            }
        }
    }
    out
}

/// The grand treasury's (and the arcane hall's) relic chest stands near the
/// middle of the hall, a little past the centre from the entry.
fn grand_chest_position(room: &Room) -> Vec3 {
    let Some(socket) = room.sockets.first() else {
        return room.center;
    };
    let inward = [-socket.outward.x, -socket.outward.z];
    let depth = inward[0].abs() * room.half[0] + inward[1].abs() * room.half[1];
    let back = (depth * 0.2).min(1.6);
    Vec3::new(
        room.center.x + inward[0] * back,
        room.center.y,
        room.center.z + inward[1] * back,
    )
}

/// Treasure and secret chests sit at the far end of the chamber, facing the
/// door the player arrives through, so the room reads as a processional
/// approach. Falls back to the legacy spot when no back position is clear of
/// every doorway and cover pillar.
fn chest_alcove(map: &Blueprint, room: &Room) -> Option<Vec3> {
    let socket = room.sockets.first()?;
    let inward = -horizontal(socket.outward).normalize_or_zero();
    if inward.length_squared() < 0.5 {
        return None;
    }
    let right = Vec3::new(inward.z, 0.0, -inward.x);
    let depth = inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1];
    let width = right.x.abs() * room.half[0] + right.z.abs() * room.half[1];
    [0.0, 0.4, -0.4, 0.2, -0.2]
        .into_iter()
        .flat_map(|lateral| {
            [2.1, 2.6, 3.2]
                .map(|back| room.center + inward * (depth - back) + right * (width * lateral))
        })
        .find(|p| {
            room.contains(p.x, p.z, 1.3)
                && room
                    .sockets
                    .iter()
                    .all(|s| horizontal(s.position - *p).length() > s.width * 0.5 + 2.2)
                && map
                    .obstacles
                    .iter()
                    .filter(|o| o.room == room.id)
                    .all(|o| horizontal(o.center - *p).length() > 1.6)
                && map.capsule_floor(*p, 0.32).is_some()
        })
}

/// Doorless straight walls of a room, the one facing the main entrance
/// first: (wall midpoint at floor level, inward direction, half length).
pub fn doorless_walls(room: &Room) -> Vec<(Vec3, Vec3, f32)> {
    let Some(entry) = room.sockets.first() else {
        return vec![];
    };
    let o = horizontal(entry.outward).normalize_or_zero();
    if o.length_squared() < 0.5 {
        return vec![];
    }
    let left = Vec3::new(o.z, 0.0, -o.x);
    [-o, left, -left, o]
        .into_iter()
        .filter(|dir| {
            !room
                .sockets
                .iter()
                .any(|s| horizontal(s.outward).normalize_or_zero().dot(*dir) > 0.9)
        })
        .map(|dir| {
            let reach = dir.x.abs() * room.half[0] + dir.z.abs() * room.half[1];
            let along = dir.z.abs() * room.half[0] + dir.x.abs() * room.half[1];
            (room.center + dir * reach, -dir, along * 0.88)
        })
        .collect()
}

/// Distance of a sanctuary's rest font from its shrine wall.
pub const SHRINE_FONT_DEPTH: f32 = 2.6;

fn font_clear(map: &Blueprint, room: &Room, p: Vec3) -> bool {
    room.contains(p.x, p.z, 1.3)
        && room
            .sockets
            .iter()
            .all(|s| horizontal(s.position - p).length() > s.width * 0.5 + 2.2)
        && map
            .obstacles
            .iter()
            .filter(|o| o.room == room.id)
            .all(|o| horizontal(o.center - p).length() > 1.6)
        && map.capsule_floor(p, 0.32).is_some()
}

/// The wall that holds a sanctuary's shrine: the first doorless wall whose
/// rest font (in front of the dais) keeps every door and pillar clear.
pub fn shrine_wall(map: &Blueprint, room: &Room) -> Option<(Vec3, Vec3, f32)> {
    doorless_walls(room)
        .into_iter()
        .find(|(wall, inward, _)| font_clear(map, room, *wall + *inward * SHRINE_FONT_DEPTH))
}

fn shrine_font(map: &Blueprint, room: &Room) -> Option<Vec3> {
    if let Some((wall, inward, _)) = shrine_wall(map, room) {
        return Some(wall + inward * SHRINE_FONT_DEPTH);
    }
    // Crossroads: stand the font on a diagonal, clear of every door-to-centre
    // walk, between the centre and a cover pillar.
    [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)]
        .into_iter()
        .flat_map(|(sx, sz)| {
            [0.32f32, 0.38, 0.26]
                .map(|f| room.center + Vec3::new(sx * room.half[0] * f, 0.0, sz * room.half[1] * f))
        })
        .find(|p| {
            room.sockets.iter().all(|s| {
                let d = horizontal(s.position - *p).length();
                let axis = horizontal(room.center - s.position).normalize_or_zero();
                let off_axis = horizontal(*p - s.position);
                let lateral = (off_axis - axis * off_axis.dot(axis)).length();
                d > s.width * 0.5 + 1.5 && lateral > 1.2
            }) && map
                .obstacles
                .iter()
                .filter(|o| o.room == room.id)
                .all(|o| horizontal(o.center - *p).length() > 1.4)
                && map.capsule_floor(*p, 0.32).is_some()
        })
}

fn entry_basis(map: &Blueprint, room: &Room) -> (Vec3, Vec3, f32, f32) {
    let inward = room
        .sockets
        .iter()
        .filter_map(|socket| {
            let connection = map.connections.get(socket.connection)?;
            if !connection.critical {
                return None;
            }
            let other = if connection.a == room.id {
                connection.b
            } else {
                connection.a
            };
            (map.rooms.get(other)?.progress < room.progress)
                .then_some(-horizontal(socket.outward).normalize_or_zero())
        })
        .find(|direction| direction.length_squared() > 0.001)
        .unwrap_or(Vec3::Z);
    let right = Vec3::new(inward.z, 0.0, -inward.x);
    let depth = (inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1]).max(1.0);
    let width = (right.x.abs() * room.half[0] + right.z.abs() * room.half[1]).max(1.0);
    (inward, right, depth, width)
}

fn flank_position(map: &Blueprint, room: &Room, side: f32, spawns: &[Spawn]) -> Option<Vec3> {
    let (inward, right, depth, width) = entry_basis(map, room);
    let lateral = (width * 0.52).min(width - 1.35).max(1.25);
    let along = (depth * 0.13).clamp(-0.6, 1.25);
    for factor in [1.0, 0.88, 0.76, 0.64] {
        let mut position = room.center + inward * along + right * (side * lateral * factor);
        if !room.contains(position.x, position.z, 0.42)
            || room
                .sockets
                .iter()
                .any(|socket| horizontal(socket.position - position).length() < 2.15)
            || spawns.iter().any(|spawn| {
                spawn.room == room.id && horizontal(spawn.position - position).length() < 1.65
            })
        {
            continue;
        }
        position.y = map.capsule_floor(position, 0.36)?;
        return Some(position);
    }
    None
}

fn miniboss_position(map: &Blueprint, room: &Room, spawns: &[Spawn]) -> Option<Vec3> {
    let (inward, right, depth, _) = entry_basis(map, room);
    let mut candidates = vec![
        room.center + inward * (depth * 0.24).min(2.0),
        room.center + inward * (depth * 0.12).min(1.0) - right * 1.6,
        room.center + inward * (depth * 0.12).min(1.0) + right * 1.6,
        room.center - inward * 1.0,
    ];
    // The first four authored anchors can be blocked by sockets or flankers.
    // Scan a deterministic room-local lattice before giving up on the mini.
    for x in [-0.55, -0.32, -0.10, 0.10, 0.32, 0.55] {
        for z in [-0.55, -0.32, -0.10, 0.10, 0.32, 0.55] {
            candidates.push(room.center + Vec3::new(room.half[0] * x, 0.0, room.half[1] * z));
        }
    }
    let mut valid = Vec::new();
    for mut position in candidates {
        if !room.contains(position.x, position.z, 0.68)
            || room
                .sockets
                .iter()
                .any(|socket| horizontal(socket.position - position).length() < 2.15)
        {
            continue;
        }
        let Some(floor) = map.capsule_floor(position, 0.48) else {
            continue;
        };
        position.y = floor;
        let nearest = spawns
            .iter()
            .filter(|spawn| spawn.room == room.id)
            .map(|spawn| horizontal(spawn.position - position).length())
            .fold(8.0, f32::min);
        let score = nearest * 2.0 - horizontal(position - room.center).length() * 0.12
            + horizontal(position - room.center - inward * 1.5)
                .normalize_or_zero()
                .dot(inward)
                * 0.08;
        valid.push((score, position));
    }
    valid
        .into_iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, position)| position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specialist_skeletons_only_appear_off_the_critical_spine() {
        let catalog = Catalog::bundled();
        let mut saw_specialist = false;
        for seed in 0..96 {
            let map = crate::generate(seed, &Config::default(), &catalog, &Difficulty::default())
                .unwrap();
            for spawn in &map.spawns {
                let room = &map.rooms[spawn.room];
                let optional = catalog
                    .enemies
                    .iter()
                    .find(|enemy| enemy.id == spawn.definition)
                    .is_some_and(|enemy| enemy.optional_only);
                if optional {
                    let encounter_crossbow =
                        spawn.definition == "bone_scout" && room.role == Role::Encounter;
                    assert!(
                        !room.critical || encounter_crossbow,
                        "optional enemy {} spawned on critical room for seed {seed}",
                        spawn.definition
                    );
                    saw_specialist = true;
                }
            }
        }
        assert!(
            saw_specialist,
            "stress seeds produced no optional specialist encounter"
        );
    }

    #[test]
    fn rest_fonts_stand_before_a_doorless_shrine_wall() {
        let catalog = Catalog::bundled();
        let (mut rests, mut shrines) = (0, 0);
        for seed in 0..96 {
            let map = crate::generate(seed, &Config::default(), &catalog, &Difficulty::default())
                .unwrap();
            for spawn in map.spawns.iter().filter(|s| s.kind == SpawnKind::Rest) {
                let room = &map.rooms[spawn.room];
                rests += 1;
                if shrine_wall(&map, room).is_none() {
                    // Only crossroads (a door on every wall) lack a shrine.
                    assert!(
                        doorless_walls(room).is_empty(),
                        "seed {seed} room {}: shrine wall rejected",
                        room.id
                    );
                    continue;
                }
                if let Some((wall, inward, _)) = shrine_wall(&map, room) {
                    let depth = horizontal(spawn.position - wall).dot(inward);
                    if (depth - 2.6).abs() < 0.01 {
                        shrines += 1;
                        assert!(room
                            .sockets
                            .iter()
                            .all(|s| horizontal(s.outward).normalize_or_zero().dot(-inward) < 0.9));
                    }
                }
            }
        }
        assert!(rests > 90);
        assert!(
            shrines * 10 >= rests * 8,
            "only {shrines}/{rests} rest fonts face a shrine"
        );
    }

    #[test]
    fn treasure_chests_wait_at_the_far_end_of_their_chamber() {
        let catalog = Catalog::bundled();
        let (mut chests, mut alcoves) = (0, 0);
        for seed in 0..96 {
            let map = crate::generate(seed, &Config::default(), &catalog, &Difficulty::default())
                .unwrap();
            for spawn in map.spawns.iter().filter(|s| s.kind == SpawnKind::Chest) {
                let room = &map.rooms[spawn.room];
                // The grand treasury and the arcane hall show their relic
                // chest in the middle of the hall instead.
                if room.shape == Shape::Bridge
                    || room.is_grand_treasury()
                    || room.role == Role::Secret
                {
                    continue;
                }
                chests += 1;
                let entry = room.sockets[0].position;
                for socket in &room.sockets {
                    assert!(
                        horizontal(socket.position - spawn.position).length() >= 1.5,
                        "seed {seed}: chest crowds a doorway in room {}",
                        room.id
                    );
                }
                if horizontal(spawn.position - entry).length()
                    > room.half[0].min(room.half[1]) * 1.2
                {
                    alcoves += 1;
                }
            }
        }
        assert!(chests > 30);
        assert!(
            alcoves * 10 >= chests * 9,
            "only {alcoves}/{chests} chests sit at the far end"
        );
    }

    #[test]
    fn stalker_rooms_get_flank_archers_and_one_midrun_miniboss() {
        let catalog = Catalog::bundled();
        for seed in 0..64 {
            let map = crate::generate(seed, &Config::default(), &catalog, &Difficulty::default())
                .unwrap();
            let encounters = map
                .rooms
                .iter()
                .filter(|room| room.role == Role::Encounter)
                .collect::<Vec<_>>();
            let minibosses = map
                .spawns
                .iter()
                .filter(|spawn| spawn.kind == SpawnKind::Miniboss)
                .collect::<Vec<_>>();
            assert_eq!(
                minibosses.len(),
                2,
                "seed {seed}: expected required boss beat plus one ambush mini"
            );
            let ambush_room = minibosses
                .iter()
                .map(|spawn| spawn.room)
                .find(|room| map.rooms[*room].role == Role::Encounter)
                .expect("an encounter room owns the ambush miniboss");
            let ranged = map
                .spawns
                .iter()
                .filter(|spawn| spawn.room == ambush_room && spawn.definition == "bone_scout")
                .count();
            assert_eq!(
                ranged, 2,
                "seed {seed}: miniboss encounter should have two flanking scouts"
            );
            for room in encounters {
                assert!(
                    map.spawns
                        .iter()
                        .filter(|spawn| spawn.room == room.id && spawn.definition == "bone_scout")
                        .count()
                        >= 2,
                    "seed {seed}: encounter room {} lacks its crossbow ambush",
                    room.id
                );
            }
        }
    }
}
