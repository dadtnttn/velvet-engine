use super::*;
pub fn validate(map: &Blueprint) -> Vec<String> {
    let mut errors = Vec::new();
    let finite = |v: Vec3| v.to_array().iter().all(|x| x.is_finite());
    if map.rooms.is_empty() {
        return vec!["no rooms".into()];
    }
    if map.entrance >= map.rooms.len()
        || map.exit >= map.rooms.len()
        || map.rooms.iter().enumerate().any(|(i, r)| {
            i != r.id
                || !finite(r.center)
                || !r.height.is_finite()
                || r.outline.len() < 3
                || r.outline.iter().flatten().any(|x| !x.is_finite())
        })
        || map.connections.iter().enumerate().any(|(i, c)| {
            c.id != i
                || c.a >= map.rooms.len()
                || c.b >= map.rooms.len()
                || c.a == c.b
                || !finite(c.start)
                || !finite(c.end)
                || !c.width.is_finite()
                || c.width < 1.2
                || !c.height.is_finite()
        })
        || map
            .spawns
            .iter()
            .enumerate()
            .any(|(i, s)| s.id != i || s.room >= map.rooms.len() || !finite(s.position))
        || map.critical_path.iter().any(|id| *id >= map.rooms.len())
    {
        return vec!["invalid indices or non-finite geometry".into()];
    }
    for r in &map.rooms {
        if map.route(map.entrance, r.id).is_empty() {
            errors.push(format!("room {} isolated", r.id));
        }
        if map.capsule_floor(r.center, PLAYER_RADIUS).is_none() {
            errors.push(format!("room {} has no player clearance", r.id));
        }
        if r.height < PLAYER_HEIGHT + 0.3 {
            errors.push(format!("room {} ceiling too low", r.id));
        }
        if r.role == Role::Boss && r.half[0].min(r.half[1]) < 5.8 {
            errors.push(format!("boss arena {} too small", r.id));
        }
        let mut used = std::collections::BTreeSet::new();
        for s in &r.sockets {
            let side = (s.outward.x.round() as i32, s.outward.z.round() as i32);
            if !used.insert(side) {
                errors.push(format!("room {} overlapping sockets", r.id));
            }
            if s.connection >= map.connections.len()
                || s.width < PLAYER_RADIUS * 2.0 + 0.6
                || s.height < PLAYER_HEIGHT + 0.3
            {
                errors.push(format!("room {} invalid socket", r.id));
            }
            if let Some(c) = map.connections.get(s.connection) {
                let expected = if c.a == r.id {
                    c.start
                } else if c.b == r.id {
                    c.end
                } else {
                    errors.push(format!("room {} orphan socket", r.id));
                    continue;
                };
                if (expected - s.position).length() > 0.01
                    || s.width != c.width
                    || s.height != c.height
                    || !s.compatible.contains(&s.kind)
                    || s.clearance < PLAYER_RADIUS * 2.0
                {
                    errors.push(format!("room {} incompatible socket {}", r.id, c.id));
                }
            }
        }
    }
    // Broad-phase AABB followed by height-aware centreline sampling. Graph
    // adjacency alone cannot detect a corridor crossing a different layer.
    for (i, a) in map.connections.iter().enumerate() {
        for b in map.connections.iter().skip(i + 1) {
            let pad = (a.width + b.width) * 0.5;
            if a.start.x.min(a.end.x) > b.start.x.max(b.end.x) + pad
                || b.start.x.min(b.end.x) > a.start.x.max(a.end.x) + pad
                || a.start.z.min(a.end.z) > b.start.z.max(b.end.z) + pad
                || b.start.z.min(b.end.z) > a.start.z.max(a.end.z) + pad
            {
                continue;
            }
            let samples = (a.length() / 0.4).ceil() as usize;
            for j in 0..=samples {
                let t = j as f32 / samples.max(1) as f32;
                let p = a.start.lerp(a.end, t);
                let (u, d) = b.project(p.x, p.z);
                if (0.0..=1.0).contains(&u) && d < pad - 0.01 {
                    let ay = a.floor_y(t);
                    let by = b.floor_y(u);
                    if ay + a.height > by + 0.05 && by + b.height > ay + 0.05 {
                        errors.push(format!("corridor overlap {} / {}", a.id, b.id));
                        break;
                    }
                }
            }
        }
    }
    for (i, a) in map.rooms.iter().enumerate() {
        for b in map.rooms.iter().skip(i + 1) {
            if (a.center.x - b.center.x).abs() < a.half[0] + b.half[0] + 0.2
                && (a.center.z - b.center.z).abs() < a.half[1] + b.half[1] + 0.2
                && a.center.y + a.height + 1.0
                    > b.center.y - if b.shape == Shape::Bridge { 4.0 } else { 0.0 }
                && b.center.y + b.height + 1.0
                    > a.center.y - if a.shape == Shape::Bridge { 4.0 } else { 0.0 }
            {
                errors.push(format!("room overlap {} / {}", a.id, b.id));
            }
        }
    }
    for c in &map.connections {
        if c.length() < 2.0 || !c.length().is_finite() {
            errors.push(format!("connection {} invalid length", c.id));
            continue;
        }
        if (c.end.y - c.start.y).abs() / c.length() > 0.42 {
            errors.push(format!("connection {} excessive slope", c.id));
        }
        if map.rooms[c.a]
            .sockets
            .iter()
            .filter(|s| s.connection == c.id)
            .count()
            != 1
            || map.rooms[c.b]
                .sockets
                .iter()
                .filter(|s| s.connection == c.id)
                .count()
                != 1
        {
            errors.push(format!("connection {} missing socket", c.id));
        }
        let steps = (c.length() / 0.25).ceil() as usize;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let mut p = c.start.lerp(c.end, t);
            p.y = c.floor_y(t) + 0.08;
            if map.capsule_floor(p, PLAYER_RADIUS).is_none() {
                errors.push(format!("connection {} blocked at {:.2}", c.id, t));
                break;
            }
            for r in &map.rooms {
                if r.id != c.a
                    && r.id != c.b
                    && r.contains(p.x, p.z, -c.width * 0.5)
                    && p.y < r.center.y + r.height
                    && p.y + c.height > r.center.y
                {
                    errors.push(format!(
                        "connection {} intersects unrelated room {}",
                        c.id, r.id
                    ));
                    break;
                }
            }
        }
        // All routes from the room centre to its door are part of physical
        // navigation, including bridge decks. Check them separately from graph.
        for (id, p) in [(c.a, c.start), (c.b, c.end)] {
            for i in 0..=24 {
                let at = map.rooms[id].center.lerp(p, i as f32 / 24.0);
                if map
                    .capsule_floor(at, PLAYER_RADIUS)
                    .is_none_or(|y| (y - at.y).abs() > 0.35)
                {
                    errors.push(format!("room {} inaccessible door {}", id, c.id));
                    break;
                }
            }
        }
    }
    for s in &map.spawns {
        if map
            .capsule_floor(s.position, 0.32)
            .is_none_or(|y| (y - s.position.y).abs() > 0.35)
        {
            errors.push(format!("spawn {} inaccessible", s.id));
        }
        if map.rooms[s.room]
            .sockets
            .iter()
            .any(|door| horizontal(door.position - s.position).length() < 1.5)
        {
            errors.push(format!("spawn {} blocks door", s.id));
        }
    }
    if !map.rooms.iter().any(|r| r.role == Role::Boss)
        || map.route(map.entrance, map.exit).is_empty()
    {
        errors.push("missing reachable boss/exit".into());
    }
    if !map
        .spawns
        .iter()
        .any(|s| s.kind == gameplay::SpawnKind::Boss)
    {
        errors.push("boss arena has no encounter".into());
    }
    errors.sort();
    errors.dedup();
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(seed: u64) -> Blueprint {
        generate(
            seed,
            &Config::default(),
            &Catalog::bundled(),
            &gameplay::Difficulty::default(),
        )
        .unwrap()
    }
    #[test]
    fn stairs_change_collision_height_at_tread_edges_in_both_directions() {
        for (start_y, end_y) in [(0.0, 3.0), (3.0, 0.0)] {
            let c = Connection {
                id: 0,
                a: 0,
                b: 1,
                start: Vec3::new(0.0, start_y, 0.0),
                end: Vec3::new(12.0, end_y, 0.0),
                width: 3.4,
                height: 3.4,
                kind: ConnectionKind::Stairs,
                critical: true,
            };
            let count = ((end_y - start_y).abs() / 0.15).ceil() as usize;
            let rise = (end_y - start_y) / count as f32;
            assert!((c.floor_y(0.0) - start_y).abs() < 1.0e-5);
            for i in 0..count {
                let mid = (i as f32 + 0.5) / count as f32;
                let tread = start_y + rise * i as f32;
                assert!(
                    (c.floor_y(mid) - tread).abs() < 1.0e-4,
                    "wrong tread level at {mid}"
                );
                let edge = (i + 1) as f32 / count as f32;
                assert!(
                    (c.floor_y(edge) - (tread + rise)).abs() < 1.0e-4,
                    "wrong riser level at {edge}"
                );
            }
            assert!((c.floor_y(1.0) - end_y).abs() < 1.0e-5);
        }
    }
    #[test]
    fn deterministic_and_rng_domains_are_independent() {
        let catalog = Catalog::bundled();
        let a = sample(4815);
        let b = sample(4815);
        assert_eq!(a.structural_hash(), b.structural_hash());
        assert_ne!(a.structural_hash(), sample(4816).structural_hash());
        let config = Config {
            decoration_density: 0.1,
            ..Config::default()
        };
        let c = generate(
            4815,
            &config,
            &catalog,
            &gameplay::Difficulty {
                group_size: 4,
                run_modifier: 2.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(a.structural_hash(), c.structural_hash());
        assert_ne!(
            serde_json::to_vec(&a.spawns).unwrap(),
            serde_json::to_vec(&c.spawns).unwrap()
        );
    }
    #[test]
    fn validates_many_seeds() {
        for seed in 0..256 {
            sample(seed);
        }
    }
    #[test]
    fn capsule_walks_every_connection_in_both_directions() {
        let m = sample(42);
        for c in &m.connections {
            let a = m.rooms[c.a].center;
            let b = m.rooms[c.b].center;
            controller::traverse(&m, &[a, c.start, c.end, b]).unwrap();
            controller::traverse(&m, &[b, c.end, c.start, a]).unwrap();
        }
    }
    #[test]
    fn corruption_is_reported_not_hidden() {
        let mut m = sample(42);
        m.connections.clear();
        assert!(!validate(&m).is_empty());
        let mut m = sample(42);
        m.rooms[2].center = m.rooms[0].center;
        assert!(validate(&m).iter().any(|s| s.contains("overlap")));
    }
    #[test]
    fn invalid_rules_fail_explicitly() {
        let c = Config {
            room_scale: f32::NAN,
            ..Default::default()
        };
        assert!(generate(0, &c, &Catalog::bundled(), &Default::default()).is_err());
    }
    #[test]
    fn malformed_indices_are_errors_not_panics() {
        let mut m = sample(2);
        m.connections[0].b = usize::MAX;
        assert!(!validate(&m).is_empty());
    }
    #[test]
    fn catalog_rejects_ambiguous_or_unsafe_content() {
        let mut c = Catalog::bundled();
        c.enemies[0].cost = 0.0;
        assert!(c.validate().is_err());
        let mut c = Catalog::bundled();
        c.biomes.push(c.biomes[0].clone());
        assert!(c.validate().is_err());
    }
    #[test]
    fn supported_parameter_extremes() {
        for seed in 0..16 {
            for config in [
                Config {
                    critical_path_length: 10,
                    room_scale: 0.8,
                    corridor_length: 10.0,
                    verticality: 1.0,
                    branch_factor: 1.0,
                    loop_probability: 1.0,
                    ..Default::default()
                },
                Config {
                    critical_path_length: 64,
                    room_scale: 1.5,
                    verticality: 0.0,
                    branch_factor: 0.0,
                    loop_probability: 0.0,
                    ..Default::default()
                },
                Config {
                    critical_path_length: 96,
                    room_scale: 1.5,
                    corridor_length: 28.0,
                    verticality: 1.0,
                    branch_factor: 1.0,
                    loop_probability: 1.0,
                    ..Default::default()
                },
            ] {
                let m = generate(seed, &config, &Catalog::bundled(), &Default::default()).unwrap();
                assert!(m.rooms.iter().any(|r| r.role == Role::Elite));
                let restored: Blueprint =
                    serde_json::from_slice(&serde_json::to_vec(&m).unwrap()).unwrap();
                assert_eq!(m.structural_hash(), restored.structural_hash());
            }
        }
    }
    #[test]
    fn controller_cannot_walk_through_wall() {
        let m = sample(15);
        let r = &m.rooms[0];
        let mut c = controller::Controller::new(r.center);
        c.yaw = -std::f32::consts::FRAC_PI_2;
        for _ in 0..300 {
            c.step(
                &m,
                controller::Input {
                    forward: 1.0,
                    ..Default::default()
                },
                1.0 / 60.0,
            );
        }
        assert!(m.capsule_floor(c.position, PLAYER_RADIUS).is_some());
    }
}
