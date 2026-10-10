//! The themes of the rooms with a role (web prototype `themes.js`
//! `planThemeBody`): the Heart, the Custodian's arena, a guard post, a lair,
//! the seal, a memorial, the explorers' camp and the return gate.
use super::super::architecture::layout;
use super::super::themes::Theme;
use super::kit::*;
use super::plan::*;
use super::stage::{yaw_toward, Stage};
use super::themekit::*;
use std::f32::consts::PI;
use velvet_dungeon::gameplay::SpawnKind;
use velvet_math::Vec3;

struct Corner {
    p: Vec3,
    into: Vec3,
}

fn flame(s: &mut Stage, p: Vec3, color: C3, size: f32) {
    s.plan.fixtures.push(Fixture { p, color, size });
}

fn light(s: &mut Stage, p: Vec3, color: C3, intensity: f32, radius: f32) {
    s.plan.light(p, color, intensity, radius);
}

fn beat_light(s: &mut Stage, p: Vec3, color: C3, intensity: f32, radius: f32) {
    s.plan.light(p, color, intensity, radius);
    s.plan.lights.last_mut().unwrap().beat = true;
}

fn sparkle(s: &mut Stage, count: usize, lo: f32, hi: f32) {
    let spread = 0.8;
    for _ in 0..count {
        let x = s.rng.range(-spread, spread) * s.room.half[0];
        let y = s.rng.range(lo, hi);
        let z = s.rng.range(-spread, spread) * s.room.half[1];
        s.plan.sparkles.push(s.c + v(x, y, z));
    }
}

fn spot(s: &Stage, p: Vec3, r: f32) -> Option<Vec3> {
    s.placer.floor_spot(p, r)
}

/// A brazier with its flame and light (web `fire`).
fn fire(s: &mut Stage, p: Vec3, color: C3, scale: f32) {
    s.piece(move |m, g, ctx| brazier(m, g, p, color, &mut ctx.rng, scale));
    s.plan.solids.push((p, 0.36 * scale));
    let top = p + Y(1.0 * scale);
    flame(s, top, color, 1.3 * scale);
    light(s, top + Y(0.3), color, 3.2 * scale, 6.5);
}

fn corner_fires(s: &mut Stage, corners: &[Corner], color: C3, count: usize, scale: f32) {
    let mut made = 0;
    for corner in corners {
        if made >= count {
            break;
        }
        let cands: Vec<Vec3> = [1.3, 1.8, 2.4]
            .iter()
            .map(|&d| corner.p + corner.into * d)
            .collect();
        if let Some(q) = s.place(&cands, 0.4 * scale, 1.2, 0.6) {
            fire(s, q, color, scale);
            made += 1;
        }
    }
}

fn shuffled_corners(s: &mut Stage, corners: &[Corner]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..corners.len()).collect();
    s.rng.shuffle(&mut order);
    order
}

fn bone_piles(s: &mut Stage, corners: &[Corner], count: usize, lo: f32, hi: f32) {
    for k in shuffled_corners(s, corners).into_iter().take(count) {
        let Corner { p, into } = corners[k];
        let d = s.rng.range(0.9, 1.4);
        if let Some(q) = spot(s, p + into * d, hi) {
            let radius = s.rng.range(lo, hi);
            s.piece(move |m, _g, ctx| bone_pile(m, q, radius, &mut ctx.rng));
        }
    }
}

fn webs(s: &mut Stage, corners: &[Corner], count: usize) {
    for k in shuffled_corners(s, corners).into_iter().take(count) {
        let Corner { p, into } = corners[k];
        let top = with_y(p + into * 0.55, s.c.y + s.rng.range(1.6, 2.6));
        let size = s.rng.range(0.55, 0.9);
        s.piece(move |m, _g, _ctx| web(m, top, into, size));
    }
}

