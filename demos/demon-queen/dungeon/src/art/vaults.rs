//! Signature chambers (web prototype `vaults.js`): the loot hall and grand
//! treasury (treasure), the arcane library (secret) and the sanctuary
//! (rest). They fill the same plan the themed rooms do, plus hoards, niche
//! relics, obelisks, sconces, open chests, glow lights and a light shaft.
use super::super::architecture::{layout, Bay, PROJECTION};
use super::arcane::{self, Niche, ARCANE_DAIS, ARCANE_DAIS_TOP};
use super::kit::*;
use super::plan::*;
use super::sanctuary::{self as sanct, DAIS};
use super::stage::{mm, yaw_toward};
use super::themekit;
use super::treasury as tr;
use std::f32::consts::PI;
use velvet_dungeon::gameplay::{shrine_wall, SpawnKind, SHRINE_FONT_DEPTH};
use velvet_dungeon::{Blueprint, Role, Room, Shape, Treasury};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

pub const OBELISK_RADIUS: f32 = 0.34;
pub const GOLD: C3 = [1.0, 0.66, 0.16];
pub const ARCANE: C3 = [0.62, 0.32, 1.0];
const WARM: C3 = [1.0, 0.62, 0.28];

/// Treasury piers: massive engaged pillars of stacked chunky blocks.
pub const PIER_HALF: f32 = 0.56;
pub const PIER_DEPTH: f32 = 0.5;
pub const PIER_RADIUS: f32 = 0.56;
pub const NICHE_DEPTH: f32 = 0.78;

pub fn is_treasury(room: &Room) -> bool {
    room.role == Role::Treasure && room.shape != Shape::Bridge
}
pub fn is_sanctuary_hall(room: &Room) -> bool {
    room.role == Role::Rest && room.shape != Shape::Bridge
}

/// Solid footprints of the treasury and sanctuary piers.
pub fn pier_solids(room: &Room) -> Vec<(Vec3, f32)> {
    if !is_treasury(room) && !is_sanctuary_hall(room) {
        return vec![];
    }
    layout(room)
        .pilasters
        .iter()
        .map(|p| (p.base + p.inward * 0.22, PIER_RADIUS))
        .collect()
}

/// Arched niches in the wall bays (web `grandNiches`).
pub fn grand_niches(room: &Room) -> Vec<Niche> {
    let plan = layout(room);
    let spring = plan.spring - 0.25;
    plan.bays
        .iter()
        .map(|bay| (bay, (bay.width * 0.5 - PIER_HALF - 0.14).min(1.0)))
        .filter(|&(_, half)| half >= 0.55 && spring + half < room.height - 0.5)
        .map(|(bay, half)| Niche {
            center: bay.center,
            inward: bay.inward,
            tangent: bay.tangent,
            width: bay.width,
            half,
            spring,
            top: spring + half,
            depth: NICHE_DEPTH,
        })
        .collect()
}

fn bay_order(bays: &[Bay], centre: Vec3) -> Vec<Bay> {
    let mut out = bays.to_vec();
    // Millimetre keys: mirror-image bays tie exactly (web `byKey`).
    out.sort_by_key(|b| mm(hz(b.center - centre).length()));
    out
}

fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let d = hz(b - a);
    let t = if d.length_squared() > 1e-4 {
        (hz(p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    hz(p - (a + d * t)).length()
}

struct V<'a> {
    map: &'a Blueprint,
    room: &'a Room,
    plan: &'a mut Plan,
    placer: Placer<'a>,
    rng: ArtRng,
    entry: Vec3,
    inward: Vec3,
    right: Vec3,
    depth: f32,
    width: f32,
}

impl V<'_> {
    fn c(&self) -> Vec3 {
        self.room.center
    }
    fn floor(&self, p: Vec3) -> Vec3 {
        with_y(p, self.room.floor_y(p.x, p.z))
    }
    fn at(&self, f: f32, r: f32) -> Vec3 {
        self.c() + self.inward * f + self.right * r
    }
    fn place(&mut self, cands: &[Vec3], r: f32, gap: f32, obstacle_gap: f32) -> Option<Vec3> {
        self.placer.try_place(cands, r, gap, obstacle_gap)
    }
    fn spot(&self, p: Vec3, r: f32) -> Option<Vec3> {
        self.placer.floor_spot(p, r)
    }
    fn light(&mut self, p: Vec3, color: C3, intensity: f32, radius: f32) {
        self.plan.light(p, color, intensity, radius);
    }
    fn piece(
        &mut self,
        build: impl Fn(&mut Mesh3D, &mut Mesh3D, &mut Ctx) + Send + Sync + 'static,
    ) {
        self.plan.piece(build);
    }
    fn candles(&mut self, base: Vec3, count: usize, spread: f32) {
        self.piece(move |m, g, ctx| themekit::candles(m, g, base, count, spread, &mut ctx.rng));
    }
    fn coins(&mut self, center: Vec3, radius: f32, count: usize) {
        self.piece(move |m, g, ctx| tr::coin_scatter(m, g, center, radius, count, &mut ctx.rng));
    }
    fn fern(&mut self, base: Vec3, scale: f32) {
        self.piece(move |m, _g, ctx| themekit::fern(m, base, scale, &mut ctx.rng));
    }
    fn piers(&mut self) {
        for (p, r) in pier_solids(self.room) {
            self.plan.solids.push((p, r));
            self.placer.taken.push((p, r));
        }
    }
}

/// Plan a signature chamber (treasure, secret or rest room), or `None`.
pub fn plan(map: &Blueprint, room: &Room) -> Option<Plan> {
    let kind = match room.role {
        Role::Treasure => VaultKind::Treasury,
        Role::Secret => VaultKind::Arcane,
        Role::Rest => VaultKind::Sanctuary,
        _ => return None,
    };
    if room.shape == Shape::Bridge || room.sockets.is_empty() {
        return None;
    }
    let wanted = if kind == VaultKind::Sanctuary {
        SpawnKind::Rest
    } else {
        SpawnKind::Chest
    };
    let focus = map
        .spawns
        .iter()
        .find(|s| s.room == room.id && s.kind == wanted)?
        .position;
    let (inward, right, depth, width) = frame(room);
    let entry = room.sockets[0].position;
    let mut plan = Plan::new(focus, inward);
    plan.vault = Some(kind);
    {
        let mut s = V {
            map,
            room,
            plan: &mut plan,
            placer: Placer::new(map, room, focus),
            rng: ArtRng::stream(map.seed, "vault_v1", room.id as u64),
            entry,
            inward,
            right,
            depth,
            width,
        };
        match kind {
            VaultKind::Sanctuary => sanctuary(&mut s),
            VaultKind::Treasury if room.treasury == Some(Treasury::Grand) => grand_treasury(&mut s),
            VaultKind::Treasury => treasury(&mut s),
            VaultKind::Arcane => arcane_hall(&mut s),
        }
    }
    // Motes: gold glints in the treasuries, violet in the arcane hall (the
    // sanctuary's follow its glow colour, set by the scene).
    plan.sparkle_tint = match kind {
        VaultKind::Treasury => Some([1.75, 1.4, 0.7]),
        VaultKind::Arcane => Some([0.7, 0.24, 1.4]),
        VaultKind::Sanctuary => None,
    };
    Some(plan)
}

fn sconces_on_pilasters(s: &mut V, lit: usize, intensity: f32, radius: f32, depth: f32) {
    for (k, pilaster) in layout(s.room).pilasters.iter().enumerate() {
        if k % 2 == 1 || s.plan.sconces.len() >= 6 {
            continue;
        }
        let backplate = pilaster.base + pilaster.inward * (depth - 0.01) + Y(1.62);
        let flame = backplate + pilaster.inward * 0.1 + Y(0.63);
        s.plan.sconces.push(Sconce {
            backplate,
            yaw: pilaster.inward.x.atan2(pilaster.inward.z),
            flame,
        });
        if s.plan.sconces.len() <= lit {
            s.light(flame, [1.0, 0.56, 0.2], intensity, radius);
        }
    }
}

