//! Cave art ported from the web prototype (`demon-queen-caves-threejs`), so
//! the native game dresses every room exactly as the Three.js caves do.
pub mod arcane;
pub mod chambers;
pub mod crypt;
pub mod finishes;
pub mod forge;
pub mod formations;
pub mod fungal;
pub mod grand;
pub mod incidents;
pub mod kit;
pub mod library;
pub mod masonry;
pub mod mine;
pub mod overgrowth;
pub mod plan;
pub mod roles;
pub mod sanctuary;
pub mod stage;
pub mod surfaces;
pub mod theme;
pub mod themekit;
pub mod treasury;
pub mod vaults;

#[cfg(test)]
mod parity {
    use velvet_dungeon::gameplay::Difficulty;
    use velvet_dungeon::*;

    /// The web prototype and the game must generate the very same caves
    /// (rooms and pillars), or ported art would dress different rooms.
    /// Enemy counts follow the game's own difficulty.
    #[test]
    fn cave_geometry_matches_the_web_prototype() {
        let catalog = Catalog::bundled();
        for (seed, obstacles, hash) in [(731942u64, 36usize, -1217011782i32), (42, 44, -2095321516)]
        {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            let mut h: i32 = 0;
            let mut push = |value: f32| {
                h = h
                    .wrapping_mul(31)
                    .wrapping_add((value as f64 * 1000.0).round() as i32)
            };
            for r in &map.rooms {
                for value in [
                    r.center.x,
                    r.center.y,
                    r.center.z,
                    r.half[0],
                    r.half[1],
                    r.biome as f32,
                    r.sockets.len() as f32,
                ] {
                    push(value);
                }
            }
            for o in &map.obstacles {
                for value in [o.room as f32, o.center.x, o.center.z, o.half[0]] {
                    push(value);
                }
            }
            assert_eq!((map.obstacles.len(), h), (obstacles, hash), "seed {seed}");
        }
    }
}

/// Vertex-for-vertex comparison with meshes dumped by the web prototype
/// (`temp/parity/<gallery>_<seed>_<room>.json`); skipped when absent.
#[cfg(test)]
mod gallery_parity {
    use velvet_dungeon::gameplay::Difficulty;
    use velvet_dungeon::*;

