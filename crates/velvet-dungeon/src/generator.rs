use super::*;
use std::{collections::BTreeSet, time::Instant};

#[derive(Clone)]
struct Node {
    role: Role,
    critical: bool,
    progress: f32,
    grid: [i32; 3],
}
#[derive(Clone)]
struct Edge {
    a: usize,
    b: usize,
    critical: bool,
    shortcut: bool,
}
const DIRECTIONS: [[i32; 2]; 4] = [[1, 0], [0, 1], [-1, 0], [0, -1]];

/// Constructive constraints avoid global retry lotteries. A rejected optional
/// branch/loop is repaired by another socket; the valid critical spine survives.
pub fn generate(
    seed: u64,
    config: &Config,
    catalog: &Catalog,
    difficulty: &gameplay::Difficulty,
) -> Result<Blueprint, String> {
    let started = Instant::now();
    config.validate()?;
    catalog.validate()?;
    difficulty.validate()?;
    let mut rng = rng::Rng::stream(seed, "layout_v2", 0);
    let n = config.critical_path_length;
    // Semantic intent is fixed before embedding; long runs insert exploration
    // between story beats, never silently remove a required objective.
    let mut roles = vec![Role::Normal; n];
    for (i, role) in roles.iter_mut().enumerate().take(n - 4).skip(4) {
        *role = if i % 12 == 0 {
            Role::Rest
        } else if i % 7 == 0 {
            Role::Event
        } else if i % 4 == 0 {
            Role::Encounter
        } else {
            Role::Normal
        };
    }
    let elite = if n / 2 + 1 >= n - 4 {
        n / 2 - 1
    } else {
        n / 2 + 1
    };
    for (i, role) in [
        (0, Role::Entrance),
        (1, Role::Normal),
        (2, Role::Encounter),
        (3, Role::Puzzle),
        (n / 2, Role::Event),
        (elite, Role::Elite),
        (n - 4, Role::Rest),
        (n - 3, Role::Miniboss),
        (n - 2, Role::Boss),
        (n - 1, Role::Exit),
    ] {
        roles[i] = role;
    }
    let mut nodes = vec![Node {
        role: roles[0],
        critical: true,
        progress: 0.0,
        grid: [0, 0, 0],
    }];
    let mut occupied = BTreeSet::from([(0, 0)]);
    let mut attempts = 0;
    let mut rejected = 0;
    let mut repairs = 0;
    // A bounded backtracking self-avoiding embedding. This is an orthogonal
    // socket graph, not a voxel dungeon. Modules retain independent silhouettes.
    fn embed(
        path: &mut Vec<[i32; 2]>,
        occupied: &mut BTreeSet<(i32, i32)>,
        n: usize,
        rng: &mut rng::Rng,
        attempts: &mut usize,
        rejected: &mut usize,
    ) -> bool {
        if path.len() == n {
            return true;
        }
        if *attempts > 25000 {
            return false;
        }
        let p = *path.last().unwrap();
        let mut directions = DIRECTIONS;
        rng.shuffle(&mut directions);
        for d in directions {
            *attempts += 1;
            let next = [p[0] + d[0], p[1] + d[1]];
            if occupied.contains(&(next[0], next[1])) {
                *rejected += 1;
                continue;
            }
            // Avoid long featureless straights, but never make progress invalid.
            if path.len() > 3
                && rng.chance(0.88)
                && (1..=3).all(|i| {
                    path[path.len() - i]
                        == [p[0] - d[0] * (i as i32 - 1), p[1] - d[1] * (i as i32 - 1)]
                })
            {
                *rejected += 1;
                continue;
            }
            path.push(next);
            occupied.insert((next[0], next[1]));
            if embed(path, occupied, n, rng, attempts, rejected) {
                return true;
            }
            occupied.remove(&(next[0], next[1]));
            path.pop();
        }
        false
    }
    let mut spine = vec![[0, 0]];
    if !embed(
        &mut spine,
        &mut occupied,
        n,
        &mut rng,
        &mut attempts,
        &mut rejected,
    ) {
        return Err(format!(
            "seed {seed}: critical embedding exhausted its budget"
        ));
    }
    let mut level = 0;
    for i in 1..n {
        if rng.chance(config.verticality) && roles[i] != Role::Exit {
            level = (level + if rng.chance(0.55) { 1 } else { -1 }).clamp(-2, 3);
        }
        nodes.push(Node {
            role: roles[i],
            critical: true,
            progress: i as f32 / (n - 1) as f32,
            grid: [spine[i][0], level, spine[i][1]],
        });
    }
    let mut edges = (1..n)
        .map(|i| Edge {
            a: i - 1,
            b: i,
            critical: true,
            shortcut: false,
        })
        .collect::<Vec<_>>();
    // Reserve a climbing side expedition returning above its starting chamber.
    // Four connectors, three height changes: actual stacked rooms with 9 m gap.
    if config.verticality > 0.35 {
        let mut anchors = (1..n - 3).collect::<Vec<_>>();
        rng.shuffle(&mut anchors);
        'stack: for a in anchors {
            let g = nodes[a].grid;
            for k in 0..4 {
                let d = DIRECTIONS[k];
                let e = DIRECTIONS[(k + 1) % 4];
                let cells = [
                    [g[0] + d[0], g[2] + d[1]],
                    [g[0] + d[0] + e[0], g[2] + d[1] + e[1]],
                    [g[0] + e[0], g[2] + e[1]],
                ];
                attempts += 1;
                if cells.iter().any(|p| occupied.contains(&(p[0], p[1]))) {
                    rejected += 1;
                    continue;
                }
                let mut prev = a;
                for (j, p) in cells.into_iter().chain([[g[0], g[2]]]).enumerate() {
                    let id = nodes.len();
                    let role = [Role::Normal, Role::Encounter, Role::Event, Role::Treasure][j];
                    nodes.push(Node {
                        role,
                        critical: false,
                        progress: nodes[a].progress,
                        grid: [p[0], g[1] + (j + 1).min(3) as i32, p[1]],
                    });
                    occupied.insert((p[0], p[1]));
                    edges.push(Edge {
                        a: prev,
                        b: id,
                        critical: false,
                        shortcut: false,
                    });
                    prev = id;
                }
                break 'stack;
            }
        }
    }
    let optional =
        (n as f32 * config.branch_factor * (0.5 + config.room_density * 0.5)).round() as usize;
    for branch in 0..optional {
        let mut parents = (1..nodes.len())
            .filter(|i| !matches!(nodes[*i].role, Role::Boss | Role::Exit | Role::Secret))
            .collect::<Vec<_>>();
        rng.shuffle(&mut parents);
        let mut placed = false;
        for a in parents {
            let g = nodes[a].grid;
            let mut dirs = DIRECTIONS;
            rng.shuffle(&mut dirs);
            for d in dirs {
                attempts += 1;
                let p = [g[0] + d[0], g[2] + d[1]];
                if occupied.contains(&(p[0], p[1])) {
                    rejected += 1;
                    continue;
                }
                let role = if (branch == 0 && config.secret_density > 0.0)
                    || rng.chance(config.secret_density)
                {
                    Role::Secret
                } else if branch == 1 || rng.chance(config.treasure_density) {
                    Role::Treasure
                } else if rng.chance(config.combat_density) {
                    Role::Encounter
                } else {
                    Role::Normal
                };
                let id = nodes.len();
                let h = g[1]
                    + if rng.chance(config.verticality * 0.5) {
                        1
                    } else {
                        0
                    };
                nodes.push(Node {
                    role,
                    critical: false,
                    progress: nodes[a].progress,
                    grid: [p[0], h, p[1]],
                });
                edges.push(Edge {
                    a,
                    b: id,
                    critical: false,
                    shortcut: false,
                });
                occupied.insert((p[0], p[1]));
                placed = true;
                break;
            }
            if placed {
                break;
            }
            repairs += 1;
        }
    }
    let mut loop_candidates = Vec::new();
    for a in 1..nodes.len() {
        for b in a + 1..nodes.len() {
            let ga = nodes[a].grid;
            let gb = nodes[b].grid;
            if (ga[0] - gb[0]).abs() + (ga[2] - gb[2]).abs() == 1
                && (ga[1] - gb[1]).abs() <= 1
                && ![nodes[a].role, nodes[b].role]
                    .iter()
                    .any(|r| matches!(r, Role::Boss | Role::Exit | Role::Secret))
                && !edges
                    .iter()
                    .any(|e| (e.a == a && e.b == b) || (e.a == b && e.b == a))
            {
                // Do not occupy an existing socket at a different elevation.
                if !edges.iter().any(|e| {
                    let (p, q) = (nodes[e.a].grid, nodes[e.b].grid);
                    ((p[0], p[2]) == (ga[0], ga[2]) && (q[0], q[2]) == (gb[0], gb[2]))
                        || ((q[0], q[2]) == (ga[0], ga[2]) && (p[0], p[2]) == (gb[0], gb[2]))
                }) {
                    loop_candidates.push((a, b));
                }
            }
        }
    }
    rng.shuffle(&mut loop_candidates);
    for (i, (a, b)) in loop_candidates.into_iter().enumerate() {
        if config.loop_probability > 0.0 && (i == 0 || rng.chance(config.loop_probability)) {
            edges.push(Edge {
                a,
                b,
                critical: false,
                shortcut: true,
            });
        }
    }
    // If the embedding offers no adjacent loop, attach a square detour to an
    // existing edge. Only empty lattice cells may be occupied (local repair).
    if config.loop_probability > 0.0 && !edges.iter().any(|e| e.shortcut) {
        'repair: for i in 1..n - 4 {
            let ga = nodes[i].grid;
            let gb = nodes[i + 1].grid;
            let d = [gb[0] - ga[0], gb[2] - ga[2]];
            for sign in [-1, 1] {
                let offset = [-d[1] * sign, d[0] * sign];
                let pa = [ga[0] + offset[0], ga[2] + offset[1]];
                let pb = [gb[0] + offset[0], gb[2] + offset[1]];
                attempts += 1;
                if occupied.contains(&(pa[0], pa[1])) || occupied.contains(&(pb[0], pb[1])) {
                    rejected += 1;
                    continue;
                }
                let a = nodes.len();
                let b = a + 1;
                nodes.push(Node {
                    role: Role::Treasure,
                    critical: false,
                    progress: nodes[i].progress,
                    grid: [pa[0], ga[1], pa[1]],
                });
                nodes.push(Node {
                    role: Role::Encounter,
                    critical: false,
                    progress: nodes[i + 1].progress,
                    grid: [pb[0], gb[1], pb[1]],
                });
                edges.extend([
                    Edge {
                        a: i,
                        b: a,
                        critical: false,
                        shortcut: false,
                    },
                    Edge {
                        a,
                        b,
                        critical: false,
                        shortcut: false,
                    },
                    Edge {
                        a: b,
                        b: i + 1,
                        critical: false,
                        shortcut: true,
                    },
                ]);
                repairs += 1;
                break 'repair;
            }
        }
    }
    let spacing = config.spacing();
    let count_biomes = config.biome_complexity.min(catalog.biomes.len());
    let mut rooms = Vec::new();
    for (id, node) in nodes.iter().enumerate() {
        let mut local = rng::Rng::stream(seed, "room_geometry_v2", id as u64);
        let total: usize = catalog.templates.iter().map(|t| t.weight as usize).sum();
        let mut pick = local.index(total);
        let mut t = &catalog.templates[0];
        for candidate in &catalog.templates {
            if pick < (candidate.weight as usize) {
                t = candidate;
                break;
            }
            pick = pick.saturating_sub(candidate.weight as usize);
        }
        if matches!(node.role, Role::Boss | Role::Miniboss) {
            t = catalog
                .templates
                .iter()
                .find(|t| t.shape == Shape::Arena)
                .unwrap_or(t);
        }
        if matches!(node.role, Role::Entrance | Role::Puzzle | Role::Rest) {
            t = catalog
                .templates
                .iter()
                .find(|t| t.shape == Shape::Vault)
                .unwrap_or(t);
        }
        let stacked_above = nodes
            .iter()
            .take(id)
            .any(|other| other.grid[0] == node.grid[0] && other.grid[2] == node.grid[2]);
        if stacked_above {
            t = catalog
                .templates
                .iter()
                .find(|t| t.shape == Shape::Vault)
                .unwrap_or(t);
        }
        let shape = if node.role == Role::Event && config.verticality > 0.1 {
            Shape::Bridge
        } else if stacked_above && t.shape == Shape::Bridge {
            Shape::Cavern
        } else {
            t.shape
        };
        let mut half = [
            local.range(t.half_width[0], t.half_width[1]) * config.room_scale,
            local.range(t.half_depth[0], t.half_depth[1]) * config.room_scale,
        ];
        if node.role == Role::Boss {
            half[0] = half[0].max(5.8);
            half[1] = half[1].max(5.8);
        }
        let center = Vec3::new(
            node.grid[0] as f32 * spacing,
            node.grid[1] as f32 * 3.0,
            node.grid[2] as f32 * spacing,
        );
        let cut = if shape == Shape::Vault {
            [0.10; 4]
        } else {
            [
                local.range(0.18, 0.42),
                local.range(0.18, 0.42),
                local.range(0.18, 0.42),
                local.range(0.18, 0.42),
            ]
        };
        let (hx, hz) = (half[0], half[1]);
        // Chamfers are computed in double precision and rounded once, as the
        // web generator does (`F(hx * (1 - cut))`).
        let k = |h: f32, c: f32| (h as f64 * (1.0 - c as f64)) as f32;
        let outline = vec![
            [-k(hx, cut[0]), -hz],
            [k(hx, cut[1]), -hz],
            [hx, -k(hz, cut[1])],
            [hx, k(hz, cut[2])],
            [k(hx, cut[2]), hz],
            [-k(hx, cut[3]), hz],
            [-hx, k(hz, cut[3])],
            [-hx, -k(hz, cut[0])],
        ];
        rooms.push(Room {
            id,
            role: node.role,
            critical: node.critical,
            progress: node.progress,
            region: (node.progress * 3.0).floor() as usize,
            grid: node.grid,
            center,
            half,
            height: t.height,
            shape,
            template: t.id.clone(),
            biome: ((node.progress * count_biomes as f32) as usize).min(count_biomes - 1),
            outline,
            sockets: vec![],
            landmark_seed: rng::Rng::stream(seed, "landmark_v2", id as u64).next_u64(),
            treasury: None,
        });
    }
    // One grand treasury per expedition: a wider, taller formal vault with an
    // open floor; every other treasure room stays a small loot hall.
    let formal = |hx: f32, hz: f32| {
        let k = |h: f32| (h as f64 * (1.0 - 0.1f64)) as f32;
        vec![
            [-k(hx), -hz],
            [k(hx), -hz],
            [hx, -k(hz)],
            [hx, k(hz)],
            [k(hx), hz],
            [-k(hx), hz],
            [-hx, k(hz)],
            [-hx, -k(hz)],
        ]
    };
    let treasures: Vec<usize> = rooms
        .iter()
        .filter(|r| r.role == Role::Treasure)
        .map(|r| r.id)
        .collect();
    if !treasures.is_empty() {
        let grand =
            treasures[rng::Rng::stream(seed, "treasury_grand_v1", 0).index(treasures.len())];
        for &id in &treasures {
            rooms[id].treasury = Some(if id == grand {
                Treasury::Grand
            } else {
                Treasury::Small
            });
        }
        let room = &mut rooms[grand];
        let (hx, hz) = (room.half[0].max(7.4), room.half[1].max(7.4));
        room.half = [hx, hz];
        room.height = 6.6;
        room.shape = Shape::Vault;
        room.outline = formal(hx, hz);
    }
    // The hidden chamber is an arcane library hall: a formal vault with an
    // open floor, tall enough for its chandelier and the shaft of light.
    for room in rooms.iter_mut().filter(|r| r.role == Role::Secret) {
        room.shape = Shape::Vault;
        room.height = room.height.max(5.6);
        room.outline = formal(room.half[0], room.half[1]);
    }
    let mut connections = Vec::new();
    for e in edges {
        let a = &rooms[e.a];
        let b = &rooms[e.b];
        let d = horizontal(b.center - a.center).normalize_or_zero();
        let start = a.center + Vec3::new(d.x * a.half[0], 0.0, d.z * a.half[1]);
        let end = b.center - Vec3::new(d.x * b.half[0], 0.0, d.z * b.half[1]);
        let kind = if b.role == Role::Secret {
            ConnectionKind::Secret
        } else if (start.y - end.y).abs() > 0.1 {
            if rng.chance(0.48) {
                ConnectionKind::Stairs
            } else {
                ConnectionKind::Ramp
            }
        } else if e.shortcut {
            ConnectionKind::Shortcut
        } else if rng.chance(0.18) {
            ConnectionKind::Bridge
        } else {
            ConnectionKind::Tunnel
        };
        let id = connections.len();
        let width = if e.critical { 3.4 } else { 2.8 };
        let height = 3.4;
        connections.push(Connection {
            id,
            a: e.a,
            b: e.b,
            start,
            end,
            width,
            height,
            kind,
            critical: e.critical,
        });
        for (r, p, dir) in [(e.a, start, d), (e.b, end, -d)] {
            rooms[r].sockets.push(Socket {
                connection: id,
                position: p,
                outward: dir,
                width,
                height,
                kind: "passage".into(),
                compatible: vec!["passage".into()],
                clearance: 1.2,
            });
        }
    }
    let stacked = rooms
        .iter()
        .enumerate()
        .map(|(i, a)| {
            rooms
                .iter()
                .skip(i + 1)
                .filter(|b| a.grid[0] == b.grid[0] && a.grid[2] == b.grid[2])
                .count()
        })
        .sum();
    let volume = rooms
        .iter()
        .map(|r| r.half[0] * r.half[1] * 4.0 * r.height)
        .sum::<f32>()
        + connections
            .iter()
            .map(|c| c.length() * c.width * c.height)
            .sum::<f32>();
    let loops = connections.len() + 1 - rooms.len();
    let vertical = connections
        .iter()
        .filter(|c| (c.start.y - c.end.y).abs() > 0.1)
        .count();
    let stats = Stats {
        generation_ms: 0.0,
        rooms: rooms.len(),
        critical_length: n,
        branches: rooms.len() - n,
        loops,
        vertical_connections: vertical,
        stacked_rooms: stacked,
        volume,
        rejected_attempts: rejected,
        attempts,
        local_repairs: repairs,
        score: 0.0,
    };
    let rules_hash = rng::hash(&serde_json::to_vec(&(GENERATOR_VERSION, config, catalog)).unwrap());
    let mut obstacles = Vec::new();
    for r in &rooms {
        if matches!(r.shape, Shape::Vault | Shape::Arena)
            && !r.is_grand_treasury()
            && r.role != Role::Secret
        {
            for (x, z) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                obstacles.push(Obstacle {
                    room: r.id,
                    center: r.center + Vec3::new(x * r.half[0] * 0.68, 0.0, z * r.half[1] * 0.68),
                    half: [0.44, 0.44],
                    height: r.height,
                });
            }
        }
    }
    let mut blueprint = Blueprint {
        seed,
        version: GENERATOR_VERSION,
        rules_hash,
        config: config.clone(),
        rooms,
        connections,
        spawns: vec![],
        obstacles,
        critical_path: (0..n).collect(),
        entrance: 0,
        exit: n - 1,
        stats,
        floors: None,
    };
    blueprint.spawns = gameplay::place(&blueprint, catalog, difficulty);
    let issues = validation::validate(&blueprint);
    if !issues.is_empty() {
        return Err(format!("seed {seed}: {}", issues.join("; ")));
    }
    blueprint.stats.score = 60.0
        + loops.min(5) as f32 * 4.0
        + (vertical as f32 / n as f32) * 10.0
        + (stacked.min(1) as f32) * 10.0;
    blueprint.stats.generation_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(blueprint)
}