/// The treasury as a plundered loot hall.
fn treasury(s: &mut V) {
    let c = s.c();
    let focus = s.plan.focus;
    let (depth, width) = (s.depth, s.width);
    let architecture = layout(s.room);
    let fire: C3 = [1.0, 0.42, 0.1];
    s.piers();

    // Relic chest on a spill of gold; its interaction pocket stays clear.
    s.placer.taken.push((focus, 1.0));
    let to_entry = norm(hz(s.entry - focus));
    let centre = s.floor(focus + to_entry * 0.35);
    let yaw = yaw_toward(focus, s.entry);
    s.piece(move |m, g, ctx| {
        tr::loot_pile(
            m,
            g,
            centre,
            1.35,
            "none",
            yaw,
            &mut ctx.rng,
            Some((focus, 0.95)),
        )
    });
    s.light(focus + Y(1.2), GOLD, 2.2, 4.5);

    // Iron braziers flanking the hall.
    for sign in [-1.0f32, 1.0] {
        let cands = [
            s.at(-depth * 0.42, sign * width * 0.64),
            s.at(-depth * 0.15, sign * width * 0.7),
            s.at(depth * 0.2, sign * width * 0.7),
            s.at(depth * 0.48, sign * width * 0.64),
        ];
        let Some(p) = s.place(&cands, 0.46, 1.3, 0.85) else {
            continue;
        };
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, g, ctx| tr::iron_brazier(m, g, p, yaw, &mut ctx.rng));
        s.plan.solids.push((p, 0.46));
        s.plan.fixtures.push(Fixture {
            p: p + Y(1.12),
            color: fire,
            size: 3.4,
        });
        for k in 0..3 {
            let a = yaw + k as f32 * TAU / 3.0;
            s.plan.fixtures.push(Fixture {
                p: p + v(a.cos() * 0.2, 1.08, a.sin() * 0.2),
                color: fire,
                size: 2.2,
            });
        }
        s.light(p + Y(1.75), [1.0, 0.6, 0.3], 7.0, 9.0);
        for k in 0..4 {
            s.plan.sparkles.push(
                p + v(
                    (k as f32 * 1.6).cos() * 0.15,
                    1.4 + k as f32 * 0.1,
                    (k as f32 * 1.6).sin() * 0.15,
                ),
            );
        }
    }

    // Amber crystals burst from rubble in the chamfered corners.
    let n = s.room.outline.len();
    let mut crystals = 0;
    for i in (1..n).step_by(2) {
        let a = s.room.outline[i];
        let b = s.room.outline[(i + 1) % n];
        let corner = v(c.x + (a[0] + b[0]) * 0.5, c.y, c.z + (a[1] + b[1]) * 0.5);
        let into = norm(hz(c - corner));
        let cands = [
            corner + into * 1.0,
            corner + into * 1.35,
            corner + into * 1.75,
            corner + into * 2.1,
        ];
        let Some(p) = s.place(&cands, 0.62, 1.2, 0.45) else {
            continue;
        };
        let scale = s.rng.range(1.1, 1.4);
        let yaw = yaw_toward(p, c);
        s.piece(move |m, g, ctx| tr::amber_crystals(m, g, p, scale, yaw, ctx.stone, &mut ctx.rng));
        s.plan.solids.push((p, 0.6 * scale));
        if crystals < 3 {
            s.plan.glow_lights.push((p + Y(0.9), 4.2, 5.5));
        }
        for k in 0..3 {
            s.plan.sparkles.push(
                p + v(
                    (k as f32 * 2.1).cos() * 0.25,
                    0.7 + k as f32 * 0.25,
                    (k as f32 * 2.1).sin() * 0.25,
                ),
            );
        }
        crystals += 1;
    }

    // Loot heaps on the flagstones, clear of every door-to-chest walk.
    let mut features = ["coffer", "toppled", "weapons", "helm"];
    s.rng.shuffle(&mut features);
    let mut piles = 0;
    let mut pile_centres: Vec<Vec3> = vec![];
    for (f, r) in [
        (0.3, -0.42),
        (0.3, 0.42),
        (-0.12, -0.45),
        (-0.12, 0.45),
        (0.58, -0.36),
        (0.58, 0.36),
        (-0.4, -0.4),
        (-0.4, 0.4),
    ] {
        if piles >= 4 {
            break;
        }
        let cands = [
            s.at(depth * f, width * r),
            s.at(depth * f * 0.9, width * r * 1.12),
            s.at(depth * f * 1.1, width * r * 0.86),
        ];
        let Some(p) = s.place(&cands, 0.8, 1.5, 0.85) else {
            continue;
        };
        let feature = features[piles % features.len()];
        let yaw = yaw_toward(p, c) + s.rng.range(-0.6, 0.6);
        let radius = s.rng.range(0.85, 1.1);
        s.piece(move |m, g, ctx| tr::loot_pile(m, g, p, radius, feature, yaw, &mut ctx.rng, None));
        pile_centres.push(p);
        if feature == "toppled" || feature == "coffer" {
            s.plan.solids.push((p, 0.44));
        } else if feature == "weapons" {
            let side = v(yaw.cos(), 0.0, -yaw.sin());
            let position = s.floor(p + side * 0.55);
            s.plan.weapons.push(Weapon {
                name: "shield",
                position,
                height: 0.72,
                rotation: v(-0.32, yaw, 0.0),
            });
        }
        if piles < 2 {
            s.light(p + Y(0.8), GOLD, 1.3, 4.0);
        }
        for k in 0..7 {
            let a = k as f32 * 2.39 + s.rng.range(0.0, 0.6);
            let d = s.rng.range(0.1, 0.8);
            let y = 0.06 + s.rng.range(0.0, 0.25);
            s.plan.sparkles.push(p + v(a.cos() * d, y, a.sin() * d));
        }
        piles += 1;
    }
    // Gold bars, coin sacks and the bones of a luckless thief. The chest's
    // own spill (radius 1.35) counts among the heaps, first.
    let heaps: Vec<Vec3> = std::iter::once(centre)
        .chain(pile_centres.iter().copied())
        .collect();
    for (k, &p) in heaps.iter().take(2).enumerate() {
        let a = k as f32 * 2.4 + 0.6;
        if let Some(q) = s.spot(p + v(a.cos(), 0.0, a.sin()) * 1.15, 0.22) {
            let yaw = s.rng.range(0.0, TAU);
            s.piece(move |m, _g, ctx| themekit::ingot_stack(m, q, yaw, &mut ctx.rng));
        }
    }
    for &p in heaps.iter().skip(1) {
        let out = norm(hz(p - c));
        let q = [1.1, 1.4, 0.9]
            .iter()
            .find_map(|&d| s.spot(p + out * d, 0.4));
        if let Some(q) = q {
            let yaw = yaw_toward(q, c);
            s.piece(move |m, _g, ctx| themekit::remains(m, q, yaw, &mut ctx.rng));
            break;
        }
    }
    let mut sacks = 0;
    for (f, r) in [(0.1, -0.78), (0.1, 0.78), (-0.5, -0.72), (0.5, 0.72)] {
        if sacks >= 2 {
            break;
        }
        let cand = [s.at(depth * f, width * r)];
        if let Some(p) = s.place(&cand, 0.28, 1.2, 0.45) {
            let yaw = yaw_toward(p, c);
            s.piece(move |m, _g, ctx| tr::coin_sack(m, p, yaw, &mut ctx.rng));
            s.plan.solids.push((p, 0.28));
            sacks += 1;
        }
    }

    // Loose coins between the heaps.
    for _ in 0..5 {
        let f = depth * s.rng.range(-0.55, 0.6);
        let r = width * s.rng.range(-0.55, 0.55);
        if let Some(q) = s.spot(s.at(f, r), 0.35) {
            s.piece(move |m, g, ctx| tr::loot_pile(m, g, q, 0.4, "none", 0.0, &mut ctx.rng, None));
        }
    }

    // Wall bays: flame banners above, crates, jars and rubble at their feet.
    let spring = architecture.spring;
    let bays = bay_order(&architecture.bays, c);
    for (k, bay) in bays.iter().enumerate() {
        let bay = *bay;
        if bay.width >= 1.3 && k < 8 {
            let top = bay.center + bay.inward * 0.02 + Y(spring - 0.14);
            let w = ((bay.width - 1.2) * 0.9).clamp(0.7, 1.0);
            let length = (spring - 1.55).clamp(1.2, 1.9);
            s.piece(move |m, g, _ctx| {
                tr::flame_banner(m, g, top, bay.inward, bay.tangent, w, length)
            });
        }
        // Bays alternate: crystals, stores, a boulder heap, jars.
        let kind = k % 4;
        let spot = bay.center + bay.inward * 0.62;
        let hug = |radius: f32| bay.center + bay.inward * (radius + 0.16);
        match kind {
            2 => {
                let at2 = hug(0.62);
                if let Some(p) = s.place(
                    &[at2, at2 + bay.tangent * 0.4, at2 - bay.tangent * 0.4],
                    0.62,
                    1.2,
                    0.45,
                ) {
                    let yaw = s.rng.range(0.0, TAU);
                    s.piece(move |m, _g, ctx| {
                        themekit::rubble(m, p, 0.85, yaw, ctx.stone, &mut ctx.rng)
                    });
                    s.plan.solids.push((p, 0.6));
                    if let Some(q) = s.spot(p + bay.tangent * 0.75 - bay.inward * 0.1, 0.3) {
                        let yaw = yaw_of(bay.inward);
                        s.piece(move |m, g, ctx| {
                            tr::amber_crystals(m, g, q, 0.6, yaw, ctx.stone, &mut ctx.rng)
                        });
                    }
                }
            }
            3 => {
                for along in [-0.45, 0.1, 0.55] {
                    if let Some(q) = s.place(
                        &[spot + bay.tangent * along - bay.inward * 0.1],
                        0.2,
                        1.2,
                        0.45,
                    ) {
                        let height = s.rng.range(0.45, 0.8);
                        if s.rng.chance(0.5) {
                            s.piece(move |m, _g, ctx| tr::amphora(m, q, height, &mut ctx.rng));
                        } else {
                            s.piece(move |m, _g, _ctx| sanct::clay_pot(m, q, height));
                        }
                        s.plan.solids.push((q, 0.2));
                    }
                }
            }
            0 => {
                // Amber crystals burst from a heap of broken stone.
                let at0 = hug(0.55);
                if let Some(p) = s.place(
                    &[at0, at0 + bay.tangent * 0.35, at0 - bay.tangent * 0.35],
                    0.55,
                    1.2,
                    0.45,
                ) {
                    let scale = s.rng.range(0.9, 1.15);
                    let yaw = yaw_of(bay.inward);
                    s.piece(move |m, g, ctx| {
                        tr::amber_crystals(m, g, p, scale, yaw, ctx.stone, &mut ctx.rng)
                    });
                    s.plan.solids.push((p, 0.5 * scale + 0.08));
                    if crystals < 4 {
                        s.plan.glow_lights.push((p + Y(0.8), 3.4, 4.8));
                    }
                    crystals += 1;
                    for j in 0..2 {
                        s.plan.sparkles.push(
                            p + v(
                                (j as f32 * 2.4).cos() * 0.2,
                                0.6 + j as f32 * 0.3,
                                (j as f32 * 2.4).sin() * 0.2,
                            ),
                        );
                    }
                    if let Some(q) = s.place(
                        &[p + bay.tangent * 0.85, p - bay.tangent * 0.85],
                        0.36,
                        1.2,
                        0.85,
                    ) {
                        let yaw = s.rng.range(0.0, TAU);
                        s.piece(move |m, _g, ctx| {
                            themekit::rubble(m, q, 0.5, yaw, ctx.stone, &mut ctx.rng)
                        });
                        s.plan.solids.push((q, 0.36));
                    }
                }
            }
            _ => {
                let set: [(&'static str, f32, f32, f32); 2] = if s.rng.chance(0.5) {
                    [
                        ("barrel_large", 0.95, 0.42, -0.4),
                        ("barrel_small", 0.55, 0.27, 0.45),
                    ]
                } else {
                    [
                        ("crates_stacked", 1.1, 0.6, 0.0),
                        ("box_small", 0.44, 0.27, 0.72),
                    ]
                };
                for (asset, height, radius, along) in set {
                    let cands = [
                        spot + bay.tangent * along,
                        spot + bay.tangent * (along * 0.6) - bay.inward * 0.08,
                    ];
                    if let Some(p) = s.place(&cands, radius, 1.2, 0.45) {
                        let yaw = yaw_of(bay.inward) + s.rng.range(-0.4, 0.4);
                        s.plan.props.push(Prop {
                            asset,
                            position: p,
                            yaw,
                            height,
                            radius,
                        });
                    }
                }
            }
        }
    }

    // Broken stone at the feet of the piers.
    for pilaster in &architecture.pilasters {
        for sg in [-1.0f32, 1.0] {
            if !s.rng.chance(0.55) {
                continue;
            }
            if let Some(q) = s.spot(
                pilaster.base + pilaster.inward * 0.42 + pilaster.tangent * (sg * 0.82),
                0.22,
            ) {
                let yaw = s.rng.range(0.0, TAU);
                s.piece(move |m, _g, ctx| {
                    themekit::rubble(m, q, 0.34, yaw, ctx.stone, &mut ctx.rng)
                });
            }
        }
    }

    // Chains hang from the vault near the walls, well above head height.
    for (k, bay) in bay_order(&architecture.bays, c).iter().take(6).enumerate() {
        if k % 2 == 1 {
            continue;
        }
        let along = s.rng.range(-0.4, 0.4);
        let top = with_y(
            bay.center + bay.inward * 1.1 + bay.tangent * along,
            s.room.center.y + s.room.height + 0.3,
        );
        let length = (s.room.height - 2.95).max(0.6);
        s.piece(move |m, _g, ctx| themekit::hanging_chain(m, top, length, &mut ctx.rng));
    }

    // A torch on every other pier; the rest carry a carved diamond.
    for (k, pilaster) in architecture.pilasters.iter().enumerate() {
        if k % 2 == 1 || s.plan.sconces.len() >= 6 {
            continue;
        }
        let backplate = pilaster.base + pilaster.inward * (PIER_DEPTH - 0.01) + Y(1.66);
        let flame = backplate + pilaster.inward * 0.1 + Y(0.63);
        s.plan.sconces.push(Sconce {
            backplate,
            yaw: yaw_of(pilaster.inward),
            flame,
        });
        s.light(flame, [1.0, 0.6, 0.3], 4.6, 6.8);
    }
    // Warm fill from under the vault so the whole hall reads.
    let h = s.room.height;
    s.light(c + Y(h - 1.1), [1.0, 0.72, 0.48], 2.4, 14.0);
}

