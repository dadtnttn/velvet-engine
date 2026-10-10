//! Themed rooms (web prototype `vaults.js` theme plans + `themes.js`): every
//! chamber that is not a signature vault fills a plan with pieces, props,
//! lights and solids, which the scene builds, lights and collides.
use super::super::themes::{self, Theme};
use super::kit::*;
use super::plan::*;
use super::stage::Stage;
use velvet_dungeon::{gameplay::SpawnKind, Blueprint, Room, Shape};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

/// Where a themed room's focus sits (its spawn, or its centre).
pub fn focus(map: &Blueprint, room: &Room) -> Vec3 {
    map.spawns
        .iter()
        .find(|s| {
            s.room == room.id
                && matches!(
                    s.kind,
                    SpawnKind::Memory
                        | SpawnKind::Entrance
                        | SpawnKind::Exit
                        | SpawnKind::Boss
                        | SpawnKind::Miniboss
                )
        })
        .map(|s| s.position)
        .unwrap_or(room.center)
}

pub fn is_gallery(theme: Theme) -> bool {
    matches!(
        theme,
        Theme::Crypt
            | Theme::Mine
            | Theme::Fungal
            | Theme::Forge
            | Theme::Library
            | Theme::Overgrowth
    )
}

/// Themes whose art has been ported; the others keep the older dressing.
pub fn ported(_theme: Theme) -> bool {
    true
}

/// The web prototype's plan for any dressed room: a signature chamber
/// (treasure, secret, rest) or a themed room.
pub fn any(map: &Blueprint, room: &Room) -> Option<Plan> {
    if let Some(plan) = super::vaults::plan(map, room) {
        return Some(plan);
    }
    plan(map, room).map(|(_, plan)| plan)
}

/// Plan a ported themed room, or `None` (signature vaults, bridges and
/// themes still dressed the old way).
pub fn plan(map: &Blueprint, room: &Room) -> Option<(Theme, Plan)> {
    let theme = themes::of(map, room)?;
    if !ported(theme) {
        return None;
    }
    plan_with(map, room, true)
}

pub fn plan_with(map: &Blueprint, room: &Room, incidents: bool) -> Option<(Theme, Plan)> {
    let theme = themes::of(map, room)?;
    if room.shape == Shape::Bridge || room.sockets.is_empty() {
        return None;
    }
    let focus = focus(map, room);
    let (inward, _, _, _) = frame(room);
    let entry = room.sockets[0].position;
    let mut plan = Plan::new(focus, inward);
    let rng = ArtRng::stream(map.seed, "room_theme_v1", room.id as u64);
    let placer = Placer::new(map, room, focus);
    let placer = {
        let mut stage = Stage::new(map, room, &mut plan, placer, rng, entry);
        match theme {
            Theme::Mine => super::mine::plan(&mut stage),
            Theme::Forge => super::forge::plan(&mut stage),
            Theme::Crypt => super::crypt::plan(&mut stage),
            Theme::Library => super::library::plan(&mut stage),
            Theme::Fungal => super::fungal::plan(&mut stage),
            Theme::Overgrowth => super::overgrowth::plan(&mut stage),
            role => super::roles::plan(&mut stage, role),
        }
        (std::mem::take(&mut stage.placer.taken), stage.rng)
    };
    if incidents {
        // A fresh stage over the same plan, placer and plan stream (web
        // `planIncidents(new Stage(t), ...)`).
        let (taken, rng) = placer;
        let mut placer = Placer::new(map, room, focus);
        placer.taken = taken;
        let mut stage = Stage::new(map, room, &mut plan, placer, rng, entry);
        let stream = ArtRng::stream(map.seed, "room_incidents_v1", room.id as u64);
        super::incidents::plan(&mut stage, theme, stream, themes::condition(map, room));
    }
    Some((theme, plan))
}

/// Build a plan's static art: (lit detail, emissive glow, water surfaces).
pub fn build(
    plan: &Plan,
    stone: C3,
    glow_color: C3,
    seed: u64,
    room_id: usize,
) -> (Mesh3D, Mesh3D, Vec<Pond>) {
    let mut m = empty();
    let mut glow = empty();
    let mut ctx = Ctx {
        stone,
        glow_color,
        rng: ArtRng::stream(seed, "treasury_art_v1", room_id as u64),
        wet: vec![],
    };
    for piece in &plan.pieces {
        piece(&mut m, &mut glow, &mut ctx);
    }
    for &(centre, radius, height) in &plan.hoards {
        super::treasury::hoard(&mut m, &mut glow, centre, radius, height, &mut ctx.rng);
    }
    for &(base, relic, yaw) in &plan.niche_relics {
        super::treasury::altar(
            &mut m,
            &mut glow,
            base,
            yaw,
            relic,
            [0.46, 0.44, 0.42],
            &mut ctx.rng,
        );
    }
    ground_occlusion(&mut m, plan.focus.y);
    (m, glow, ctx.wet)
}

/// Build the pieces that beat with the Heart (web `world.js`: their own art
/// stream, the room id offset by 7919).
pub fn build_pulse(
    plan: &Plan,
    stone: C3,
    glow_color: C3,
    seed: u64,
    room_id: usize,
) -> (Mesh3D, Mesh3D) {
    let mut m = empty();
    let mut glow = empty();
    let mut ctx = Ctx {
        stone,
        glow_color,
        rng: ArtRng::stream(seed, "treasury_art_v1", room_id as u64 + 7919),
        wet: vec![],
    };
    for piece in &plan.pulse_pieces {
        piece(&mut m, &mut glow, &mut ctx);
    }
    ground_occlusion(&mut m, plan.focus.y);
    (m, glow)
}
