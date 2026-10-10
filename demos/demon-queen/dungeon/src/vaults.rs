//! Signature chambers: the Treasury (Role::Treasure) and the Arcane Vault
//! (Role::Secret). A deterministic plan per room drives three layers:
//!  - static geometry: the treasury's procedural art kit (`treasury.rs`) is
//!    built into one vertex-coloured detail mesh; the arcane obelisks are
//!    baked into the room chunk,
//!  - one emissive "glow" mesh per room (flames, gem cores, gold threads or
//!    the rune circle),
//!  - authored props, sconces, lights and animated sparkles drawn by the scene.
//! Every free-standing element is also a collider through `decor`.
use super::decor::Prop;
use super::visual::{quad, tint};
use std::f32::consts::TAU;
use velvet_dungeon::{gameplay::SpawnKind, horizontal, rng::Rng, Blueprint, Role, Room, Shape};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Treasury,
    Arcane,
    Sanctuary,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Relic {
    Chalice,
    Crown,
    Gems,
    Helm,
    Armillary,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pedestal {
    pub base: Vec3,
    pub relic: Relic,
    pub yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub asset: &'static str,
    pub anchor: Vec3,
    pub yaw: f32,
    pub height: f32,
}

/// Procedural treasury pieces, built by `treasury::build`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    Carpet {
        from: Vec3,
        to: Vec3,
        half_width: f32,
    },
    Rug {
        center: Vec3,
        radius: f32,
    },
    Urn {
        base: Vec3,
        height: f32,
        style: u8,
        yaw: f32,
    },
    Candles {
        base: Vec3,
        count: u8,
        spread: f32,
    },
    Candelabrum {
        base: Vec3,
        yaw: f32,
    },
    Chandelier {
        ceiling: Vec3,
        ring_y: f32,
        radius: f32,
    },
    Banner {
        top: Vec3,
        inward: Vec3,
        tangent: Vec3,
        width: f32,
        length: f32,
        sun: bool,
    },
    Shrine {
        wall: Vec3,
        inward: Vec3,
        width: f32,
    },
    CandleRow {
        from: Vec3,
        to: Vec3,
        count: u8,
        gap: bool,
    },
    CrystalBrazier {
        base: Vec3,
        scale: f32,
    },
    Fern {
        base: Vec3,
        scale: f32,
    },
    Pot {
        base: Vec3,
        height: f32,
    },
    SunRelief {
        center: Vec3,
        normal: Vec3,
        radius: f32,
    },
    PrayerRug {
        center: Vec3,
        along: Vec3,
        half: [f32; 2],
    },
    CoinScatter {
        center: Vec3,
        radius: f32,
        count: u16,
    },
    ChestHoard {
        base: Vec3,
        yaw: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VaultLight {
    pub position: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
}

/// Torch on a pilaster face: KayKit wall torch plus a live flame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sconce {
    pub backplate: Vec3,
    pub yaw: f32,
    pub flame: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vault {
    pub kind: Kind,
    /// Chest position (the room's focus) and the direction from the entry.
    pub focus: Vec3,
    pub inward: Vec3,
    pub props: Vec<Prop>,
    pub mounts: Vec<Mount>,
    pub pedestals: Vec<Pedestal>,
    /// (centre, radius, height) of each gold hoard mound.
    pub hoards: Vec<(Vec3, f32, f32)>,
    /// Rune obelisks (Arcane) standing in the corners.
    pub obelisks: Vec<Vec3>,
    /// Flame points drawn as live, animated flames (arcane candles).
    pub flames: Vec<Vec3>,
    pub chandelier: Option<Vec3>,
    /// Twinkling glints on gold / drifting motes in the arcane vault.
    pub sparkles: Vec<Vec3>,
    pub medallion_radius: f32,
    pub pieces: Vec<Piece>,
    pub lights: Vec<VaultLight>,
    pub sconces: Vec<Sconce>,
    /// Open, overflowing decorative chests: (position, yaw).
    pub open_chests: Vec<(Vec3, f32)>,
    /// Extra procedural solids: (centre, radius).
    pub solids: Vec<(Vec3, f32)>,
    /// Lights tinted by the biome glow at draw time: (position, intensity, radius).
    pub glow_lights: Vec<(Vec3, f32, f32)>,
}

pub const PEDESTAL_RADIUS: f32 = 0.42;
pub const OBELISK_RADIUS: f32 = 0.34;
pub const GOLD: [f32; 3] = [1.0, 0.66, 0.16];
pub const ARCANE: [f32; 3] = [0.62, 0.32, 1.0];
const WARM: [f32; 3] = [1.0, 0.62, 0.28];

fn frame(room: &Room) -> (Vec3, Vec3, f32, f32) {
    let inward = room
        .sockets
        .first()
        .map(|s| -horizontal(s.outward).normalize_or_zero())
        .filter(|d| d.length_squared() > 0.5)
        .unwrap_or(Vec3::Z);
    let right = Vec3::new(inward.z, 0.0, -inward.x);
    let depth = inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1];
    let width = right.x.abs() * room.half[0] + right.z.abs() * room.half[1];
    (inward, right, depth, width)
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let d = horizontal(b - a);
    let t = if d.length_squared() > 1.0e-4 {
        (horizontal(p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    horizontal(p - (a + d * t)).length()
}

fn yaw_toward(from: Vec3, to: Vec3) -> f32 {
    let d = horizontal(to - from);
    d.x.atan2(d.z)
}

struct Placer<'a> {
    map: &'a Blueprint,
    room: &'a Room,
    focus: Vec3,
    taken: Vec<(Vec3, f32)>,
}

impl Placer<'_> {
    /// Keeps doors, every door→chest approach, gameplay spawns, cover
    /// pillars and previously placed pieces clear. Returns the floor point.
    fn try_place(&mut self, candidates: &[Vec3], radius: f32, focus_gap: f32) -> Option<Vec3> {
        for c in candidates {
            let mut p = *c;
            if !self.room.contains(p.x, p.z, radius + 0.12) {
                continue;
            }
            p.y = self.room.floor_y(p.x, p.z);
            if self.clear(p, radius, focus_gap) {
                self.taken.push((p, radius));
                return Some(p);
            }
        }
        None
    }

    fn clear(&self, p: Vec3, radius: f32, focus_gap: f32) -> bool {
        self.room.sockets.iter().all(|s| {
            horizontal(s.position - p).length() > s.width * 0.5 + radius + 1.2
                && segment_distance(p, s.position, self.focus) > radius + 1.0
        }) && horizontal(p - self.focus).length() > radius + focus_gap
            && self
                .map
                .spawns
                .iter()
                .filter(|s| {
                    s.room == self.room.id && horizontal(s.position - self.focus).length() > 0.01
                })
                .all(|s| horizontal(s.position - p).length() > radius + 1.4)
            && self
                .map
                .obstacles
                .iter()
                .filter(|o| o.room == self.room.id)
                .all(|o| horizontal(o.center - p).length() > radius + 0.85)
            && self
                .taken
                .iter()
                .all(|(q, r)| horizontal(*q - p).length() > radius + r + 0.3)
    }

    /// Small non-solid dressing (candles, coins): inside the room, out of
    /// doorways and off every solid, but free to sit near the carpet edge.
    fn floor_spot(&self, p: Vec3, radius: f32) -> Option<Vec3> {
        let mut p = p;
        if !self.room.contains(p.x, p.z, radius + 0.15) {
            return None;
        }
        p.y = self.room.floor_y(p.x, p.z);
        let ok = self
            .room
            .sockets
            .iter()
            .all(|s| horizontal(s.position - p).length() > s.width * 0.5 + radius + 0.5)
            && self
                .taken
                .iter()
                .all(|(q, r)| horizontal(*q - p).length() > radius + r + 0.08)
            && self
                .map
                .obstacles
                .iter()
                .filter(|o| o.room == self.room.id)
                .all(|o| horizontal(o.center - p).length() > radius + 0.7);
        ok.then_some(p)
    }
}

pub fn plan(map: &Blueprint, room: &Room) -> Option<Vault> {
    let kind = match room.role {
        Role::Treasure => Kind::Treasury,
        Role::Secret => Kind::Arcane,
        Role::Rest => Kind::Sanctuary,
        _ => return None,
    };
    if room.shape == Shape::Bridge || room.sockets.is_empty() {
        return None;
    }
    let focus = map
        .spawns
        .iter()
        .find(|s| {
            s.room == room.id
                && s.kind
                    == if kind == Kind::Sanctuary {
                        SpawnKind::Rest
                    } else {
                        SpawnKind::Chest
                    }
        })?
        .position;
    let (inward, right, depth, width) = frame(room);
    let entry = room.sockets[0].position;
    let c = room.center;
    let mut rng = Rng::stream(map.seed, "vault_v1", room.id as u64);
    let mut placer = Placer {
        map,
        room,
        focus,
        taken: vec![],
    };
    let mut vault = Vault {
        kind,
        focus,
        inward,
        props: vec![],
        mounts: vec![],
        pedestals: vec![],
        hoards: vec![],
        obelisks: vec![],
        flames: vec![],
        chandelier: None,
        sparkles: vec![],
        medallion_radius: 1.3,
        pieces: vec![],
        lights: vec![],
        sconces: vec![],
        open_chests: vec![],
        solids: vec![],
        glow_lights: vec![],
    };
    let at = |f: f32, r: f32| c + inward * f + right * r;
    let side = {
        let s = horizontal(focus - entry).normalize_or_zero();
        Vec3::new(s.z, 0.0, -s.x)
    };
    let toward = horizontal(entry - focus).normalize_or_zero();
    let bays = super::architecture::layout(room).bays;

    match kind {
        Kind::Sanctuary => plan_sanctuary(map, room, &mut vault, &mut placer, &mut rng),
        Kind::Treasury => plan_treasury(
            map,
            room,
            &mut vault,
            &mut placer,
            &mut rng,
            entry,
            inward,
            right,
            depth,
            width,
        ),
        Kind::Arcane => {
            placer.taken.push((focus, 0.55));
            // Flanking light for the focus: two standing candelabras.
            for sign in [-1.0f32, 1.0] {
                let p = focus + side * (sign * 1.35) - toward * 0.25;
                if let Some(p) = placer.try_place(&[p, focus + side * (sign * 1.6)], 0.3, 1.0) {
                    vault.props.push(Prop {
                        asset: "candelabra_stand",
                        position: p,
                        yaw: toward.x.atan2(toward.z),
                        height: 1.45,
                        radius: 0.3,
                    });
                    vault.flames.push(p + Vec3::Y * 1.47);
                }
            }
            vault.medallion_radius = 1.55;
            // Rune obelisks in the four corners.
            for (f, r) in [(0.78, -0.72), (0.78, 0.72), (-0.62, -0.74), (-0.62, 0.74)] {
                let candidates = [
                    at(depth * f, width * r),
                    at(depth * f * 0.85, width * r * 0.85),
                    at(depth * f * 0.6, width * r * 0.92),
                    at(depth * f * 0.95, width * r * 0.5),
                    at(depth * f * 0.4, width * r * 0.95),
                ];
                if let Some(p) = placer.try_place(&candidates, OBELISK_RADIUS, 1.2) {
                    vault.obelisks.push(p);
                }
            }
            // Reading nooks: book stands with stacks and candles at their feet.
            for (f, r) in [(0.25, -0.8), (0.25, 0.8)] {
                if let Some(p) = placer.try_place(
                    &[at(depth * f, width * r), at(depth * f, width * r * 0.9)],
                    0.32,
                    1.4,
                ) {
                    let yaw = horizontal(c - p).x.atan2(horizontal(c - p).z);
                    vault.props.push(Prop {
                        asset: "book_stand",
                        position: p,
                        yaw,
                        height: 1.4,
                        radius: 0.32,
                    });
                    let perp = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
                    let stack = p + perp * 0.55;
                    vault.props.push(Prop {
                        asset: "book_stack",
                        position: stack,
                        yaw: yaw + 0.4,
                        height: 0.22,
                        radius: 0.0,
                    });
                    let candle = p - perp * 0.5;
                    vault.props.push(Prop {
                        asset: "candle_lit",
                        position: candle,
                        yaw,
                        height: 0.45,
                        radius: 0.0,
                    });
                    vault.flames.push(candle + Vec3::Y * 0.47);
                }
            }
            // Potions and vases scattered on the floor by the walls.
            for (asset, height, f, r) in [
                ("potion_tall", 0.3, -0.35, -0.86),
                ("potion_round", 0.22, -0.3, -0.82),
                ("vase_small", 0.5, 0.6, 0.85),
                ("potion_small", 0.16, 0.62, 0.78),
            ] {
                if let Some(p) = placer.try_place(&[at(depth * f, width * r)], 0.12, 1.2) {
                    vault.props.push(Prop {
                        asset,
                        position: p,
                        yaw: rng.range(0.0, 6.28),
                        height,
                        radius: 0.0,
                    });
                }
            }
            // Alchemist shelves on the walls.
            for bay in bays.iter().take(3) {
                vault.mounts.push(Mount {
                    asset: "shelf_bottles",
                    anchor: bay.center + bay.inward * 0.03 + Vec3::Y * 1.25,
                    yaw: bay.inward.x.atan2(bay.inward.z),
                    height: 0.62,
                });
            }
            // Drifting motes fill the vault volume.
            for _ in 0..22 {
                let p = c + Vec3::new(
                    rng.range(-0.75, 0.75) * room.half[0],
                    rng.range(0.4, room.height.min(4.2) - 0.4),
                    rng.range(-0.75, 0.75) * room.half[1],
                );
                vault.sparkles.push(p);
            }
        }
    }
    Some(vault)
}

/// The treasury: a carpeted approach to the chest on a round rug, flanked by
/// candelabra; relic altars along the walls; coin hoards; an overflowing open
/// chest; painted urns; candles everywhere; heraldry, sconces, a chandelier.
#[allow(clippy::too_many_arguments)]
fn plan_treasury(
    map: &Blueprint,
    room: &Room,
    vault: &mut Vault,
    placer: &mut Placer,
    rng: &mut Rng,
    entry: Vec3,
    inward: Vec3,
    right: Vec3,
    depth: f32,
    width: f32,
) {
    let c = room.center;
    let focus = vault.focus;
    let floor = |p: Vec3| Vec3::new(p.x, room.floor_y(p.x, p.z), p.z);
    let at = |f: f32, r: f32| c + inward * f + right * r;
    let architecture = super::architecture::layout(room);

    // Round rug under the chest; shrink it until it fits inside the walls.
    let mut rug_r: f32 = 1.75;
    while rug_r > 1.1
        && (0..12).any(|k| {
            let a = k as f32 * std::f32::consts::TAU / 12.0;
            let p = focus + Vec3::new(a.cos(), 0.0, a.sin()) * (rug_r + 0.1);
            !room.contains(p.x, p.z, 0.1)
        })
    {
        rug_r -= 0.1;
    }
    vault.medallion_radius = rug_r;
    vault.pieces.push(Piece::Rug {
        center: floor(focus),
        radius: rug_r,
    });

    // Red carpet from the main door straight to the rug.
    let from = floor(entry + inward * 0.3);
    let dir = horizontal(focus - from).normalize_or_zero();
    let to = floor(focus - dir * (rug_r - 0.1));
    let carpet_hw = 0.85;
    if horizontal(to - from).length() > 0.8 && horizontal(to - from).dot(dir) > 0.0 {
        vault.pieces.push(Piece::Carpet {
            from,
            to,
            half_width: carpet_hw,
        });
    }
    let side = Vec3::new(dir.z, 0.0, -dir.x);

    // Candelabra flank the rug on the far half.
    for sign in [-1.0f32, 1.0] {
        let candidates = [
            focus + side * (sign * (rug_r + 0.4)) + dir * 0.35,
            focus + side * (sign * (rug_r + 0.45)),
            focus + side * (sign * (rug_r + 0.4)) + dir * 0.8,
        ];
        if let Some(p) = placer.try_place(&candidates, 0.3, rug_r + 0.1) {
            vault.pieces.push(Piece::Candelabrum {
                base: p,
                yaw: yaw_toward(p, focus),
            });
            vault.solids.push((p, 0.3));
            vault.lights.push(VaultLight {
                position: p + Vec3::Y * 1.35,
                color: WARM,
                intensity: 2.4,
                radius: 5.0,
            });
        }
    }
    placer.taken.push((focus, rug_r - 0.2));
    // Coin towers and spilled gold framing the rug on the far side.
    for sign in [-1.0f32, 1.0] {
        let p = focus + side * (sign * (rug_r + 0.25)) - dir * 0.9;
        if let Some(q) = placer.floor_spot(p, 0.35) {
            vault.pieces.push(Piece::CoinScatter {
                center: q,
                radius: 0.4,
                count: 16,
            });
        }
    }

    // Coin hoards: back corners first, then the sides.
    for scale in [1.0f32, 0.75, 0.52, 0.4] {
        for (f, r) in [
            (0.7, -0.62),
            (0.7, 0.62),
            (0.1, -0.72),
            (0.1, 0.72),
            (0.42, -0.7),
            (0.42, 0.7),
            (-0.3, -0.72),
            (-0.3, 0.72),
            (0.82, 0.0),
        ] {
            if vault.hoards.len() >= 4 {
                break;
            }
            let radius = rng.range(1.0, 1.3) * scale;
            let candidates = [
                at(depth * f, width * r),
                at(depth * f * 0.82, width * r * 0.86),
                at(depth * f * 0.92, width * r * 0.7),
            ];
            if let Some(p) = placer.try_place(&candidates, radius, 1.2) {
                vault
                    .hoards
                    .push((p, radius, rng.range(0.72, 0.95) * scale.sqrt()));
            }
        }
        if !vault.hoards.is_empty() {
            break;
        }
    }

    // Relic altars along the walls, facing the room.
    let relics = [
        Relic::Crown,
        Relic::Helm,
        Relic::Armillary,
        Relic::Gems,
        Relic::Chalice,
    ];
    let mut relic = 0;
    for (f, r) in [
        (-0.2, -0.82),
        (-0.2, 0.82),
        (0.5, -0.84),
        (0.5, 0.84),
        (0.86, 0.0),
        (0.86, -0.45),
        (0.86, 0.45),
        (-0.62, -0.55),
        (-0.62, 0.55),
    ] {
        if relic >= 4 {
            break;
        }
        let candidates = [
            at(depth * f, width * r),
            at(depth * f * 0.88, width * r * 0.85),
            at(depth * f * 0.7, width * r * 0.65),
        ];
        if let Some(p) = placer.try_place(&candidates, PEDESTAL_RADIUS, 1.2) {
            vault.pedestals.push(Pedestal {
                base: p,
                relic: relics[relic],
                yaw: yaw_toward(p, c),
            });
            vault.lights.push(VaultLight {
                position: p + Vec3::Y * 2.0,
                color: [1.0, 0.8, 0.5],
                intensity: 1.8,
                radius: 2.8,
            });
            relic += 1;
        }
    }

    // An overflowing open chest against the back wall.
    let chest_candidates = [
        at(depth * 0.8, width * 0.32),
        at(depth * 0.8, -width * 0.32),
        at(depth * 0.78, 0.0),
        at(depth * 0.55, width * 0.8),
        at(depth * 0.55, -width * 0.8),
    ];
    if let Some(p) = placer.try_place(&chest_candidates, 0.55, 1.4) {
        let yaw = yaw_toward(p, c);
        vault.open_chests.push((p, yaw));
        vault.pieces.push(Piece::ChestHoard { base: p, yaw });
        vault.solids.push((p, 0.5));
        vault.lights.push(VaultLight {
            position: p + Vec3::Y * 0.9,
            color: GOLD,
            intensity: 2.0,
            radius: 3.2,
        });
    }

    // A gilded chest and an iron-bound trunk near the entrance corners.
    for (asset, height, radius, f, r) in [
        ("chest_gold_closed", 0.62, 0.55, -0.72, -0.74),
        ("trunk_large", 0.55, 0.55, -0.72, 0.74),
    ] {
        let candidates = [
            at(depth * f, width * r),
            at(depth * f * 0.8, width * r * 0.85),
        ];
        if let Some(p) = placer.try_place(&candidates, radius, 1.2) {
            vault.props.push(Prop {
                asset,
                position: p,
                yaw: yaw_toward(p, c),
                height,
                radius,
            });
        }
    }

    // Painted urns along the walls.
    let mut urns = 0u8;
    for (f, r) in [
        (-0.45, -0.9),
        (-0.45, 0.9),
        (0.2, -0.9),
        (0.2, 0.9),
        (0.62, -0.9),
        (0.62, 0.9),
        (0.9, -0.62),
        (0.9, 0.62),
    ] {
        if urns >= 6 {
            break;
        }
        let height = rng.range(0.72, 1.0);
        let radius = 0.3 * height;
        let candidates = [at(depth * f, width * r), at(depth * f, width * r * 0.9)];
        if let Some(p) = placer.try_place(&candidates, radius, 1.2) {
            vault.pieces.push(Piece::Urn {
                base: p,
                height,
                style: urns % 3,
                yaw: rng.range(0.0, 6.28),
            });
            vault.solids.push((p, radius));
            // Votive candles at the urn's foot, toward the room.
            if urns % 2 == 0 {
                if let Some(q) = placer.floor_spot(
                    p + horizontal(c - p).normalize_or_zero() * (radius + 0.3),
                    0.18,
                ) {
                    vault.pieces.push(Piece::Candles {
                        base: q,
                        count: 3,
                        spread: 0.16,
                    });
                }
            }
            urns += 1;
        }
    }

    // Hoards: glints, glow, spilled coins and candles at their feet.
    for (index, (h, radius, height)) in vault.hoards.clone().into_iter().enumerate() {
        for k in 0..10 {
            let a = k as f32 * 2.39 + rng.range(0.0, 0.6);
            let d = radius * rng.range(0.1, 0.88);
            let y = height * (1.0 - (d / radius).powf(0.9)).max(0.0);
            vault
                .sparkles
                .push(h + Vec3::new(a.cos() * d, y + 0.05, a.sin() * d));
        }
        vault.lights.push(VaultLight {
            position: h + Vec3::Y * (height + 0.6),
            color: GOLD,
            intensity: 1.0 + radius * 0.4,
            radius: 3.6,
        });
        let to_focus = horizontal(focus - h).normalize_or_zero();
        if let Some(q) = placer.floor_spot(h + to_focus * (radius + 0.45), 0.4) {
            vault.pieces.push(Piece::CoinScatter {
                center: q,
                radius: 0.55,
                count: 22,
            });
        }
        let perp =
            Vec3::new(-to_focus.z, 0.0, to_focus.x) * if index % 2 == 0 { 1.0 } else { -1.0 };
        if let Some(q) = placer.floor_spot(h + perp * (radius + 0.3) + to_focus * 0.3, 0.22) {
            vault.pieces.push(Piece::Candles {
                base: q,
                count: 4,
                spread: 0.22,
            });
        }
    }

    // Welcoming candles and loose coins along the carpet.
    let carpet_len = horizontal(to - from).length();
    for (t, sign) in [(0.35f32, -1.0f32), (0.35, 1.0), (0.75, -1.0), (0.75, 1.0)] {
        let p = from + dir * (carpet_len * t) + side * (sign * (carpet_hw + 0.3));
        if let Some(q) = placer.floor_spot(p, 0.2) {
            if t < 0.5 {
                vault.pieces.push(Piece::Candles {
                    base: q,
                    count: 3,
                    spread: 0.18,
                });
            } else {
                vault.pieces.push(Piece::CoinScatter {
                    center: q,
                    radius: 0.35,
                    count: 9,
                });
            }
        }
    }

    // Heraldry and trophies in the blind-arch bays.
    let spring = architecture.spring;
    let mut banners = 0;
    let mut trophies = 0;
    for (k, bay) in bays_by_distance(&architecture.bays, c)
        .into_iter()
        .enumerate()
    {
        if bay.width < 1.3 {
            continue;
        }
        let top = bay.center + Vec3::Y * (spring - 0.22);
        if k % 3 == 2 && trophies < 2 {
            vault.mounts.push(Mount {
                asset: "gold_relic",
                anchor: bay.center + bay.inward * 0.04 + Vec3::Y * 1.5,
                yaw: bay.inward.x.atan2(bay.inward.z),
                height: 0.95,
            });
            trophies += 1;
        } else if banners < 6 {
            let length = (spring - 1.35).clamp(1.2, 2.6);
            vault.pieces.push(Piece::Banner {
                top,
                inward: bay.inward,
                tangent: bay.tangent,
                width: (bay.width * 0.55).clamp(0.7, 1.0),
                length,
                sun: false,
            });
            banners += 1;
        }
    }

    // Torches on every other pilaster.
    for (k, pilaster) in architecture.pilasters.iter().enumerate() {
        if k % 2 == 1 || vault.sconces.len() >= 6 {
            continue;
        }
        let backplate = pilaster.base
            + pilaster.inward * (super::architecture::PROJECTION - 0.01)
            + Vec3::Y * 1.62;
        let flame = backplate + pilaster.inward * 0.1 + Vec3::Y * 0.63;
        vault.sconces.push(Sconce {
            backplate,
            yaw: pilaster.inward.x.atan2(pilaster.inward.z),
            flame,
        });
        if vault.sconces.len() <= 4 {
            vault.lights.push(VaultLight {
                position: flame,
                color: [1.0, 0.56, 0.2],
                intensity: 5.5,
                radius: 6.2,
            });
        }
    }

    // Red-and-gold chandelier over the room centre.
    let ring_y = floor(c).y + (room.height - 1.3).clamp(3.3, 4.8);
    vault.pieces.push(Piece::Chandelier {
        ceiling: Vec3::new(c.x, floor(c).y + room.height, c.z),
        ring_y,
        radius: 0.85,
    });
    vault.lights.push(VaultLight {
        position: Vec3::new(c.x, ring_y + 0.1, c.z),
        color: [1.0, 0.7, 0.36],
        intensity: 5.0,
        radius: 9.5,
    });
    vault.lights.push(VaultLight {
        position: focus + Vec3::Y * 1.1,
        color: GOLD,
        intensity: 1.3,
        radius: 3.6,
    });
    let _ = map;
}

/// The sanctuary: a stepped shrine against a doorless wall with the guardian
/// statue, the rest font on a prayer rug in front of it, flanking crystal
/// braziers, a red runner from the entrance, heraldry with the sun sigil,
/// candles on every step, ferns and clay pots, sun reliefs over the doors.
fn plan_sanctuary(
    map: &Blueprint,
    room: &Room,
    vault: &mut Vault,
    placer: &mut Placer,
    rng: &mut Rng,
) {
    let c = room.center;
    let focus = vault.focus;
    let floor = |p: Vec3| Vec3::new(p.x, room.floor_y(p.x, p.z), p.z);
    let architecture = super::architecture::layout(room);
    let shrine = velvet_dungeon::gameplay::shrine_wall(map, room).filter(|(wall, inward, _)| {
        horizontal(*wall + *inward * velvet_dungeon::gameplay::SHRINE_FONT_DEPTH - focus).length()
            < 0.05
    });
    // Axis of the composition: toward the shrine, or away from the entry.
    let inward = shrine.map(|(_, inward, _)| inward).unwrap_or(vault.inward);
    let side = Vec3::new(inward.z, 0.0, -inward.x);

    if let Some((wall, _, half_len)) = shrine {
        let wall = floor(wall);
        let width = (half_len * 2.0 - 1.4).clamp(2.2, 3.4);
        vault.pieces.push(Piece::Shrine {
            wall,
            inward,
            width,
        });
        for k in [-1.0f32, 0.0, 1.0] {
            let p = wall + inward * 0.8 + side * (k * width * 0.33);
            vault.solids.push((p, 0.72));
            placer.taken.push((p, 0.72));
        }
        // Candles along every tread edge; the top step frames the statue.
        for (k, &(depth, inset, top)) in super::sanctuary::DAIS.iter().enumerate() {
            let hw = (width - inset) * 0.5 - 0.12;
            let y = Vec3::Y * top;
            let edge = wall + inward * (depth - 0.12) + y;
            vault.pieces.push(Piece::CandleRow {
                from: edge - side * hw,
                to: edge + side * hw,
                count: (hw * 2.0 / 0.17) as u8,
                gap: true,
            });
            if k == 2 {
                let back = wall + inward * 0.22 + y;
                vault.pieces.push(Piece::CandleRow {
                    from: back - side * hw,
                    to: back + side * hw,
                    count: (hw * 2.0 / 0.2) as u8,
                    gap: true,
                });
            }
        }
        for sign in [-1.0f32, 1.0] {
            let p = wall + inward * 1.85 + side * (sign * (width * 0.5 + 0.15));
            if let Some(q) = placer.floor_spot(p, 0.25) {
                vault.pieces.push(Piece::Candles {
                    base: q,
                    count: 5,
                    spread: 0.24,
                });
            }
            let fern = wall + inward * 1.2 + side * (sign * (width * 0.5 + 0.35));
            if let Some(q) = placer.floor_spot(fern, 0.3) {
                vault.pieces.push(Piece::Fern {
                    base: q,
                    scale: 1.2,
                });
            }
        }
        // Uplight on the statue and warm glow over the candle steps.
        vault.lights.push(VaultLight {
            position: wall + inward * 2.3 + Vec3::Y * 0.35,
            color: [1.0, 0.7, 0.4],
            intensity: 2.1,
            radius: 5.0,
        });
        vault.lights.push(VaultLight {
            position: wall + inward * 1.25 + Vec3::Y * 0.7,
            color: [1.0, 0.58, 0.22],
            intensity: 2.0,
            radius: 3.8,
        });
        // Tall banners flanking the shrine.
        for sign in [-1.0f32, 1.0] {
            let x = width * 0.5 + 0.75;
            if x + 0.5 < half_len {
                vault.pieces.push(Piece::Banner {
                    top: wall + side * (sign * x) + Vec3::Y * (architecture.spring - 0.15),
                    inward,
                    tangent: side,
                    width: 0.7,
                    length: (architecture.spring - 1.25).clamp(1.3, 2.3),
                    sun: true,
                });
            }
        }
    }

    // The rest font on its prayer rug.
    vault.pieces.push(Piece::PrayerRug {
        center: floor(focus),
        along: inward,
        half: [1.05, 1.45],
    });
    // A low offering font keeps the statue in view from the entrance.
    vault.pieces.push(Piece::CrystalBrazier {
        base: floor(focus),
        scale: 0.82,
    });
    vault.solids.push((focus, 0.42));
    placer.taken.push((focus, 1.2));
    vault.glow_lights.push((focus + Vec3::Y * 1.2, 5.0, 6.5));
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0;
        vault
            .sparkles
            .push(focus + Vec3::new(a.cos() * 0.15, 1.0, a.sin() * 0.15));
    }

    // Red runner from the main door to the rug when they are aligned.
    let entry = room.sockets[0].position;
    let dir = horizontal(focus - entry).normalize_or_zero();
    if dir.dot(-inward) > 0.95 {
        let from = floor(entry + dir * 0.3);
        let to = floor(focus - dir * 1.22);
        if horizontal(to - from).dot(dir) > 1.0 {
            vault.pieces.push(Piece::Carpet {
                from,
                to,
                half_width: 0.8,
            });
        }
    }

    // Crystal braziers: a pair flanking the font, a pair in the nave.
    let mut braziers = Vec::new();
    for sign in [-1.0f32, 1.0] {
        let candidates = [
            focus + side * (sign * 2.0),
            focus + side * (sign * 2.3) - inward * 0.5,
            focus + side * (sign * 1.8) - inward * 1.0,
            // Crossroads: sit between the converging door paths.
            focus + (side * sign + inward).normalize_or_zero() * 1.9,
            focus + (side * sign - inward).normalize_or_zero() * 1.9,
            focus + (side * sign + inward).normalize_or_zero() * 2.5,
            focus + (side * sign - inward).normalize_or_zero() * 2.5,
        ];
        if let Some(p) = placer.try_place(&candidates, 0.34, 1.6) {
            braziers.push(p);
        }
    }
    let (depth, width) = {
        let d = inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1];
        let w = side.x.abs() * room.half[0] + side.z.abs() * room.half[1];
        (d, w)
    };
    for sign in [-1.0f32, 1.0] {
        let candidates = [
            c - inward * (depth * 0.35) + side * (sign * width * 0.62),
            c - inward * (depth * 0.1) + side * (sign * width * 0.66),
            c - inward * (depth * 0.55) + side * (sign * width * 0.55),
        ];
        if let Some(p) = placer.try_place(&candidates, 0.34, 1.6) {
            braziers.push(p);
        }
    }
    // Rooms crossed by many doors: fill the quadrants between their paths.
    for (a, b) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        if braziers.len() >= 4 {
            break;
        }
        let candidates = [
            c + inward * (a * depth * 0.45) + side * (b * width * 0.45),
            c + inward * (a * depth * 0.84) + side * (b * width * 0.84),
            c + inward * (a * depth * 0.3) + side * (b * width * 0.62),
            c + inward * (a * depth * 0.62) + side * (b * width * 0.3),
        ];
        if let Some(p) = placer.try_place(&candidates, 0.34, 1.6) {
            braziers.push(p);
        }
    }
    for p in &braziers {
        vault.pieces.push(Piece::CrystalBrazier {
            base: *p,
            scale: 1.0,
        });
        vault.solids.push((*p, 0.34));
        vault.glow_lights.push((*p + Vec3::Y * 1.2, 3.6, 5.0));
        for k in 0..3 {
            let a = k as f32 * 2.1;
            vault
                .sparkles
                .push(*p + Vec3::new(a.cos() * 0.1, 1.0, a.sin() * 0.1));
        }
        // A fern and a clay pot at each brazier's foot.
        let out = horizontal(*p - c).normalize_or_zero();
        if let Some(q) = placer.floor_spot(*p + out * 0.5 + side * 0.25, 0.28) {
            vault.pieces.push(Piece::Fern {
                base: q,
                scale: rng.range(0.85, 1.15),
            });
        }
        if let Some(q) = placer.floor_spot(*p - out * 0.1 + side * -0.55, 0.2) {
            vault.pieces.push(Piece::Pot {
                base: q,
                height: rng.range(0.4, 0.6),
            });
        }
        if let Some(q) = placer.floor_spot(*p + inward * 0.55, 0.2) {
            vault.pieces.push(Piece::Candles {
                base: q,
                count: 3,
                spread: 0.16,
            });
        }
    }

    // Sun-sigil banners in the wall bays; sun reliefs over every doorway.
    let mut banners = 0;
    for bay in bays_by_distance(&architecture.bays, focus) {
        if banners >= 6 || bay.width < 1.3 {
            continue;
        }
        vault.pieces.push(Piece::Banner {
            top: bay.center + Vec3::Y * (architecture.spring - 0.22),
            inward: bay.inward,
            tangent: bay.tangent,
            width: (bay.width * 0.5).clamp(0.7, 0.95),
            length: (architecture.spring - 1.35).clamp(1.2, 2.4),
            sun: true,
        });
        banners += 1;
    }
    for socket in &room.sockets {
        let into = -horizontal(socket.outward).normalize_or_zero();
        let y = socket.height + 0.62;
        if y + 0.45 < room.height {
            vault.pieces.push(Piece::SunRelief {
                center: floor(socket.position) + into * 0.03 + Vec3::Y * y,
                normal: into,
                radius: 0.4,
            });
        }
    }

    // Torches on every other pilaster.
    for (k, pilaster) in architecture.pilasters.iter().enumerate() {
        if k % 2 == 1 || vault.sconces.len() >= 6 {
            continue;
        }
        let backplate = pilaster.base
            + pilaster.inward * (super::architecture::PROJECTION - 0.01)
            + Vec3::Y * 1.62;
        let flame = backplate + pilaster.inward * 0.1 + Vec3::Y * 0.63;
        vault.sconces.push(Sconce {
            backplate,
            yaw: pilaster.inward.x.atan2(pilaster.inward.z),
            flame,
        });
        if vault.sconces.len() <= 3 {
            vault.lights.push(VaultLight {
                position: flame,
                color: [1.0, 0.56, 0.2],
                intensity: 4.5,
                radius: 6.0,
            });
        }
    }
    // Ferns and candles at the foot of the cover pillars.
    for o in map.obstacles.iter().filter(|o| o.room == room.id) {
        let toward = horizontal(c - o.center).normalize_or_zero();
        let across = Vec3::new(toward.z, 0.0, -toward.x);
        if let Some(q) = placer.floor_spot(o.center + toward * 1.05 + across * 0.3, 0.3) {
            vault.pieces.push(Piece::Fern {
                base: q,
                scale: rng.range(0.8, 1.05),
            });
        }
        if let Some(q) = placer.floor_spot(o.center + toward * 1.0 - across * 0.45, 0.2) {
            vault.pieces.push(Piece::Candles {
                base: q,
                count: 4,
                spread: 0.2,
            });
        }
    }
    // Scattered ferns along the walls soften the stone.
    for (f, r) in [(-0.7, -0.85), (-0.7, 0.85), (0.1, -0.88), (0.1, 0.88)] {
        let p = c + inward * (depth * f) + side * (width * r);
        if let Some(q) = placer.floor_spot(p, 0.3) {
            vault.pieces.push(Piece::Fern {
                base: q,
                scale: rng.range(0.8, 1.1),
            });
        }
    }
}