/// The grand treasury: carpet, rug, chandelier, relic niches and gold.
fn grand_treasury(s: &mut V) {
    let c = s.c();
    let focus = s.plan.focus;
    let (depth, width) = (s.depth, s.width);
    let room = s.room;
    let architecture = layout(room);
    let niches = grand_niches(room);
    s.piers();

    // Round rug and red carpet leading to the relic chest.
    let mut rug_r: f32 = 2.0;
    let rug_fits = |r: f32| {
        (0..12).all(|k| {
            let a = k as f32 * TAU / 12.0;
            let p = focus + v(a.cos(), 0.0, a.sin()) * (r + 0.1);
            room.contains(p.x, p.z, 0.1)
        })
    };
    while rug_r > 1.2 && !rug_fits(rug_r) {
        rug_r -= 0.1;
    }
    s.plan.medallion_radius = rug_r;
    let rug_at = s.floor(focus);
    s.piece(move |m, g, _ctx| tr::rug(m, g, rug_at, rug_r));
    let from = s.floor(s.entry + s.inward * 0.3);
    let dir = norm(hz(focus - from));
    let to = s.floor(focus - dir * (rug_r - 0.1));
    let carpet_hw = 0.95;
    if hz(to - from).length() > 0.8 && hz(to - from).dot(dir) > 0.0 {
        s.piece(move |m, g, _ctx| tr::carpet(m, g, from, to, carpet_hw));
    }
    let side = v(dir.z, 0.0, -dir.x);
    s.placer.taken.push((focus, 1.0));
    s.light(focus + Y(1.3), GOLD, 2.4, 4.8);

    // Tall candelabra beside the chest, on the rug's rim.
    for sign in [-1.0f32, 1.0] {
        let cands = [
            focus + side * (sign * (rug_r - 0.35)) - dir * 0.5,
            focus + side * (sign * (rug_r + 0.3)),
            focus + side * (sign * (rug_r - 0.2)) + dir * 0.6,
        ];
        let Some(p) = s.place(&cands, 0.3, 1.2, 0.85) else {
            continue;
        };
        let yaw = yaw_toward(p, focus);
        s.piece(move |m, g, ctx| tr::candelabrum(m, g, p, yaw, &mut ctx.rng));
        s.plan.solids.push((p, 0.3));
        s.light(p + Y(1.4), WARM, 2.2, 5.0);
    }
    for sign in [-1.0f32, 1.0] {
        if let Some(q) = s.spot(focus + side * (sign * (rug_r * 0.62)) + dir * 0.2, 0.3) {
            s.coins(q, 0.35, 12);
        }
    }

    // The candle chandelier hangs over the chest.
    let hub = focus + dir * 0.4;
    let fy = s.floor(c).y;
    let ring_y = fy + (room.height - 2.2).max(3.4).min(room.height - 2.0);
    let ceiling = v(hub.x, fy + room.height, hub.z);
    s.piece(move |m, g, ctx| tr::chandelier(m, g, ceiling, ring_y, 1.15, &mut ctx.rng, 12));
    s.light(v(hub.x, ring_y + 0.1, hub.z), [1.0, 0.7, 0.38], 10.0, 16.0);
    for (f, r) in [(0.55, -0.55), (0.55, 0.55), (-0.45, -0.55), (-0.45, 0.55)] {
        let p = s.at(depth * f, width * r) + Y(3.2);
        s.light(p, [1.0, 0.66, 0.32], 3.2, 9.0);
    }

    // Great gold mounds in the corners.
    let n = room.outline.len();
    for i in (1..n).step_by(2) {
        let a = room.outline[i];
        let b = room.outline[(i + 1) % n];
        let corner = v(c.x + (a[0] + b[0]) * 0.5, c.y, c.z + (a[1] + b[1]) * 0.5);
        let into = norm(hz(c - corner));
        let radius = s.rng.range(1.35, 1.7);
        let cands: Vec<Vec3> = [0.5, 0.8, 1.1, 1.4, 1.8]
            .iter()
            .map(|&d| corner + into * (radius + d))
            .collect();
        if let Some(p) = s.place(&cands, radius, 1.3, 0.45) {
            let h = s.rng.range(0.9, 1.15);
            s.plan.hoards.push((p, radius, h));
        }
    }

    // A mountain of gold rises behind the relic chest.
    let cands = [
        focus + dir * (rug_r + 1.5),
        focus + dir * (rug_r + 1.9),
        focus + dir * (rug_r + 1.2),
    ];
    if let Some(back) = s.place(&cands, 1.35, rug_r + 0.3, 0.45) {
        s.plan.hoards.push((back, 1.35, 1.15));
    }

    // Niches: relics on altars, gold spilling out, or urns.
    let mut rest = ["armillary", "gems", "chalice", "crown"];
    s.rng.shuffle(&mut rest);
    let relics = [["helm"].as_slice(), rest.as_slice()].concat();
    let mut relic = 0;
    let mut ordered = niches.clone();
    let entry = s.entry;
    ordered.sort_by_key(|n| mm(-hz(n.center - entry).length()));
    for (k, niche) in ordered.iter().enumerate() {
        let mut kind = ["relic", "hoard", "relic", "urns"][k % 4];
        if kind == "relic" && relic >= relics.len() {
            kind = if (k / 2) % 2 == 1 { "hoard" } else { "urns" };
        }
        let (center, inward, tangent) = (niche.center, niche.inward, niche.tangent);
        match kind {
            "relic" => {
                let base = s.floor(center - inward * 0.42);
                s.plan
                    .niche_relics
                    .push((base, relics[relic], yaw_of(inward)));
                relic += 1;
                s.light(center + inward * 0.35 + Y(2.2), [1.0, 0.8, 0.52], 1.8, 3.4);
                // Votive candles stand inside the niche, flanking the altar.
                for sg in [-1.0f32, 1.0] {
                    let q = s.floor(center - inward * 0.25 + tangent * (sg * (niche.half - 0.16)));
                    s.candles(q, 3, 0.1);
                }
                for sg in [-1.0f32, 1.0] {
                    if let Some(q) = s.spot(
                        center + inward * 0.4 + tangent * (sg * (niche.half - 0.1)),
                        0.16,
                    ) {
                        s.candles(q, 3, 0.14);
                    }
                }
            }
            "hoard" => {
                let radius = s.rng.range(0.95, 1.2);
                let cands: Vec<Vec3> = [0.2, 0.4, 0.6, 0.85]
                    .iter()
                    .map(|&d| center + inward * (radius + d))
                    .collect();
                if let Some(p) = s.place(&cands, radius, 1.3, 0.45) {
                    let h = s.rng.range(0.8, 1.0);
                    s.plan.hoards.push((p, radius, h));
                    let q = s.floor(center - inward * 0.3);
                    s.coins(q, 0.5, 18);
                }
            }
            _ => {
                for sg in [-1.0f32, 1.0] {
                    let base = s.floor(
                        center - inward * 0.4 + tangent * (sg * (niche.half - 0.33).min(0.42)),
                    );
                    let height = s.rng.range(0.85, 1.1);
                    let style = (k + usize::from(sg > 0.0)) % 3;
                    let yaw = s.rng.range(0.0, TAU);
                    s.piece(move |m, _g, _ctx| tr::urn(m, base, height, style, yaw));
                }
                if let Some(q) = s.spot(center + inward * 0.35, 0.2) {
                    s.candles(q, 4, 0.2);
                }
            }
        }
    }

    // An open chest overflowing with gems, a crown on its block, a gem ledge.
    for sign in [1.0f32, -1.0] {
        let cands = [
            s.at(depth * 0.62, sign * width * 0.45),
            s.at(depth * 0.4, sign * width * 0.55),
            s.at(depth * 0.2, sign * width * 0.6),
        ];
        let Some(open) = s.place(&cands, 0.55, 1.4, 0.85) else {
            continue;
        };
        let yaw = yaw_toward(open, c);
        s.plan.open_chests.push((open, yaw));
        s.piece(move |m, g, ctx| tr::chest_hoard(m, g, open, yaw, &mut ctx.rng));
        s.plan.solids.push((open, 0.5));
        s.light(open + Y(0.9), GOLD, 2.0, 3.4);
        if let Some(q) = s.spot(open + norm(hz(c - open)) * 0.9, 0.4) {
            s.coins(q, 0.5, 20);
        }
    }
    let cands = [
        s.at(-depth * 0.36, width * 0.42),
        s.at(-depth * 0.36, -width * 0.42),
        s.at(-depth * 0.15, width * 0.5),
    ];
    if let Some(crown) = s.place(&cands, 0.46, 1.4, 0.85) {
        let yaw = yaw_toward(crown, s.entry);
        s.piece(move |m, g, ctx| tr::crown_block(m, g, crown, yaw, &mut ctx.rng));
        s.plan.solids.push((crown, 0.46));
        s.light(crown + Y(1.6), [1.0, 0.78, 0.45], 1.6, 3.2);
    }
    let cands = [
        s.at(-depth * 0.58, -width * 0.62),
        s.at(-depth * 0.58, width * 0.62),
        s.at(-depth * 0.3, -width * 0.7),
    ];
    if let Some(ledge) = s.place(&cands, 0.72, 1.4, 0.85) {
        let yaw = yaw_toward(ledge, c);
        s.piece(move |m, g, ctx| tr::gem_ledge(m, g, ledge, yaw, &mut ctx.rng));
        s.plan.solids.push((ledge, 0.72));
        s.plan.glow_lights.push((ledge + Y(1.3), 1.4, 3.0));
    }

    // Urns along the carpet's far sides and loose gold everywhere.
    let mut urns = 0;
    for (f, r) in [
        (-0.1, -0.6),
        (-0.1, 0.6),
        (0.35, -0.62),
        (0.35, 0.62),
        (-0.55, -0.35),
        (-0.55, 0.35),
    ] {
        if urns >= 4 {
            break;
        }
        let height = s.rng.range(0.8, 1.05);
        let cand = [s.at(depth * f, width * r)];
        let Some(p) = s.place(&cand, 0.3 * height, 1.3, 0.85) else {
            continue;
        };
        let style = urns % 3;
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, _g, _ctx| tr::urn(m, p, height, style, yaw));
        s.plan.solids.push((p, 0.3 * height));
        urns += 1;
    }
    let hoards = s.plan.hoards.clone();
    for (index, &(h, radius, height)) in hoards.iter().enumerate() {
        for k in 0..10 {
            let a = k as f32 * 2.39 + s.rng.range(0.0, 0.6);
            let d = radius * s.rng.range(0.1, 0.88);
            let y = height * (1.0 - (d / radius).powf(0.9)).max(0.0);
            s.plan
                .sparkles
                .push(h + v(a.cos() * d, y + 0.05, a.sin() * d));
        }
        if index < 4 {
            s.light(h + Y(height + 0.6), GOLD, 1.4 + radius * 0.3, 4.0);
        }
        let toward = norm(hz(focus - h));
        let sideways = v(-toward.z, 0.0, toward.x) * if index % 2 == 1 { 1.0 } else { -1.0 };
        if let Some(q) = s.spot(
            h + toward * (radius * 0.6 + 0.35) + sideways * (radius * 0.7),
            0.22,
        ) {
            s.candles(q, 5, 0.22);
        }
        let bars = s.spot(
            h + toward * (radius + 0.35) - sideways * (radius * 0.4),
            0.22,
        );
        if let Some(q) = bars.filter(|_| index % 2 == 0) {
            let yaw = s.rng.range(0.0, TAU);
            s.piece(move |m, _g, ctx| themekit::ingot_stack(m, q, yaw, &mut ctx.rng));
        }
        if let Some(q) = s.spot(h + norm(hz(focus - h)) * (radius + 0.4), 0.3) {
            s.coins(q, 0.45, 16);
        }
    }
    // Gold drifts across the flagstones, off the carpet and the walks.
    for _ in 0..16 {
        let f = depth * s.rng.range(-0.7, 0.75);
        let r = width * s.rng.range(-0.78, 0.78);
        let q = s.spot(s.at(f, r), 0.45);
        if let Some(q) = q.filter(|q| hz(*q - from).dot(side).abs() > carpet_hw + 0.5) {
            let radius = s.rng.range(0.35, 0.6);
            let count = 10 + s.rng.index(14);
            s.coins(q, radius, count);
        }
    }
    let carpet_len = hz(to - from).length();
    for (t, sign) in [(0.3, -1.0f32), (0.3, 1.0), (0.7, -1.0), (0.7, 1.0)] {
        let Some(q) = s.spot(
            from + dir * (carpet_len * t) + side * (sign * (carpet_hw + 0.35)),
            0.2,
        ) else {
            continue;
        };
        if t < 0.5 {
            s.candles(q, 3, 0.16);
        } else {
            s.coins(q, 0.3, 8);
        }
    }

    // The royal crest crowns the entrance, above the doorway arch.
    let door = &room.sockets[0];
    let crest_y = door.height + 1.35;
    if crest_y + 1.0 < room.height {
        let into = norm(v(-door.outward.x, 0.0, -door.outward.z));
        let center = s.floor(s.entry) + into * 0.04 + Y(crest_y);
        let tangent = v(-into.z, 0.0, into.x);
        s.piece(move |m, g, _ctx| tr::royal_crest(m, g, center, into, tangent));
    }
    // Sacks of coin wait along the walls.
    let mut sacks = 0;
    for (f, r) in [(0.25, -0.8), (0.25, 0.8), (-0.2, -0.82), (-0.2, 0.82)] {
        if sacks >= 3 {
            break;
        }
        let cands = [s.at(depth * f, width * r), s.at(depth * f, width * r * 0.9)];
        if let Some(p) = s.place(&cands, 0.28, 1.3, 0.45) {
            let yaw = yaw_toward(p, c);
            s.piece(move |m, _g, ctx| tr::coin_sack(m, p, yaw, &mut ctx.rng));
            s.plan.solids.push((p, 0.28));
            sacks += 1;
        }
    }

    // Every pier: a star banner high up; torches and trophies alternate below.
    let banner_top = room.height - 0.45;
    for (k, pilaster) in architecture.pilasters.iter().enumerate() {
        let face = pilaster.base + pilaster.inward * (PIER_DEPTH - 0.02);
        if k % 2 == 1 {
            let (top, inw, tan) = (face + Y(banner_top), pilaster.inward, pilaster.tangent);
            let length = (banner_top - 3.45).min(2.6);
            s.piece(move |m, g, _ctx| tr::banner(m, g, top, inw, tan, 1.0, length, false));
        }
        if k % 2 == 0 && s.plan.sconces.len() < 8 {
            let backplate = face + pilaster.inward * 0.01 + Y(1.66);
            let flame = backplate + pilaster.inward * 0.1 + Y(0.63);
            s.plan.sconces.push(Sconce {
                backplate,
                yaw: yaw_of(pilaster.inward),
                flame,
            });
            s.light(flame, [1.0, 0.6, 0.26], 4.6, 7.0);
        } else if k % 4 == 1 {
            s.plan.mounts.push(Mount {
                asset: "gold_relic",
                anchor: face + pilaster.inward * 0.02 + Y(2.3),
                yaw: yaw_of(pilaster.inward),
                height: 0.95,
            });
        }
    }
}

