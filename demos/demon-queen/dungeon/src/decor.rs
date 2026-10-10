//! Solid room decor shared by rendering AND player collision: wall-bay prop
//! clusters (barrels, crates, kegs, rubble, candles), heraldic banners and the
//! biome beacons in the chamfered corners. One deterministic list per room,
//! so what the player sees is exactly what the player bumps into.
use super::architecture;
use velvet_dungeon::{horizontal, rng::Rng, Blueprint, Role, Room, Shape};
use velvet_math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prop {
    pub asset: &'static str,
    pub position: Vec3,
    pub yaw: f32,
    /// Normalized draw height passed to `Assets::draw`.
    pub height: f32,
    /// Collision radius on the floor plane; 0 for wall-hung decor.
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Banner {
    /// Wall anchor of the banner's back plane (bottom of the cloth).
    pub anchor: Vec3,
    pub yaw: f32,
    pub asset: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoomDecor {
    pub props: Vec<Prop>,
    pub banners: Vec<Banner>,
    pub beacons: Vec<Vec3>,
    /// Treasury / arcane vault plan; replaces the generic dressing.
    pub vault: Option<super::vaults::Vault>,
    /// Themed room art ported from the web prototype; replaces the generic
    /// dressing, as a vault does.
    pub theme: Option<ThemeArt>,
}

/// A themed room's plan, shared (it holds its deferred piece builders).
#[derive(Clone, Debug)]
pub struct ThemeArt(pub std::sync::Arc<super::art::plan::Plan>);
impl PartialEq for ThemeArt {
    /// Same plan: same pieces, solids, props, lights and sparkles.
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (&self.0, &other.0);
        a.pieces.len() == b.pieces.len()
            && a.solids == b.solids
            && a.sparkles == b.sparkles
            && a.props
                .iter()
                .map(|p| (p.asset, p.position))
                .eq(b.props.iter().map(|p| (p.asset, p.position)))
            && a.lights
                .iter()
                .map(|l| l.position)
                .eq(b.lights.iter().map(|l| l.position))
    }
}

/// Collision radius of a corner beacon (brazier plinth / ember ring).
pub const BEACON_RADIUS: f32 = 0.42;

fn clear_of_play(map: &Blueprint, room: &Room, p: Vec3, radius: f32) -> bool {
    room.contains(p.x, p.z, radius + 0.05)
        && room
            .sockets
            .iter()
            .all(|s| horizontal(s.position - p).length() > s.width * 0.5 + radius + 1.1)
        && map
            .spawns
            .iter()
            .filter(|s| s.room == room.id)
            .all(|s| horizontal(s.position - p).length() > radius + 1.5)
        && map
            .obstacles
            .iter()
            .filter(|o| o.room == room.id)
            .all(|o| horizontal(o.center - p).length() > radius + 0.9)
}

/// Deterministic beacon anchors in a room's chamfered corners, pulled inward
/// off the wall. Doors, objectives, cover pillars and each other stay clear.
pub fn beacon_points(map: &Blueprint, room: &Room) -> Vec<Vec3> {
    if room.shape == Shape::Bridge || room.outline.len() < 8 {
        return vec![];
    }
    let wanted = match room.role {
        Role::Miniboss | Role::Boss => 4,
        _ => 2,
    };
    let n = room.outline.len();
    let first = (room.landmark_seed % 4) as usize;
    let mut points: Vec<Vec3> = Vec::new();
    // Opposite corners first, so a pair of beacons frames the whole room.
    for k in [0, 2, 1, 3].into_iter().take(n / 2) {
        let i = (2 * ((first + k) % (n / 2)) + 1) % n;
        let a = room.outline[i];
        let b = room.outline[(i + 1) % n];
        let corner = room.center + Vec3::new((a[0] + b[0]) * 0.5, 0.0, (a[1] + b[1]) * 0.5);
        let inward = horizontal(room.center - corner).normalize_or_zero();
        let tangent = horizontal(Vec3::new(b[0] - a[0], 0.0, b[1] - a[1])).normalize_or_zero();
        for slide in [0.0, 1.2, -1.2, 2.0, -2.0] {
            let mut p = corner + inward * 0.8 + tangent * slide;
            p.y = room.floor_y(p.x, p.z);
            let clear = room.contains(p.x, p.z, 0.55)
                && room
                    .sockets
                    .iter()
                    .all(|socket| horizontal(socket.position - p).length() > 2.4)
                && map
                    .spawns
                    .iter()
                    .filter(|spawn| spawn.room == room.id)
                    .all(|spawn| horizontal(spawn.position - p).length() > 1.7)
                && map
                    .obstacles
                    .iter()
                    .filter(|o| o.room == room.id)
                    .all(|o| horizontal(o.center - p).length() > 1.1)
                && points.iter().all(|q| horizontal(*q - p).length() > 2.0);
            if clear {
                points.push(p);
                break;
            }
        }
        if points.len() >= wanted {
            break;
        }
    }
    points
}

/// Everything solid or wall-hung that dresses one room.
pub fn room(map: &Blueprint, room: &Room) -> RoomDecor {
    let mut decor = RoomDecor {
        beacons: beacon_points(map, room),
        ..Default::default()
    };
    if room.shape == Shape::Bridge {
        return decor;
    }
    if let Some(plan) = super::art::theme::any(map, room) {
        // What the web prototype plans, the player sees and bumps into.
        decor.beacons.clear();
        for p in &plan.props {
            decor.props.push(Prop {
                asset: p.asset,
                position: p.position,
                yaw: p.yaw,
                height: p.height,
                radius: p.radius,
            });
        }
        let solids = plan
            .hoards
            .iter()
            .map(|&(p, r, _)| (p, r * 0.9))
            .chain(
                plan.obelisks
                    .iter()
                    .map(|&p| (p, super::art::vaults::OBELISK_RADIUS)),
            )
            .chain(plan.solids.iter().copied());
        for (position, radius) in solids {
            decor.props.push(Prop {
                asset: "",
                position,
                yaw: 0.0,
                height: 0.0,
                radius,
            });
        }
        decor.theme = Some(ThemeArt(std::sync::Arc::new(plan)));
        return decor;
    }
    if let Some(vault) = super::vaults::plan(map, room) {
        decor.beacons.clear();
        decor.props = vault.props.clone();
        decor.props.extend(
            super::vaults::colliders(&vault)
                .into_iter()
                .filter(|p| p.asset.is_empty()),
        );
        decor.vault = Some(vault);
        return decor;
    }
    for incident in super::incidents::plan(map, room) {
        if incident.solid_radius() > 0.0 {
            decor.props.push(Prop {
                asset: "",
                position: incident.center,
                yaw: 0.0,
                height: 0.0,
                radius: incident.solid_radius(),
            });
        }
    }
    let plan = architecture::layout(room);
    let mut rng = Rng::stream(map.seed, "room_decor_v1", room.id as u64);
    // Heraldry: special rooms hang more banners, ordinary halls one or two.
    let banner_budget = match room.role {
        Role::Boss | Role::Miniboss | Role::Elite => 4,
        Role::Entrance | Role::Exit | Role::Rest | Role::Puzzle => 2,
        _ => 1 + rng.index(2),
    };
    let mut bays = plan.bays.clone();
    rng.shuffle(&mut bays);
    let (banner_bays, prop_bays) = bays.split_at(banner_budget.min(bays.len()));
    for bay in banner_bays {
        decor.banners.push(Banner {
            anchor: bay.center + bay.inward * 0.03 + Vec3::Y * (plan.spring - 2.55).max(0.55),
            yaw: bay.inward.x.atan2(bay.inward.z),
            asset: "banner_shield_red",
        });
    }
    // Clutter a subset of the remaining bays; leave some walls breathing.
    let clutter = match room.role {
        Role::Treasure | Role::Secret | Role::Rest => 0.8,
        Role::Boss | Role::Miniboss => 0.25,
        _ => 0.55,
    };
    const CLUSTERS: [&[(&str, f32, f32, f32, f32)]; 6] = [
        // (asset, height, radius, along, out)
        &[
            ("barrel_large", 0.95, 0.42, -0.35, 0.5),
            ("barrel_small", 0.55, 0.27, 0.45, 0.36),
        ],
        &[
            ("box_large", 0.72, 0.43, 0.0, 0.47),
            ("box_small", 0.44, 0.27, 0.62, 0.34),
        ],
        &[
            ("keg", 0.8, 0.42, 0.0, 0.48),
            ("candle_triple", 0.32, 0.0, 0.62, 0.28),
        ],
        &[("crates_stacked", 1.15, 0.62, 0.0, 0.66)],
        &[
            ("rubble_half", 0.5, 0.36, -0.2, 0.38),
            ("barrel_small", 0.55, 0.27, 0.55, 0.34),
        ],
        &[
            ("barrel_large", 0.95, 0.42, 0.0, 0.5),
            ("candle_triple", 0.32, 0.0, -0.6, 0.26),
        ],
    ];
    for bay in prop_bays {
        if !rng.chance(clutter) {
            continue;
        }
        let cluster = CLUSTERS[rng.index(CLUSTERS.len())];
        let mut placed = Vec::new();
        for &(asset, height, radius, along, out) in cluster {
            let mut p = bay.center + bay.tangent * along + bay.inward * out;
            p.y = room.floor_y(p.x, p.z);
            let solid_radius = radius.max(0.15);
            let clear = clear_of_play(map, room, p, solid_radius)
                && decor
                    .beacons
                    .iter()
                    .all(|b| horizontal(*b - p).length() > BEACON_RADIUS + solid_radius + 0.3)
                && decor.props.iter().all(|other| {
                    horizontal(other.position - p).length() > other.radius + solid_radius + 0.25
                });
            if clear {
                placed.push(Prop {
                    asset,
                    position: p,
                    yaw: bay.inward.x.atan2(bay.inward.z) + rng.range(-0.35, 0.35),
                    height,
                    radius,
                });
            }
        }
        decor.props.extend(placed);
    }
    // Gallery-specific authored props use a separate art stream. These are
    // native WGPU assets, not Three.js objects or web-rendered overlays.
    if let Some(theme) = super::themes::of(map, room) {
        use super::themes::Theme;
        let palette: &[(&str, f32, f32)] = match theme {
            Theme::Crypt => &[
                ("mk_chest", 0.8, 0.42),
                ("candle_thin_lit", 0.38, 0.0),
                ("rubble_large", 0.42, 0.3),
                ("vase_small", 0.52, 0.22),
            ],
            Theme::Mine => &[
                ("table_medium_broken", 0.82, 0.5),
                ("barrel_stack", 0.68, 0.35),
                ("box_stacked", 0.72, 0.4),
                ("stool", 0.48, 0.22),
            ],
            Theme::Fungal => &[
                ("nat_mushroom", 0.56, 0.22),
                ("nat_laetiporus", 0.42, 0.2),
                ("nat_fern", 0.66, 0.24),
                ("nat_rock2", 0.48, 0.3),
            ],
            Theme::Forge => &[
                ("table_long", 0.9, 0.6),
                ("barrel_large_deco", 0.85, 0.4),
                ("keg_deco", 0.72, 0.35),
                ("box_stacked", 0.75, 0.4),
            ],
            Theme::Library => &[
                ("mk_bookcase", 2.45, 0.55),
                ("mk_cabinet", 1.45, 0.5),
                ("mk_table", 0.87, 0.58),
                ("mk_nightstand", 0.72, 0.32),
            ],
            Theme::Overgrowth => &[
                ("nat_plant1_big", 0.95, 0.28),
                ("nat_plant7", 0.78, 0.27),
                ("nat_fern", 0.62, 0.23),
                ("nat_rock3", 0.5, 0.32),
            ],
            _ => &[],
        };
        if !palette.is_empty() {
            let mut art_rng = Rng::stream(map.seed, "gallery_props_v1", room.id as u64);
            let mut available = plan.bays.clone();
            art_rng.shuffle(&mut available);
            for (index, bay) in available.iter().enumerate() {
                let &(asset, height, radius) = &palette[index % palette.len()];
                let clearance = radius.max(0.18);
                let mut p = bay.center + bay.inward * (clearance + 0.68);
                p.y = room.floor_y(p.x, p.z);
                if !clear_of_play(map, room, p, clearance)
                    || decor
                        .beacons
                        .iter()
                        .any(|b| horizontal(*b - p).length() < BEACON_RADIUS + clearance + 0.24)
                    || decor.props.iter().any(|q| {
                        horizontal(q.position - p).length() < q.radius.max(0.18) + clearance + 0.26
                    })
                {
                    continue;
                }
                decor.props.push(Prop {
                    asset,
                    position: p,
                    yaw: bay.inward.x.atan2(bay.inward.z),
                    height,
                    radius,
                });
                if decor.props.len() >= 14 {
                    break;
                }
            }
            // Small floor details articulate the walls without filling the
            // centreline used by navigation and encounters.
            let scatter: &[(&str, f32)] = match theme {
                Theme::Crypt => &[("candle_melted", 0.24), ("nat_pebble1", 0.14)],
                Theme::Mine => &[("nat_pebble_sq1", 0.18), ("bottle_a_brown", 0.22)],
                Theme::Fungal => &[("nat_clover2", 0.17), ("nat_mushroom", 0.26)],
                Theme::Forge => &[("nat_pebble2", 0.18), ("bottle_c_brown", 0.24)],
                Theme::Library => &[("mk_books", 0.22), ("bottle_labeled", 0.2)],
                Theme::Overgrowth => &[("nat_grass", 0.25), ("nat_flowers3", 0.3)],
                _ => &[],
            };
            for _ in 0..12 {
                let angle = art_rng.range(0.0, std::f32::consts::TAU);
                let distance = art_rng.range(0.6, 0.84);
                let mut p = room.center
                    + Vec3::new(
                        angle.cos() * room.half[0] * distance,
                        0.0,
                        angle.sin() * room.half[1] * distance,
                    );
                p.y = room.floor_y(p.x, p.z);
                if !clear_of_play(map, room, p, 0.18)
                    || decor
                        .props
                        .iter()
                        .any(|q| horizontal(q.position - p).length() < q.radius.max(0.16) + 0.22)
                {
                    continue;
                }
                let &(asset, height) = &scatter[art_rng.index(scatter.len())];
                decor.props.push(Prop {
                    asset,
                    position: p,
                    yaw: art_rng.range(0.0, std::f32::consts::TAU),
                    height,
                    radius: 0.0,
                });
            }
        }
    }
    decor
}

/// Push a capsule centre out of every solid decor footprint in the room.
pub fn resolve(decor: &RoomDecor, position: Vec3, radius: f32) -> Vec3 {
    let mut p = position;
    let solids = decor
        .props
        .iter()
        .filter(|prop| prop.radius > 0.0)
        .map(|prop| (prop.position, prop.radius))
        .chain(decor.beacons.iter().map(|b| (*b, BEACON_RADIUS)));
    for (center, r) in solids {
        if (p.y - center.y).abs() > 1.2 {
            continue;
        }
        let offset = horizontal(p - center);
        let distance = offset.length();
        let limit = r + radius;
        if distance < limit {
            let normal = if distance > 1.0e-4 {
                offset * (1.0 / distance)
            } else {
                Vec3::X
            };
            p += normal * (limit - distance);
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config, PLAYER_RADIUS};

    #[test]
    fn decor_is_deterministic_and_never_blocks_doors_objectives_or_routes() {
        let catalog = Catalog::bundled();
        let mut props = 0;
        let mut banners = 0;
        for seed in 0..48 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for r in &map.rooms {
                let decor = room(&map, r);
                assert_eq!(decor, room(&map, r));
                if let Some(theme) = &decor.theme {
                    // Themed rooms follow the web prototype's plan (its own
                    // layout tests); here they must furnish and keep doors free.
                    props += decor.props.len();
                    banners += theme.0.mounts.len();
                    for s in &r.sockets {
                        let landing = s.position - s.outward * 0.8;
                        assert!(
                            horizontal(resolve(&decor, landing, PLAYER_RADIUS) - landing).length()
                                < 1.0e-4,
                            "seed {seed} room {} door landing blocked",
                            r.id
                        );
                    }
                    continue;
                }
                if decor.vault.is_some() {
                    continue; // vault layouts have their own tests
                }
                props += decor.props.len();
                banners += decor.banners.len();
                for prop in &decor.props {
                    assert!(r.contains(prop.position.x, prop.position.z, prop.radius));
                    for s in &r.sockets {
                        assert!(
                            horizontal(s.position - prop.position).length() > s.width * 0.5 + 1.0
                        );
                    }
                    for spawn in map.spawns.iter().filter(|s| s.room == r.id) {
                        assert!(horizontal(spawn.position - prop.position).length() > 1.4);
                    }
                }
                // The room centre (spawn point, navigation hub) stays reachable.
                let centre = resolve(&decor, r.center, PLAYER_RADIUS);
                assert!(horizontal(centre - r.center).length() < 1.0e-4);
                // Each door's landing stays free for the capsule.
                for s in &r.sockets {
                    let landing = s.position - s.outward * 0.8;
                    assert!(
                        horizontal(resolve(&decor, landing, PLAYER_RADIUS) - landing).length()
                            < 1.0e-4
                    );
                }
            }
        }
        assert!(props > 400, "rooms should be furnished: {props}");
        assert!(banners > 150, "rooms should carry heraldry: {banners}");
    }
}