/// Plan the body of a role theme into the stage's plan.
pub fn plan(s: &mut Stage, theme: Theme) {
    let (map, room) = (s.map, s.room);
    let c = s.c;
    let (inward, right, depth, width) = (s.inward, s.right, s.depth, s.width);
    let height = room.height;
    let bays = layout(room).bays;
    let n = room.outline.len();
    let corners: Vec<Corner> = (1..n)
        .step_by(2)
        .map(|i| {
            let a = room.outline[i];
            let b = room.outline[(i + 1) % n];
            let p = v(c.x + (a[0] + b[0]) * 0.5, c.y, c.z + (a[1] + b[1]) * 0.5);
            Corner {
                p,
                into: norm(hz(c - p)),
            }
        })
        .collect();
    let wall_bays = |s: &mut Stage, k: usize| {
        let mut list = bays.clone();
        s.rng.shuffle(&mut list);
        list.truncate(k);
        list
    };
    let obstacles: Vec<_> = map
        .obstacles
        .iter()
        .filter(|o| o.room == room.id)
        .cloned()
        .collect();
    s.plan.sparkle_tint = Some([1.2, 1.0, 0.6]);

    match theme {
        Theme::Heart => {
            let r = room.half[0].min(room.half[1]);
            let g = s.floor(c);
            s.plan.replace_pillars = true;
            let sigil = (4.2f32).min(r * 0.56);
            s.plan
                .pulse(move |m, glow, _ctx| heart_sigil(glow, m, g, sigil));
            s.plan.pulse(move |_m, glow, ctx| {
                floor_cracks(glow, g, 12, sigil + 0.1, r - 0.5, &mut ctx.rng, BLOOD_GLOW)
            });
            let heart_y = height * 0.58;
            s.plan.heart = Some((c + Y(heart_y), 1.3));
            // Veins hang the Heart from the vault, splaying toward the corners.
            for k in 0..6 {
                let a = (k as f32 / 6.0) * TAU + 0.3;
                let from = c + Y(heart_y + 1.2) + v(a.cos() * 0.35, 0.0, a.sin() * 0.35);
                let to = c + Y(height + 0.3) + v(a.cos() * r * 0.62, 0.0, a.sin() * r * 0.62);
                let mid =
                    (from + to) * 0.5 + Y(-0.9) + v((a + 0.6).cos(), 0.0, (a + 0.6).sin()) * 0.6;
                s.plan.pulse(move |m, glow, ctx| {
                    vein(
                        m,
                        glow,
                        &bezier(from, mid, to, 16),
                        0.2,
                        0.12,
                        &mut ctx.rng,
                        true,
                    )
                });
            }
            for o in &obstacles {
                let (base, half) = (o.center, o.half[0]);
                s.plan.pulse(move |m, glow, ctx| {
                    vein_pillar(m, glow, base, height, half, ctx.stone, &mut ctx.rng)
                });
            }
            corner_fires(s, &corners, [1.0, 0.2, 0.12], 4, 1.1);
            bone_piles(s, &corners, 4, 0.5, 0.8);
            for k in 0..8 {
                let a = (k as f32 / 8.0) * TAU + 0.2;
                let p = c + v(a.cos() * r * 0.44, 0.0, a.sin() * r * 0.44);
                let top = with_y(p, c.y + height);
                let length = s.rng.range(1.2, 2.4);
                s.piece(move |m, _g, ctx| hanging_chain(m, top, length, &mut ctx.rng));
            }
            beat_light(s, c + Y(heart_y), [1.0, 0.16, 0.12], 11.0, 16.0);
            beat_light(s, g + Y(0.8), [1.0, 0.2, 0.14], 3.5, 7.0);
            s.plan.sparkle_tint = Some([1.6, 0.25, 0.2]);
            for _ in 0..40 {
                let a = s.rng.range(0.0, TAU);
                let rr = s.rng.unit64().sqrt() as f32 * r * 0.7;
                let y = s.rng.range(0.4, height - 1.0);
                s.plan.sparkles.push(c + v(a.cos() * rr, y, a.sin() * rr));
            }
        }
        Theme::Custodian => {
            let r = room.half[0].min(room.half[1]);
            let g = s.floor(c);
            s.plan
                .pulse(move |_m, glow, _ctx| rune_circle(glow, g, r * 0.52, [1.0, 0.45, 0.15], 12));
            // The throne backs onto the far wall, but never onto a doorway.
            let cands: Vec<Vec3> = [1.4, 2.0, 2.8]
                .iter()
                .map(|&d| c + inward * (depth - d))
                .collect();
            if let Some(far) = s.place(&cands, 1.05, 2.0, 0.9) {
                let yaw = yaw_toward(far, c);
                s.piece(move |m, glow, ctx| bone_throne(m, glow, far, yaw, ctx.stone));
                s.plan.solids.push((far, 1.05));
                light(s, far + Y(2.4), [1.0, 0.45, 0.16], 3.2, 6.0);
            }
            corner_fires(s, &corners, [1.0, 0.46, 0.12], 4, 1.0);
            bone_piles(s, &corners, 3, 0.45, 0.75);
            for o in &obstacles {
                for sg in [-1.0f32, 1.0] {
                    let top = v(o.center.x + sg * 0.3, c.y + height, o.center.z + sg * 0.2);
                    let length = s.rng.range(2.2, 3.6);
                    s.piece(move |m, _g, ctx| hanging_chain(m, top, length, &mut ctx.rng));
                }
            }
            for bay in wall_bays(s, 4) {
                let asset = if s.rng.chance(0.5) {
                    "gold_relic"
                } else {
                    "broken_relic"
                };
                s.plan.mounts.push(Mount {
                    asset,
                    anchor: bay.center + bay.inward * 0.05 + Y(2.2),
                    yaw: bay.inward.x.atan2(bay.inward.z),
                    height: 0.9,
                });
            }
            s.plan.sparkle_tint = Some([1.7, 0.8, 0.3]);
            sparkle(s, 24, 0.4, height - 1.0);
        }
        Theme::Guardpost => {
            for bay in wall_bays(s, 3) {
                let Some(p) = s.place(&[bay.center + bay.inward * 0.6], 0.7, 1.4, 0.6) else {
                    continue;
                };
                let yaw = bay.inward.x.atan2(bay.inward.z) + PI;
                s.piece(move |m, _g, _ctx| weapon_rack(m, p, yaw));
                s.plan.solids.push((p, 0.55));
            }
            for k in 0..2 {
                let sx = if k == 1 { 0.5 } else { -0.5 };
                let f = depth * s.rng.range(-0.2, 0.5);
                let cands = [
                    s.at(f, width * sx),
                    s.at(depth * 0.1, width * if k == 1 { 0.35 } else { -0.35 }),
                ];
                let Some(p) = s.place(&cands, 0.35, 1.4, 0.85) else {
                    continue;
                };
                let yaw = yaw_toward(p, c);
                s.piece(move |m, _g, _ctx| dummy(m, p, yaw));
                s.plan.solids.push((p, 0.3));
            }
            let cands = [
                s.at(-depth * 0.35, width * 0.55),
                s.at(-depth * 0.35, -width * 0.55),
                s.at(depth * 0.45, width * 0.6),
            ];
            if let Some(table) = s.place(&cands, 0.6, 1.4, 0.85) {
                s.plan.props.push(Prop {
                    asset: "box_large",
                    position: table,
                    yaw: yaw_toward(table, c),
                    height: 0.85,
                    radius: 0.6,
                });
                s.plan.props.push(Prop {
                    asset: "candle_triple",
                    position: table + Y(0.85),
                    yaw: 0.0,
                    height: 0.32,
                    radius: 0.0,
                });
                flame(s, table + Y(1.15), [1.0, 0.62, 0.26], 0.5);
            }
            for bay in wall_bays(s, 2) {
                s.plan.mounts.push(Mount {
                    asset: "banner_shield_red",
                    anchor: bay.center + bay.inward * 0.03 + Y(1.1),
                    yaw: bay.inward.x.atan2(bay.inward.z),
                    height: 2.1,
                });
            }
            if let Some(trap) = map
                .spawns
                .iter()
                .find(|sp| sp.room == room.id && sp.kind == SpawnKind::Trap)
            {
                let base = s.floor(trap.position);
                s.piece(move |m, g, ctx| vent(m, g, base, &mut ctx.rng));
            }
            corner_fires(s, &corners, [1.0, 0.56, 0.24], 2, 0.95);
            for (asset, h) in [
                ("barrel_large", 0.95),
                ("crates_stacked", 1.1),
                ("keg", 0.8),
            ] {
                let cn = &corners[s.rng.index(corners.len())];
                let cands = [cn.p + cn.into * 1.2, cn.p + cn.into * 1.6];
                if let Some(p) = s.place(&cands, 0.5, 1.2, 0.5) {
                    let yaw = s.rng.range(0.0, TAU);
                    s.plan.props.push(Prop {
                        asset,
                        position: p,
                        yaw,
                        height: h,
                        radius: 0.5,
                    });
                }
            }
        }
        Theme::Lair => {
            bone_piles(s, &corners, 4, 0.5, 0.85);
            webs(s, &corners, 3);
            for k in 0..2 {
                let cn = &corners[(k * 2 + s.rng.index(2)) % corners.len()];
                if let Some(q) = spot(s, cn.p + cn.into * 1.6, 0.8) {
                    s.piece(move |m, _g, ctx| nest(m, q, 0.65, &mut ctx.rng));
                }
            }
            for _ in 0..3 {
                let f = depth * s.rng.range(-0.6, 0.6);
                let rr = width * s.rng.range(-0.6, 0.6);
                if let Some(q) = spot(s, s.at(f, rr), 0.5) {
                    let yaw = s.rng.range(0.0, TAU);
                    s.piece(move |m, _g, ctx| remains(m, q, yaw, &mut ctx.rng));
                }
            }
            for name in ["blade", "axe", "shield"] {
                let f = depth * s.rng.range(-0.5, 0.5);
                let rr = width * s.rng.range(-0.55, 0.55);
                let Some(q) = spot(s, s.at(f, rr), 0.4) else {
                    continue;
                };
                let yaw = s.rng.range(0.0, TAU);
                let h = s.rng.range(0.58, 0.78);
                s.plan.weapons.push(Weapon {
                    name,
                    position: q + Y(if name == "shield" { 0.13 } else { 0.14 }),
                    height: h,
                    rotation: v(0.08, yaw, if name == "axe" { 0.78 } else { 1.12 }),
                });
            }
            for _ in 0..2 {
                let cn = &corners[s.rng.index(corners.len())];
                if let Some(q) = spot(s, cn.p + cn.into * 1.1, 0.7) {
                    let yaw = s.rng.range(0.0, TAU);
                    s.piece(move |m, _g, ctx| rubble(m, q, 0.6, yaw, ctx.stone, &mut ctx.rng));
                }
            }
            light(s, c + Y(2.6), [0.5, 0.9, 0.3], 1.6, 9.0);
            s.plan.sparkle_tint = Some([0.6, 1.2, 0.4]);
            sparkle(s, 18, 0.3, 2.4);
        }
        Theme::Seal => {
            let g = s.floor(c);
            let r = room.half[0].min(room.half[1]) * 0.5;
            s.plan
                .pulse(move |_m, glow, _ctx| rune_circle(glow, g, r, [0.66, 0.4, 1.0], 8));
            let colors: [C3; 3] = [[0.66, 0.4, 1.0], [0.3, 0.9, 0.85], [1.0, 0.78, 0.3]];
            let focus = s.plan.focus;
            let seals: Vec<Vec3> = map
                .spawns
                .iter()
                .filter(|sp| sp.room == room.id && sp.kind == SpawnKind::Seal)
                .map(|sp| sp.position)
                .collect();
            for (k, at) in seals.into_iter().enumerate() {
                let p = s.floor(at);
                let (yaw, color, glyph) = (yaw_toward(p, c), colors[k % 3], k % 3);
                s.piece(move |m, glow, ctx| sigil_stone(m, glow, p, yaw, ctx.stone, color, glyph));
                // Seal spawns are fixed by the generator: a stone that stands
                // on a doorway's approach to the focus stays walk-through.
                let on_approach = room.sockets.iter().any(|sk| {
                    let start = v(
                        sk.position.x - sk.outward.x * 0.6,
                        sk.position.y,
                        sk.position.z - sk.outward.z * 0.6,
                    );
                    let d = hz(focus - start);
                    let l = d.length();
                    if l < 1e-6 {
                        return hdist(start, p) < 1.2;
                    }
                    let u = d * (1.0 / l);
                    let along = hz(p - start).dot(u).clamp(0.0, l);
                    hdist(start + u * along, p) < 0.45 + 0.75
                });
                if !on_approach {
                    s.plan.solids.push((p, 0.45));
                }
                s.placer.taken.push((p, 0.6));
                light(s, p + Y(1.3), color, 1.8, 4.0);
            }
            for cn in &corners {
                if let Some(q) = spot(s, cn.p + cn.into * 1.3, 0.25) {
                    s.piece(move |m, g, ctx| candles(m, g, q, 5, 0.24, &mut ctx.rng));
                    light(s, q + Y(0.6), [1.0, 0.7, 0.4], 0.9, 3.0);
                }
            }
            s.plan.sparkle_tint = Some([0.9, 0.55, 1.5]);
            sparkle(s, 22, 0.4, 3.0);
        }
        Theme::Memory => {
            let focus = s.plan.focus;
            let to_entry = norm(hz(s.entry - focus));
            let base = s.floor(focus);
            let yaw = to_entry.x.atan2(to_entry.z);
            s.piece(move |m, glow, ctx| monolith(m, glow, base, yaw, ctx.stone, [0.46, 0.7, 1.0]));
            s.plan.solids.push((focus, 0.75));
            s.placer.taken.push((focus, 0.9));
            light(s, focus + Y(1.6), [0.46, 0.7, 1.0], 2.4, 6.0);
            for k in 0..5 {
                let a = to_entry.z.atan2(to_entry.x) + PI * 0.35 + (k as f32 / 4.0) * PI * 1.3;
                let cands: Vec<Vec3> = [2.8, 3.3, 2.4]
                    .iter()
                    .map(|&r| focus + v(a.cos() * r, 0.0, a.sin() * r))
                    .collect();
                let Some(p) = s.place(&cands, 0.45, 1.4, 0.6) else {
                    continue;
                };
                let yaw = yaw_toward(p, focus);
                s.piece(move |m, _g, ctx| statue(m, p, yaw, ctx.stone));
                s.plan.solids.push((p, 0.42));
                if let Some(q) = spot(s, p + norm(hz(focus - p)) * 0.75, 0.2) {
                    s.piece(move |m, g, ctx| candles(m, g, q, 3, 0.14, &mut ctx.rng));
                }
            }
            for _ in 0..4 {
                let f = depth * s.rng.range(-0.6, 0.6);
                let rr = width * s.rng.range(-0.6, 0.6);
                if let Some(q) = spot(s, s.at(f, rr), 0.3) {
                    let scale = s.rng.range(0.7, 1.1);
                    s.piece(move |m, _g, ctx| fern(m, q, scale, &mut ctx.rng));
                }
            }
            s.plan.sparkle_tint = Some([0.6, 0.95, 1.6]);
            sparkle(s, 26, 0.4, 3.4);
        }
        Theme::Threshold => {
            let side = right * if s.rng.chance(0.5) { 1.0 } else { -1.0 };
            let cands = [
                c + side * (width * 0.5) + inward * (depth * 0.1),
                c + side * (width * 0.45) - inward * (depth * 0.2),
            ];
            let fire_at = s.place(&cands, 0.6, 2.2, 0.85);
            if let Some(fire_at) = fire_at {
                s.piece(move |m, g, ctx| campfire(m, g, fire_at, &mut ctx.rng));
                s.plan.solids.push((fire_at, 0.5));
                flame(s, fire_at + Y(0.22), [1.0, 0.56, 0.2], 2.2);
                light(s, fire_at + Y(0.7), [1.0, 0.58, 0.26], 5.5, 9.0);
                for a in [0.7f32, -1.9] {
                    if let Some(q) = spot(s, fire_at + v(a.cos() * 1.3, 0.0, a.sin() * 1.3), 0.5) {
                        let yaw = yaw_toward(q, fire_at);
                        let color = if a > 0.0 {
                            [0.4, 0.22, 0.14]
                        } else {
                            [0.2, 0.26, 0.34]
                        };
                        s.piece(move |m, _g, _ctx| bedroll(m, q, yaw, color));
                    }
                }
            }
            let cn = corners
                .iter()
                .find(|cn| fire_at.is_some_and(|f| hdist(cn.p, f) < width))
                .unwrap_or(&corners[0]);
            let cands = [cn.p + cn.into * 2.0, cn.p + cn.into * 2.5];
            if let Some(tent_at) = s.place(&cands, 1.2, 1.8, 0.6) {
                let yaw = yaw_toward(tent_at, c);
                s.piece(move |m, _g, _ctx| tent(m, tent_at, yaw, [0.62, 0.5, 0.32]));
                s.plan.solids.push((tent_at, 1.05));
            }
            for (asset, h) in [
                ("crates_stacked", 1.1),
                ("barrel_small", 0.6),
                ("box_small", 0.5),
                ("trunk_large", 0.6),
            ] {
                let cn2 = &corners[s.rng.index(corners.len())];
                let cands = [cn2.p + cn2.into * 1.2, cn2.p + cn2.into * 1.7];
                if let Some(p) = s.place(&cands, 0.5, 1.6, 0.5) {
                    let yaw = s.rng.range(0.0, TAU);
                    s.plan.props.push(Prop {
                        asset,
                        position: p,
                        yaw,
                        height: h,
                        radius: 0.5,
                    });
                }
            }
            let top = with_y(c + side * (-width * 0.4), c.y + height);
            s.piece(move |m, g, ctx| lantern(m, g, top, 1.4, &mut ctx.rng));
            light(
                s,
                c + side * (-width * 0.4) + Y(height - 1.8),
                [1.0, 0.66, 0.3],
                2.2,
                6.0,
            );
        }
        Theme::Return => {
            let g = s.floor(c);
            let color = [0.31, 0.89, 0.82];
            s.plan
                .pulse(move |_m, glow, _ctx| rune_circle(glow, g, 2.3, color, 6));
            for k in 0..4 {
                let a = (k as f32 / 4.0) * TAU + PI / 4.0;
                let cands: Vec<Vec3> = [3.2, 3.6, 2.8]
                    .iter()
                    .map(|&r| c + v(a.cos() * r, 0.0, a.sin() * r))
                    .collect();
                let Some(p) = s.place(&cands, 0.35, 1.2, 0.6) else {
                    continue;
                };
                s.piece(move |m, glow, ctx| pylon(m, glow, p, ctx.stone, color, &mut ctx.rng));
                s.plan.solids.push((p, 0.32));
                light(s, p + Y(1.3), color, 1.6, 4.5);
            }
            let cands: Vec<Vec3> = [1.3, 1.8, 2.4]
                .iter()
                .map(|&d| c + inward * (depth - d))
                .collect();
            if let Some(far) = s.place(&cands, 0.9, 1.6, 0.8) {
                let center = far + Y(2.1);
                s.piece(move |m, glow, ctx| {
                    portal_ring(m, glow, center, -inward, 1.6, ctx.stone, color)
                });
                s.plan.solids.push((far, 0.9));
            }
            s.plan.sparkle_tint = Some([0.5, 1.5, 1.4]);
            sparkle(s, 20, 0.4, 3.0);
        }
        _ => {}
    }
}