/// The sanctuary: the guardian's shrine, the rest font on its mosaic,
/// ember braziers, banners, votive shelves, ferns and lanterns.
fn sanctuary(s: &mut V) {
    let (map, room) = (s.map, s.room);
    let c = s.c();
    let focus = s.plan.focus;
    let architecture = layout(room);
    s.piers();
    let mut shrine = shrine_wall(map, room);
    if shrine.is_some_and(|(wall, inward, _)| {
        hz(wall + inward * SHRINE_FONT_DEPTH - focus).length() >= 0.05
    }) {
        shrine = None;
    }
    let inward = shrine.map_or(s.plan.inward, |(_, inward, _)| inward);
    let side = v(inward.z, 0.0, -inward.x);
    if let Some((wall, _, half_len)) = shrine {
        let wall = s.floor(wall);
        let width = (half_len * 2.0 - 1.4).clamp(2.2, 3.4);
        s.piece(move |m, g, ctx| {
            sanct::shrine(
                m,
                g,
                wall,
                inward,
                width,
                ctx.stone,
                ctx.glow_color,
                &mut ctx.rng,
            )
        });
        s.piece(move |m, _g, ctx| sanct::shrine_arch(m, wall, inward, width, ctx.stone));
        for k in [-1.0f32, 0.0, 1.0] {
            let p = wall + inward * 0.8 + side * (k * width * 0.33);
            s.plan.solids.push((p, 0.72));
            s.placer.taken.push((p, 0.72));
        }
        for (k, &(depth, inset, top)) in DAIS.iter().enumerate() {
            let hw = (width - inset) * 0.5 - 0.12;
            let edge_at = wall + inward * (depth - 0.12) + Y(top);
            let (a, b) = (edge_at - side * hw, edge_at + side * hw);
            let count = (hw as f64 * 2.0 / 0.17).floor() as usize;
            s.piece(move |m, g, ctx| sanct::candle_row(m, g, a, b, count, true, &mut ctx.rng));
            if k == 2 {
                let back = wall + inward * 0.22 + Y(top);
                let (a, b) = (back - side * hw, back + side * hw);
                let count = (hw as f64 * 2.0 / 0.2).floor() as usize;
                s.piece(move |m, g, ctx| sanct::candle_row(m, g, a, b, count, true, &mut ctx.rng));
            }
        }
        for sign in [-1.0f32, 1.0] {
            let cands = [
                wall + inward * 1.85 + side * (sign * (width * 0.5 + 0.3)),
                wall + inward * 2.1 + side * (sign * (width * 0.5 + 0.45)),
                wall + inward * 1.6 + side * (sign * (width * 0.5 + 0.55)),
            ];
            if let Some(stand) = s.place(&cands, 0.26, 1.35, 0.7) {
                let yaw = yaw_of(inward);
                s.piece(move |m, g, ctx| tr::candelabrum(m, g, stand, yaw, &mut ctx.rng));
                s.plan.solids.push((stand, 0.26));
            } else if let Some(q) = s.spot(
                wall + inward * 1.85 + side * (sign * (width * 0.5 + 0.15)),
                0.25,
            ) {
                s.candles(q, 5, 0.24);
            }
            if let Some(q) = s.spot(
                wall + inward * 1.2 + side * (sign * (width * 0.5 + 0.35)),
                0.3,
            ) {
                s.fern(q, 1.2);
            }
        }
        s.light(wall + inward * 2.3 + Y(0.35), [1.0, 0.7, 0.4], 2.1, 5.0);
        s.light(wall + inward * 1.25 + Y(0.7), [1.0, 0.58, 0.22], 2.0, 3.8);
        for sign in [-1.0f32, 1.0] {
            let x = (width * 0.5 + 0.05).min(1.35) + 1.0;
            if x + 0.5 < half_len {
                let top = wall + side * (sign * x) + Y(architecture.spring - 0.15);
                let length = (architecture.spring - 1.25).clamp(1.3, 2.3);
                s.piece(move |m, g, _ctx| tr::banner(m, g, top, inward, side, 0.7, length, true));
            }
        }
    } else {
        // Crossroads sanctuary: the guardian stands on a round shrine that
        // takes the place of the cover pillar on the font's diagonal.
        let radius = 1.05;
        let mut hosts: Vec<Vec3> = map
            .obstacles
            .iter()
            .filter(|o| o.room == room.id)
            .map(|o| o.center)
            .collect();
        hosts.sort_by_key(|p| mm(hz(*p - focus).length()));
        let host = hosts.into_iter().find(|&p| {
            room.sockets.iter().all(|sk| {
                hz(p - sk.position).length() > sk.width * 0.5 + radius + 1.0
                    && segment_distance(p, sk.position, focus) > radius + 0.7
            })
        });
        if let Some(host) = host {
            let spot = s.floor(host);
            s.plan.host_pillar = Some(spot);
            s.placer.taken.push((spot, radius));
            let facing = norm(hz(c - spot));
            s.piece(move |m, g, ctx| {
                sanct::round_shrine(
                    m,
                    g,
                    spot,
                    facing,
                    ctx.stone,
                    ctx.glow_color,
                    &mut ctx.rng,
                    radius,
                )
            });
            s.plan.solids.push((spot, radius));
            s.light(spot + facing * 1.6 + Y(0.5), [1.0, 0.7, 0.4], 2.2, 5.0);
            for k in 0..4 {
                s.plan.sparkles.push(
                    spot + v(
                        (k as f32 * 1.6).cos() * 0.3,
                        2.6 + k as f32 * 0.2,
                        (k as f32 * 1.6).sin() * 0.3,
                    ),
                );
            }
        }
    }
    // The rest font stands on a sun mosaic; pilgrims kneel on the room side.
    let out = if shrine.is_some() {
        inward
    } else if hz(c - focus).length_squared() > 0.01 {
        norm(hz(c - focus))
    } else {
        norm(v(
            -room.sockets[0].outward.x,
            0.0,
            -room.sockets[0].outward.z,
        ))
    };
    let kneel = v(out.z, 0.0, -out.x);
    let mut mosaic: f32 = 1.9;
    for o in map.obstacles.iter().filter(|o| o.room == room.id) {
        mosaic = mosaic.min(hdist(o.center, focus) - o.half[0].hypot(o.half[1]) - 0.1);
    }
    for sk in &room.sockets {
        mosaic = mosaic.min(hdist(sk.position, focus) - sk.width * 0.5 - 0.4);
    }
    while mosaic > 1.3 && !room.contains(focus.x, focus.z, mosaic) {
        mosaic -= 0.1;
    }
    let mosaic = mosaic.max(1.3);
    let ground = s.floor(focus);
    s.piece(move |m, _g, ctx| sanct::sun_mosaic(m, ground, mosaic, out, ctx.stone, &mut ctx.rng));
    let circle = (mosaic - 0.12).min(1.75);
    s.piece(move |_m, g, _ctx| sanct::rest_circle(g, ground, circle, [1.0, 0.62, 0.2]));
    s.piece(move |m, g, ctx| sanct::rest_font(m, g, ground, out, ctx.stone, ctx.glow_color));
    for sign in [-1.0f32, 1.0] {
        let base = s.floor(focus + out * 1.34 + kneel * (sign * 0.36));
        let yaw = yaw_of(out) + sign * 0.12;
        s.piece(move |m, _g, _ctx| sanct::cushion(m, base, yaw));
    }
    s.piece(move |m, _g, ctx| sanct::petals(m, ground, mosaic - 0.1, 34, &mut ctx.rng, 0.0, 1.15));
    for sign in [-1.0f32, 1.0] {
        let base = s.floor(focus + out * 1.45 + kneel * (sign * 1.0));
        s.piece(move |m, _g, ctx| sanct::offering_bowl(m, base, &mut ctx.rng));
    }
    s.plan.solids.push((focus, 0.9));
    s.placer.taken.push((focus, 1.55));
    s.plan.glow_lights.push((focus + Y(1.3), 5.0, 6.5));
    s.plan.font_core = Some(ground + Y(1.32));
    s.plan.shaft = Some(Shaft {
        top: ground + Y(room.height + 0.2),
        bottom: ground,
        radius_top: 0.6,
        radius_bottom: 1.35,
    });
    for k in 0..14 {
        let a = k as f32 * 2.39996;
        let r = 0.25 + 0.6 * ((k as f32 * 0.618) % 1.0);
        s.plan
            .sparkles
            .push(focus + v(a.cos() * r, 0.9 + (k % 5) as f32 * 0.55, a.sin() * r));
    }
    for sign in [-1.0f32, 1.0] {
        let hang = focus + kneel * (sign * 1.55) + out * 0.35;
        if !room.contains(hang.x, hang.z, 0.5) {
            continue;
        }
        let top = s.floor(hang) + Y(room.height + 0.1);
        let length = (room.height - 2.6).max(0.8);
        s.piece(move |m, g, ctx| sanct::censer(m, g, top, length, ctx.glow_color));
    }
    let entry = room.sockets[0].position;
    let dir = norm(hz(focus - entry));
    // Patterned rugs pave the aisle from the entrance to the font.
    {
        let from = s.floor(entry + dir * 0.4);
        let span = hz(focus - from).length() - mosaic - 0.2;
        let rugs = (span / 2.7).floor().max(0.0) as usize;
        for k in 0..rugs {
            let centre = s.floor(from + dir * (1.35 + k as f32 * 2.7));
            s.piece(move |m, g, _ctx| sanct::prayer_rug(m, g, centre, dir, [1.2, 0.86]));
        }
    }
    let mut braziers: Vec<Vec3> = vec![];
    for sign in [-1.0f32, 1.0] {
        let cands = [
            focus + side * (sign * 2.35) + inward * 0.8,
            focus + side * (sign * 2.4) + inward * 0.3,
            focus + side * (sign * 2.2) + inward * 1.3,
            focus + norm(side * sign + inward) * 2.4,
            focus + norm(side * sign + inward) * 2.8,
            focus + side * (sign * 2.5) - inward * 0.5,
            focus + norm(side * sign - inward) * 2.8,
        ];
        if let Some(p) = s.place(&cands, 0.34, 1.6, 0.85) {
            braziers.push(p);
        }
    }
    let depth = inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1];
    let width = side.x.abs() * room.half[0] + side.z.abs() * room.half[1];
    for sign in [-1.0f32, 1.0] {
        let cands = [
            c - inward * (depth * 0.35) + side * (sign * width * 0.62),
            c - inward * (depth * 0.1) + side * (sign * width * 0.66),
            c - inward * (depth * 0.55) + side * (sign * width * 0.55),
        ];
        if let Some(p) = s.place(&cands, 0.34, 1.6, 0.85) {
            braziers.push(p);
        }
    }
    for (a, b) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        if braziers.len() >= 6 {
            break;
        }
        let cands = [
            c + inward * (a * depth * 0.45) + side * (b * width * 0.45),
            c + inward * (a * depth * 0.84) + side * (b * width * 0.84),
            c + inward * (a * depth * 0.3) + side * (b * width * 0.62),
            c + inward * (a * depth * 0.62) + side * (b * width * 0.3),
        ];
        if let Some(p) = s.place(&cands, 0.34, 1.6, 0.85) {
            braziers.push(p);
        }
    }
    for &p in &braziers {
        s.piece(move |m, g, ctx| {
            sanct::crystal_brazier(m, g, p, 1.0, ctx.stone, ctx.glow_color, &mut ctx.rng)
        });
        s.plan.solids.push((p, 0.34));
        s.plan.fixtures.push(Fixture {
            p: p + Y(0.86),
            color: [1.0, 0.55, 0.2],
            size: 0.75,
        });
        s.plan.glow_lights.push((p + Y(1.2), 3.6, 5.0));
        for k in 0..3 {
            s.plan.sparkles.push(
                p + v(
                    (k as f32 * 2.1).cos() * 0.1,
                    1.0,
                    (k as f32 * 2.1).sin() * 0.1,
                ),
            );
        }
        let out = norm(hz(p - c));
        if let Some(q) = s.spot(p + out * 0.5 + side * 0.25, 0.28) {
            let scale = s.rng.range(0.85, 1.15);
            s.fern(q, scale);
        }
        if let Some(q) = s.spot(p - out * 0.1 + side * -0.55, 0.2) {
            let height = s.rng.range(0.4, 0.6);
            s.piece(move |m, _g, _ctx| sanct::clay_pot(m, q, height));
        }
        if let Some(q) = s.spot(p + inward * 0.55, 0.2) {
            s.candles(q, 3, 0.16);
        }
    }
    // Sun banners hang under the lintels, on the nave columns and from the vault.
    let mut banners = 0;
    let mut shelves = 0;
    for bay in bay_order(&architecture.bays, focus) {
        if bay.width < 1.3 {
            continue;
        }
        if banners >= 4 {
            if shelves < 4 && bay.width >= 2.2 {
                let w = (bay.width - 1.4).min(1.5);
                s.piece(move |m, g, ctx| {
                    sanct::votive_shelf(
                        m,
                        g,
                        bay.center,
                        bay.inward,
                        bay.tangent,
                        w,
                        ctx.stone,
                        &mut ctx.rng,
                    )
                });
                if shelves < 2 {
                    s.light(
                        bay.center + bay.inward * 0.5 + Y(1.4),
                        [1.0, 0.66, 0.32],
                        1.6,
                        3.4,
                    );
                }
                shelves += 1;
            }
            continue;
        }
        let top = bay.center + Y(architecture.spring - 0.44);
        let w = (bay.width * 0.42).clamp(0.7, 0.95);
        let length = (architecture.spring - 1.7).clamp(1.1, 2.0);
        s.piece(move |m, g, _ctx| tr::banner(m, g, top, bay.inward, bay.tangent, w, length, true));
        banners += 1;
    }
    let host = s.plan.host_pillar;
    let hosted = |o: &velvet_dungeon::Obstacle| {
        host.is_some_and(|h| (h.x - o.center.x).hypot(h.z - o.center.z) < 0.01)
    };
    let pillars: Vec<_> = map
        .obstacles
        .iter()
        .filter(|o| o.room == room.id && !hosted(o))
        .cloned()
        .collect();
    for o in &pillars {
        let towards = hz(c - o.center);
        let n = if towards.x.abs() > towards.z.abs() {
            v(towards.x.signum(), 0.0, 0.0)
        } else {
            v(0.0, 0.0, towards.z.signum())
        };
        let face = o.center + n * (o.half[0].min(o.half[1]) - 0.02);
        let top = face + Y(room.height - 0.62);
        let tangent = v(-n.z, 0.0, n.x);
        let length = (room.height - 3.3).min(1.7);
        s.piece(move |m, g, _ctx| tr::banner(m, g, top, n, tangent, 0.66, length, true));
    }
    let hanging = ((hz(focus - entry).length() - 3.0) / 3.2).floor().max(0.0) as usize;
    for k in 0..hanging.max(1) {
        let at = s.floor(entry + dir * (3.8 + k as f32 * 3.2));
        if hz(at - focus).length() < 1.4 {
            continue;
        }
        let top = at + Y(room.height + 0.25);
        let length = (room.height - 3.55).max(0.5);
        s.piece(move |m, g, ctx| themekit::lantern(m, g, top, length, &mut ctx.rng));
        s.light(at + Y(room.height - 1.45), [1.0, 0.64, 0.3], 2.4, 5.5);
    }
    for k in 0..hanging {
        for sign in [-1.0f32, 1.0] {
            let at = entry + dir * (2.2 + k as f32 * 3.2) + v(dir.z, 0.0, -dir.x) * (sign * 1.55);
            let top = s.floor(at) + Y(room.height + 0.05);
            let inw = v(dir.z, 0.0, -dir.x) * -sign;
            let length = (room.height - 3.4).min(1.25);
            s.piece(move |m, g, _ctx| tr::banner(m, g, top, inw, dir, 0.62, length, true));
        }
    }
    for socket in &room.sockets {
        let into = norm(v(-socket.outward.x, 0.0, -socket.outward.z));
        let y = socket.height + 0.72;
        if y + 0.4 < room.height {
            let center = s.floor(socket.position) + into * 0.03 + Y(y);
            let radius = (room.height - y - 0.08).min(0.62);
            s.piece(move |m, g, ctx| {
                sanct::sun_relief(m, g, center, into, radius, ctx.stone, ctx.glow_color)
            });
        }
    }
    sconces_on_pilasters(s, 4, 4.8, 6.5, PIER_DEPTH);
    for o in &pillars {
        let towards = norm(hz(c - o.center));
        let across = v(towards.z, 0.0, -towards.x);
        if let Some(q) = s.spot(o.center + towards * 1.05 + across * 0.3, 0.3) {
            let scale = s.rng.range(0.8, 1.05);
            s.fern(q, scale);
        }
        if let Some(q) = s.spot(o.center + towards * 1.0 - across * 0.45, 0.2) {
            s.candles(q, 4, 0.2);
        }
    }
    for (f, r) in [(-0.7, -0.85), (-0.7, 0.85), (0.1, -0.88), (0.1, 0.88)] {
        if let Some(q) = s.spot(c + inward * (depth * f) + side * (width * r), 0.3) {
            let scale = s.rng.range(0.8, 1.1);
            s.fern(q, scale);
        }
    }
}

