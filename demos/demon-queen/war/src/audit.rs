//! Every battle map played out, as the campaign would fight it: the AI on both sides, in attack (her
//! army against the region's garrison) and in defence (the strongest Alliance army invading the
//! region she holds). For each: who wins, when and how, the losses, when the lines first meet, the
//! regiments that march without getting anywhere (and where), soldiers inside solid things or in
//! deep water, and a siege's works and engines.
//!
//! ```text
//! cargo test -p demon-queen-war --profile fast-release every_battle_map_is_played_out -- --ignored --nocapture
//! VELVET_WAR_AUDIT=vado,puente   only those maps
//! VELVET_WAR_AUDIT_OUT=<file>    the report as JSON
//! VELVET_WAR_AUDIT_MAPS=<dir>    a plan of each battle (PPM): solid ground, water, the deployment
//!                                zones, each regiment's way and where they stood marching
//! ```
use crate::ai::{BattleAi, Posture};
use crate::battle::{ArmySetup, Battle, Setup, SetupUnit};
use crate::data::{province, unit, Faction, Role};
use crate::rules::Campaign;
use crate::sim::{RState, S_ALIVE};

/// What one battle showed.
#[derive(Debug, Default, serde::Serialize)]
pub struct Played {
    pub map: String,
    pub attacks: bool,
    pub winner: Option<u8>,
    pub end: f32,
    /// her side's and the enemy's share of men lost
    pub lost: [f32; 2],
    /// when two enemy regiments first fought
    pub contact: Option<f32>,
    /// regiment-seconds spent marching without moving, and where it happened (x, z, seconds)
    pub stuck: f32,
    pub stuck_at: Vec<(f32, f32, f32)>,
    /// the most soldiers at once inside something solid, or in water deeper than a man
    pub blocked: usize,
    pub drowning: usize,
    /// a siege's works broken, engines built and standing at the end, openings, keep held (s)
    pub works_broken: usize,
    pub engines: usize,
    pub engines_standing: usize,
    pub openings: usize,
    pub held: f32,
    pub wall_ms: u128,
}