fn bays_by_distance(
    bays: &[super::architecture::Bay],
    center: Vec3,
) -> Vec<super::architecture::Bay> {
    let mut sorted = bays.to_vec();
    sorted.sort_by(|a, b| {
        horizontal(a.center - center)
            .length()
            .total_cmp(&horizontal(b.center - center).length())
    });
    sorted
}

/// Solid footprints of the plan for player collision.
pub fn colliders(vault: &Vault) -> Vec<Prop> {
    let mut out: Vec<Prop> = vault
        .props
        .iter()
        .filter(|p| p.radius > 0.0)
        .copied()
        .collect();
    let solid = |position: Vec3, radius: f32| Prop {
        asset: "",
        position,
        yaw: 0.0,
        height: 0.0,
        radius,
    };
    for (p, r, _) in &vault.hoards {
        out.push(solid(*p, *r * 0.9));
    }
    for p in &vault.pedestals {
        out.push(solid(p.base, PEDESTAL_RADIUS));
    }
    for p in &vault.obelisks {
        out.push(solid(*p, OBELISK_RADIUS));
    }
    for (p, r) in &vault.solids {
        out.push(solid(*p, *r));
    }
    out
}

/// Half-width of the obelisk shaft at height `y` above its base.
fn obelisk_half(y: f32) -> f32 {
    0.22 - ((y - 0.3) / 1.9).clamp(0.0, 1.0) * 0.08
}