/// The hidden chamber as an arcane library.
fn arcane_hall(s: &mut V) {
    let c = s.c();
    let focus = s.plan.focus;
    let (depth, width) = (s.depth, s.width);
    let room = s.room;
    let architecture = layout(room);
    let niches = grand_niches(room);
    let dais_r = ARCANE_DAIS[0].0;
    let from = s.floor(s.entry + s.inward * 0.3);
    let dir = norm(hz(focus - from));
    let side = v(dir.z, 0.0, -dir.x);
    let to_entry = -dir;
    let top = s.floor(focus) + Y(ARCANE_DAIS_TOP);

    // The dais: a solid, stepped plinth with the rune circle on its top.
    let base = s.floor(focus);
    s.piece(move |m, g, ctx| arcane::arcane_dais(m, g, base, to_entry, ctx.stone, &mut ctx.rng));
    s.plan.solids.push((focus, dais_r - 0.03));
    s.placer.taken.push((focus, dais_r));
    s.plan.medallion_radius = ARCANE_DAIS[1].0 - 0.2;
    s.plan.medallion_y = ARCANE_DAIS_TOP;
    s.light(top + Y(0.7), ARCANE, 2.6, 4.5);
    let to = s.floor(focus - dir * (dais_r + 0.02));
    let runner_hw = 0.78;
    let runner_len = hz(to - from).length();
    if runner_len > 0.8 && hz(to - from).dot(dir) > 0.0 {
        s.piece(move |m, g, _ctx| arcane::arcane_runner(m, g, from, to, runner_hw));
    }

    // Candelabra flank the chest on the dais; the chandelier hangs above.
    for sign in [-1.0f32, 1.0] {
        let p = top + side * (sign * 0.98) - dir * 0.12;
        let yaw = yaw_toward(p, focus);
        s.piece(move |m, g, ctx| tr::candelabrum(m, g, p, yaw, &mut ctx.rng));
        s.light(p + Y(1.4), WARM, 1.8, 4.5);
    }
    let fy = s.floor(c).y;
    let ring_y = fy + (room.height - 1.7).min(3.7);
    let ceiling = v(focus.x, fy + room.height, focus.z);
    s.piece(move |m, g, ctx| arcane::crystal_chandelier(m, g, ceiling, ring_y, 0.85, &mut ctx.rng));
    s.light(
        v(focus.x, ring_y + 0.15, focus.z),
        [1.0, 0.68, 0.36],
        6.5,
        13.0,
    );
    s.light(v(focus.x, ring_y - 0.6, focus.z), ARCANE, 2.2, 6.0);
    s.plan.shaft = Some(Shaft {
        top: ceiling,
        bottom: top,
        radius_top: 0.55,
        radius_bottom: 1.35,
    });

    // Rune obelisks: one pair behind the dais, one pair along the runner.
    for sign in [-1.0f32, 1.0] {
        let cands: Vec<Vec3> = [0.4, 0.9, 0.0, -0.4]
            .iter()
            .flat_map(|&f| [2.9, 3.3, 2.6].map(|sd| focus + side * (sign * sd) + dir * f))
            .collect();
        if let Some(p) = s.place(&cands, OBELISK_RADIUS, 1.2, 0.85) {
            s.plan.obelisks.push(p);
        }
    }
    for sign in [-1.0f32, 1.0] {
        let cands: Vec<Vec3> = [0.45, 0.3, 0.6]
            .iter()
            .flat_map(|&t| {
                [2.3, 2.7, 2.0].map(|sd| from + dir * (runner_len * t) + side * (sign * sd))
            })
            .collect();
        if let Some(p) = s.place(&cands, OBELISK_RADIUS, 1.2, 0.85) {
            s.plan.obelisks.push(p);
        }
    }

    // Bookshelves fill every niche; ivy spills from the cornice above them.
    let cornice = niches.first().map_or(room.height - 1.5, |n| n.top) + 0.35;
    for (k, niche) in niches.iter().enumerate() {
        let niche = *niche;
        s.piece(move |m, g, ctx| arcane::bookshelf(m, g, &niche, &mut ctx.rng));
        if k % 2 == 0 {
            s.light(
                niche.center + niche.inward * 0.45 + Y(1.9),
                [1.0, 0.7, 0.4],
                1.3,
                3.4,
            );
        }
        let reach = niche.half + 0.45;
        let a = niche.center - niche.tangent * reach + niche.inward * 0.18 + Y(cornice);
        let b = niche.center + niche.tangent * reach + niche.inward * 0.18 + Y(cornice);
        let inw = niche.inward;
        s.piece(move |m, g, ctx| arcane::ivy(m, g, a, b, inw, 1.1, &mut ctx.rng));
    }

    // Lecterns with open books, each with a stack and a candle beside it.
    for sign in [-1.0f32, 1.0] {
        let cands: Vec<Vec3> = [(0.05, 0.62), (-0.2, 0.6), (0.3, 0.64), (-0.4, 0.58)]
            .iter()
            .map(|&(f, r)| s.at(depth * f, sign * width * r))
            .collect();
        let Some(p) = s.place(&cands, 0.32, 1.4, 0.85) else {
            continue;
        };
        let yaw = yaw_toward(p, focus);
        s.plan.props.push(Prop {
            asset: "book_stand",
            position: p,
            yaw,
            height: 1.4,
            radius: 0.32,
        });
        let perp = v(yaw.cos(), 0.0, -yaw.sin());
        if let Some(stack) = s.spot(p + perp * 0.6, 0.15) {
            s.plan.props.push(Prop {
                asset: "book_stack",
                position: stack,
                yaw: yaw + 0.4,
                height: 0.24,
                radius: 0.0,
            });
        }
        if let Some(q) = s.spot(p - perp * 0.55, 0.16) {
            s.candles(q, 3, 0.12);
        }
    }

    // Trunks and urns wait by the walls, with coins spilled around them.
    let mut trunks = 0;
    for (f, r) in [
        (0.7, -0.62),
        (0.7, 0.62),
        (-0.45, -0.66),
        (-0.45, 0.66),
        (0.3, -0.7),
    ] {
        if trunks >= 3 {
            break;
        }
        let cands = [
            s.at(depth * f, width * r),
            s.at(depth * f * 0.9, width * r * 0.9),
        ];
        let Some(p) = s.place(&cands, 0.42, 1.4, 0.45) else {
            continue;
        };
        s.plan.props.push(Prop {
            asset: "chest",
            position: p,
            yaw: yaw_toward(p, c),
            height: 0.5,
            radius: 0.42,
        });
        if let Some(q) = s.spot(p + norm(hz(c - p)) * 0.7, 0.3) {
            s.coins(q, 0.3, 8);
        }
        trunks += 1;
    }
    let mut urns = 0;
    for (f, r) in [
        (0.5, -0.72),
        (0.5, 0.72),
        (-0.1, -0.74),
        (-0.1, 0.74),
        (-0.65, -0.5),
        (-0.65, 0.5),
    ] {
        if urns >= 4 {
            break;
        }
        let height = s.rng.range(0.75, 1.0);
        let cand = [s.at(depth * f, width * r)];
        let Some(p) = s.place(&cand, 0.3 * height, 1.3, 0.45) else {
            continue;
        };
        let style = urns % 3;
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, _g, _ctx| tr::urn(m, p, height, style, yaw));
        s.plan.solids.push((p, 0.3 * height));
        urns += 1;
    }
    for (asset, height, f, r) in [
        ("potion_tall", 0.3, -0.3, -0.8),
        ("potion_round", 0.22, -0.34, -0.74),
        ("vase_small", 0.5, 0.6, 0.8),
        ("potion_small", 0.16, 0.64, 0.74),
    ] {
        if let Some(q) = s.spot(s.at(depth * f, width * r), 0.14) {
            let yaw = s.rng.range(0.0, TAU);
            s.plan.props.push(Prop {
                asset,
                position: q,
                yaw,
                height,
                radius: 0.0,
            });
        }
    }

    // Candles, books and coins around the dais and the obelisks.
    for sign in [-1.0f32, 1.0] {
        if let Some(q) = s.spot(
            focus - dir * (dais_r + 0.35) + side * (sign * (runner_hw + 0.45)),
            0.2,
        ) {
            s.candles(q, 4, 0.18);
        }
        if let Some(q) = s.spot(focus + side * (sign * (dais_r + 0.4)) + dir * 0.3, 0.15) {
            let yaw = s.rng.range(0.0, TAU);
            s.plan.props.push(Prop {
                asset: "book_stack",
                position: q,
                yaw,
                height: 0.22,
                radius: 0.0,
            });
        }
        if let Some(q) = s.spot(focus + side * (sign * (dais_r + 0.45)) - dir * 0.6, 0.3) {
            s.coins(q, 0.3, 9);
        }
    }
    for p in s.plan.obelisks.clone() {
        if let Some(q) = s.spot(p + norm(hz(focus - p)) * 0.72, 0.18) {
            s.candles(q, 3, 0.14);
        }
    }

    // Eye banners hang on alternate pilasters; the others carry sconces.
    let banner_top = room.height - 0.85;
    for (k, pilaster) in architecture.pilasters.iter().enumerate() {
        if k % 2 == 1 {
            let face = pilaster.base + pilaster.inward * (PROJECTION + 0.16);
            let (top, inw, tan) = (face + Y(banner_top), pilaster.inward, pilaster.tangent);
            let length = (banner_top - 1.6).min(3.1);
            s.piece(move |m, g, _ctx| arcane::arcane_banner(m, g, top, inw, tan, 0.74, length));
        }
    }
    sconces_on_pilasters(s, 4, 3.0, 6.5, PROJECTION);
    for (f, r) in [(0.55, -0.55), (0.55, 0.55), (-0.45, -0.55), (-0.45, 0.55)] {
        let p = s.at(depth * f, width * r) + Y(3.0);
        s.light(p, [1.0, 0.64, 0.34], 1.8, 8.0);
    }

    // Violet motes drift through the whole hall, thickest in the light shaft.
    for _ in 0..30 {
        let x = s.rng.range(-0.8, 0.8) * room.half[0];
        let y = s.rng.range(0.5, room.height.min(4.6) - 0.3);
        let z = s.rng.range(-0.8, 0.8) * room.half[1];
        s.plan.sparkles.push(c + v(x, y, z));
    }
    for _ in 0..14 {
        let a = s.rng.range(0.0, TAU);
        let r = s.rng.unit64().sqrt() as f32 * 1.1;
        let y = s.rng.range(0.6, room.height - 1.2);
        s.plan.sparkles.push(top + v(a.cos() * r, y, a.sin() * r));
    }
    let _ = PI;
}