/// Her army and the enemy's for a region's battle: attacking its garrison, or holding it against
/// the Alliance.
fn armies(map: &str, attacks: bool) -> (Vec<SetupUnit>, ArmySetup) {
    let region = crate::baked::BATTLE_MAPS
        .iter()
        .find(|(m, _)| *m == map)
        .map_or("vado", |(_, r)| *r);
    let p = province(region);
    let c = Campaign::new(7, true);
    let strongest = |f: Faction| {
        c.armies
            .iter()
            .filter(|a| a.faction == f)
            .max_by_key(|a| a.units.iter().map(|u| u.men).sum::<u32>())
            .map(|a| a.units.iter().map(|u| u.ty.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let to_units = |list: &[String]| -> Vec<SetupUnit> {
        list.iter()
            .enumerate()
            .map(|(k, t)| SetupUnit {
                ty: t.clone(),
                men: (unit(t).role == Role::Hero).then_some(1),
                id: Some(format!("u{k}")),
            })
            .collect()
    };
    let queen = strongest(Faction::Reina);
    let garrison: Vec<String> = p.garrison.iter().map(|t| (*t).to_owned()).collect();
    if attacks {
        // her army against the region's own people (or the Alliance where she holds it)
        let (faction, foe) = if p.owner == Faction::Reina || garrison.is_empty() {
            (Faction::Alianza, strongest(Faction::Alianza))
        } else {
            (p.owner, garrison)
        };
        (
            to_units(&queen),
            ArmySetup {
                faction,
                units: to_units(&foe),
            },
        )
    } else {
        // the Alliance invading: her army and the region's garrison hold it
        let mut mine = queen;
        if p.owner == Faction::Reina {
            mine.extend(garrison);
        }
        mine.truncate(crate::data::MAX_UNITS + 2);
        (
            to_units(&mine),
            ArmySetup {
                faction: Faction::Alianza,
                units: to_units(&strongest(Faction::Alianza)),
            },
        )
    }
}

pub fn play(map: &str, attacks: bool) -> Played {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/guerra/batallas");
    crate::baked::ensure(map, &root).expect("the map");
    let (mine, enemy) = armies(map, attacks);
    let setup = Setup {
        map: map.to_owned(),
        title: map.to_owned(),
        kicker: String::new(),
        subtitle: String::new(),
        player: ArmySetup {
            faction: Faction::Reina,
            units: mine,
        },
        enemy,
        player_attacks: attacks,
        enemy_posture: Some(if attacks {
            Posture::Defend
        } else {
            Posture::Attack
        }),
        seed: 11,
        dimming: true,
        deploy: true,
    };
    let mut b = Battle::new(setup);
    // her side sets up and fights as the enemy general would
    if let Some(mut s) = b.sim.siege.take() {
        s.ai_setup(&mut b.sim, 0, b.zone);
        b.sim.siege = Some(s);
    }
    b.auto_player = Some(BattleAi::new(
        0,
        if attacks {
            Posture::Attack
        } else {
            Posture::Defend
        },
    ));
    let engines0 = b.sim.siege.as_ref().map_or(0, |s| s.engines.len());
    b.begin();
    let t0 = std::time::Instant::now();
    let mut out = Played {
        map: map.to_owned(),
        attacks,
        ..Played::default()
    };
    let men0: [u32; 2] = [0, 1].map(|s| {
        b.sim
            .regiments
            .iter()
            .filter(|r| r.side == s)
            .map(|r| r.men0)
            .sum()
    });
    let step = 5.0;
    let mut last: Vec<(f32, f32)> = b.sim.regiments.iter().map(|r| (r.cx, r.cz)).collect();
    let mut trails: Vec<Vec<(f32, f32)>> = last.iter().map(|p| vec![*p]).collect();
    let mut cells: std::collections::HashMap<(i32, i32), f32> = Default::default();
    while b.sim.over.is_none() && b.sim.time < 16.5 * 60.0 {
        b.skip(step, false);
        if out.contact.is_none() && b.sim.regiments.iter().any(|r| r.engaged > 0) {
            out.contact = Some(b.sim.time);
        }
        for (k, r) in b.sim.regiments.iter().enumerate() {
            let (x, z) = last[k];
            let moved = (r.cx - x).hypot(r.cz - z);
            if r.alive > 0 && r.state == RState::Moving && r.engaged == 0 && moved < 0.5 {
                out.stuck += step;
                *cells
                    .entry(((r.cx / 10.0).round() as i32, (r.cz / 10.0).round() as i32))
                    .or_default() += step;
            }
            last[k] = (r.cx, r.cz);
            if r.alive > 0 {
                trails[k].push((r.cx, r.cz));
            }
        }
        let (mut blocked, mut drowning) = (0, 0);
        for i in 0..b.sim.n {
            if b.sim.state[i] != S_ALIVE || b.sim.regiments[b.sim.reg[i]].ty.flyer {
                continue;
            }
            if b.sim.field.blocked(b.sim.px[i], b.sim.pz[i]) {
                blocked += 1;
            }
            // (on a bridge's deck the water runs below her feet)
            if b.sim.field.water_y(b.sim.px[i], b.sim.pz[i])
                .is_some_and(|w| b.sim.py[i] < w - crate::field::DEEP_BAKED - 0.05)
            {
                drowning += 1;
            }
        }
        out.blocked = out.blocked.max(blocked);
        out.drowning = out.drowning.max(drowning);
    }
    out.winner = b.sim.over.map(|o| o.winner);
    out.end = b.sim.time;
    for s in 0..2u8 {
        let left: u32 = b
            .sim
            .regiments
            .iter()
            .filter(|r| r.side == s)
            .map(|r| r.alive)
            .sum();
        out.lost[s as usize] = 1.0 - left as f32 / men0[s as usize].max(1) as f32;
    }
    let mut spots: Vec<_> = cells.into_iter().collect();
    spots.sort_by(|a, b| b.1.total_cmp(&a.1));
    out.stuck_at = spots
        .into_iter()
        .take(4)
        .map(|((x, z), t)| (x as f32 * 10.0, z as f32 * 10.0, t))
        .collect();
    if let Some(s) = b.sim.siege.as_ref() {
        out.works_broken = s.works.iter().filter(|w| w.broken).count();
        out.engines = s.engines.len().saturating_sub(engines0);
        out.engines_standing = s.engines.iter().filter(|e| e.alive).count();
        out.openings = s.openings.len();
        out.held = s.hold;
    }
    out.wall_ms = t0.elapsed().as_millis();
    if let Ok(dir) = std::env::var("VELVET_WAR_AUDIT_MAPS") {
        let sides: Vec<u8> = b.sim.regiments.iter().map(|r| r.side).collect();
        plan(&b, &trails, &sides, &out, std::path::Path::new(&dir));
    }
    out
}

/// A plan of the battle, a pixel a metre: free ground shaded by height, solid ground black, water
/// blue (deep darker), the zones' outlines (hers green, theirs red), each regiment's way (hers
/// violet, theirs amber) and red marks where they marched without moving.
fn plan(b: &Battle, trails: &[Vec<(f32, f32)>], sides: &[u8], p: &Played, dir: &std::path::Path) {
    let f = &b.sim.field;
    let (w, h) = (f.w.ceil() as usize, f.d.ceil() as usize);
    let mut px = vec![[0u8; 3]; w * h];
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for z in 0..h {
        for x in 0..w {
            let y = f.height_at(f.x0 + x as f32, f.z0 + z as f32);
            lo = lo.min(y);
            hi = hi.max(y);
        }
    }
    for z in 0..h {
        for x in 0..w {
            let (wx, wz) = (f.x0 + x as f32 + 0.5, f.z0 + z as f32 + 0.5);
            let y = f.height_at(wx, wz);
            let k = ((y - lo) / (hi - lo).max(1.0) * 120.0) as u8;
            let depth = f.depth_at(wx, wz);
            px[z * w + x] = if f.blocked(wx, wz) {
                [20, 18, 22]
            } else if depth > 1.2 {
                [30, 60, 140]
            } else if depth > 0.05 {
                [90, 140, 200]
            } else {
                [70 + k, 80 + k, 60 + k]
            };
        }
    }
    let mut dot = |x: f32, z: f32, r: i32, c: [u8; 3]| {
        let (cx, cz) = ((x - f.x0) as i32, (z - f.z0) as i32);
        for dz in -r..=r {
            for dx in -r..=r {
                let (ix, iz) = (cx + dx, cz + dz);
                if dx * dx + dz * dz <= r * r && ix >= 0 && iz >= 0 && (ix as usize) < w && (iz as usize) < h {
                    px[iz as usize * w + ix as usize] = c;
                }
            }
        }
    };
    for (zone, c) in [(b.zone, [60u8, 220, 90]), (b.enemy_zone, [230, 60, 50])] {
        let steps = ((zone.x1 - zone.x0).abs() + (zone.z1 - zone.z0).abs()) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            let x = zone.x0 + (zone.x1 - zone.x0) * t;
            let z = zone.z0 + (zone.z1 - zone.z0) * t;
            dot(x, zone.z0, 0, c);
            dot(x, zone.z1, 0, c);
            dot(zone.x0, z, 0, c);
            dot(zone.x1, z, 0, c);
        }
    }
    for (k, t) in trails.iter().enumerate() {
        let c = if sides[k] == 0 { [190, 120, 255] } else { [255, 180, 60] };
        for pair in t.windows(2) {
            let ((ax, az), (bx, bz)) = (pair[0], pair[1]);
            let n = ((bx - ax).hypot(bz - az) as i32).max(1);
            for i in 0..=n {
                let s = i as f32 / n as f32;
                dot(ax + (bx - ax) * s, az + (bz - az) * s, 0, c);
            }
        }
        if let Some(&(x, z)) = t.last() {
            dot(x, z, 2, c);
        }
    }
    for &(x, z, secs) in &p.stuck_at {
        dot(x, z, (2.0 + secs.sqrt() * 0.4) as i32, [255, 30, 30]);
    }
    let _ = std::fs::create_dir_all(dir);
    let name = format!("{}_{}.ppm", p.map, if p.attacks { "ataque" } else { "defensa" });
    let mut bytes = format!("P6
{w} {h}
255
").into_bytes();
    for c in px {
        bytes.extend_from_slice(&c);
    }
    let _ = std::fs::write(dir.join(name), bytes);
}

#[test]
#[ignore]
fn every_battle_map_is_played_out() {
    let only = std::env::var("VELVET_WAR_AUDIT").ok();
    let maps: Vec<&str> = crate::baked::BATTLE_MAPS
        .iter()
        .map(|(m, _)| *m)
        .filter(|m| only.as_deref().is_none_or(|o| o.split(',').any(|x| x == *m)))
        .collect();
    let mut all = Vec::new();
    for map in maps {
        for attacks in [true, false] {
            let p = play(map, attacks);
            eprintln!(
                "{:<17} {:<7} winner {:<4} at {:>4.0}s  lost {:>3.0}%/{:>3.0}%  contact {:>5}  stuck {:>4.0}s {:?}  blocked {} water {}  siege broken {} engines +{} ({} up) openings {} held {:.0}  [{} ms]",
                p.map,
                if p.attacks { "ataque" } else { "defensa" },
                match p.winner {
                    Some(0) => "ella",
                    Some(_) => "ene",
                    None => "-",
                },
                p.end,
                p.lost[0] * 100.0,
                p.lost[1] * 100.0,
                p.contact.map_or("-".to_owned(), |t| format!("{t:.0}s")),
                p.stuck,
                p.stuck_at.iter().map(|(x, z, t)| format!("({x:.0},{z:.0}) {t:.0}s")).collect::<Vec<_>>(),
                p.blocked,
                p.drowning,
                p.works_broken,
                p.engines,
                p.engines_standing,
                p.openings,
                p.held,
                p.wall_ms
            );
            all.push(p);
        }
    }
    if let Ok(path) = std::env::var("VELVET_WAR_AUDIT_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&all).unwrap()).unwrap();
    }
}

/// A whole campaign played: the Queen recruits two regiments a turn where she can, wakes a free
/// people when her spark allows, and sends each army at the weakest neighbour it beats on paper;
/// the battles are reckoned. What it shows: when each region changed hands, how often the others
/// invaded her, her purse along the way, and whether anyone won.
///
/// ```text
/// cargo test -p demon-queen-war --profile fast-release a_campaign_is_played_to_its_end -- --ignored --nocapture
/// ```
#[test]
#[ignore]
fn a_campaign_is_played_to_its_end() {
    use crate::data::PROVINCES;
    use crate::rules::MoveResult;
    for seed in [42u32, 7, 1234] {
        let mut s = Campaign::new(seed, true);
        let (mut won, mut lost, mut invaded, mut fell) = (0, 0, 0, 0);
        let mut events: Vec<String> = Vec::new();
        let owners = |s: &Campaign| {
            [Faction::Reina, Faction::Alianza, Faction::Rebeldes, Faction::Libres]
                .map(|f| PROVINCES.iter().filter(|p| s.owner(p.id) == f).count())
        };
        for _ in 0..150 {
            if s.winner.is_some() {
                break;
            }
            let before: Vec<Faction> = PROVINCES.iter().map(|p| s.owner(p.id)).collect();
            // her turn
            // the castle first, then the rest of her land
            let mut mine: Vec<&str> = PROVINCES
                .iter()
                .filter(|p| s.owner(p.id) == Faction::Reina)
                .map(|p| p.id)
                .collect();
            mine.sort_by_key(|p| *p != "castillo");
            let mut bought = 0;
            for p in &mine {
                for o in s.recruit_options(p, Faction::Reina) {
                    if o.ok && bought < 2 && s.recruit(p, o.ty, Faction::Reina).is_some() {
                        bought += 1;
                    }
                }
            }
            if let Some(p) = PROVINCES.iter().find(|p| s.awaken_blocked(p.id).is_none()) {
                s.awaken(p.id);
            }
            let ids: Vec<String> = s
                .armies
                .iter()
                .filter(|a| a.faction == Faction::Reina && !a.garrison)
                .map(|a| a.id.clone())
                .collect();
            // a threat within two steps of the castle keeps her own army home
            let threat = PROVINCES.iter().any(|p| {
                crate::rules::hops(p.id, "castillo") <= 2
                    && s.enemies_at(p.id, Faction::Reina).any(|e| e.faction != Faction::Libres)
            });
            for id in ids {
                let Some(a) = s.army(&id) else {
                    continue;
                };
                if threat && a.has("reina") {
                    if a.at != "castillo" && s.reachable(a).contains(&"castillo") {
                        s.move_army(&id, "castillo");
                    }
                    continue;
                }
                let power = a.power() / 1.3;
                let pick = s
                    .reachable(a)
                    .into_iter()
                    .filter(|p| s.owner(p) != Faction::Reina)
                    .map(|p| {
                        let foe: f32 = s.enemies_at(p, Faction::Reina).map(|e| e.power()).sum();
                        (p, foe)
                    })
                    .filter(|(_, foe)| *foe < power * 0.9)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((p, _)) = pick {
                    if let MoveResult::Battle(b) = s.move_army(&id, p) {
                        let res = s.auto_resolve(&b);
                        if res.winner == 0 {
                            won += 1;
                        } else {
                            lost += 1;
                        }
                        s.apply_battle(&b, &res, 0, 1.0);
                    }
                }
            }
            // the others' turn
            s.end_player_turn();
            loop {
                let (_, battle) = s.run_queue();
                let Some(b) = battle else {
                    break;
                };
                let on_her = s
                    .army(&b.defender)
                    .is_some_and(|d| d.faction == Faction::Reina);
                let res = s.auto_resolve(&b);
                if on_her {
                    invaded += 1;
                    if res.winner == 0 {
                        fell += 1;
                    }
                }
                s.apply_battle(&b, &res, 0, 1.0);
            }
            s.start_turn();
            for (k, p) in PROVINCES.iter().enumerate() {
                let now = s.owner(p.id);
                if now != before[k] {
                    events.push(format!("t{} {} {:?}→{:?}", s.turn, p.id, before[k], now));
                }
            }
            if s.turn % 15 == 0 {
                let f = s.purse(Faction::Reina);
                eprintln!(
                    "  seed {seed} turn {:>3}: regions R/A/Reb/L {:?}  her gold {} food {} spark {}  armies {}",
                    s.turn,
                    owners(&s),
                    f.gold,
                    f.food,
                    f.spark,
                    s.armies.iter().filter(|a| a.faction == Faction::Reina).count()
                );
            }
        }
        eprintln!(
            "seed {seed}: turn {} winner {:?} · her battles won {won} lost {lost} · invaded {invaded} times, {fell} lost · regions {:?}",
            s.turn,
            s.winner,
            owners(&s)
        );
        for e in events.iter().take(60) {
            eprintln!("    {e}");
        }
    }
}

/// Every battle map lets its armies meet once what hands can break is broken (gates, palisades):
/// either deployment zone walks to the other. (The bake once left a seam across a drawbridge and
/// a gatehouse's vault as solid rock, and the attackers never arrived.)
#[test]
fn every_battle_map_lets_its_armies_meet() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/guerra/batallas");
    if !root.is_dir() {
        return;
    }
    let mut bad = Vec::new();
    for &(map, _) in crate::baked::BATTLE_MAPS.iter() {
        if crate::baked::ensure(map, &root).is_none() {
            continue;
        }
        let mut setup = Setup::scenario("vado", 0.3);
        setup.map = map.to_owned();
        setup.player_attacks = true;
        let mut b = Battle::new(setup);
        let breakable: Vec<usize> = b
            .sim
            .siege
            .as_ref()
            .map(|s| {
                s.works
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| {
                        matches!(
                            w.kind,
                            crate::baked::WorkKind::Gate | crate::baked::WorkKind::Palisade
                        )
                    })
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        for w in breakable {
            b.sim.field.set_open(w, true);
        }
        let mid = |r: crate::maps::Rect| ((r.x0 + r.x1) / 2.0, (r.z0 + r.z1) / 2.0);
        let (a, d) = (mid(b.zone), mid(b.enemy_zone));
        if !b.sim.field.reachable(a.0, a.1, d.0, d.1) {
            bad.push(map);
        }
    }
    assert!(bad.is_empty(), "armies that cannot meet on {bad:?}");
}