/// Static detail + glow meshes of one vault, cached by the scene.
pub fn meshes(
    vault: &Vault,
    biome: &velvet_dungeon::Biome,
    seed: u64,
    room_id: usize,
) -> (Mesh3D, Mesh3D) {
    let stone = super::surfaces::style(biome).stone;
    let glow = super::surfaces::mood(biome).glow;
    match vault.kind {
        Kind::Treasury | Kind::Sanctuary => {
            let built = super::treasury::build(vault, stone, glow, seed, room_id);
            (built.detail, built.glow)
        }
        Kind::Arcane => (super::visual::empty(), glow_mesh(vault)),
    }
}

/// Emissive floor medallion (gold inlay) or rune circle (arcane), plus the
/// obelisk rune bands. Drawn with emission, rebuilt only when the seed changes.
pub fn glow_mesh(vault: &Vault) -> Mesh3D {
    let mut m = super::visual::empty();
    let color = match vault.kind {
        Kind::Treasury | Kind::Sanctuary => GOLD,
        Kind::Arcane => ARCANE,
    };
    let c = vault.focus + Vec3::Y * 0.018;
    let ring = |m: &mut Mesh3D, r0: f32, r1: f32, segments: usize, shade: f32| {
        for i in 0..segments {
            let a0 = i as f32 * std::f32::consts::TAU / segments as f32;
            let a1 = (i + 1) as f32 * std::f32::consts::TAU / segments as f32;
            let p = |a: f32, r: f32| c + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            quad(
                m,
                [p(a0, r1), p(a1, r1), p(a1, r0), p(a0, r0)],
                Vec3::Y,
                tint(color, shade),
            );
        }
    };
    let r = vault.medallion_radius;
    ring(&mut m, r - 0.07, r, 48, 1.0);
    ring(&mut m, r * 0.72 - 0.035, r * 0.72, 40, 0.85);
    // Radiating glyph strokes between the rings.
    let strokes = if vault.kind == Kind::Arcane { 12 } else { 8 };
    for i in 0..strokes {
        let a = i as f32 * std::f32::consts::TAU / strokes as f32;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let perp = Vec3::new(-dir.z, 0.0, dir.x);
        let inner = c + dir * (r * 0.76);
        let outer = c + dir * (r - 0.12);
        let w = if vault.kind == Kind::Arcane {
            0.03
        } else {
            0.022
        };
        quad(
            &mut m,
            [
                inner - perp * w,
                outer - perp * w,
                outer + perp * w,
                inner + perp * w,
            ],
            Vec3::Y,
            tint(color, 0.9),
        );
        if vault.kind == Kind::Arcane {
            // Small rune diamonds on the outer band.
            let q = c + Vec3::new((a + 0.26).cos(), 0.0, (a + 0.26).sin()) * (r * 0.86);
            let s = 0.07;
            quad(
                &mut m,
                [q - dir * s, q + perp * s, q + dir * s, q - perp * s],
                Vec3::Y,
                tint(color, 1.1),
            );
        }
    }
    for p in &vault.obelisks {
        // A column of glowing glyph strokes down every face.
        for face in 0..4 {
            let a = face as f32 * std::f32::consts::FRAC_PI_2;
            let n = Vec3::new(a.cos(), 0.0, a.sin());
            let t = Vec3::new(-n.z, 0.0, n.x);
            for k in 0..9 {
                let y = 0.5 + k as f32 * 0.19;
                let o = *p + n * (obelisk_half(y) + 0.012) + Vec3::Y * y;
                let w = [0.07, 0.035, 0.06, 0.03, 0.075, 0.045, 0.065, 0.03, 0.05][(k + face) % 9];
                let h = if k % 3 == 1 { 0.1 } else { 0.028 };
                quad(
                    &mut m,
                    [
                        o - t * w,
                        o + t * w,
                        o + t * w + Vec3::Y * h,
                        o - t * w + Vec3::Y * h,
                    ],
                    n,
                    tint(color, 0.9),
                );
            }
            let band = *p + n * (obelisk_half(2.17) + 0.03) + Vec3::Y * 2.14;
            quad(
                &mut m,
                [
                    band - t * 0.16,
                    band + t * 0.16,
                    band + t * 0.16 + Vec3::Y * 0.02,
                    band - t * 0.16 + Vec3::Y * 0.02,
                ],
                n,
                color,
            );
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config, PLAYER_RADIUS};

    #[test]
    fn vaults_are_furnished_deterministic_and_keep_the_chest_reachable() {
        let catalog = Catalog::bundled();
        let (mut treasuries, mut arcane, mut sanctuaries, mut shrines) = (0, 0, 0, 0);
        for seed in 0..64 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in &map.rooms {
                let Some(vault) = plan(&map, room) else {
                    continue;
                };
                assert_eq!(Some(vault.clone()), plan(&map, room));
                match vault.kind {
                    Kind::Treasury => {
                        treasuries += 1;
                        assert!(
                            !vault.hoards.is_empty(),
                            "seed {seed} room {} has no gold",
                            room.id
                        );
                        assert!(!vault.pedestals.is_empty());
                        assert!(vault.pieces.iter().any(|p| matches!(p, Piece::Rug { .. })));
                        assert!(vault
                            .pieces
                            .iter()
                            .any(|p| matches!(p, Piece::Chandelier { .. })));
                        // The chandelier hangs above jump headroom.
                        for piece in &vault.pieces {
                            if let Piece::Chandelier { ring_y, .. } = piece {
                                assert!(ring_y - room.center.y >= 3.25);
                                assert!(ring_y + 0.8 <= room.center.y + room.height);
                            }
                        }
                    }
                    Kind::Arcane => {
                        arcane += 1;
                        assert!(vault.obelisks.len() >= 2);
                    }
                    Kind::Sanctuary => {
                        sanctuaries += 1;
                        shrines += usize::from(
                            vault
                                .pieces
                                .iter()
                                .any(|p| matches!(p, Piece::Shrine { .. })),
                        );
                        assert!(vault
                            .pieces
                            .iter()
                            .any(|p| matches!(p, Piece::CrystalBrazier { .. })));
                    }
                }
                let solids = colliders(&vault);
                // Every door's walk to the chest stays open.
                for socket in &room.sockets {
                    let start = socket.position - socket.outward * 0.6;
                    for i in 0..=20 {
                        let p = start.lerp(vault.focus, i as f32 / 20.0);
                        for s in &solids {
                            // The sanctuary's rest font is the destination itself.
                            if horizontal(s.position - vault.focus).length() < 0.01 {
                                continue;
                            }
                            assert!(
                                horizontal(s.position - p).length()
                                    > s.radius + PLAYER_RADIUS - 0.05,
                                "seed {seed} room {}: {} blocks the chest approach",
                                room.id,
                                s.asset
                            );
                        }
                    }
                }
                for s in &solids {
                    assert!(room.contains(s.position.x, s.position.z, s.radius));
                }
                let (detail, glow) = meshes(&vault, &catalog.biomes[room.biome], map.seed, room.id);
                assert!(!glow.indices.is_empty());
                assert!(detail.indices.len() % 3 == 0 && glow.indices.len() % 3 == 0);
                assert!(detail.vertices.iter().chain(glow.vertices.iter()).all(|v| v
                    .position
                    .iter()
                    .chain(v.normal.iter())
                    .all(|x| x.is_finite())));
                if vault.kind != Kind::Arcane {
                    // Rich but bounded: the treasury stays a two-draw budget.
                    let triangles = detail.indices.len() / 3;
                    let minimum = if vault.kind == Kind::Sanctuary {
                        3_000
                    } else {
                        8_000
                    };
                    assert!(
                        triangles > minimum,
                        "seed {seed} room {}: sparse treasury ({triangles})",
                        room.id
                    );
                    assert!(
                        triangles < 140_000,
                        "seed {seed} room {}: heavy treasury ({triangles})",
                        room.id
                    );
                }
            }
        }
        assert!(treasuries > 20 && arcane > 20, "{treasuries} / {arcane}");
        assert!(
            sanctuaries > 40 && shrines * 10 >= sanctuaries * 8,
            "{shrines} shrines / {sanctuaries} sanctuaries"
        );
    }
}