fn obelisk_half(y: f32) -> f32 {
    0.22 - ((y - 0.3) / 1.9).clamp(0.0, 1.0) * 0.08
}

/// Tapered rune obelisk: stepped plinth, frustum shaft, pyramidion.
pub fn obelisk(m: &mut Mesh3D, base: Vec3, stone: C3) {
    cuboid(m, base + Y(0.06), v(0.4, 0.06, 0.4), tint(stone, 0.5));
    cuboid(m, base + Y(0.17), v(0.33, 0.05, 0.33), tint(stone, 0.7));
    cuboid(m, base + Y(0.26), v(0.27, 0.04, 0.27), tint(stone, 0.9));
    let (y0, y1) = (0.3, 2.2);
    let corner = |sx: f32, sz: f32, y: f32| base + v(sx * obelisk_half(y), y, sz * obelisk_half(y));
    let faces = [
        ([1.0, -1.0], [1.0, 1.0]),
        ([1.0, 1.0], [-1.0, 1.0]),
        ([-1.0, 1.0], [-1.0, -1.0]),
        ([-1.0, -1.0], [1.0, -1.0]),
    ];
    for (face, (a, b)) in faces.iter().enumerate() {
        let n = norm(v(a[0] + b[0], 0.12, a[1] + b[1]));
        let shade = [0.62, 0.74, 0.58, 0.68][face];
        quad(
            m,
            [
                corner(a[0], a[1], y0),
                corner(b[0], b[1], y0),
                corner(b[0], b[1], y1),
                corner(a[0], a[1], y1),
            ],
            n,
            tint(stone, shade),
        );
        tri(
            m,
            corner(a[0], a[1], y1),
            corner(b[0], b[1], y1),
            base + Y(2.46),
            norm(n + VY),
            tint(stone, shade * 1.25),
        );
    }
    cuboid(
        m,
        base + Y(y1 - 0.03),
        v(obelisk_half(y1) + 0.025, 0.03, obelisk_half(y1) + 0.025),
        tint(stone, 0.95),
    );
}

