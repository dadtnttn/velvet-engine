//! Narrative wear in the authored cave galleries. A separate deterministic
//! stream plans it; the same plan feeds static geometry and solid footprints.
use super::{
    architecture,
    themes::{self, Theme},
    water,
};
use velvet_dungeon::{horizontal, rng::Rng, Blueprint, Room, Shape};
use velvet_math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Collapse,
    Rift,
    LavaRift,
    Leak,
    Grate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incident {
    pub kind: Kind,
    pub center: Vec3,
    pub direction: Vec3,
    pub length: f32,
    pub radius: f32,
}

impl Incident {
    pub fn solid_radius(self) -> f32 {
        if self.kind == Kind::Collapse {
            0.95
        } else {
            0.0
        }
    }
}

fn fits(theme: Theme) -> &'static [Kind] {
    use Kind::*;
    match theme {
        Theme::Threshold => &[Collapse, Rift],
        Theme::Return | Theme::Seal => &[Rift, Leak],
        Theme::Memory => &[Leak, Rift],
        Theme::Lair => &[Collapse, Rift],
        Theme::Guardpost => &[Grate, Collapse],
        Theme::Custodian | Theme::Heart => &[LavaRift, Collapse],
        Theme::Crypt => &[Collapse, Leak, Rift],
        Theme::Mine => &[Collapse, Leak, Grate],
        Theme::Fungal => &[Leak, Collapse],
        Theme::Forge => &[Collapse, LavaRift, Grate],
        Theme::Library => &[Leak, Collapse],
        Theme::Overgrowth => &[Leak, Rift, Collapse],
    }
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = horizontal(b - a);
    let t = (horizontal(p - a).dot(ab) / ab.length_squared().max(1.0e-5)).clamp(0.0, 1.0);
    horizontal(p - (a + ab * t)).length()
}

fn solid_clear(map: &Blueprint, room: &Room, p: Vec3, radius: f32) -> bool {
    room.contains(p.x, p.z, radius + 0.18)
        && map.capsule_floor(p, radius.min(0.4)).is_some()
        && room.sockets.iter().all(|socket| {
            segment_distance(p, room.center, socket.position) > radius + 1.0
                && horizontal(p - socket.position).length() > radius + socket.width * 0.5 + 1.1
        })
        && map
            .spawns
            .iter()
            .filter(|s| s.room == room.id)
            .all(|s| horizontal(p - s.position).length() > radius + 1.2)
        && map
            .obstacles
            .iter()
            .filter(|o| o.room == room.id)
            .all(|o| horizontal(p - o.center).length() > radius + o.half[0].max(o.half[1]) + 0.4)
        && super::decor::beacon_points(map, room)
            .iter()
            .all(|b| horizontal(p - *b).length() > radius + super::decor::BEACON_RADIUS + 0.25)
}

/// Room art must never consume the layout/gameplay/loot RNG streams.
pub fn plan(map: &Blueprint, room: &Room) -> Vec<Incident> {
    if room.shape == Shape::Bridge {
        return Vec::new();
    }
    let Some(theme) = themes::of(map, room) else {
        return Vec::new();
    };
    let mut rng = Rng::stream(map.seed, "room_incidents_v1", room.id as u64);
    let mut choices = fits(theme).to_vec();
    rng.shuffle(&mut choices);
    choices.truncate(if rng.chance(0.55) { 2 } else { 1 });
    let mut bays = architecture::layout(room).bays;
    rng.shuffle(&mut bays);
    let mut result: Vec<Incident> = Vec::new();
    for kind in choices {
        if kind == Kind::Leak {
            if let Some(water) = water::for_room(map, room) {
                result.push(Incident {
                    kind,
                    center: water.center,
                    direction: Vec3::Y,
                    length: room.height,
                    radius: 0.58,
                });
            }
            continue;
        }
        for bay in &bays {
            let offset = match kind {
                Kind::Collapse => 1.52,
                Kind::Rift | Kind::LavaRift => 0.95,
                Kind::Grate => 0.88,
                Kind::Leak => unreachable!(),
            };
            let mut center =
                bay.center + bay.inward * offset + bay.tangent * rng.range(-0.28, 0.28);
            center.y = room.floor_y(center.x, center.z);
            let radius = match kind {
                Kind::Collapse => 1.1,
                Kind::Rift | Kind::LavaRift => 0.45,
                _ => 0.6,
            };
            if !room.contains(center.x, center.z, radius + 0.15)
                || room
                    .sockets
                    .iter()
                    .any(|s| horizontal(center - s.position).length() < radius + 1.6)
                || result.iter().any(|other| {
                    horizontal(center - other.center).length() < radius + other.radius + 0.4
                })
            {
                continue;
            }
            if kind == Kind::Collapse && !solid_clear(map, room, center, 0.95) {
                continue;
            }
            if kind == Kind::Grate
                && map
                    .spawns
                    .iter()
                    .filter(|s| s.room == room.id)
                    .any(|s| horizontal(center - s.position).length() < 1.2)
            {
                continue;
            }
            let direction = bay.inward.normalize_or_zero();
            let length = if matches!(kind, Kind::Rift | Kind::LavaRift) {
                rng.range(2.4, room.half[0].min(room.half[1]).min(4.5))
            } else {
                0.0
            };
            result.push(Incident {
                kind,
                center,
                direction,
                length,
                radius,
            });
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};
    #[test]
    fn incidents_are_reproducible_and_solids_avoid_navigation() {
        let catalog = Catalog::bundled();
        let config = Config::default();
        let mut kinds = std::collections::BTreeSet::new();
        for seed in 0..100 {
            let map = generate(seed, &config, &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let planned = plan(&map, room);
                assert_eq!(planned, plan(&map, room));
                for e in planned {
                    kinds.insert(e.kind as u8);
                    assert!(e.center.to_array().iter().all(|v| v.is_finite()));
                    assert!(room.contains(e.center.x, e.center.z, e.radius.min(0.4)));
                    if e.solid_radius() > 0.0 {
                        assert!(solid_clear(&map, room, e.center, e.solid_radius()));
                    }
                }
            }
        }
        assert!(kinds.len() >= 4, "only {kinds:?} incident types");
    }
}