    fn compare(name: &str, seed: u64, room_id: usize) {
        let path = format!(
            "{}/../../../temp/parity/{name}_{seed}_{room_id}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let Ok(text) = std::fs::read_to_string(&path) else {
            eprintln!("no reference {path}");
            return;
        };
        let reference: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut map = generate(
            seed,
            &Config::default(),
            &Catalog::bundled(),
            &Difficulty::default(),
        )
        .unwrap();
        // Enemy counts follow each generator's difficulty: the prototype's
        // spawns stand in for this room, so only the art is compared.
        if let (Some(spawns), Some(template)) =
            (reference["spawns"].as_array(), map.spawns.first().cloned())
        {
            map.spawns.retain(|s| s.room != room_id);
            for s in spawns {
                use velvet_dungeon::gameplay::SpawnKind as K;
                let kind = match s[0].as_str().unwrap() {
                    "enemy" => K::Enemy,
                    "elite" => K::Elite,
                    "miniboss" => K::Miniboss,
                    "boss" => K::Boss,
                    "chest" => K::Chest,
                    "seal" => K::Seal,
                    "rest" => K::Rest,
                    "memory" => K::Memory,
                    "exit" => K::Exit,
                    "entrance" => K::Entrance,
                    _ => K::Trap,
                };
                let position = velvet_math::Vec3::new(
                    s[1].as_f64().unwrap() as f32,
                    s[2].as_f64().unwrap() as f32,
                    s[3].as_f64().unwrap() as f32,
                );
                map.spawns.push(gameplay::Spawn {
                    room: room_id,
                    kind,
                    position,
                    ..template.clone()
                });
            }
        }
        let room = &map.rooms[room_id];
        let plan = if name.starts_with("vault-") {
            super::vaults::plan(&map, room).expect("signature chamber")
        } else {
            super::theme::plan_with(&map, room, name.starts_with("full-"))
                .expect("themed room")
                .1
        };
        if name.starts_with("vault-") {
            let fmt3 = |p: velvet_math::Vec3| format!("{:.2} {:.2} {:.2}", p.x, p.y, p.z);
            let arr = |key: &str| reference[key].as_array().cloned().unwrap_or_default();
            let got: Vec<String> = plan
                .lights
                .iter()
                .map(|l| format!("{} {:.2}", fmt3(l.position), l.intensity))
                .collect();
            let want: Vec<String> = arr("lights")
                .iter()
                .map(|l| {
                    format!(
                        "{:.2} {:.2} {:.2} {:.2}",
                        l[0].as_f64().unwrap(),
                        l[1].as_f64().unwrap(),
                        l[2].as_f64().unwrap(),
                        l[3].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: lights");
            let got: Vec<String> = plan
                .hoards
                .iter()
                .map(|h| format!("{:.2} {:.2} {:.3} {:.3}", h.0.x, h.0.z, h.1, h.2))
                .collect();
            let want: Vec<String> = arr("hoards")
                .iter()
                .map(|h| {
                    format!(
                        "{:.2} {:.2} {:.3} {:.3}",
                        h[0].as_f64().unwrap(),
                        h[1].as_f64().unwrap(),
                        h[2].as_f64().unwrap(),
                        h[3].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: hoards");
            let got: Vec<String> = plan.obelisks.iter().map(|&p| fmt3(p)).collect();
            let want: Vec<String> = arr("obelisks")
                .iter()
                .map(|p| {
                    format!(
                        "{:.2} {:.2} {:.2}",
                        p[0].as_f64().unwrap(),
                        p[1].as_f64().unwrap(),
                        p[2].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: obelisks");
            let got: Vec<String> = plan.sconces.iter().map(|s| fmt3(s.flame)).collect();
            let want: Vec<String> = arr("sconces")
                .iter()
                .map(|p| {
                    format!(
                        "{:.2} {:.2} {:.2}",
                        p[0].as_f64().unwrap(),
                        p[1].as_f64().unwrap(),
                        p[2].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: sconces");
            assert_eq!(
                plan.glow_lights.len() as u64,
                reference["glowLights"].as_u64().unwrap(),
                "{name} {seed} room {room_id}: glow lights"
            );
            assert_eq!(
                plan.open_chests.len() as u64,
                reference["openChests"].as_u64().unwrap(),
                "{name} {seed} room {room_id}: open chests"
            );
            let got: Vec<String> = plan
                .props
                .iter()
                .map(|p| format!("{} {:.2} {:.2}", p.asset, p.position.x, p.position.z))
                .collect();
            let want: Vec<String> = arr("props")
                .iter()
                .map(|p| {
                    format!(
                        "{} {:.2} {:.2}",
                        p[0].as_str().unwrap(),
                        p[1].as_f64().unwrap(),
                        p[2].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: props");
        }
        assert_eq!(
            plan.pieces.len() as u64,
            reference["pieces"].as_u64().unwrap(),
            "{name} {seed} room {room_id}: pieces"
        );
        assert_eq!(
            plan.props.len(),
            reference["props"].as_array().unwrap().len(),
            "props"
        );
        assert_eq!(
            plan.solids.len(),
            reference["solids"].as_array().unwrap().len(),
            "solids"
        );
        // Piece by piece, so a divergence names the piece it starts in.
        if let Some(marks) = reference["marks"].as_array() {
            let mut m = super::kit::empty();
            let mut g = super::kit::empty();
            let mut ctx = super::plan::Ctx {
                stone: [0.4, 0.38, 0.36],
                glow_color: [1.0, 0.6, 0.3],
                rng: super::kit::ArtRng::stream(map.seed, "treasury_art_v1", room.id as u64),
                wet: vec![],
            };
            let detail_ref: Vec<f64> = reference["detail"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            for (k, piece) in plan.pieces.iter().enumerate() {
                let start = m.vertices.len();
                piece(&mut m, &mut g, &mut ctx);
                let mark = &marks[k];
                let (kind, dv, gv) = (
                    mark[0].as_str().unwrap(),
                    mark[1].as_u64().unwrap() as usize,
                    mark[2].as_u64().unwrap() as usize,
                );
                assert_eq!(
                    (m.vertices.len(), g.vertices.len()),
                    (dv, gv),
                    "piece {k} ({kind}) vertex counts"
                );
                for i in start..m.vertices.len() {
                    for a in 0..3 {
                        let d = (m.vertices[i].position[a] as f64 - detail_ref[i * 3 + a]).abs();
                        assert!(
                            d < 5e-3,
                            "seed {seed} piece {k} ({kind}) vertex {} differs by {d}: {:?} vs {:?}",
                            i - start,
                            m.vertices[i].position,
                            &detail_ref[i * 3..i * 3 + 3]
                        );
                    }
                }
            }
        }
        let (pulse, pulse_glow) =
            super::theme::build_pulse(&plan, [0.4, 0.38, 0.36], [1.0, 0.6, 0.3], map.seed, room.id);
        for (mesh, key) in [(&pulse, "pulse"), (&pulse_glow, "pulseGlow")] {
            let Some(pos) = reference[key].as_array() else {
                continue;
            };
            assert_eq!(
                mesh.vertices.len() * 3,
                pos.len(),
                "{name} {seed} room {room_id}: {key} vertex count"
            );
            for (i, vtx) in mesh.vertices.iter().enumerate() {
                for a in 0..3 {
                    let d = (vtx.position[a] as f64 - pos[i * 3 + a].as_f64().unwrap()).abs();
                    assert!(
                        d < 5e-3,
                        "{name} {seed} room {room_id}: {key} vertex {i} differs by {d}"
                    );
                }
            }
        }
        if let Some(mounts) = reference["mounts"].as_array() {
            let got: Vec<String> = plan
                .mounts
                .iter()
                .map(|m| format!("{} {:.2} {:.2}", m.asset, m.anchor.x, m.anchor.z))
                .collect();
            let want: Vec<String> = mounts
                .iter()
                .map(|m| {
                    format!(
                        "{} {:.2} {:.2}",
                        m[0].as_str().unwrap(),
                        m[1].as_f64().unwrap(),
                        m[2].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: mounts");
        }
        if let Some(weapons) = reference["weapons"].as_array() {
            let got: Vec<String> = plan
                .weapons
                .iter()
                .map(|w| {
                    format!(
                        "{} {:.2} {:.2} {:.3}",
                        w.name, w.position.x, w.position.z, w.height
                    )
                })
                .collect();
            let want: Vec<String> = weapons
                .iter()
                .map(|w| {
                    format!(
                        "{} {:.2} {:.2} {:.3}",
                        w[0].as_str().unwrap(),
                        w[1].as_f64().unwrap(),
                        w[2].as_f64().unwrap(),
                        w[3].as_f64().unwrap()
                    )
                })
                .collect();
            assert_eq!(got, want, "{name} {seed} room {room_id}: weapons");
        }
        if let Some(n) = reference["sparkles"].as_u64() {
            assert_eq!(
                plan.sparkles.len() as u64,
                n,
                "{name} {seed} room {room_id}: sparkles"
            );
        }
        let (detail, glow, water) =
            super::theme::build(&plan, [0.4, 0.38, 0.36], [1.0, 0.6, 0.3], map.seed, room.id);
        assert_eq!(
            water.len(),
            reference["water"].as_array().unwrap().len(),
            "water"
        );
        for (mesh, key) in [(&detail, "detail"), (&glow, "glow")] {
            let pos: Vec<f64> = reference[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(mesh.vertices.len() * 3, pos.len(), "{key} vertex count");
            let mut worst = 0.0f64;
            for (i, vtx) in mesh.vertices.iter().enumerate() {
                for a in 0..3 {
                    worst = worst.max((vtx.position[a] as f64 - pos[i * 3 + a]).abs());
                }
            }
            assert!(worst < 5e-3, "{key}: vertices differ by {worst}");
        }
    }

    /// Every reference dumped by the prototype for this gallery.
    fn compare_all(name: &str) {
        let dir = format!("{}/../../../temp/parity", env!("CARGO_MANIFEST_DIR"));
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().to_string();
            let Some(rest) = file
                .strip_prefix(&format!("{name}_"))
                .and_then(|r| r.strip_suffix(".json"))
            else {
                continue;
            };
            let mut parts = rest.split('_');
            let seed: u64 = parts.next().unwrap().parse().unwrap();
            let room: usize = parts.next().unwrap().parse().unwrap();
            compare(name, seed, room);
        }
    }

    #[test]
    fn the_mine_is_built_exactly_as_in_the_web_prototype() {
        compare_all("mine");
    }
    #[test]
    fn the_forge_is_built_exactly_as_in_the_web_prototype() {
        compare_all("forge");
    }
    #[test]
    fn the_crypt_is_built_exactly_as_in_the_web_prototype() {
        compare_all("crypt");
    }
    #[test]
    fn the_library_is_built_exactly_as_in_the_web_prototype() {
        compare_all("library");
    }
    #[test]
    fn the_fungal_grotto_is_built_exactly_as_in_the_web_prototype() {
        compare_all("fungal");
    }
    #[test]
    fn the_flooded_roots_are_built_exactly_as_in_the_web_prototype() {
        compare_all("overgrowth");
    }
    /// Signature chambers: loot halls, the grand treasury, the arcane
    /// library and the sanctuary.
    #[test]
    fn signature_chambers_are_built_exactly_as_in_the_web_prototype() {
        for name in [
            "vault-loot",
            "vault-grand",
            "vault-arcane",
            "vault-sanctuary",
        ] {
            compare_all(name);
        }
    }

    /// Whole themed rooms: the gallery plus its incidents.
    #[test]
    fn incidents_are_planned_and_built_exactly_as_in_the_web_prototype() {
        for name in [
            "mine",
            "forge",
            "crypt",
            "library",
            "fungal",
            "overgrowth",
            "heart",
            "custodian",
            "guardpost",
            "lair",
            "seal",
            "memory",
            "threshold",
            "return",
        ] {
            compare_all(&format!("full-{name}"));
        }
    }
}

#[cfg(test)]
mod layout_rules {
    use super::kit::*;
    use velvet_dungeon::gameplay::Difficulty;
    use velvet_dungeon::*;

    /// Web `art.test.js`: every doorway's approach to the focus stays open,
    /// every solid stands in the room, and the art stays within budget.
    #[test]
    fn themed_rooms_keep_every_approach_open() {
        let catalog = Catalog::bundled();
        let mut themed = 0;
        for seed in [0u64, 1, 7, 42, 99, 4815, 8675309, 731942] {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let Some((_, plan)) = super::theme::plan(&map, room) else {
                    continue;
                };
                themed += 1;
                let solids = plan.colliders();
                for socket in &room.sockets {
                    let start = socket.position - socket.outward * 0.6;
                    for i in 0..=20 {
                        let p = start.lerp(plan.focus, i as f32 / 20.0);
                        for &(position, radius) in &solids {
                            if hdist(position, plan.focus) < 0.01 {
                                continue;
                            }
                            assert!(
                                hdist(position, p) > radius + PLAYER_RADIUS - 0.05,
                                "seed {seed} room {}: approach blocked",
                                room.id
                            );
                        }
                    }
                }
                for &(position, radius) in &solids {
                    let wall = plan
                        .wall_solids
                        .iter()
                        .any(|&(q, _)| hdist(q, position) < 1e-6);
                    assert!(
                        room.contains(position.x, position.z, if wall { 0.0 } else { radius }),
                        "seed {seed} room {}: solid outside",
                        room.id
                    );
                }
                let (detail, glow, _) = super::theme::build(
                    &plan,
                    [0.4, 0.38, 0.36],
                    [1.0, 0.6, 0.2],
                    map.seed,
                    room.id,
                );
                assert!(detail
                    .indices
                    .iter()
                    .all(|&i| (i as usize) < detail.vertices.len()));
                assert!(detail.vertices.iter().chain(glow.vertices.iter()).all(|v| v
                    .position
                    .iter()
                    .chain(v.normal.iter())
                    .all(|x| x.is_finite())));
                assert!(
                    detail.indices.len() / 3 < 160000,
                    "seed {seed} room {}: theme too heavy",
                    room.id
                );
            }
        }
        assert!(themed > 3, "{themed} themed rooms");
    }
}

#[cfg(test)]
mod shell_parity {
    use velvet_dungeon::gameplay::Difficulty;
    use velvet_dungeon::*;

    fn load(name: &str) -> Option<Vec<f32>> {
        let path = format!(
            "{}/../../../temp/parity/shell/{name}.bin",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(path).ok()?;
        Some(
            bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        )
    }

    fn compare(name: &str, mesh: &velvet_render::Mesh3D, failures: &mut Vec<String>) {
        let Some(want) = load(name) else { return };
        let n = want.len() / 6;
        let counts = if mesh.vertices.len() != n {
            format!(" ({} vertices, web {n})", mesh.vertices.len())
        } else {
            String::new()
        };
        for (i, vtx) in mesh.vertices.iter().enumerate().take(n) {
            let d = (0..3)
                .map(|a| (vtx.position[a] - want[i * 6 + a]).abs())
                .fold(0.0f32, f32::max);
            let c = (0..3)
                .map(|a| (vtx.color[a] - want[i * 6 + 3 + a]).abs())
                .fold(0.0f32, f32::max);
            if d > 5e-3 || c > 5e-3 {
                failures.push(format!(
                    "{name}{counts}: vertex {i} differs by {d:.4} (colour {c:.4}): {:?} vs {:?}",
                    vtx.position,
                    &want[i * 6..i * 6 + 3]
                ));
                return;
            }
        }
        if !counts.is_empty() {
            failures.push(format!("{name}{counts}: extra vertices at the end"));
        }
    }

    /// Room and passage shells are built exactly as in the web prototype.
    #[test]
    fn room_shells_match_the_web_prototype() {
        let catalog = Catalog::bundled();
        let mut failures = vec![];
        for seed in 0..3u64 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let plan = super::theme::any(&map, room);
                let mesh = super::chambers::room_mesh(&map, &catalog, room.id, plan.as_ref());
                compare(&format!("room_{seed}_{}", room.id), &mesh, &mut failures);
            }
            for c in &map.connections {
                let mesh = super::chambers::corridor_mesh(&map, &catalog, c.id);
                compare(&format!("corridor_{seed}_{}", c.id), &mesh, &mut failures);
            }
        }
        assert!(
            failures.is_empty(),
            "{} shells differ:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