/// Emissive floor medallion / rune circle and obelisk rune bands (arcane).
pub fn vault_glow_mesh(plan: &Plan) -> Mesh3D {
    let mut m = empty();
    let arcane = plan.vault == Some(VaultKind::Arcane);
    let color = if arcane { ARCANE } else { GOLD };
    let c = plan.focus + Y(plan.medallion_y + 0.018);
    let ring = |m: &mut Mesh3D, r0: f32, r1: f32, segments: usize, shade: f32| {
        for i in 0..segments {
            let a0 = i as f32 * TAU / segments as f32;
            let a1 = (i + 1) as f32 * TAU / segments as f32;
            let p = |a: f32, r: f32| c + v(a.cos() * r, 0.0, a.sin() * r);
            quad(
                m,
                [p(a0, r1), p(a1, r1), p(a1, r0), p(a0, r0)],
                VY,
                tint(color, shade),
            );
        }
    };
    let r = plan.medallion_radius;
    ring(&mut m, r - 0.07, r, 48, 1.0);
    ring(&mut m, r * 0.72 - 0.035, r * 0.72, 40, 0.85);
    let strokes = if arcane { 12 } else { 8 };
    for i in 0..strokes {
        let a = i as f32 * TAU / strokes as f32;
        let dir = v(a.cos(), 0.0, a.sin());
        let perp = v(-dir.z, 0.0, dir.x);
        let inner = c + dir * (r * 0.76);
        let outer = c + dir * (r - 0.12);
        let w = if arcane { 0.03 } else { 0.022 };
        quad(
            &mut m,
            [
                inner - perp * w,
                outer - perp * w,
                outer + perp * w,
                inner + perp * w,
            ],
            VY,
            tint(color, 0.9),
        );
        if arcane {
            let q = c + v((a + 0.26).cos(), 0.0, (a + 0.26).sin()) * (r * 0.86);
            let sz = 0.07;
            quad(
                &mut m,
                [q - dir * sz, q + perp * sz, q + dir * sz, q - perp * sz],
                VY,
                tint(color, 1.1),
            );
        }
    }
    for &p in &plan.obelisks {
        for face in 0..4 {
            let a = face as f32 * PI / 2.0;
            let n = v(a.cos(), 0.0, a.sin());
            let t = v(-n.z, 0.0, n.x);
            for k in 0..9 {
                let y = 0.5 + k as f32 * 0.19;
                let o = p + n * (obelisk_half(y) + 0.012) + Y(y);
                let w = [0.07, 0.035, 0.06, 0.03, 0.075, 0.045, 0.065, 0.03, 0.05][(k + face) % 9];
                let h = if k % 3 == 1 { 0.1 } else { 0.028 };
                quad(
                    &mut m,
                    [o - t * w, o + t * w, o + t * w + Y(h), o - t * w + Y(h)],
                    n,
                    tint(color, 0.9),
                );
            }
            let band = p + n * (obelisk_half(2.17) + 0.03) + Y(2.14);
            quad(
                &mut m,
                [
                    band - t * 0.16,
                    band + t * 0.16,
                    band + t * 0.16 + Y(0.02),
                    band - t * 0.16 + Y(0.02),
                ],
                n,
                color,
            );
        }
    }
    m
}
