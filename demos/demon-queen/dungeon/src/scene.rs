use super::{fp, hud::Hud, runes, visual, Dungeon};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use velvet_dungeon::*;
use velvet_math::{Mat4, Vec3};
use velvet_render::*;
const CACHE_LIMIT: usize = 48;
const CHEST_LOOT_REVEAL_PROGRESS: f32 = 0.68;

fn draw_view_arm(
    draws: &mut Vec<MeshDraw3D>,
    fist: MeshHandle,
    forearm: MeshHandle,
    upper_arm: MeshHandle,
    hand: Vec3,
    blade: Vec3,
    shoulder: Vec3,
    pole: Vec3,
    right: bool,
    roll: f32,
) {
    let grip = fp::grip_frame(hand, blade, shoulder, roll);
    let wrist = fp::wrist(grip, right);
    let (shoulder, elbow) = fp::arm_ik(shoulder, wrist, pole);
    let white = Material3D::rgb(1.0, 1.0, 1.0);
    for (mesh, transform) in [
        (upper_arm, fp::limb_transform(elbow, shoulder)),
        (forearm, fp::limb_transform(wrist, elbow)),
        (
            fist,
            grip * Mat4::from_scale(Vec3::new(fp::VIEW_SCALE, fp::VIEW_SCALE, fp::VIEW_SCALE)),
        ),
    ] {
        draws.push(MeshDraw3D {
            mesh,
            transform,
            material: white,
            texture: None,
        });
    }
}

/// A themed room's moving art on the GPU.
#[derive(Default)]
struct ThemeExtra {
    pulse_glow: Option<MeshHandle>,
    /// Heart flesh and glowing arteries.
    heart: Option<(Option<MeshHandle>, Option<MeshHandle>)>,
    /// Orrery rings (mesh, tilt, speed) and its sun.
    orrery: Option<(Vec<(MeshHandle, f32, f32)>, Option<MeshHandle>)>,
    /// The rest font's floating ember core and its two gilded rings.
    font_core: Option<(Option<MeshHandle>, Option<MeshHandle>, Option<MeshHandle>)>,
    moving_built: bool,
}

impl ThemeExtra {
    fn handles(self) -> Vec<MeshHandle> {
        let mut out: Vec<MeshHandle> = self.pulse_glow.into_iter().collect();
        if let Some((a, b)) = self.heart {
            out.extend(a);
            out.extend(b);
        }
        if let Some((rings, sun)) = self.orrery {
            out.extend(rings.into_iter().map(|r| r.0));
            out.extend(sun);
        }
        if let Some((core, a, b)) = self.font_core {
            out.extend(core);
            out.extend(a);
            out.extend(b);
        }
        out
    }
}

/// The model of a weapon the web prototype names by its KayKit short name.
pub fn weapon_asset(name: &str) -> &'static str {
    match name {
        "axe" => "skeleton_axe",
        "shield" => "skeleton_shield",
        _ => "skeleton_blade",
    }
}

fn chest_lid_openness(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    let shifted = t - 1.0;
    let spring = 1.0 + 2.70158 * shifted.powi(3) + 1.70158 * shifted.powi(2);
    let rattle = (t * std::f32::consts::TAU * 4.0).sin() * 0.025 * (1.0 - t / 0.2).clamp(0.0, 1.0);
    (spring + rattle).clamp(0.0, 1.08)
}

#[derive(Clone, Copy)]
struct EntryPortal {
    position: Vec3,
    outward: Vec3,
}

/// Picks a deterministic arrival doorway for every room by traversing from the
/// dungeon entrance. Treasure props can face the route taken into their room,
/// even when the generated graph contains optional loops.
fn room_entry_portals(map: &Blueprint) -> Vec<Option<EntryPortal>> {
    let mut entries = vec![None; map.rooms.len()];
    let mut seen = vec![false; map.rooms.len()];
    if map.entrance >= map.rooms.len() {
        return entries;
    }

    let mut queue = VecDeque::from([map.entrance]);
    seen[map.entrance] = true;
    while let Some(room_id) = queue.pop_front() {
        for (next, connection) in map.neighbors(room_id) {
            if next >= map.rooms.len() || seen[next] {
                continue;
            }
            let fallback = if next == connection.a {
                EntryPortal {
                    position: connection.start,
                    outward: (connection.end - connection.start).normalize_or_zero(),
                }
            } else {
                EntryPortal {
                    position: connection.end,
                    outward: (connection.start - connection.end).normalize_or_zero(),
                }
            };
            entries[next] = Some(
                map.rooms[next]
                    .sockets
                    .iter()
                    .find(|socket| socket.connection == connection.id)
                    .map(|socket| EntryPortal {
                        position: socket.position,
                        outward: socket.outward,
                    })
                    .unwrap_or(fallback),
            );
            seen[next] = true;
            queue.push_back(next);
        }
    }
    entries
}

fn chest_facing_yaw(position: Vec3, entry: EntryPortal) -> f32 {
    let toward_entry = horizontal(entry.position - position);
    if toward_entry.length_squared() > 0.001 {
        toward_entry.x.atan2(toward_entry.z)
    } else {
        entry.outward.x.atan2(entry.outward.z)
    }
}

fn chest_point(origin: Vec3, local_offset: Vec3, yaw: f32) -> Vec3 {
    origin + Mat4::from_rotation_y(yaw).transform_vector3(local_offset)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RoomIdentityKind {
    LooseCoin,
    CoinStackSmall,
    CoinStackMedium,
    CoinStackLarge,
    Gem,
    Crown,
    Corpse,
    Shield,
    Blade,
    Axe,
    Herb,
    GoldRelic,
    BrokenRelic,
    Banner,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RoomIdentityProp {
    kind: RoomIdentityKind,
    position: Vec3,
    yaw: f32,
    scale: f32,
    tint: [f32; 3],
}

fn segment_distance(point: Vec3, start: Vec3, end: Vec3) -> f32 {
    let direction = horizontal(end - start);
    let length_squared = direction.length_squared();
    let along = if length_squared > 0.001 {
        (horizontal(point - start).dot(direction) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    horizontal(point - (start + direction * along)).length()
}

/// Sample dressing points independently from layout/gameplay streams. Every
/// floor prop keeps door clearance and the direct approach to the room's focus.
fn sample_room_identity_point(
    room: &Room,
    rng: &mut rng::Rng,
    kind: RoomIdentityKind,
    origin: Vec3,
    min_radius: f32,
    max_radius: f32,
    occupied: &[Vec3],
    entry: Option<EntryPortal>,
    focus: Option<Vec3>,
    focus_clearance: f32,
) -> Option<Vec3> {
    let minimum_spacing = match kind {
        RoomIdentityKind::LooseCoin => 0.44,
        RoomIdentityKind::CoinStackSmall
        | RoomIdentityKind::CoinStackMedium
        | RoomIdentityKind::CoinStackLarge => 0.50,
        RoomIdentityKind::Gem | RoomIdentityKind::Crown | RoomIdentityKind::Shield => 0.28,
        RoomIdentityKind::Corpse => 0.62,
        _ => 0.42,
    };
    for _ in 0..96 {
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let radius = rng.range(min_radius, max_radius.max(min_radius));
        let mut point = origin + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        if !room.contains(point.x, point.z, 0.62)
            || room.shape == Shape::Bridge && room.floor_y(point.x, point.z) < room.center.y - 0.1
            || room
                .sockets
                .iter()
                .any(|socket| horizontal(socket.position - point).length() < 1.8)
            || occupied
                .iter()
                .any(|placed| horizontal(*placed - point).length() < minimum_spacing)
        {
            continue;
        }
        if focus.is_some_and(|target| horizontal(point - target).length() < focus_clearance) {
            continue;
        }
        if let (Some(entry), Some(target)) = (entry, focus) {
            if segment_distance(point, entry.position, target) < 0.95 {
                continue;
            }
        }
        point.y = room.floor_y(point.x, point.z);
        return Some(point);
    }
    None
}

fn add_room_identity_prop(
    props: &mut Vec<RoomIdentityProp>,
    occupied: &mut Vec<Vec3>,
    room: &Room,
    rng: &mut rng::Rng,
    kind: RoomIdentityKind,
    origin: Vec3,
    min_radius: f32,
    max_radius: f32,
    entry: Option<EntryPortal>,
    focus: Option<Vec3>,
    focus_clearance: f32,
    scale: f32,
    tint: [f32; 3],
) -> Option<Vec3> {
    let position = sample_room_identity_point(
        room,
        rng,
        kind,
        origin,
        min_radius,
        max_radius,
        occupied,
        entry,
        focus,
        focus_clearance,
    )?;
    occupied.push(position);
    props.push(RoomIdentityProp {
        kind,
        position,
        yaw: rng.range(0.0, std::f32::consts::TAU),
        scale,
        tint,
    });
    Some(position)
}

fn room_banner(room: &Room, entry: Option<EntryPortal>, lateral_offset: f32) -> RoomIdentityProp {
    let outward = entry
        .map(|portal| horizontal(portal.outward).normalize_or_zero())
        .filter(|direction| direction.length_squared() > 0.001)
        .unwrap_or(Vec3::new(0.0, 0.0, -1.0));
    let inward = -outward;
    let right = Vec3::new(inward.z, 0.0, -inward.x);
    let back_extent = (inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1]).max(1.0);
    let mut position = room.center + inward * (back_extent - 0.75) + right * lateral_offset;
    for _ in 0..4 {
        if room.contains(position.x, position.z, 0.05) {
            break;
        }
        position = position - inward * 0.35 - right * lateral_offset.signum() * 0.18;
    }
    position.y = room.floor_y(position.x, position.z) + 2.0;
    RoomIdentityProp {
        kind: RoomIdentityKind::Banner,
        position,
        yaw: super::enemies::facing_yaw(outward),
        scale: 2.0,
        tint: [1.0; 3],
    }
}

fn room_identity_plan(
    room: &Room,
    seed: u64,
    entry: Option<EntryPortal>,
    focus: Option<Vec3>,
) -> Vec<RoomIdentityProp> {
    // Treasury and arcane vaults are dressed by `vaults`.
    if room.shape == Shape::Bridge
        || matches!(room.role, Role::Treasure | Role::Secret | Role::Rest)
    {
        return Vec::new();
    }

    let mut rng = rng::Rng::stream(seed, "room_identity_v1", room.id as u64);
    let mut props = Vec::new();
    let mut occupied = Vec::new();
    let inward = entry
        .map(|portal| -horizontal(portal.outward).normalize_or_zero())
        .filter(|direction| direction.length_squared() > 0.001)
        .unwrap_or_else(|| {
            let angle = rng.range(0.0, std::f32::consts::TAU);
            Vec3::new(angle.cos(), 0.0, angle.sin())
        });
    let right = Vec3::new(inward.z, 0.0, -inward.x);
    let depth = (inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1]).max(1.0);
    let width = (right.x.abs() * room.half[0] + right.z.abs() * room.half[1]).max(1.0);
    let focus = focus.filter(|point| room.contains(point.x, point.z, 0.1));

    match room.role {
        Role::Treasure | Role::Secret => {
            let side_of_focus = focus
                .map(|point| (point - room.center).dot(right).signum())
                .filter(|side| side.abs() > 0.1)
                .unwrap_or(1.0);
            let corpse_focus = room.center + inward * (depth * 0.34).min(2.2)
                - right * side_of_focus * (width * 0.20).min(1.35);
            let corpse = if room.role == Role::Treasure || rng.chance(0.48) {
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Corpse,
                    corpse_focus,
                    0.0,
                    0.95,
                    entry,
                    focus,
                    1.45,
                    1.06,
                    [1.0; 3],
                )
            } else {
                None
            };
            let hoard = corpse.unwrap_or(corpse_focus);
            let flank_offset = (width * 0.34).min(2.25);
            let front_depth = (depth * 0.12).min(1.65);
            let left_spill = room.center + inward * front_depth + right * flank_offset;
            let right_spill = room.center + inward * (front_depth + 0.45) - right * flank_offset;

            if room.role == Role::Treasure {
                let second_corpse = add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Corpse,
                    left_spill,
                    0.0,
                    0.62,
                    entry,
                    focus,
                    1.10,
                    1.10,
                    [1.0; 3],
                );
                if second_corpse.is_none() {
                    add_room_identity_prop(
                        &mut props,
                        &mut occupied,
                        room,
                        &mut rng,
                        RoomIdentityKind::Corpse,
                        right_spill,
                        0.0,
                        0.62,
                        entry,
                        focus,
                        1.10,
                        1.10,
                        [1.0; 3],
                    );
                }
            }

            if corpse.is_some() {
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Crown,
                    hoard,
                    0.45,
                    0.95,
                    entry,
                    focus,
                    0.95,
                    1.20,
                    [1.0; 3],
                );
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Shield,
                    hoard,
                    0.65,
                    1.25,
                    entry,
                    focus,
                    0.95,
                    0.82,
                    [1.0; 3],
                );
            }

            // Three readable piles: the fallen guard's hoard, then two
            // flanking spills closer to the entry. The route and chest
            // interaction pocket remain deliberately clear.
            let stack_anchors = [hoard, left_spill, right_spill];
            let stack_kinds = [
                RoomIdentityKind::CoinStackLarge,
                RoomIdentityKind::CoinStackMedium,
                RoomIdentityKind::CoinStackLarge,
                RoomIdentityKind::CoinStackMedium,
                RoomIdentityKind::CoinStackLarge,
                RoomIdentityKind::CoinStackMedium,
                RoomIdentityKind::CoinStackSmall,
                RoomIdentityKind::CoinStackSmall,
                RoomIdentityKind::CoinStackSmall,
            ];
            for (index, kind) in stack_kinds.into_iter().enumerate() {
                let origin = stack_anchors[index % stack_anchors.len()];
                let (low, high) = if index % 3 == 0 {
                    (0.42, 1.45)
                } else {
                    (0.34, 1.18)
                };
                let scale = rng.range(0.24, 0.36);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    kind,
                    origin,
                    low,
                    high,
                    entry,
                    focus,
                    0.88,
                    scale,
                    [1.0; 3],
                );
            }

            let gem_colors = [
                [0.12, 0.82, 1.0],
                [0.38, 0.56, 1.0],
                [0.40, 1.0, 0.62],
                [0.86, 0.42, 1.0],
                [0.98, 0.78, 0.28],
            ];
            let gem_count = if room.role == Role::Treasure { 5 } else { 3 };
            for index in 0..gem_count {
                let origin = match index % 3 {
                    0 => hoard,
                    1 => left_spill,
                    _ => right_spill,
                };
                let scale = rng.range(0.26, 0.42);
                let color = gem_colors[rng.index(gem_colors.len())];
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Gem,
                    origin,
                    0.62,
                    2.30,
                    entry,
                    focus,
                    0.84,
                    scale,
                    color,
                );
            }

            let loose_coin_count = if room.role == Role::Treasure { 12 } else { 6 };
            for index in 0..loose_coin_count {
                let origin = match index % 3 {
                    0 => hoard,
                    1 => left_spill,
                    _ => right_spill,
                };
                let scale = rng.range(0.08, 0.13);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::LooseCoin,
                    origin,
                    0.25,
                    1.75,
                    entry,
                    focus,
                    0.78,
                    scale,
                    [1.0; 3],
                );
            }
        }
        Role::Rest => {
            let anchor = focus.unwrap_or(room.center);
            for _ in 0..4 {
                let scale = rng.range(0.38, 0.54);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Herb,
                    anchor,
                    1.15,
                    2.55,
                    entry,
                    focus,
                    0.88,
                    scale,
                    [0.66, 1.0, 0.72],
                );
            }
            for _ in 0..2 {
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Gem,
                    anchor,
                    1.25,
                    2.1,
                    entry,
                    focus,
                    0.92,
                    0.38,
                    [0.24, 1.0, 0.58],
                );
            }
        }
        Role::Event => {
            let anchor = focus.unwrap_or(room.center);
            for _ in 0..5 {
                let scale = rng.range(0.30, 0.48);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Gem,
                    anchor,
                    1.05,
                    2.40,
                    entry,
                    focus,
                    0.95,
                    scale,
                    [0.46, 0.70, 1.0],
                );
            }
        }
        Role::Encounter => {
            for (kind, radius) in [
                (RoomIdentityKind::Blade, (2.0, 3.4)),
                (RoomIdentityKind::Axe, (2.0, 3.5)),
                (RoomIdentityKind::Shield, (2.1, 3.3)),
            ] {
                let scale = rng.range(0.58, 0.78);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    kind,
                    room.center,
                    radius.0,
                    radius.1,
                    entry,
                    None,
                    0.0,
                    scale,
                    [1.0; 3],
                );
            }
        }
        Role::Elite => {
            props.push(room_banner(room, entry, 0.0));
            add_room_identity_prop(
                &mut props,
                &mut occupied,
                room,
                &mut rng,
                RoomIdentityKind::GoldRelic,
                room.center,
                2.0,
                3.4,
                entry,
                None,
                0.0,
                0.90,
                [1.0; 3],
            );
        }
        Role::Miniboss | Role::Boss => {
            props.push(room_banner(room, entry, -1.25));
            props.push(room_banner(room, entry, 1.25));
            for _ in 0..3 {
                let scale = rng.range(0.34, 0.52);
                add_room_identity_prop(
                    &mut props,
                    &mut occupied,
                    room,
                    &mut rng,
                    RoomIdentityKind::Gem,
                    room.center,
                    2.0,
                    3.6,
                    entry,
                    None,
                    0.0,
                    scale,
                    [0.78, 0.10, 0.07],
                );
            }
            add_room_identity_prop(
                &mut props,
                &mut occupied,
                room,
                &mut rng,
                RoomIdentityKind::BrokenRelic,
                room.center,
                2.4,
                3.8,
                entry,
                None,
                0.0,
                0.86,
                [1.0; 3],
            );
        }
        Role::Entrance | Role::Exit | Role::Normal | Role::Puzzle => {}
    }
    props
}

struct Slot {
    key: usize,
    /// GPU meshes owned by this slot; the first `used` hold its pieces.
    parts: Vec<MeshHandle>,
    used: usize,
    last_used: u64,
    triangles: usize,
}
#[derive(Clone, Copy)]
struct ProceduralLandmarks {
    crystal: MeshHandle,
    runestone: MeshHandle,
    altar: MeshHandle,
}
#[derive(Default, Clone, Copy)]
pub struct RenderStats {
    pub visible_rooms: usize,
    pub resident_chunks: usize,
    pub draws: usize,
    pub triangles: usize,
    pub uploads: usize,
    pub dropped_chunks: usize,
    /// CPU time spent building chunk meshes this frame (cache misses).
    pub generation_ms: f64,
    /// Time spent handing new chunk meshes to the GPU this frame.
    pub upload_ms: f64,
    /// CPU milliseconds per stage of the frame, in `PHASES` order.
    pub phases: [f32; 9],
    /// `draws.len()` at the end of each stage: which draws each one added.
    pub phase_draws: [usize; 9],
}

/// Names of the stages timed in `RenderStats::phases` (read by the lab).
#[allow(dead_code)]
pub const PHASES: [&str; 9] = [
    "chunks+water",
    "stream",
    "rooms",
    "foes",
    "view+fx",
    "lights+env",
    "submit",
    "finish",
    "hud",
];

/// Milliseconds since `clock`, restarting it.
fn lap(clock: &mut std::time::Instant) -> f32 {
    let now = std::time::Instant::now();
    let ms = (now - *clock).as_secs_f32() * 1000.0;
    *clock = now;
    ms
}
pub struct Scene {
    signature: Option<(u64, u64)>,
    streamer: Option<super::streaming::Streamer>,
    room_identity: BTreeMap<usize, Vec<RoomIdentityProp>>,
    slots: Vec<Slot>,
    tick: u64,
    cube: MeshHandle,
    crystal: MeshHandle,
    fracture_marks: Vec<MeshHandle>,
    flame: MeshHandle,
    hand_torch: MeshHandle,
    fist_right: MeshHandle,
    fist_left: MeshHandle,
    sleeve_cuff: MeshHandle,
    sleeve_upper: MeshHandle,
    weapon_trail: MeshHandle,
    ambient_particles: MeshHandle,
    ambient_cache: BTreeMap<usize, Vec<super::ambient::Particle>>,
    enemy: MeshHandle,
    slime_body: MeshHandle,
    /// One deformable body per grove slime.
    grove_bodies: Vec<MeshHandle>,
    slime_base: Vec<MeshVertex3D>,
    slime_eye: MeshHandle,
    slime_mouth: MeshHandle,
    slime_puddle: MeshHandle,
    healing_potion: MeshHandle,
    chest_gold: MeshHandle,
    crown: MeshHandle,
    crown_triangles: usize,
    /// Enemy weapon trails in flight, by spawn, and the meshes they draw with.
    enemy_trails: BTreeMap<usize, super::combat_fx::Trail>,
    trail_meshes: Vec<MeshHandle>,
    fx_ring: MeshHandle,
    fx_orb: MeshHandle,
    spell_fissure: MeshHandle,
    enemies: super::enemies::Enemies,
    rune_spiral: MeshHandle,
    landmarks: Vec<ProceduralLandmarks>,
    stone: TextureHandle,
    assets: super::assets::Assets,
    secret_walls: Vec<MeshHandle>,
    braziers: Vec<MeshHandle>,
    ember_rings: Vec<MeshHandle>,
    moods: Vec<super::surfaces::Mood>,
    /// Per-room vault meshes: (detail, glow, triangles), rebuilt on reseed.
    vault_meshes: BTreeMap<usize, (Option<MeshHandle>, Option<MeshHandle>, usize)>,
    vault_mesh_pool: Vec<MeshHandle>,
    water_rooms: BTreeMap<usize, Vec<super::water::WaterGpu>>,
    /// Water planned by a ported themed room's art, known once it is built.
    theme_water: BTreeMap<usize, Vec<super::water::Surface>>,
    /// A themed room's moving art: the beating glow, the Heart, the orrery.
    theme_extra: BTreeMap<usize, ThemeExtra>,
    /// The Heart's life (web `heartLife`): fades once its guardian falls.
    heart_life: f32,
    heart_clock: f32,
    water_pool: Vec<MeshHandle>,
    finish: super::finish::Finish,
    /// The hand-made mine's meshes, while its run is the one drawn, and its standing water.
    mine_art: Option<super::mine_art::MineArt>,
    mine_waters: Vec<super::water::WaterGpu>,
    pub stats: RenderStats,
    pub hud: Hud,
}
impl Scene {
    pub fn new(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        renderer.set_shading_style(ShadingStyle3D::Toon);
        let mut cube = visual::empty();
        visual::cuboid(&mut cube, Vec3::ZERO, Vec3::new(0.5, 0.5, 0.5), [1.0; 3]);
        let mut crystal = visual::empty();
        visual::crystal(&mut crystal, Vec3::ZERO, 0.5, 1.0, [1.0; 3], 6);
        let crown = visual::crown();
        let crown_triangles = crown.indices.len() / 3;
        let healing_potion = visual::healing_potion();
        let spell_fissure = visual::spell_fissure();
        let slime_body = super::slime::body_mesh();
        let slime_base = slime_body.vertices.clone();
        let slime_eye = Mesh3D::uv_sphere(1.0, 12, 8, Material3D::rgb(0.04, 0.06, 0.12));
        let slime_mouth = super::slime::mouth_mesh();
        let slime_puddle = Mesh3D::uv_sphere(1.0, 16, 4, Material3D::rgb(0.3, 0.7, 1.0));
        let fracture_marks = [1, 2, 3]
            .into_iter()
            .map(|stage| renderer.upload_mesh(device, &visual::fracture_marks(stage)))
            .collect::<Result<Vec<_>, _>>()?;
        let (albedo, normal) = visual::stone_texture();
        let stone = renderer.upload_texture_with_normal(device, queue, &albedo, Some(&normal))?;
        let assets = super::assets::Assets::load(renderer, device, queue);
        let enemies = super::enemies::Enemies::load(renderer, device, queue);
        let catalog = Catalog::bundled();
        let landmarks = catalog
            .biomes
            .iter()
            .map(|biome| -> anyhow::Result<ProceduralLandmarks> {
                let stone = super::surfaces::style(biome).stone;
                Ok(ProceduralLandmarks {
                    crystal: renderer
                        .upload_mesh(device, &visual::crystal_cluster(stone, biome.accent))?,
                    runestone: renderer.upload_mesh(device, &visual::runestone(stone))?,
                    altar: renderer
                        .upload_mesh(device, &visual::exit_altar(stone, biome.accent))?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rune_spiral = renderer.upload_mesh(device, &visual::rune_spiral())?;
        let secret_walls = catalog
            .biomes
            .iter()
            .map(|biome| renderer.upload_mesh(device, &super::surfaces::secret_wall(biome)))
            .collect::<Result<Vec<_>, _>>()?;
        let braziers = catalog
            .biomes
            .iter()
            .map(|biome| {
                renderer.upload_mesh(
                    device,
                    &visual::brazier(super::surfaces::style(biome).stone),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let ember_rings = catalog
            .biomes
            .iter()
            .map(|biome| {
                renderer.upload_mesh(
                    device,
                    &visual::ember_ring(super::surfaces::style(biome).stone),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let moods = catalog.biomes.iter().map(super::surfaces::mood).collect();
        Ok(Self {
            signature: None,
            streamer: None,
            room_identity: BTreeMap::new(),
            slots: vec![],
            tick: 0,
            cube: renderer.upload_mesh(device, &cube)?,
            crystal: renderer.upload_mesh(device, &crystal)?,
            fracture_marks,
            flame: renderer.upload_mesh(device, &visual::flame())?,
            hand_torch: renderer.upload_mesh(device, &visual::hand_torch())?,
            fist_right: renderer.upload_mesh(device, &fp::fist_mesh(true))?,
            fist_left: renderer.upload_mesh(device, &fp::fist_mesh(false))?,
            sleeve_cuff: renderer.upload_mesh(device, &fp::sleeve_mesh(true))?,
            sleeve_upper: renderer.upload_mesh(device, &fp::sleeve_mesh(false))?,
            weapon_trail: renderer.upload_mesh(device, &fp::trail_mesh())?,
            ambient_particles: renderer.upload_mesh(device, &super::ambient::mesh())?,
            ambient_cache: BTreeMap::new(),
            enemy: renderer.upload_mesh(device, &visual::enemy())?,
            slime_body: renderer.upload_mesh(device, &slime_body)?,
            grove_bodies: (0..12)
                .map(|_| renderer.upload_mesh(device, &slime_body))
                .collect::<Result<_, _>>()?,
            slime_base,
            slime_eye: renderer.upload_mesh(device, &slime_eye)?,
            slime_mouth: renderer.upload_mesh(device, &slime_mouth)?,
            slime_puddle: renderer.upload_mesh(device, &slime_puddle)?,
            healing_potion: renderer.upload_mesh(device, &healing_potion)?,
            chest_gold: renderer.upload_mesh(device, &visual::chest_gold())?,
            crown: renderer.upload_mesh(device, &crown)?,
            crown_triangles,
            enemy_trails: BTreeMap::new(),
            trail_meshes: (0..6)
                .map(|_| renderer.upload_mesh(device, &super::combat_fx::trail_mesh()))
                .collect::<Result<_, _>>()?,
            fx_ring: renderer.upload_mesh(device, &super::combat_fx::ring_mesh(0.86, 48))?,
            fx_orb: renderer.upload_mesh(
                device,
                &Mesh3D::uv_sphere(1.0, 12, 8, Material3D::rgb(1.0, 1.0, 1.0)),
            )?,
            spell_fissure: renderer.upload_mesh(device, &spell_fissure)?,
            enemies,
            rune_spiral,
            landmarks,
            stone,
            assets,
            secret_walls,
            braziers,
            ember_rings,
            moods,
            vault_meshes: BTreeMap::new(),
            vault_mesh_pool: Vec::new(),
            water_rooms: BTreeMap::new(),
            theme_water: BTreeMap::new(),
            theme_extra: BTreeMap::new(),
            heart_life: 1.0,
            heart_clock: 0.0,
            water_pool: Vec::new(),
            finish: super::finish::Finish::new(device, format),
            mine_art: None,
            mine_waters: Vec::new(),
            stats: Default::default(),
            hud: Hud::new(device, queue, format)?,
        })
    }
    fn visible(&self, run: &Dungeon, camera: Camera3D) -> BTreeSet<usize> {
        let map = &run.map;
        let p = run.controller.position;
        let start = map.region_at(p);
        let mut visible = BTreeSet::from([start]);
        let mut q = VecDeque::from([(start, 0)]);
        // Conservative portal traversal: retain two rings to avoid popping at
        // thresholds; further rooms require a visible portal. Closed rock bends
        // stop propagation. Frustum rejection uses bounding spheres, not centres.
        while let Some((id, depth)) = q.pop_front() {
            if depth >= 3 {
                continue;
            }
            for (next, c) in map.neighbors(id) {
                if visible.contains(&next) {
                    continue;
                }
                let r = &map.rooms[next];
                let center = r.center + Vec3::Y * (r.height * 0.5);
                let delta = center - camera.position;
                let radius =
                    (r.half[0] * r.half[0] + r.half[1] * r.half[1] + r.height * r.height).sqrt();
                if delta.length() - radius > 85.0 {
                    continue;
                }
                if depth > 0
                    && delta.dot((camera.target - camera.position).normalize_or_zero()) < -radius
                {
                    continue;
                }
                if depth >= 2
                    && !map.line_of_sight(camera.position, c.start + Vec3::Y * 1.65)
                    && !map.line_of_sight(camera.position, c.end + Vec3::Y * 1.65)
                {
                    continue;
                }
                visible.insert(next);
                q.push_back((next, depth + 1));
            }
        }
        visible
    }
    fn mesh(
        &mut self,
        key: usize,
        run: &Dungeon,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        pinned: &BTreeSet<usize>,
    ) -> anyhow::Result<(Vec<MeshHandle>, usize)> {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.key == key) {
            slot.last_used = self.tick;
            return Ok((slot.parts[..slot.used].to_vec(), slot.triangles));
        }
        use super::streaming::{Built, Job};
        let prefetched = self
            .streamer
            .as_mut()
            .and_then(|s| s.take_or_wait(Job::Chunk(key), std::time::Duration::from_millis(12)));
        let mesh = if let Some(Built::Chunk(pieces)) = prefetched {
            pieces
        } else {
            let started = std::time::Instant::now();
            let pieces = super::streaming::chunk_pieces(&run.map, &run.catalog, key);
            self.stats.generation_ms += started.elapsed().as_secs_f64() * 1000.0;
            pieces
        };
        self.insert_chunk(key, mesh, renderer, device, pinned)
    }

    /// Upload a chunk into a free slot, or replace the least recently used
    /// slot that is not currently visible.
    fn insert_chunk(
        &mut self,
        key: usize,
        pieces: Vec<Mesh3D>,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        pinned: &BTreeSet<usize>,
    ) -> anyhow::Result<(Vec<MeshHandle>, usize)> {
        let upload_started = std::time::Instant::now();
        let triangles = pieces.iter().map(|m| m.indices.len() / 3).sum();
        self.stats.uploads += 1;
        // Reuse the slot's GPU meshes piece by piece, adding any it lacks.
        let fill =
            |renderer: &mut Wgpu3DRenderer, parts: &mut Vec<MeshHandle>| -> anyhow::Result<()> {
                for (i, piece) in pieces.iter().enumerate() {
                    if let Some(&handle) = parts.get(i) {
                        renderer.replace_mesh(device, handle, piece)?;
                    } else {
                        parts.push(renderer.upload_mesh(device, piece)?);
                    }
                }
                Ok(())
            };
        let handles;
        if self.slots.len() >= CACHE_LIMIT || self.slots.iter().any(|s| s.key == usize::MAX) {
            let index = self
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| !pinned.contains(&s.key))
                .min_by_key(|(_, s)| s.last_used)
                .map(|(i, _)| i)
                .ok_or_else(|| anyhow::anyhow!("visible set exceeds GPU chunk budget"))?;
            let mut parts = std::mem::take(&mut self.slots[index].parts);
            // Mark the slot free first: a failed upload leaves it reusable.
            self.slots[index].key = usize::MAX;
            self.slots[index].used = 0;
            let filled = fill(renderer, &mut parts);
            self.slots[index].parts = parts;
            filled?;
            let slot = &mut self.slots[index];
            *slot = Slot {
                key,
                parts: std::mem::take(&mut slot.parts),
                used: pieces.len(),
                last_used: self.tick,
                triangles,
            };
            handles = slot.parts[..pieces.len()].to_vec();
        } else {
            let mut parts = Vec::new();
            fill(renderer, &mut parts)?;
            handles = parts.clone();
            self.slots.push(Slot {
                key,
                parts,
                used: pieces.len(),
                last_used: self.tick,
                triangles,
            });
        }
        self.stats.upload_ms += upload_started.elapsed().as_secs_f64() * 1000.0;
        Ok((handles, triangles))
    }
    pub fn render(
        &mut self,
        run: &Dungeon,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
    ) {
        self.tick += 1;
        let mut clock = std::time::Instant::now();
        let mut phases = [0.0f32; 9];
        self.finish.resize(device, size);
        self.stats = Default::default();
        let signature = (run.map.seed, run.map.rules_hash);
        if self.signature != Some(signature) {
            for s in &mut self.slots {
                s.key = usize::MAX;
                s.last_used = 0;
            }
            self.room_identity.clear();
            self.ambient_cache.clear();
            let released = std::mem::take(&mut self.vault_meshes);
            for (detail, glow, _) in released.into_values() {
                self.vault_mesh_pool.extend(detail);
                self.vault_mesh_pool.extend(glow);
            }
            for extra in std::mem::take(&mut self.theme_extra).into_values() {
                self.vault_mesh_pool.extend(extra.handles());
            }
            let water = std::mem::take(&mut self.water_rooms);
            self.water_pool
                .extend(water.into_values().flatten().map(|gpu| gpu.handle));
            self.theme_water.clear();
            self.signature = Some(signature);
            // The hand-made mine has its art modelled: nothing to build room by room.
            self.streamer = run
                .authored
                .is_none()
                .then(|| super::streaming::Streamer::new(&run.map, &run.catalog));
        }
        let authored = run.authored.clone();
        if let Some(mine) = &authored {
            if self
                .mine_art
                .as_ref()
                .is_none_or(|art| art.key != mine.rock)
            {
                self.mine_art = match super::mine_art::MineArt::load(mine, renderer, device, queue)
                {
                    Ok(art) => Some(art),
                    Err(error) => {
                        eprintln!("[mina] its art did not load: {error}");
                        None
                    }
                };
                let old = std::mem::take(&mut self.mine_waters);
                self.water_pool.extend(old.into_iter().map(|w| w.handle));
                for pool in &mine.pools {
                    let surface = super::water::Surface {
                        center: pool.center,
                        outline: pool.outline.clone(),
                        color: pool.color,
                        reflect: pool.reflect,
                        depth: 0.6,
                        halo: 1.1,
                        drip: None,
                    };
                    let mesh = super::water::mesh(&surface);
                    let handle = match self.water_pool.pop() {
                        Some(reuse) => renderer
                            .replace_mesh(device, reuse, &mesh)
                            .ok()
                            .map(|_| reuse),
                        None => renderer.upload_mesh(device, &mesh).ok(),
                    };
                    if let Some(handle) = handle {
                        self.mine_waters.push(super::water::WaterGpu {
                            handle,
                            base: mesh.vertices,
                            surface,
                            triangles: mesh.indices.len() / 3,
                        });
                    }
                }
            }
        }
        // Reuse invalidated slots on reseed instead of appending GPU buffers.
        // In the hand-made mine every room may hold what is seen (its foes, chests and runes);
        // the generated rooms' art (chunks, water, motes, torches) is only for generated runs.
        let rooms: BTreeSet<usize> = if authored.is_some() {
            (0..run.map.rooms.len()).collect()
        } else {
            self.visible(run, camera)
        };
        let art_rooms: BTreeSet<usize> = if authored.is_some() {
            BTreeSet::new()
        } else {
            rooms.clone()
        };
        self.stats.visible_rooms = rooms.len();
        // Gallery GLTFs are optional and numerous: bring in one new model per
        // frame from the current portal set, instead of blocking scene start.
        let current_room = run.map.region_at(run.controller.position);
        let mut needed_art: Vec<&str> = run.decor[current_room]
            .props
            .iter()
            .map(|p| p.asset)
            .collect();
        for &id in &rooms {
            if id != current_room {
                needed_art.extend(run.decor[id].props.iter().map(|p| p.asset));
            }
        }
        self.assets
            .load_next_visible(needed_art, renderer, device, queue);
        let mut keys = art_rooms.clone();
        for c in &run.map.connections {
            if rooms.contains(&c.a) || rooms.contains(&c.b) {
                keys.insert(run.map.rooms.len() + c.id);
            }
        }
        let mut draws = Vec::with_capacity(keys.len() + rooms.len() * 16 + 80);
        // Nearest geometry first: the depth buffer then rejects the hidden
        // parts of rooms further away before they are shaded.
        let mut ordered: Vec<usize> = keys.iter().copied().collect();
        let eye = camera.position;
        let reach = |key: usize| -> f32 {
            if let Some(r) = run.map.rooms.get(key) {
                let lo = r.center - Vec3::new(r.half[0], 0.0, r.half[1]);
                let hi = r.center + Vec3::new(r.half[0], r.height, r.half[1]);
                let near = Vec3::new(
                    eye.x.clamp(lo.x, hi.x),
                    eye.y.clamp(lo.y, hi.y),
                    eye.z.clamp(lo.z, hi.z),
                );
                (near - eye).length()
            } else {
                let c = &run.map.connections[key - run.map.rooms.len()];
                segment_distance(eye, c.start, c.end)
            }
        };
        ordered.sort_by(|&a, &b| reach(a).total_cmp(&reach(b)));
        // Visible pieces not yet resident go to the builders first, nearest
        // first, so several build in parallel instead of one after another.
        if let Some(streamer) = self.streamer.as_mut() {
            let slots = &self.slots;
            streamer.prefetch(
                ordered.iter().map(|&key| super::streaming::Job::Chunk(key)),
                |job| matches!(job, super::streaming::Job::Chunk(key) if slots.iter().any(|s| s.key == key)),
            );
        }
        for key in ordered {
            match self.mesh(key, run, renderer, device, &keys) {
                Ok((pieces, triangles)) => {
                    for mesh in pieces {
                        draws.push(MeshDraw3D {
                            mesh,
                            transform: Mat4::IDENTITY,
                            material: Material3D::rgb(1.0, 1.0, 1.0).with_ambient_occlusion(0.7),
                            texture: Some(self.stone),
                        });
                    }
                    self.stats.triangles += triangles;
                }
                Err(e) => {
                    self.stats.dropped_chunks += 1;
                    eprintln!("[dungeon] chunk {key}: {e}")
                }
            }
        }
        for &id in &art_rooms {
            let themed = run.decor[id].theme.is_some();
            if !self.water_rooms.contains_key(&id)
                && (!themed || self.theme_water.contains_key(&id))
            {
                let surfaces: Vec<super::water::Surface> = if themed {
                    self.theme_water.get(&id).cloned().unwrap_or_default()
                } else {
                    super::water::for_room(&run.map, &run.map.rooms[id])
                        .into_iter()
                        .collect()
                };
                let mut gpus = Vec::new();
                for surface in surfaces {
                    let mesh = super::water::mesh(&surface);
                    let handle = if let Some(reuse) = self.water_pool.pop() {
                        match renderer.replace_mesh(device, reuse, &mesh) {
                            Ok(()) => Some(reuse),
                            Err(error) => {
                                eprintln!("[dungeon] water room {id}: {error}");
                                None
                            }
                        }
                    } else {
                        renderer.upload_mesh(device, &mesh).ok()
                    };
                    if let Some(handle) = handle {
                        gpus.push(super::water::WaterGpu {
                            handle,
                            base: mesh.vertices,
                            surface,
                            triangles: mesh.indices.len() / 3,
                        });
                    }
                }
                self.water_rooms.insert(id, gpus);
            }
            for water in self.water_rooms.get(&id).into_iter().flatten() {
                let vertices = super::water::animated_vertices(
                    &water.base,
                    &water.surface,
                    run.elapsed,
                    camera.position,
                );
                if renderer
                    .update_mesh_vertices(queue, water.handle, &vertices)
                    .is_ok()
                {
                    draws.push(MeshDraw3D {
                        mesh: water.handle,
                        transform: Mat4::IDENTITY,
                        material: Material3D {
                            base_color: [1.0, 1.0, 1.0, 0.92],
                            texture: None,
                            occlusion_window: [0.38, 0.0, 0.0, 0.0],
                        },
                        texture: None,
                    });
                    self.stats.triangles += water.triangles;
                }
                if let Some(drip) = water.surface.drip {
                    if let Some(y) = super::water::drop_height(drip, run.elapsed) {
                        self.draw(
                            &mut draws,
                            self.slime_eye,
                            Vec3::new(drip.x, y, drip.z),
                            Vec3::new(0.011, 0.019, 0.011),
                            0.0,
                            [0.68, 0.83, 0.91],
                        );
                    }
                }
            }
        }
        let mut ambient = Vec::with_capacity(art_rooms.len() * 30);
        for &id in &art_rooms {
            let plan = self.ambient_cache.entry(id).or_insert_with(|| {
                super::ambient::for_room(
                    &run.map,
                    &run.catalog,
                    &run.map.rooms[id],
                    self.moods[run.map.rooms[id].biome].glow,
                )
            });
            ambient.extend(plan.iter().copied());
            if ambient.len() >= super::ambient::MAX_PARTICLES {
                break;
            }
        }
        if !ambient.is_empty() {
            let vertices = super::ambient::animated_vertices(&ambient, camera, run.elapsed);
            if renderer
                .update_mesh_vertices(queue, self.ambient_particles, &vertices)
                .is_ok()
            {
                draws.push(MeshDraw3D {
                    mesh: self.ambient_particles,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(1.0, 1.0, 1.0).with_emission(1.4),
                    texture: None,
                });
                self.stats.triangles += super::ambient::MAX_PARTICLES * 8;
            }
        }
        phases[0] = lap(&mut clock);
        self.stats.phase_draws[0] = draws.len();
        self.stream_ahead(run, renderer, device, &keys);
        phases[1] = lap(&mut clock);
        self.stats.phase_draws[1] = draws.len();
        let mut lights = Vec::new();
        let mut fixtures = Vec::new();
        let room_entries = room_entry_portals(&run.map);
        for &id in &art_rooms {
            let r = &run.map.rooms[id];
            let mood = self.room_mood(run, id);
            for s in &r.sockets {
                let Some(mount) = super::dressing::wall_torch_for(r, s) else {
                    continue;
                };
                let p = mount.flame;
                let color = mood.flame;
                lights.push(PointLight3D::new(
                    p,
                    color,
                    (13.0 + (run.elapsed * 3.0 + id as f32).sin() * 0.6) * mood.flame_intensity,
                    8.6,
                ));
                fixtures.push((p, color, 1.45));
                if let Some(tris) = self.assets.draw_back_mounted(
                    "torch",
                    &mut draws,
                    mount.backplate,
                    0.83,
                    mount.yaw,
                ) {
                    self.stats.triangles += tris;
                } else {
                    self.draw(
                        &mut draws,
                        self.cube,
                        mount.backplate + mount.inward * 0.095 + Vec3::Y * 0.42,
                        Vec3::new(0.17, 0.4, 0.19),
                        mount.yaw,
                        [0.25, 0.19, 0.12],
                    );
                }
            }
            // Biome signature: braziers, glowing crystal groves or ember pits
            // in the room's chamfered corners give every chamber a lit silhouette
            // and a readable identity. Bridge rooms glow from the abyss instead.
            if r.shape == Shape::Bridge {
                self.draw_abyss_glow(&mut draws, &mut lights, r, mood, run.elapsed);
            } else {
                let decor = &run.decor[id];
                for (index, p) in decor.beacons.iter().copied().enumerate() {
                    self.draw_beacon(
                        &mut draws,
                        &mut lights,
                        &mut fixtures,
                        r,
                        mood,
                        p,
                        index,
                        run.elapsed,
                    );
                }
            }

            let room_distance = (r.center - camera.position).length();
            if room_distance < 40.0 {
                if let Some(theme) = run.decor[id].theme.clone() {
                    self.draw_theme(
                        run,
                        id,
                        &theme.0,
                        renderer,
                        device,
                        camera,
                        &mut draws,
                        &mut lights,
                        &mut fixtures,
                    );
                }
                if let Some(vault) = run.decor[id].vault.as_ref() {
                    self.draw_vault(
                        run,
                        id,
                        vault,
                        renderer,
                        device,
                        camera,
                        &mut draws,
                        &mut lights,
                        &mut fixtures,
                    );
                }
            }
            if room_distance < 34.0 {
                let decor = &run.decor[id];
                for prop in &decor.props {
                    let distance = (prop.position - camera.position).length();
                    if distance > 30.0 {
                        continue;
                    }
                    if let Some(tris) = self.assets.draw(
                        prop.asset,
                        &mut draws,
                        prop.position,
                        prop.height,
                        prop.yaw,
                    ) {
                        self.stats.triangles += tris;
                    }
                }
                for banner in &decor.banners {
                    if let Some(tris) = self.assets.draw_back_mounted(
                        banner.asset,
                        &mut draws,
                        banner.anchor,
                        2.1,
                        banner.yaw,
                    ) {
                        self.stats.triangles += tris;
                    }
                }
            }
            if room_distance < 32.0 {
                let focus = run
                    .map
                    .spawns
                    .iter()
                    .find(|spawn| {
                        spawn.room == id
                            && matches!(
                                spawn.kind,
                                gameplay::SpawnKind::Chest
                                    | gameplay::SpawnKind::Rest
                                    | gameplay::SpawnKind::Memory
                            )
                    })
                    .map(|spawn| spawn.position);
                let entry = room_entries.get(id).copied().flatten();
                let identity = self
                    .room_identity
                    .entry(id)
                    .or_insert_with(|| room_identity_plan(r, run.seed(), entry, focus))
                    .clone();
                self.draw_room_identity(
                    id,
                    run.seed(),
                    run.elapsed,
                    &identity,
                    room_distance,
                    renderer,
                    device,
                    queue,
                    &mut draws,
                    &mut lights,
                );
            }

            // Near-field props only; collision remains defined by the blueprint.
            if room_distance < 32.0 {
                let mut rng = rng::Rng::stream(run.seed(), "near_detail", id as u64);
                let natural_detail_count = match r.role {
                    Role::Treasure => 0,
                    Role::Secret | Role::Rest | Role::Puzzle | Role::Event => 2,
                    Role::Elite | Role::Miniboss | Role::Boss => 3,
                    _ => 8,
                };
                for i in 0..natural_detail_count {
                    let theta = rng.range(0.0, std::f32::consts::TAU);
                    let mut p = r.center
                        + Vec3::new(
                            theta.cos() * r.half[0] * 0.80,
                            0.0,
                            theta.sin() * r.half[1] * 0.80,
                        );
                    if !r.contains(p.x, p.z, 0.7)
                        || r.sockets
                            .iter()
                            .any(|s| horizontal(s.position - p).length() < 2.0)
                    {
                        continue;
                    }
                    p.y = r.floor_y(p.x, p.z);
                    let name = if i % 3 == 0 && r.biome == 1 {
                        "mushroom"
                    } else {
                        "rock"
                    };
                    if let Some(tris) =
                        self.assets
                            .draw(name, &mut draws, p, rng.range(0.12, 0.28), theta)
                    {
                        self.stats.triangles += tris;
                    }
                }
            }
        }
        self.draw_pickups(run, &mut draws, &mut lights);
        for water in &self.mine_waters {
            if (water.surface.center - camera.position).length() > 60.0 {
                continue;
            }
            let vertices = super::water::animated_vertices(
                &water.base,
                &water.surface,
                run.elapsed,
                camera.position,
            );
            if renderer
                .update_mesh_vertices(queue, water.handle, &vertices)
                .is_ok()
            {
                draws.push(MeshDraw3D {
                    mesh: water.handle,
                    transform: Mat4::IDENTITY,
                    material: Material3D {
                        base_color: [1.0, 1.0, 1.0, 0.92],
                        texture: None,
                        occlusion_window: [0.38, 0.0, 0.0, 0.0],
                    },
                    texture: None,
                });
                self.stats.triangles += water.triangles;
            }
        }
        if let (Some(mine), Some(art)) = (&authored, &self.mine_art) {
            let aspect = size.0 as f32 / size.1.max(1) as f32;
            let heart = super::mine_art::heart_pose(run);
            self.stats.triangles += art.draw(camera, aspect, heart.as_ref(), &mut draws);
            super::mine_art::lamps(
                mine,
                camera.position,
                run.elapsed,
                heart.as_ref(),
                &mut lights,
            );
            // the Heart's ring of blood racing over the floor
            if let Some((origin, age)) = run.heart.wave {
                let radius = age * super::HEART_WAVE_SPEED;
                let mut material = Material3D::rgb(0.9, 0.04, 0.07).with_emission(2.6);
                material.base_color[3] = (1.0 - radius / 17.0).clamp(0.2, 1.0);
                draws.push(MeshDraw3D {
                    mesh: self.fx_ring,
                    transform: Transform3D {
                        position: origin + Vec3::Y * 0.07,
                        rotation: Vec3::ZERO,
                        scale: Vec3::new(radius.max(0.3), 1.0, radius.max(0.3)),
                    }
                    .matrix(),
                    material,
                    texture: None,
                });
                lights.push(PointLight3D::new(
                    origin + Vec3::Y * 0.6,
                    [1.0, 0.1, 0.12],
                    7.0,
                    radius + 3.0,
                ));
            }
            for npc in &mine.npcs {
                self.draw_stick_figure(&mut draws, &mut lights, npc, run);
            }
            if !(run.seal && run.crest) {
                for gate in &mine.gates {
                    self.draw_mine_gate(&mut draws, &mut lights, gate, run.elapsed);
                }
            }
        }
        for (p, color, size) in fixtures {
            // ACES desaturates bright input, so the visible flame uses a
            // squared (more saturated) version of the light colour.
            let hot = [
                color[0] * color[0],
                color[1] * color[1],
                color[2] * color[2],
            ];
            // Three offset tongues with independent flicker read as fire; a
            // single solid cone read as a glowing crystal.
            for tongue in 0..3 {
                let t = tongue as f32;
                let phase = run.elapsed * (7.0 + t * 2.3) + p.x * 3.1 + p.z * 1.7 + t * 2.0;
                let lick = phase.sin() * 0.022 + (phase * 1.7).sin() * 0.012;
                let spread = if tongue == 0 { 0.0 } else { 0.035 * size };
                let angle = t * 2.1 + run.elapsed * 0.6;
                let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
                let height = if tongue == 0 { 0.25 } else { 0.16 };
                let width = if tongue == 0 { 0.11 } else { 0.07 };
                self.draw(
                    &mut draws,
                    self.flame,
                    p + offset,
                    Vec3::new(width * size, (height + lick) * size, width * size),
                    angle,
                    // Low albedo + strong emission: the flame keeps its own
                    // saturated colour even inside its point light's hot zone.
                    if tongue == 0 {
                        visual::tint(hot, 0.16)
                    } else {
                        visual::tint(color, 0.16)
                    },
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material =
                        draw.material
                            .with_emission(if tongue == 0 { 5.6 } else { 5.0 });
                }
            }
            // A tiny white-hot core is solid faceted geometry, not a billboard.
            if (p - camera.position).length_squared() < 144.0 {
                self.draw(
                    &mut draws,
                    self.crystal,
                    p + Vec3::Y * 0.02,
                    Vec3::new(0.035 * size, 0.11 * size, 0.035 * size),
                    0.1,
                    visual::tint(color, 0.18),
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(6.5);
                }
            }
        }
        for c in &run.map.connections {
            if !rooms.contains(&c.a) && !rooms.contains(&c.b) {
                continue;
            }
            let secret = c.kind == ConnectionKind::Secret && !run.opened.contains(&c.id);
            let gate = run.map.rooms[c.b].role == Role::Boss && !(run.seal && run.crest);
            if secret || gate {
                let p = if secret { c.start } else { c.end };
                if secret {
                    let biome = run.map.rooms[c.a].biome;
                    let mesh = self.secret_walls[biome.min(self.secret_walls.len() - 1)];
                    let axis = (c.end - c.start).normalize_or_zero();
                    self.draw(
                        &mut draws,
                        mesh,
                        p,
                        Vec3::new(c.width, c.height, 1.0),
                        axis.x.atan2(axis.z),
                        [1.0; 3],
                    );
                } else {
                    self.draw_sealed_gate(&mut draws, &mut lights, run, c, p);
                }
            }
        }
        if let Some(slime) = &run.slime {
            if rooms.contains(&slime.room) {
                self.draw_slime(
                    slime,
                    self.slime_body,
                    renderer,
                    queue,
                    &mut draws,
                    &mut lights,
                );
            }
        }
        for (slime, body) in run
            .groves
            .iter()
            .filter(|slime| rooms.contains(&slime.room))
            .zip(self.grove_bodies.clone())
        {
            self.draw_slime(slime, body, renderer, queue, &mut draws, &mut lights);
        }
        phases[2] = lap(&mut clock);
        self.stats.phase_draws[2] = draws.len();
        // Fallen foes are not drawn: their meshes serve the next ones.
        self.enemies.release(
            run.actors
                .iter()
                .filter(|a| {
                    a.health <= 0.0
                        && a.death >= super::combat::DEATH_SECONDS
                        && run.map.spawns[a.spawn].health > 0.0
                })
                .map(|a| a.spawn),
        );
        for a in &run.actors {
            let s = &run.map.spawns[a.spawn];
            if !rooms.contains(&s.room) || (a.used && s.kind != gameplay::SpawnKind::Chest) {
                continue;
            }
            if s.health > 0.0 {
                // The fallen play their death, then sink away.
                let dying = a.health <= 0.0;
                if dying && a.death >= super::combat::DEATH_SECONDS {
                    continue;
                }
                let size = super::authored::foe_size(run.authored.as_deref(), s);
                let loadout = super::combat::loadout(&s.definition, s.kind, s.id);
                let moves = super::combat::moves(&s.definition, s.kind, s.id);
                let yaw = a.yaw;
                let swing = a
                    .attack
                    .filter(|_| !dying)
                    .map(|swing| (swing, moves[swing.index.min(moves.len() - 1)]));
                // Web `playClip`: the blow's clip, a hit, the gait, or the
                // weapon's guard stance.
                let (animation, animation_time) = if dying {
                    (if s.id % 2 == 0 { "Death_A" } else { "Death_B" }, a.death)
                } else if let Some((swing, mv)) = swing {
                    (mv.name, swing.t.min(mv.duration - 0.001))
                } else if a.stagger > 0.0 {
                    ("Hit_B", (0.42 - a.hit_reaction).max(0.0))
                } else {
                    match a.gait {
                        super::combat::Gait::Run => ("Running_A", run.elapsed * 1.12),
                        super::combat::Gait::Walk => ("Walking_A", run.elapsed),
                        super::combat::Gait::Guard => (guard_clip(loadout.weapon), run.elapsed),
                        super::combat::Gait::Idle => ("Idle_A", run.elapsed),
                    }
                };
                // QA: every foe frozen in one clip (`guard` and `attack` pick its own)
                let (animation, animation_time) = match qa_pose() {
                    Some((clip, t)) => (
                        match clip.as_str() {
                            "guard" => guard_clip(loadout.weapon),
                            "attack" => moves[0].name,
                            "attack2" => moves[moves.len().min(2) - 1].name,
                            _ => clip.as_str(),
                        },
                        *t,
                    ),
                    None => (animation, animation_time),
                };
                let distance = (a.position - camera.position).length();
                // A leaping champion is off the ground.
                let lift = swing
                    .map(|(swing, mv)| super::combat::leap_lift(&mv, swing.t))
                    .unwrap_or(0.0)
                    - if dying {
                        (a.death - super::combat::SINK_AFTER).max(0.0) * 0.55 * size
                    } else {
                        0.0
                    };
                let imported = self.enemies.draw_armed(
                    loadout,
                    a.spawn,
                    a.position + Vec3::Y * lift,
                    yaw,
                    super::combat::body_height(s.kind),
                    animation,
                    animation_time,
                    run.elapsed,
                    distance,
                    a.fracture_stage(s.health),
                    a.hit_reaction,
                    renderer,
                    device,
                    queue,
                    &mut draws,
                );
                if s.kind == gameplay::SpawnKind::Miniboss {
                    let charge = swing
                        .filter(|(swing, mv)| swing.t < mv.strike)
                        .map(|(swing, mv)| (swing.t / mv.strike).clamp(0.0, 1.0))
                        .unwrap_or(0.0);
                    lights.push(PointLight3D::new(
                        a.position + Vec3::Y * (1.15 * size),
                        [0.92, 0.17, 0.045],
                        0.62 + charge * 1.55,
                        4.1,
                    ));
                }
                let (fracture_anchor, weapon_edge) = if let Some(pose) = imported {
                    self.stats.triangles += pose.triangles;
                    (pose.fracture_anchor, pose.weapon_edge)
                } else {
                    self.draw(
                        &mut draws,
                        self.enemy,
                        a.position
                            + Vec3::Y * (0.05 + (run.elapsed * 2.4 + a.spawn as f32).sin() * 0.07),
                        Vec3::new(size, size, size),
                        yaw,
                        if swing.is_some() {
                            [1.5, 0.8, 0.55]
                        } else {
                            [1.0; 3]
                        },
                    );
                    (Vec3::new(0.0, 0.92, 0.17) * size, None)
                };
                // A caster's staff gathers its orb through the wind-up.
                if let (Some((swing, mv)), Some(edge)) = (swing, weapon_edge) {
                    let gathers = matches!(
                        mv.kind,
                        super::combat::MoveKind::Cast
                            | super::combat::MoveKind::Barrage
                            | super::combat::MoveKind::Nova
                    );
                    if gathers && swing.t < mv.strike {
                        let charge = (swing.t / mv.strike).clamp(0.0, 1.0);
                        let color = if s.kind == gameplay::SpawnKind::Boss {
                            super::combat::BOSS_CRIMSON
                        } else {
                            super::combat::MAGE_VIOLET
                        };
                        let pulse = 1.0 + (run.elapsed * 19.0).sin() * 0.1 * charge;
                        let fx_meshes = super::combat_fx::FxMeshes {
                            cube: self.cube,
                            crystal: self.crystal,
                            ring: self.fx_ring,
                            fissure: self.spell_fissure,
                            orb: self.fx_orb,
                        };
                        super::combat_fx::draw_orb(
                            &fx_meshes,
                            &mut draws,
                            edge.tip,
                            (0.05 + 0.2 * charge) * pulse,
                            color,
                        );
                        lights.push(PointLight3D::new(edge.tip, color, 0.6 + 2.2 * charge, 3.5));
                    }
                }
                // The swung weapon leaves a trail through the strike (web
                // `advanceAttack`); bolts and casts do not.
                if let (Some((swing, mv)), Some(edge)) = (swing, weapon_edge) {
                    let (from, to) = if mv.kind == super::combat::MoveKind::Whirl {
                        (0.5, 1.45)
                    } else {
                        (mv.strike - 0.13, mv.strike + 0.1)
                    };
                    if !mv.ranged
                        && mv.kind != super::combat::MoveKind::Roar
                        && swing.t > from
                        && swing.t < to
                    {
                        let color = match s.kind {
                            gameplay::SpawnKind::Boss => [1.0, 0.45, 0.18],
                            gameplay::SpawnKind::Miniboss => [1.0, 0.7, 0.35],
                            _ => [0.75, 0.85, 1.0],
                        };
                        self.enemy_trails
                            .entry(a.spawn)
                            .or_insert_with(|| super::combat_fx::Trail::new(color))
                            .push(edge.base, edge.tip, run.elapsed);
                    }
                }
                let scale = size;
                let damage_stage = a.fracture_stage(s.health);
                let fracture_position =
                    a.position + Mat4::from_rotation_y(yaw).transform_vector3(fracture_anchor);
                if damage_stage > 0 && !dying {
                    let fracture_mesh = self.fracture_marks[(damage_stage - 1) as usize];
                    draws.push(MeshDraw3D {
                        mesh: fracture_mesh,
                        transform: Transform3D {
                            position: fracture_position,
                            rotation: Vec3::new(0.0, yaw, 0.0),
                            scale: Vec3::new(scale, scale, scale * 0.12),
                        }
                        .matrix(),
                        material: Material3D::rgb(1.0, 1.0, 1.0).with_emission(0.12),
                        texture: None,
                    });
                }
                if a.hit_reaction > 0.0 && !dying {
                    let progress = ((0.42 - a.hit_reaction) / 0.42).clamp(0.0, 1.0);
                    let fragment_count = 2 + usize::from(damage_stage);
                    let origin = fracture_position;
                    for fragment in 0..fragment_count {
                        let angle = yaw
                            + fragment as f32 * std::f32::consts::TAU / fragment_count as f32
                            + progress * 0.75;
                        let radius = (0.035 + progress * 0.24) * scale;
                        let fragment_position = origin
                            + Vec3::new(
                                angle.sin() * radius,
                                progress * 0.13,
                                angle.cos() * radius,
                            );
                        let size = (0.052 * (1.0 - progress * 0.42)).max(0.025) * scale;
                        self.draw_rotating(
                            &mut draws,
                            self.crystal,
                            fragment_position,
                            Vec3::new(size * 0.64, size * 1.35, size * 0.64),
                            angle,
                            angle * 1.7 + progress * 3.2,
                            [0.92, 0.79, 0.52],
                        );
                        if let Some(draw) = draws.last_mut() {
                            draw.material = draw.material.with_emission(0.22 * (1.0 - progress));
                        }
                    }
                }
            } else {
                use gameplay::SpawnKind::*;
                let color = match s.kind {
                    Chest => [0.65, 0.39, 0.15],
                    Seal => [0.4, 0.72, 0.75],
                    Rest => [0.45, 0.9, 0.65],
                    Entrance | Exit => [0.9, 0.75, 0.39],
                    Trap => [0.9, 0.22, 0.09],
                    _ => [0.54, 0.4, 0.7],
                };
                match s.kind {
                    Chest => {
                        let chest_yaw = run
                            .authored
                            .as_ref()
                            .and_then(|mine| mine.face(s.id))
                            .map(|face| face.x.atan2(face.z))
                            .or_else(|| {
                                room_entries
                                    .get(s.room)
                                    .and_then(|entry| *entry)
                                    .map(|entry| chest_facing_yaw(a.position, entry))
                            })
                            .unwrap_or_else(|| {
                                super::enemies::facing_yaw(run.controller.position - a.position)
                            });
                        let distance = (a.position - camera.position).length();
                        // The hand-made mine's chests come in three kinds (the chest 2.0): a
                        // plain one, a rich one and the treasure's, bigger.
                        let tier = run.authored.as_ref().and(match s.definition.as_str() {
                            "comun" => Some(("cofre_comun", 1.0)),
                            "rico" => Some(("cofre_rico", 1.1)),
                            "tesoro" => Some(("cofre_tesoro", 1.32)),
                            _ => None,
                        });
                        let (model, tier_size) = tier.unwrap_or(("chest", 1.0));
                        // Opening, it lets out a warm light that stays while the loot lies there.
                        if tier.is_some() && a.chest_open_progress > 0.0 && !a.used {
                            let warm = a.chest_open_progress.min(1.0);
                            let shimmer = 0.9 + 0.1 * (run.elapsed * 5.0 + a.spawn as f32).sin();
                            lights.push(PointLight3D::new(
                                a.position + Vec3::Y * 0.55,
                                [1.0, 0.72, 0.3],
                                5.5 * warm * shimmer * tier_size,
                                3.4,
                            ));
                        }
                        // The vault's chest is the room's centrepiece: larger,
                        // with its revealed loot scaled to match.
                        let k = if tier.is_some() {
                            tier_size
                        } else if run.decor[s.room].vault.is_some()
                            || run.decor[s.room]
                                .theme
                                .as_ref()
                                .is_some_and(|t| t.0.vault.is_some())
                        {
                            1.3
                        } else {
                            1.0
                        };
                        if let Some(tris) = self.assets.draw_hinged(
                            model,
                            &mut draws,
                            a.position,
                            0.62 * k,
                            chest_yaw,
                            distance,
                            chest_lid_openness(a.chest_open_progress),
                        ) {
                            self.stats.triangles += tris;
                        } else {
                            self.draw(
                                &mut draws,
                                self.cube,
                                a.position + Vec3::Y * 0.32,
                                Vec3::new(0.95, 0.55, 0.62),
                                chest_yaw,
                                color,
                            );
                            for x in [-0.34, 0.34] {
                                self.draw(
                                    &mut draws,
                                    self.cube,
                                    chest_point(a.position, Vec3::new(x, 0.34, 0.0), chest_yaw),
                                    Vec3::new(0.08, 0.59, 0.66),
                                    chest_yaw,
                                    [0.82, 0.64, 0.25],
                                );
                            }
                        }
                        if !a.used && a.chest_open_progress >= CHEST_LOOT_REVEAL_PROGRESS {
                            let reveal = ((a.chest_open_progress - CHEST_LOOT_REVEAL_PROGRESS)
                                / (1.0 - CHEST_LOOT_REVEAL_PROGRESS))
                                .clamp(0.0, 1.0);
                            let reveal = reveal * reveal * (3.0 - 2.0 * reveal);
                            let bob = (run.elapsed * 2.4 + a.spawn as f32).sin() * 0.006 * reveal;
                            let sparkle =
                                0.97 + (run.elapsed * 3.1 + a.spawn as f32).sin().abs() * 0.03;
                            self.draw(
                                &mut draws,
                                self.chest_gold,
                                chest_point(
                                    a.position,
                                    Vec3::new(-0.16, 0.18 + 0.015 * reveal, -0.14) * k,
                                    chest_yaw,
                                ),
                                Vec3::new(0.38, 0.46, 0.38) * (reveal.max(0.001) * k),
                                chest_yaw,
                                [1.0, sparkle, 0.84],
                            );
                            // Keep every reward below the chest rim and inside
                            // its footprint: gold left, tonic center, herbs right.
                            let potion_position = chest_point(
                                a.position,
                                Vec3::new(0.0, 0.17 + 0.02 * reveal + bob, -0.14) * k,
                                chest_yaw,
                            );
                            let scale = Vec3::new(0.17, 0.18, 0.17) * (reveal.max(0.001) * k);
                            self.draw(
                                &mut draws,
                                self.healing_potion,
                                potion_position,
                                scale,
                                chest_yaw + (run.elapsed * 0.25).sin() * 0.08,
                                [1.0; 3],
                            );
                            let herb_count = super::chest_herb_count(run.seed(), a.spawn);
                            let herb_models = if herb_count > 1 { 2 } else { 1 };
                            for herb_index in 0..herb_models {
                                let x = if herb_models == 1 {
                                    0.17
                                } else {
                                    0.10 + herb_index as f32 * 0.13
                                };
                                let herb_position = chest_point(
                                    a.position,
                                    Vec3::new(x, 0.17 + 0.02 * reveal + bob, -0.14) * k,
                                    chest_yaw,
                                );
                                if let Some(triangles) = self.assets.draw(
                                    "healing_herbs",
                                    &mut draws,
                                    herb_position,
                                    0.15 * reveal.max(0.001) * k,
                                    chest_yaw
                                        + (run.elapsed * 0.22 + herb_index as f32 * 0.12).sin()
                                            * 0.05,
                                ) {
                                    self.stats.triangles += triangles;
                                }
                            }
                        }
                    }
                    Trap => {
                        for i in 0..6 {
                            let theta = i as f32;
                            self.draw(
                                &mut draws,
                                self.crystal,
                                a.position + Vec3::new(theta.cos() * 0.5, 0.0, theta.sin() * 0.5),
                                Vec3::new(0.25, 0.2, 0.25),
                                theta,
                                color,
                            );
                        }
                    }
                    _ => {
                        let props = self.landmarks[run.map.rooms[s.room].biome];
                        let approach = run
                            .authored
                            .as_ref()
                            .and_then(|mine| mine.face(s.id))
                            .or_else(|| {
                                run.map.rooms[s.room]
                                    .sockets
                                    .first()
                                    .map(|socket| socket.position - a.position)
                            })
                            .unwrap_or(Vec3::Z);
                        let yaw = approach.x.atan2(approach.z);
                        let seal_index = (s.kind == Seal).then(|| {
                            s.definition
                                .trim_start_matches("sigil_")
                                .parse::<usize>()
                                .unwrap_or(0)
                                .min(2)
                        });
                        if let Some(index) = seal_index {
                            self.draw(
                                &mut draws,
                                props.runestone,
                                a.position,
                                Vec3::new(0.72, 1.35, 0.52),
                                yaw,
                                [1.0; 3],
                            );
                            let facing = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                            self.draw_rotating(
                                &mut draws,
                                self.rune_spiral,
                                a.position + Vec3::Y * 0.80 + facing * 0.10,
                                Vec3::new(0.30, 0.30, 0.06),
                                yaw,
                                run.rune_angle(index),
                                runes::phase_tint(run.sigils[index]),
                            );
                            runes::add_light(
                                &mut lights,
                                a.position,
                                yaw,
                                run.sigils[index],
                                run.elapsed,
                                run.rune_changed_at[index],
                            );
                        } else if s.kind == Exit {
                            self.draw(
                                &mut draws,
                                props.altar,
                                a.position,
                                Vec3::new(1.15, 0.95, 1.0),
                                yaw,
                                [1.0; 3],
                            );
                        } else if s.kind == Rest
                            && (run.decor[s.room].vault.is_some()
                                || run.decor[s.room]
                                    .theme
                                    .as_ref()
                                    .is_some_and(|t| t.0.vault.is_some()))
                        {
                        } else if s.kind == Rest {
                            self.draw(
                                &mut draws,
                                props.crystal,
                                a.position,
                                Vec3::new(0.82, 1.15, 0.82),
                                yaw,
                                [1.0; 3],
                            );
                        } else {
                            self.draw(
                                &mut draws,
                                props.runestone,
                                a.position,
                                Vec3::new(0.72, 1.35, 0.52),
                                yaw,
                                [1.0; 3],
                            );
                            let facing = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                            self.draw_rotating(
                                &mut draws,
                                self.rune_spiral,
                                a.position + Vec3::Y * 0.80 + facing * 0.10,
                                Vec3::new(0.30, 0.30, 0.06),
                                yaw,
                                0.0,
                                [1.0, 0.78, 0.40],
                            );
                        }
                        if s.kind != Seal && s.kind != Rest {
                            let float_height = if s.kind == Exit { 1.15 } else { 1.55 };
                            self.draw(
                                &mut draws,
                                self.crystal,
                                a.position
                                    + Vec3::Y * (float_height + (run.elapsed * 1.4).sin() * 0.045),
                                Vec3::new(0.16, 0.3, 0.16),
                                run.elapsed * 0.24,
                                color,
                            );
                        }
                    }
                }
                if s.kind != Seal {
                    lights.push(PointLight3D::new(
                        a.position + Vec3::Y * 1.1,
                        color,
                        1.3,
                        6.0,
                    ));
                }
            }
        }
        phases[3] = lap(&mut clock);
        self.stats.phase_draws[3] = draws.len();
        let front = (camera.target - camera.position).normalize_or_zero();
        let right = front.cross(Vec3::Y).normalize_or_zero();
        let speed = Vec3::new(run.controller.velocity.x, 0.0, run.controller.velocity.z).length();
        let moving = (speed / 3.1).clamp(0.0, 1.0);
        let gait = run.controller.distance * 2.35;
        let walk_sway = gait.sin() * moving;
        let walk_bob = (gait * 2.0).sin().abs() * moving;
        let breath = (run.elapsed * 1.6).sin();
        // Lantern stays in the left hand; the shop's wooden sword is the
        // player's right-hand weapon and follows a short, readable slash arc.
        if run.torch {
            let torch_sway = breath * 0.012 + walk_sway * 0.035;
            let torch_bob = breath * 0.008 + walk_bob * 0.025;
            let torch_position = camera.position + front * 0.66 - right * 0.30 + right * torch_sway
                - Vec3::Y * (0.43 + torch_bob);
            let torch_rotation = Vec3::new(
                -0.12 - walk_sway * 0.045,
                run.controller.yaw + 0.10,
                0.14 + walk_sway * 0.11 + breath * 0.025,
            );
            let torch_transform = Transform3D {
                position: torch_position,
                rotation: torch_rotation,
                scale: Vec3::new(0.92, 0.92, 0.92),
            }
            .matrix();
            draws.push(MeshDraw3D {
                mesh: self.hand_torch,
                transform: torch_transform,
                material: Material3D::rgb(1.0, 1.0, 1.0),
                texture: None,
            });
            let left_hand = torch_transform.transform_point3(Vec3::new(0.0, -0.23, 0.0));
            let left_up = torch_transform
                .transform_vector3(Vec3::Y)
                .normalize_or_zero();
            draw_view_arm(
                &mut draws,
                self.fist_left,
                self.sleeve_cuff,
                self.sleeve_upper,
                left_hand,
                left_up,
                camera.position + fp::camera_vector(camera, Vec3::new(-0.22, -0.48, 0.06)),
                fp::camera_vector(camera, Vec3::new(-0.7, -1.0, 0.25)).normalize_or_zero(),
                false,
                0.0,
            );

            let flicker =
                (run.elapsed * 17.0).sin() * 0.65 + (run.elapsed * 29.0 + 0.7).sin() * 0.35;
            let flame_rotation = torch_rotation + Vec3::new(0.0, 0.0, flicker * 0.055);
            let outer_flame = Transform3D {
                position: torch_position,
                rotation: flame_rotation,
                scale: Vec3::new(0.072, 0.175 + flicker * 0.016, 0.072),
            }
            .matrix();
            draws.push(MeshDraw3D {
                mesh: self.flame,
                transform: outer_flame,
                material: Material3D::rgb(0.16, 0.034, 0.004).with_emission(5.4),
                texture: None,
            });
            let inner_flame = Transform3D {
                position: torch_position + Vec3::Y * 0.012,
                rotation: flame_rotation + Vec3::new(0.0, 0.0, -flicker * 0.07),
                scale: Vec3::new(0.032, 0.105 + flicker * 0.008, 0.032),
            }
            .matrix();
            draws.push(MeshDraw3D {
                mesh: self.flame,
                transform: inner_flame,
                material: Material3D::rgb(0.18, 0.10, 0.025).with_emission(6.4),
                texture: None,
            });
            for i in 0..3 {
                let phase = (run.elapsed * 0.72 + i as f32 / 3.0).fract();
                let ember = torch_position
                    + Vec3::Y * (0.17 + phase * 0.30)
                    + right * ((phase * 5.0 + i as f32).sin() * 0.034);
                let size = 0.007 * (1.0 - phase * 0.68);
                self.draw(
                    &mut draws,
                    self.crystal,
                    ember,
                    Vec3::new(size, size * 2.5, size),
                    run.elapsed * 1.6 + i as f32,
                    [1.0, 0.35, 0.045],
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(2.8);
                }
            }
            lights.push(PointLight3D::new(
                torch_position + Vec3::Y * 0.12,
                [1.0, 0.55, 0.22],
                2.3 + flicker * 0.16,
                6.4 + flicker * 0.25,
            ));
        }

        let sword_pose = if run.sword_swing > 0.0 {
            let clip = super::fp::SWINGS[run.sword_combo];
            super::fp::pose(
                run.sword_combo,
                clip.duration - run.sword_swing,
                run.sword_from,
            )
        } else {
            super::fp::REST
        };
        let held = super::fp::world_pose(camera, sword_pose, walk_sway, walk_bob, breath);
        draw_view_arm(
            &mut draws,
            self.fist_right,
            self.sleeve_cuff,
            self.sleeve_upper,
            held.hand,
            held.blade,
            held.shoulder,
            fp::camera_vector(camera, Vec3::new(0.7, -1.0, 0.25)).normalize_or_zero(),
            true,
            sword_pose.roll,
        );
        // The sword, when one is worn (taken off, the fist is empty), slime and all.
        if run.armed {
            if let Some(triangles) = self.assets.draw_held_sword(
                &mut draws,
                held.hand,
                held.blade,
                held.shoulder,
                sword_pose.roll,
                run.relic_sword,
                run.sword_goo.map(|c| (c, run.elapsed)),
            ) {
                self.stats.triangles += triangles;
            }
        }
        if run.sword_swing > 0.0 {
            let clip = fp::SWINGS[run.sword_combo];
            let elapsed = clip.duration - run.sword_swing;
            if (clip.trail.0..=clip.trail.1).contains(&elapsed) {
                let vertices = fp::trail_vertices(
                    camera,
                    run.sword_combo,
                    elapsed,
                    run.sword_from,
                    walk_sway,
                    walk_bob,
                    breath,
                );
                if renderer
                    .update_mesh_vertices(queue, self.weapon_trail, &vertices)
                    .is_ok()
                {
                    let mut material = Material3D::rgb(1.0, 0.65, 0.24).with_emission(1.2);
                    material.base_color[3] = 0.36;
                    draws.push(MeshDraw3D {
                        mesh: self.weapon_trail,
                        transform: Mat4::IDENTITY,
                        material,
                        texture: None,
                    });
                    self.stats.triangles += 24;
                }
            }
        }
        if run.heart.rain > 0.01 {
            self.draw_blood_rain(run, camera, &mut draws, &mut lights);
        }
        // Enemy weapon trails and the combat effects in flight.
        let now = run.elapsed;
        self.enemy_trails.retain(|_, trail| trail.prune(now));
        for (trail, mesh) in self.enemy_trails.values().zip(&self.trail_meshes) {
            if renderer
                .update_mesh_vertices(queue, *mesh, &trail.vertices(now))
                .is_ok()
            {
                let mut material = Material3D::rgb(1.0, 1.0, 1.0).with_emission(1.2);
                material.base_color[3] = 0.9;
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: Mat4::IDENTITY,
                    material,
                    texture: None,
                });
                self.stats.triangles += super::combat_fx::TRAIL_SEGMENTS * 4;
            }
        }
        let fx_meshes = super::combat_fx::FxMeshes {
            cube: self.cube,
            crystal: self.crystal,
            ring: self.fx_ring,
            fissure: self.spell_fissure,
            orb: self.fx_orb,
        };
        self.stats.triangles +=
            super::combat_fx::draw(&run.fx, &fx_meshes, &mut draws, &mut lights);
        phases[4] = lap(&mut clock);
        self.stats.phase_draws[4] = draws.len();
        lights.sort_by(|a, b| {
            (a.position - camera.position)
                .length_squared()
                .total_cmp(&(b.position - camera.position).length_squared())
        });
        lights.truncate(16);
        let r = &run.map.rooms[run.map.region_at(run.controller.position)];
        let mood = self.room_mood(run, r.id);
        let mut environment = EnvironmentFrame3D::default();
        // Blood rain stains the air red.
        let rain = run.heart.rain;
        let mut mood = mood;
        for (i, red) in [0.32f32, 0.02, 0.03].into_iter().enumerate() {
            mood.fog[i] += (red - mood.fog[i]) * rain * 0.6;
        }
        // Out of the hand-made mine's mouth (it opens south, towards +z) it is night: a moonlit
        // sky and fill instead of the cave's dark.
        let outside = if authored.is_some() {
            let t = ((camera.position.z + 2.0) / 6.0).clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        } else {
            0.0
        };
        for (i, (sky, fill)) in [(0.045f32, 0.30f32), (0.05, 0.33), (0.11, 0.5)]
            .into_iter()
            .enumerate()
        {
            mood.fog[i] += (sky - mood.fog[i]) * outside;
            mood.ambient[i] += (fill - mood.ambient[i]) * outside;
        }
        environment.clear_color = [mood.fog[0], mood.fog[1], mood.fog[2], 1.0];
        environment.celestial = Celestial3D::default();
        // Darker, tinted indirect light: torches, braziers and glows carve
        // readable pools instead of a flat, evenly lit sandbox.
        environment.lighting = Lighting3D {
            direction_to_light: Vec3::new(-0.4, 0.7, 0.25).normalize_or_zero(),
            key_color: [
                mood.ambient[0] * (0.25 + 1.1 * outside),
                mood.ambient[1] * (0.25 + 1.1 * outside),
                mood.ambient[2] * (0.25 + 1.1 * outside),
            ],
            ambient_color: mood.ambient,
            bounce_color: [
                mood.glow[0] * 0.018,
                mood.glow[1] * 0.018,
                mood.glow[2] * 0.018,
            ],
            shadow_extent: 24.0,
        };
        environment.atmosphere = Atmosphere3D {
            fog_color: mood.fog,
            fog_start: 12.0 + 18.0 * outside,
            fog_end: 62.0 + 60.0 * outside,
            fog_density: 0.011,
            height_falloff: 0.0,
        };
        self.stats.resident_chunks = self.slots.len();
        self.stats.draws = draws.len();
        let previous_toon = renderer.replace_toon_style(ToonStyle3D {
            softness: 0.10,
            specular: 0.2,
            normal_detail: 0.30,
            shadow_tint: [0.58, 0.68, 0.95],
            rim: 0.02,
        });
        // Caves are lit by local lights only: skip the directional shadow
        // pass (a full second geometry pass) and restore it for other scenes.
        let previous_shadows = renderer.replace_directional_shadows(false);
        phases[5] = lap(&mut clock);
        self.stats.phase_draws[5] = draws.len();
        renderer.render_with_environment_and_lights(
            device,
            queue,
            &self.finish.scene,
            size,
            camera,
            environment,
            &lights,
            &draws,
        );
        renderer.replace_toon_style(previous_toon);
        renderer.replace_directional_shadows(previous_shadows);
        phases[6] = lap(&mut clock);
        self.finish.render(device, queue, target);
        phases[7] = lap(&mut clock);
        self.hud
            .render(run, self.stats, device, queue, target, size);
        phases[8] = lap(&mut clock);
        self.stats.phases = phases;
        for i in 6..9 {
            self.stats.phase_draws[i] = self.stats.phase_draws[5];
        }
    }
    /// Biome mood of a room; the treasury glows gold whatever the biome.
    fn room_mood(&self, run: &Dungeon, id: usize) -> super::surfaces::Mood {
        let r = &run.map.rooms[id];
        if r.role == Role::Treasure && r.shape != Shape::Bridge {
            let mut mood = self.moods[0];
            mood.ambient = [0.11, 0.072, 0.045];
            mood.fog = [0.05, 0.028, 0.012];
            mood.glow = [1.0, 0.62, 0.2];
            mood
        } else {
            let mood = self.moods[r.biome.min(self.moods.len() - 1)];
            super::themes::of(&run.map, r)
                .map(|theme| super::themes::mood(theme, mood))
                .unwrap_or(mood)
        }
    }

    fn insert_vault(
        &mut self,
        id: usize,
        detail: Mesh3D,
        glow: Mesh3D,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
    ) {
        let mut upload = |mesh: &Mesh3D| -> Option<MeshHandle> {
            if mesh.indices.is_empty() {
                return None;
            }
            match self.vault_mesh_pool.pop() {
                Some(handle) => renderer
                    .replace_mesh(device, handle, mesh)
                    .ok()
                    .map(|_| handle),
                None => renderer.upload_mesh(device, mesh).ok(),
            }
        };
        let entry = (
            upload(&detail),
            upload(&glow),
            detail.indices.len() / 3 + glow.indices.len() / 3,
        );
        self.vault_meshes.insert(id, entry);
    }

    /// Upload a themed room's beating glow (`pulse`), and once its plan is
    /// known (`moving`), the Heart and the orrery parts.
    fn insert_theme_extra(
        &mut self,
        id: usize,
        pulse: &Mesh3D,
        moving: Option<(&Blueprint, &Catalog, &super::art::plan::Plan)>,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
    ) {
        let pool = &mut self.vault_mesh_pool;
        let mut upload = |mesh: &Mesh3D| -> Option<MeshHandle> {
            if mesh.indices.is_empty() {
                return None;
            }
            match pool.pop() {
                Some(handle) => renderer
                    .replace_mesh(device, handle, mesh)
                    .ok()
                    .map(|_| handle),
                None => renderer.upload_mesh(device, mesh).ok(),
            }
        };
        let extra = self.theme_extra.entry(id).or_default();
        if moving.is_none() {
            extra.pulse_glow = upload(pulse);
            return;
        }
        let Some((run_map, run_catalog, plan)) = moving else {
            return;
        };
        let seed = run_map.seed;
        extra.moving_built = true;
        if plan.heart.is_some() {
            let (mut body, mut glow) = (super::visual::empty(), super::visual::empty());
            let mut rng = super::art::kit::ArtRng::stream(seed, "heart_body_v1", id as u64);
            super::art::themekit::heart_body(&mut body, &mut glow, &mut rng);
            extra.heart = Some((upload(&body), upload(&glow)));
        }
        if plan.font_core.is_some() {
            let glow = super::streaming::art_glow(&run_map, &run_catalog, &run_map.rooms[id]);
            let core = super::art::sanctuary::font_core_mesh(glow);
            let (mut a, mut b) = (super::visual::empty(), super::visual::empty());
            super::art::kit::tube_ring(
                &mut a,
                Vec3::ZERO,
                Vec3::Y,
                0.25,
                0.01,
                40,
                [1.05, 0.66, 0.2],
            );
            super::art::kit::tube_ring(
                &mut b,
                Vec3::ZERO,
                Vec3::Y,
                0.19,
                0.007,
                32,
                [1.05, 0.66, 0.2],
            );
            extra.font_core = Some((upload(&core), upload(&a), upload(&b)));
        }
        if plan.orrery.is_some() {
            let (rings, sun) = super::art::library::orrery_parts();
            let rings = rings
                .iter()
                .filter_map(|ring| upload(&ring.mesh).map(|h| (h, ring.tilt, ring.speed)))
                .collect();
            extra.orrery = Some((rings, upload(&sun)));
        }
    }

    /// Queue builds around the player and pre-upload at most one finished
    /// build per frame, so walking into new halls never stalls on meshing.
    fn stream_ahead(
        &mut self,
        run: &Dungeon,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        visible: &BTreeSet<usize>,
    ) {
        use super::streaming::{horizon, Built, Job};
        if run.authored.is_some() {
            return;
        }
        let region = run.map.region_at(run.controller.position);
        let slots = &self.slots;
        let vaults = &self.vault_meshes;
        let resident = |job: Job| match job {
            Job::Chunk(key) => slots.iter().any(|s| s.key == key),
            Job::Vault(id) => vaults.contains_key(&id),
        };
        let Some(streamer) = self.streamer.as_mut() else {
            return;
        };
        // Four rooms out: the portal view reaches corridors leaving rooms
        // three hops away, so those are built before they come into view.
        streamer.prefetch(horizon(&run.map, region, 4), resident);
        let slots = &self.slots;
        let vaults = &self.vault_meshes;
        let ready = streamer.next_ready(|job| match job {
            Job::Chunk(key) => slots.iter().any(|s| s.key == key),
            Job::Vault(id) => vaults.contains_key(&id),
        });
        match ready {
            Some((Job::Chunk(key), Built::Chunk(mesh))) => {
                let _ = self.insert_chunk(key, mesh, renderer, device, visible);
            }
            Some((Job::Vault(id), Built::Vault(detail, glow))) => {
                self.insert_vault(id, detail, glow, renderer, device);
            }
            Some((Job::Vault(id), Built::Theme(detail, glow, pulse, water))) => {
                self.insert_vault(id, detail, glow, renderer, device);
                self.insert_theme_extra(id, &pulse, None, renderer, device);
                self.theme_water.insert(id, water);
            }
            _ => {}
        }
    }

    /// A ported themed room: its baked art, mounts, lights, flames and motes.
    #[allow(clippy::too_many_arguments)]
    fn draw_theme(
        &mut self,
        run: &Dungeon,
        id: usize,
        plan: &super::art::plan::Plan,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        camera: Camera3D,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        fixtures: &mut Vec<(Vec3, [f32; 3], f32)>,
    ) {
        let t = run.elapsed;
        if !self.vault_meshes.contains_key(&id) {
            use super::streaming::{Built, Job};
            let built = match self.streamer.as_mut().and_then(|s| s.take(Job::Vault(id))) {
                Some(built @ Built::Theme(..)) => built,
                _ => super::streaming::theme_art(&run.map, &run.catalog, plan, id),
            };
            if let Built::Theme(detail, glow, pulse, water) = built {
                self.insert_vault(id, detail, glow, renderer, device);
                self.insert_theme_extra(id, &pulse, None, renderer, device);
                self.theme_water.insert(id, water);
            }
        }
        if self
            .theme_extra
            .get(&id)
            .is_some_and(|extra| !extra.moving_built)
        {
            self.insert_theme_extra(
                id,
                &super::visual::empty(),
                Some((&run.map, &run.catalog, plan)),
                renderer,
                device,
            );
        }
        if let Some(&(detail, glow, triangles)) = self.vault_meshes.get(&id) {
            if let Some(mesh) = detail {
                draws.push(MeshDraw3D {
                    mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(1.0, 1.0, 1.0).with_ambient_occlusion(0.4),
                    texture: None,
                });
            }
            if let Some(mesh) = glow {
                let flicker = 0.93 + 0.05 * (t * 9.0).sin() + 0.03 * (t * 23.0 + 1.3).sin();
                draws.push(MeshDraw3D {
                    mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(7.2 * flicker),
                    texture: None,
                });
            }
            self.stats.triangles += triangles;
        }
        // The Heart beats twice a cycle (web `#beatHeart`); when its guardian
        // falls, it slows and goes dark.
        let ph = (t % 1.18) / 1.18;
        let beat =
            (-((ph - 0.07) / 0.05).powi(2)).exp() + 0.62 * (-((ph - 0.31) / 0.06).powi(2)).exp();
        let dead = run
            .actors
            .iter()
            .any(|a| run.map.spawns[a.spawn].kind == gameplay::SpawnKind::Boss && a.health <= 0.0);
        let delta = (t - self.heart_clock).clamp(0.0, 0.25);
        self.heart_clock = t;
        self.heart_life = if dead {
            (self.heart_life - delta * 0.35).max(0.0)
        } else {
            (self.heart_life + delta).min(1.0)
        };
        let life = self.heart_life;
        let heart_room = plan.heart.is_some();
        if let Some(extra) = self.theme_extra.get(&id) {
            if let Some(mesh) = extra.pulse_glow {
                // Veins, sigils and runes glow with the Heart or the arcane pulse.
                let strength = if heart_room {
                    0.18 + life * (0.55 + 0.75 * beat)
                } else {
                    0.75 + (0.8 + 0.2 * (t * 1.7).sin()) * 0.35
                };
                draws.push(MeshDraw3D {
                    mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(7.2 * strength),
                    texture: None,
                });
            }
            if let (Some((center, scale)), Some((flesh, veins))) = (plan.heart, extra.heart) {
                let sc = scale * (1.0 + 0.055 * beat * life);
                let transform =
                    Mat4::from_translation(center + Vec3::Y * ((t * 0.8).sin() * 0.06 * life))
                        * Mat4::from_rotation_y(t * 0.12 * (0.2 + 0.8 * life))
                        * Mat4::from_scale(Vec3::new(sc, sc * (1.0 - 0.02 * beat * life), sc));
                if let Some(mesh) = flesh {
                    draws.push(MeshDraw3D {
                        mesh,
                        transform,
                        material: Material3D::rgb(1.0, 1.0, 1.0).with_ambient_occlusion(0.4),
                        texture: None,
                    });
                }
                if let Some(mesh) = veins {
                    draws.push(MeshDraw3D {
                        mesh,
                        transform,
                        material: Material3D::rgb(0.25, 0.25, 0.25)
                            .with_emission(7.2 * (0.18 + life * (0.55 + 0.75 * beat))),
                        texture: None,
                    });
                }
            }
            if let (Some(center), Some((rings, sun))) = (plan.orrery, extra.orrery.as_ref()) {
                // Brass rings turning at their own pace about a glowing sun.
                for &(mesh, tilt, speed) in rings {
                    let transform = Mat4::from_translation(center)
                        * Mat4::from_rotation_x(tilt)
                        * Mat4::from_rotation_z(tilt * 0.5)
                        * Mat4::from_rotation_y(t * speed);
                    draws.push(MeshDraw3D {
                        mesh,
                        transform,
                        material: Material3D::rgb(1.0, 1.0, 1.0).with_ambient_occlusion(0.4),
                        texture: None,
                    });
                }
                if let Some(mesh) = *sun {
                    let flicker = 0.93 + 0.05 * (t * 9.0).sin() + 0.03 * (t * 23.0 + 1.3).sin();
                    draws.push(MeshDraw3D {
                        mesh,
                        transform: Mat4::from_translation(center),
                        material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(7.2 * flicker),
                        texture: None,
                    });
                }
            }
        }
        if let (Some(center), Some((core, ring_a, ring_b))) = (
            plan.font_core,
            self.theme_extra.get(&id).and_then(|e| e.font_core),
        ) {
            // The rest font's ember core floats, turns and breathes.
            let root = center + Vec3::Y * ((t * 1.3).sin() * 0.05);
            let scale = 1.0 + 0.05 * (t * 2.1).sin();
            if let Some(mesh) = core {
                let transform = Mat4::from_translation(root)
                    * Mat4::from_rotation_y(t * 0.7)
                    * Mat4::from_scale(Vec3::new(scale, scale, scale));
                draws.push(MeshDraw3D {
                    mesh,
                    transform,
                    material: Material3D::rgb(0.25, 0.25, 0.25)
                        .with_emission(7.2 * (0.9 + 0.12 * (t * 2.1).sin())),
                    texture: None,
                });
            }
            if let Some(mesh) = ring_a {
                let transform = Mat4::from_translation(root)
                    * Mat4::from_rotation_y(t * 0.9)
                    * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2 + (t * 0.5).sin() * 0.35);
                draws.push(MeshDraw3D {
                    mesh,
                    transform,
                    material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(5.0),
                    texture: None,
                });
            }
            if let Some(mesh) = ring_b {
                let transform = Mat4::from_translation(root)
                    * Mat4::from_rotation_z(0.4)
                    * Mat4::from_rotation_y(-t * 1.3)
                    * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2 + (t * 0.6).cos() * 0.5);
                draws.push(MeshDraw3D {
                    mesh,
                    transform,
                    material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(5.0),
                    texture: None,
                });
            }
        }
        let distance = (plan.focus - camera.position).length();
        // Open chests overflowing with the baked gold.
        for &(position, yaw) in &plan.open_chests {
            let d = (position - camera.position).length();
            if let Some(tris) = self
                .assets
                .draw_hinged("chest", draws, position, 0.62, yaw, d, 1.0)
            {
                self.stats.triangles += tris;
            }
        }
        // Pilaster sconces with live flames.
        for sconce in &plan.sconces {
            if let Some(tris) =
                self.assets
                    .draw_back_mounted("torch", draws, sconce.backplate, 0.83, sconce.yaw)
            {
                self.stats.triangles += tris;
            }
            fixtures.push((sconce.flame, [1.0, 0.56, 0.18], 1.35));
        }
        let glow_color = super::streaming::art_glow(&run.map, &run.catalog, &run.map.rooms[id]);
        for (index, &(position, intensity, radius)) in plan.glow_lights.iter().enumerate() {
            let flicker = 1.0 + 0.08 * (t * 5.3 + index as f32 * 2.1).sin();
            lights.push(PointLight3D::new(
                position,
                glow_color,
                intensity * flicker,
                radius,
            ));
        }
        for weapon in &plan.weapons {
            if let Some(tris) = self.assets.draw_oriented(
                weapon_asset(weapon.name),
                draws,
                weapon.position,
                weapon.height,
                weapon.rotation,
                distance,
            ) {
                self.stats.triangles += tris;
            }
        }
        for mount in &plan.mounts {
            if let Some(tris) = self.assets.draw_back_mounted(
                mount.asset,
                draws,
                mount.anchor,
                mount.height,
                mount.yaw,
            ) {
                self.stats.triangles += tris;
            }
        }
        for f in &plan.fixtures {
            fixtures.push((f.p, f.color, f.size));
        }
        for (index, light) in plan.lights.iter().enumerate() {
            let flicker = if light.beat {
                (0.25 + 0.75 * life) * (0.62 + 0.9 * beat * life)
            } else {
                1.0 + 0.05 * (t * 7.3 + index as f32 * 1.7).sin()
            };
            lights.push(PointLight3D::new(
                light.position,
                light.color,
                light.intensity * flicker,
                light.radius,
            ));
        }
        // Motes drift upward through the hall, tinted by its theme.
        if (plan.focus - camera.position).length() < 26.0 {
            let tint = match plan.vault {
                Some(super::art::plan::VaultKind::Sanctuary) => visual::tint(
                    super::streaming::art_glow(&run.map, &run.catalog, &run.map.rooms[id]),
                    1.5,
                ),
                _ => plan.sparkle_tint.unwrap_or([1.2, 1.0, 0.6]),
            };
            for (k, p) in plan.sparkles.iter().enumerate() {
                let phase = t * 0.9 + k as f32 * 1.37;
                let twinkle = phase.sin() * 0.5 + 0.5;
                let rise = (t * 0.12 + k as f32 * 0.173).fract();
                let position = *p
                    + Vec3::new(
                        (t * 0.4 + k as f32).sin() * 0.25,
                        rise * 1.4 - 0.7,
                        (t * 0.33 + k as f32).cos() * 0.25,
                    );
                let size = 0.028 * (0.4 + twinkle);
                self.draw_rotating(
                    draws,
                    self.crystal,
                    position,
                    Vec3::new(size, size * 2.2, size),
                    t + k as f32,
                    std::f32::consts::FRAC_PI_4,
                    visual::tint(tint, 0.12),
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(7.0);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_vault(
        &mut self,
        run: &Dungeon,
        id: usize,
        vault: &super::vaults::Vault,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        camera: Camera3D,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        fixtures: &mut Vec<(Vec3, [f32; 3], f32)>,
    ) {
        use super::vaults::{Kind, ARCANE};
        let t = run.elapsed;
        let arcane = vault.kind == Kind::Arcane;
        let pulse = 0.8 + 0.2 * (t * if arcane { 1.7 } else { 1.1 }).sin();

        // Static detail + glow meshes, built once per room and seed.
        if !self.vault_meshes.contains_key(&id) {
            use super::streaming::{Built, Job};
            let (detail, glow) = match self.streamer.as_mut().and_then(|s| s.take(Job::Vault(id))) {
                Some(Built::Vault(detail, glow)) => (detail, glow),
                _ => super::vaults::meshes(
                    vault,
                    &run.catalog.biomes[run.map.rooms[id].biome],
                    run.seed(),
                    id,
                ),
            };
            self.insert_vault(id, detail, glow, renderer, device);
        }
        if let Some(&(detail, glow, triangles)) = self.vault_meshes.get(&id) {
            if let Some(mesh) = detail {
                draws.push(MeshDraw3D {
                    mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(1.0, 1.0, 1.0).with_ambient_occlusion(0.4),
                    texture: None,
                });
            }
            if let Some(mesh) = glow {
                // Low albedo, strong emission: flames and gems keep their hue
                // even inside their own point lights; a shared flicker.
                let flicker = 0.93 + 0.05 * (t * 9.0).sin() + 0.03 * (t * 23.0 + 1.3).sin();
                let strength = if arcane {
                    4.5 + pulse * 2.0
                } else {
                    7.2 * flicker
                };
                draws.push(MeshDraw3D {
                    mesh,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(0.25, 0.25, 0.25).with_emission(strength),
                    texture: None,
                });
            }
            self.stats.triangles += triangles;
        }

        // Authored props (gilded chest, trunk, arcane book stands...).
        for prop in vault.props.iter().filter(|p| !p.asset.is_empty()) {
            if let Some(tris) =
                self.assets
                    .draw(prop.asset, draws, prop.position, prop.height, prop.yaw)
            {
                self.stats.triangles += tris;
            }
        }
        for mount in &vault.mounts {
            if let Some(tris) = self.assets.draw_back_mounted(
                mount.asset,
                draws,
                mount.anchor,
                mount.height,
                mount.yaw,
            ) {
                self.stats.triangles += tris;
            }
        }
        // Open chests overflowing with the baked gold.
        for &(position, yaw) in &vault.open_chests {
            let distance = (position - camera.position).length();
            if let Some(tris) = self
                .assets
                .draw_hinged("chest", draws, position, 0.62, yaw, distance, 1.0)
            {
                self.stats.triangles += tris;
            }
        }
        // Pilaster sconces with live flames.
        for sconce in &vault.sconces {
            if let Some(tris) =
                self.assets
                    .draw_back_mounted("torch", draws, sconce.backplate, 0.83, sconce.yaw)
            {
                self.stats.triangles += tris;
            }
            fixtures.push((sconce.flame, [1.0, 0.56, 0.18], 1.35));
        }
        for flame in &vault.flames {
            fixtures.push((*flame, [1.0, 0.62, 0.2], 0.42));
        }
        for (index, flame) in vault.flames.iter().enumerate() {
            if index % 2 == 0 {
                lights.push(PointLight3D::new(
                    *flame + Vec3::Y * 0.2,
                    [1.0, 0.62, 0.26],
                    3.2 + (t * 6.0 + index as f32).sin() * 0.25,
                    5.5,
                ));
            }
        }
        let glow_color = self.room_mood(run, id).glow;
        for (index, &(position, intensity, radius)) in vault.glow_lights.iter().enumerate() {
            let flicker = 1.0 + 0.08 * (t * 5.3 + index as f32 * 2.1).sin();
            lights.push(PointLight3D::new(
                position,
                glow_color,
                intensity * flicker,
                radius,
            ));
        }
        for (index, light) in vault.lights.iter().enumerate() {
            let flicker = 1.0
                + 0.05 * (t * 7.3 + index as f32 * 1.7).sin()
                + 0.03 * (t * 13.1 + index as f32).sin();
            lights.push(PointLight3D::new(
                light.position,
                light.color,
                light.intensity * flicker,
                light.radius,
            ));
        }

        // Sparkles: gold glints twinkle in place; arcane motes drift upward.
        if (vault.focus - camera.position).length() < 26.0 {
            for (k, p) in vault.sparkles.iter().enumerate() {
                let drift = vault.kind != Kind::Treasury;
                let phase = t * if drift { 0.9 } else { 2.6 } + k as f32 * 1.37;
                let wave = phase.sin() * 0.5 + 0.5;
                let twinkle = if drift { wave } else { wave.powi(6) };
                let (position, size) = if drift {
                    let rise = (t * 0.12 + k as f32 * 0.173).fract();
                    (
                        *p + Vec3::new(
                            (t * 0.4 + k as f32).sin() * 0.25,
                            rise * 1.4 - 0.7,
                            (t * 0.33 + k as f32).cos() * 0.25,
                        ),
                        0.028 * (0.4 + twinkle),
                    )
                } else {
                    (*p, 0.06 * twinkle)
                };
                if size < 0.004 {
                    continue;
                }
                self.draw_rotating(
                    draws,
                    self.crystal,
                    position,
                    Vec3::new(size, size * 2.2, size),
                    t + k as f32,
                    std::f32::consts::FRAC_PI_4,
                    match vault.kind {
                        Kind::Arcane => [0.1, 0.03, 0.2],
                        Kind::Sanctuary => visual::tint(glow_color, 0.22),
                        Kind::Treasury => [0.25, 0.2, 0.1],
                    },
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(if arcane { 5.5 } else { 7.0 });
                }
            }
        }

        if arcane {
            // Floating heart-crystal over the chest, with orbiting shards.
            let heart = vault.focus + Vec3::Y * (2.25 + (t * 1.2).sin() * 0.08);
            self.draw_rotating(
                draws,
                self.crystal,
                heart - Vec3::Y * 0.3,
                Vec3::new(0.2, 0.62, 0.2),
                t * 0.6,
                0.0,
                [0.1, 0.035, 0.22],
            );
            if let Some(draw) = draws.last_mut() {
                draw.material = draw.material.with_emission(5.0 + pulse * 1.5);
            }
            for k in 0..5 {
                let a = t * 0.9 + k as f32 * std::f32::consts::TAU / 5.0;
                let p = heart + Vec3::new(a.cos() * 0.62, (a * 2.0).sin() * 0.12, a.sin() * 0.62);
                self.draw_rotating(
                    draws,
                    self.crystal,
                    p - Vec3::Y * 0.08,
                    Vec3::new(0.05, 0.16, 0.05),
                    a,
                    0.3,
                    visual::tint(ARCANE, 0.35),
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(7.0);
                }
            }
            lights.push(PointLight3D::new(heart, ARCANE, 8.5 + pulse * 1.5, 9.5));
            for (k, p) in vault.obelisks.iter().enumerate() {
                let tip = *p + Vec3::Y * 2.3;
                self.draw_rotating(
                    draws,
                    self.crystal,
                    tip + Vec3::Y * 0.12,
                    Vec3::new(0.09, 0.26, 0.09),
                    t * 0.8 + k as f32,
                    0.0,
                    [0.09, 0.03, 0.2],
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(5.0);
                }
                if k % 2 == 0 {
                    lights.push(PointLight3D::new(tip, ARCANE, 3.4 + pulse, 5.0));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_beacon(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        fixtures: &mut Vec<(Vec3, [f32; 3], f32)>,
        room: &Room,
        mood: super::surfaces::Mood,
        p: Vec3,
        index: usize,
        elapsed: f32,
    ) {
        use super::surfaces::Beacon;
        let biome = room.biome.min(self.braziers.len() - 1);
        let phase = elapsed * 2.3 + index as f32 * 1.7 + room.id as f32;
        let yaw = (room.landmark_seed as f32 * 0.37 + index as f32 * 1.1).rem_euclid(1.0) * 1.57;
        match mood.beacon {
            Beacon::Brazier => {
                self.draw(
                    draws,
                    self.braziers[biome],
                    p,
                    Vec3::new(1.0, 1.0, 1.0),
                    yaw,
                    [1.0; 3],
                );
                let flame = p + Vec3::Y * 1.12;
                fixtures.push((flame, mood.flame, 2.5));
                lights.push(PointLight3D::new(
                    flame + Vec3::Y * 0.35,
                    mood.flame,
                    (8.5 + phase.sin() * 0.5) * mood.flame_intensity,
                    7.5,
                ));
            }
            Beacon::Crystal => {
                self.draw(
                    draws,
                    self.landmarks[biome].crystal,
                    p,
                    Vec3::new(1.25, 1.55, 1.25),
                    yaw,
                    [1.0; 3],
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(0.55 + phase.sin().abs() * 0.12);
                }
                for (dx, dz, size) in [(0.75, 0.2, 0.55), (-0.55, 0.6, 0.42)] {
                    let offset =
                        Mat4::from_rotation_y(yaw).transform_vector3(Vec3::new(dx, 0.0, dz));
                    self.draw(
                        draws,
                        self.crystal,
                        p + offset,
                        Vec3::new(size * 0.35, size, size * 0.35),
                        yaw + dx,
                        visual::tint(mood.glow, 0.9),
                    );
                    if let Some(draw) = draws.last_mut() {
                        draw.material = draw.material.with_emission(0.7);
                    }
                }
                lights.push(PointLight3D::new(
                    p + Vec3::Y * 0.9,
                    mood.glow,
                    5.2 + phase.sin() * 0.35,
                    7.0,
                ));
            }
            Beacon::Embers => {
                self.draw(
                    draws,
                    self.ember_rings[biome],
                    p,
                    Vec3::new(1.0, 1.0, 1.0),
                    yaw,
                    [1.0; 3],
                );
                for coal in 0..5 {
                    let a = coal as f32 * 1.26 + yaw;
                    let r = 0.12 + (coal % 3) as f32 * 0.1;
                    self.draw(
                        draws,
                        self.crystal,
                        p + Vec3::new(a.cos() * r, 0.04, a.sin() * r),
                        Vec3::new(0.12, 0.10, 0.12),
                        a,
                        visual::tint(mood.glow, 0.9),
                    );
                    if let Some(draw) = draws.last_mut() {
                        draw.material = draw
                            .material
                            .with_emission(1.4 + (phase * 1.7 + coal as f32).sin() * 0.4);
                    }
                }
                fixtures.push((p + Vec3::Y * 0.08, mood.flame, 1.35));
                lights.push(PointLight3D::new(
                    p + Vec3::Y * 0.6,
                    mood.glow,
                    (7.0 + (phase * 1.3).sin() * 0.8) * mood.flame_intensity,
                    6.8,
                ));
            }
        }
    }

    /// Bridge rooms: a coloured glow rises from each pit quadrant, with a few
    /// luminous shards on the pit floor, so the drop reads as depth, not void.
    fn draw_abyss_glow(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        room: &Room,
        mood: super::surfaces::Mood,
        elapsed: f32,
    ) {
        for (index, (sx, sz)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            let x = sx * (1.8 + room.half[0]) * 0.5;
            let z = sz * (1.8 + room.half[1]) * 0.5;
            let floor = room.center + Vec3::new(x, -4.0, z);
            if !room.contains(floor.x, floor.z, 0.6) {
                continue;
            }
            for shard in 0..3 {
                let a = shard as f32 * 2.1 + index as f32;
                self.draw(
                    draws,
                    self.crystal,
                    floor + Vec3::new(a.cos() * 0.7, 0.0, a.sin() * 0.7),
                    Vec3::new(0.22, 0.55 + shard as f32 * 0.2, 0.22),
                    a,
                    visual::tint(mood.glow, 0.85),
                );
                if let Some(draw) = draws.last_mut() {
                    draw.material = draw.material.with_emission(0.9);
                }
            }
            if index % 2 == 0 {
                lights.push(PointLight3D::new(
                    floor + Vec3::Y * 1.2,
                    mood.glow,
                    7.5 + (elapsed * 1.4 + index as f32).sin() * 0.6,
                    8.5,
                ));
            }
        }
    }

    fn draw_gate_block(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        origin: Vec3,
        yaw: f32,
        local_center: Vec3,
        size: Vec3,
        color: [f32; 3],
        emission: f32,
    ) {
        let offset = Mat4::from_rotation_y(yaw).transform_vector3(local_center);
        self.draw(draws, self.cube, origin + offset, size, yaw, color);
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(emission);
        }
    }

    fn draw_sealed_gate(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        run: &Dungeon,
        connection: &Connection,
        origin: Vec3,
    ) {
        let direction = horizontal(connection.end - connection.start).normalize_or_zero();
        let yaw = direction.x.atan2(direction.z);
        let toward_arrival = -direction;
        let width = connection.width.max(2.2);
        let height = connection.height.max(3.1);
        let biome = &run.catalog.biomes[run.map.rooms[connection.a].biome];
        let stone = biome.wall;
        let warm = [0.96, 0.60, 0.22];
        let energy = if run.seal {
            [0.53, 0.43, 0.92]
        } else {
            biome.accent
        };
        let post_x = width * 0.5 - 0.12;

        // Carved pillars and brass caps echo the dungeon arches. The open
        // lattice keeps the blocked chamber visible instead of covering the
        // player's view with a flat plane.
        for side in [-1.0, 1.0] {
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(side * post_x, height * 0.5, 0.0),
                Vec3::new(0.24, height - 0.06, 0.28),
                visual::tint(stone, 0.74),
                0.0,
            );
            for y in [0.20, height - 0.20] {
                self.draw_gate_block(
                    draws,
                    origin,
                    yaw,
                    Vec3::new(side * post_x, y, 0.0),
                    Vec3::new(0.36, 0.16, 0.34),
                    visual::tint(warm, 0.88),
                    0.05,
                );
            }
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(side * (post_x - 0.08), height * 0.5, -0.16),
                Vec3::new(0.035, height - 0.56, 0.035),
                warm,
                0.14,
            );
        }

        for y in [0.13, height - 0.13] {
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(0.0, y, 0.0),
                Vec3::new(width - 0.12, 0.16, 0.26),
                visual::tint(stone, 0.82),
                0.0,
            );
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(0.0, y, -0.15),
                Vec3::new(width - 0.35, 0.035, 0.035),
                warm,
                0.12,
            );
        }

        let inner_width = width - 0.58;
        let pulse = 0.82 + 0.18 * (run.elapsed * 2.2).sin().abs();
        for index in 0..5 {
            let x = -inner_width * 0.5 + inner_width * (index + 1) as f32 / 6.0;
            let bar_color = if index % 2 == 0 {
                energy
            } else {
                visual::tint(energy, 0.72)
            };
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(x, height * 0.5, 0.0),
                Vec3::new(0.055, height - 0.50, 0.075),
                visual::tint(bar_color, pulse),
                0.22 + pulse * 0.34,
            );
        }
        for y in [0.52, height - 0.52] {
            self.draw_gate_block(
                draws,
                origin,
                yaw,
                Vec3::new(0.0, y, 0.0),
                Vec3::new(inner_width, 0.055, 0.075),
                visual::tint(energy, 0.82),
                0.28,
            );
        }

        // Reuse the carved spiral language of the dungeon's three seals for
        // the central lock; a small faceted crystal gives it a living pulse.
        let medallion_y = height * 0.5;
        let face_yaw = yaw + std::f32::consts::PI;
        let face_offset = toward_arrival * 0.20;
        self.draw(
            draws,
            self.landmarks[run.map.rooms[connection.a].biome].runestone,
            origin + face_offset + Vec3::Y * (medallion_y - 0.63),
            Vec3::new(0.76, 1.30, 0.48),
            face_yaw,
            [1.0; 3],
        );
        let rune_position = origin + face_offset + toward_arrival * 0.07 + Vec3::Y * medallion_y;
        self.draw_rotating(
            draws,
            self.rune_spiral,
            rune_position,
            Vec3::new(0.77, 0.77, 0.14),
            face_yaw,
            run.elapsed * 0.18,
            [1.0; 3],
        );
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(0.34 + pulse * 0.30);
        }
        self.draw(
            draws,
            self.crystal,
            rune_position - Vec3::Y * 0.13 + toward_arrival * 0.025,
            Vec3::new(0.15, 0.30, 0.14),
            face_yaw + run.elapsed * 0.24,
            visual::tint(energy, 1.18),
        );
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(0.42 + pulse * 0.40);
        }
        lights.push(PointLight3D::new(
            rune_position,
            visual::tint(energy, 0.84),
            1.3 + pulse * 0.7,
            4.5,
        ));
    }

    fn draw_room_identity(
        &mut self,
        room_id: usize,
        seed: u64,
        world_time: f32,
        props: &[RoomIdentityProp],
        distance: f32,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
    ) {
        let mut coin_lights = 0;
        let mut gem_lights = 0;
        for (prop_index, prop) in props.iter().enumerate() {
            let p = prop.position;
            match prop.kind {
                RoomIdentityKind::LooseCoin => {
                    if let Some(triangles) =
                        self.assets.draw("coin", draws, p, prop.scale, prop.yaw)
                    {
                        self.stats.triangles += triangles;
                    }
                }
                RoomIdentityKind::CoinStackSmall
                | RoomIdentityKind::CoinStackMedium
                | RoomIdentityKind::CoinStackLarge => {
                    let name = match prop.kind {
                        RoomIdentityKind::CoinStackSmall => "coin_stack_small",
                        RoomIdentityKind::CoinStackMedium => "coin_stack_medium",
                        _ => "coin_stack_large",
                    };
                    if let Some(triangles) = self.assets.draw(name, draws, p, prop.scale, prop.yaw)
                    {
                        self.stats.triangles += triangles;
                    }
                    if prop.kind == RoomIdentityKind::CoinStackLarge
                        && coin_lights < 2
                        && distance < 22.0
                    {
                        lights.push(PointLight3D::new(
                            p + Vec3::Y * 0.35,
                            [1.0, 0.58, 0.14],
                            1.05,
                            4.8,
                        ));
                        coin_lights += 1;
                    }
                }
                RoomIdentityKind::Gem => {
                    let size = prop.scale;
                    self.draw(
                        draws,
                        self.crystal,
                        p + Vec3::Y * 0.025,
                        Vec3::new(size * 0.42, size, size * 0.42),
                        prop.yaw,
                        prop.tint,
                    );
                    if let Some(draw) = draws.last_mut() {
                        draw.material = draw.material.with_emission(0.22);
                    }
                    if gem_lights < 2 && distance < 22.0 {
                        lights.push(PointLight3D::new(
                            p + Vec3::Y * (size * 0.58),
                            prop.tint,
                            0.62,
                            3.2,
                        ));
                        gem_lights += 1;
                    }
                }
                RoomIdentityKind::Crown => {
                    self.draw(
                        draws,
                        self.crown,
                        p,
                        Vec3::new(prop.scale, prop.scale, prop.scale),
                        prop.yaw,
                        prop.tint,
                    );
                    self.stats.triangles += self.crown_triangles;
                }
                RoomIdentityKind::Corpse => {
                    let corpse_id = usize::MAX - room_id - prop_index - 1;
                    if let Some((animation, pose_time)) =
                        self.enemies.corpse_pose("fallen", seed, corpse_id)
                    {
                        if let Some(pose) = self.enemies.draw(
                            "fallen", seed, corpse_id, p, prop.yaw, prop.scale, animation,
                            pose_time, world_time, distance, 0, 0.0, renderer, device, queue,
                            draws,
                        ) {
                            self.stats.triangles += pose.triangles;
                        }
                    }
                }
                RoomIdentityKind::Shield => {
                    if let Some(triangles) = self.assets.draw_oriented(
                        "skeleton_shield",
                        draws,
                        p + Vec3::Y * 0.13,
                        prop.scale,
                        Vec3::new(0.08, prop.yaw, 1.12),
                        distance,
                    ) {
                        self.stats.triangles += triangles;
                    }
                }
                RoomIdentityKind::Blade | RoomIdentityKind::Axe => {
                    let (name, rotation) = if prop.kind == RoomIdentityKind::Blade {
                        ("skeleton_blade", 1.12)
                    } else {
                        ("skeleton_axe", 0.78)
                    };
                    if let Some(triangles) = self.assets.draw_oriented(
                        name,
                        draws,
                        p + Vec3::Y * 0.14,
                        prop.scale,
                        Vec3::new(0.08, prop.yaw, rotation),
                        distance,
                    ) {
                        self.stats.triangles += triangles;
                    }
                }
                RoomIdentityKind::Herb => {
                    if let Some(triangles) =
                        self.assets
                            .draw("healing_herbs", draws, p, prop.scale, prop.yaw)
                    {
                        self.stats.triangles += triangles;
                    }
                }
                RoomIdentityKind::GoldRelic | RoomIdentityKind::BrokenRelic => {
                    let name = if prop.kind == RoomIdentityKind::GoldRelic {
                        "gold_relic"
                    } else {
                        "broken_relic"
                    };
                    if let Some(triangles) = self.assets.draw(name, draws, p, prop.scale, prop.yaw)
                    {
                        self.stats.triangles += triangles;
                    }
                }
                RoomIdentityKind::Banner => {
                    if let Some(triangles) = self.assets.draw_back_mounted(
                        "banner_shield_red",
                        draws,
                        p,
                        prop.scale,
                        prop.yaw,
                    ) {
                        self.stats.triangles += triangles;
                    } else {
                        self.draw(
                            draws,
                            self.cube,
                            p,
                            Vec3::new(0.42, 0.85, 0.09),
                            prop.yaw,
                            [0.58, 0.10, 0.12],
                        );
                    }
                }
            }
        }
    }

    /// What fallen foes left on the floor: each thing bobbing and turning where it lies (or on
    /// its way to her), the rare ones glowing.
    fn draw_pickups(
        &self,
        run: &Dungeon,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
    ) {
        use super::pickups::Loot;
        let t = run.elapsed;
        let mut glows: Vec<(f32, Vec3, [f32; 3])> = Vec::new();
        for p in &run.pickups {
            let bob = if p.landed {
                0.06 * (t * 3.0 + p.phase).sin() + 0.08
            } else {
                0.0
            };
            let at = p.position + Vec3::Y * bob;
            let yaw = t * 2.4 + p.phase;
            let glow = match p.loot {
                Loot::Gold(n) => {
                    let size = if n >= 10 { 0.075 } else { 0.055 };
                    if self.assets.draw("coin", draws, at, size, yaw).is_none() {
                        self.draw(
                            draws,
                            self.crystal,
                            at,
                            Vec3::new(0.08, 0.08, 0.08),
                            yaw,
                            [1.0, 0.78, 0.2],
                        );
                    }
                    [1.0, 0.8, 0.35]
                }
                Loot::Tonic => {
                    self.draw(
                        draws,
                        self.healing_potion,
                        at,
                        Vec3::new(0.2, 0.22, 0.2),
                        yaw,
                        [1.0; 3],
                    );
                    [1.0, 0.3, 0.45]
                }
                Loot::Herbs(_) => {
                    if self
                        .assets
                        .draw("healing_herbs", draws, at, 0.2, yaw)
                        .is_none()
                    {
                        self.draw(
                            draws,
                            self.crystal,
                            at,
                            Vec3::new(0.07, 0.1, 0.07),
                            yaw,
                            [0.3, 0.8, 0.3],
                        );
                    }
                    [0.45, 1.0, 0.45]
                }
                Loot::Material(id, _) => {
                    let (mesh, scale, color, emission, light) = match id {
                        "ceniza_arcana" => (
                            self.crystal,
                            Vec3::new(0.07, 0.13, 0.07),
                            [0.6, 0.3, 1.0],
                            1.8,
                            [0.65, 0.35, 1.0],
                        ),
                        "esquirla_corazon" => (
                            self.crystal,
                            Vec3::new(0.09, 0.17, 0.09),
                            [1.0, 0.12, 0.15],
                            2.4,
                            [1.0, 0.15, 0.2],
                        ),
                        "craneo_campeon" => (
                            self.fx_orb,
                            Vec3::new(0.08, 0.085, 0.095),
                            [0.86, 0.8, 0.66],
                            0.15,
                            [1.0, 0.9, 0.7],
                        ),
                        "gelatina_azul" => (
                            self.fx_orb,
                            Vec3::new(0.1, 0.07, 0.1),
                            [0.3, 0.6, 1.0],
                            0.6,
                            [0.3, 0.6, 1.0],
                        ),
                        "gelatina_verde" => (
                            self.fx_orb,
                            Vec3::new(0.1, 0.07, 0.1),
                            [0.4, 1.0, 0.45],
                            0.6,
                            [0.4, 1.0, 0.45],
                        ),
                        // an old bone: a shaft with its knobs
                        _ => {
                            let along = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
                            for end in [-1.0f32, 1.0] {
                                self.draw(
                                    draws,
                                    self.fx_orb,
                                    at + along * (0.11 * end),
                                    Vec3::new(0.04, 0.04, 0.04),
                                    0.0,
                                    [0.85, 0.79, 0.64],
                                );
                            }
                            (
                                self.cube,
                                Vec3::new(0.035, 0.035, 0.2),
                                [0.82, 0.76, 0.6],
                                0.1,
                                [0.0; 3],
                            )
                        }
                    };
                    self.draw(
                        draws,
                        mesh,
                        at,
                        scale,
                        yaw + std::f32::consts::FRAC_PI_2,
                        color,
                    );
                    if let Some(draw) = draws.last_mut() {
                        draw.material = draw.material.with_emission(emission);
                    }
                    light
                }
            };
            let rarity = p.loot.rarity();
            if rarity > 0.0 {
                glows.push((rarity, at, glow));
            }
        }
        // the brightest few light the floor round them
        glows.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (rarity, at, color) in glows.into_iter().take(5) {
            lights.push(PointLight3D::new(
                at + Vec3::Y * 0.3,
                color,
                1.6 + 2.2 * rarity,
                2.6,
            ));
        }
    }

    /// One of the mine's people, for now a figure of sticks (where its model will stand): it
    /// breathes, turns its head to her when she comes near and waves now and then; a miner's helmet
    /// with its lamp, a pick resting on the floor.
    fn draw_stick_figure(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        npc: &super::authored::Npc,
        run: &Dungeon,
    ) {
        let t = run.elapsed;
        let feet = npc.position;
        let face = if npc.face.length_squared() > 0.5 {
            npc.face
        } else {
            Vec3::Z
        };
        let side = Vec3::new(face.z, 0.0, -face.x);
        let up = Vec3::Y;
        let to_her = run.controller.position - feet;
        let near = Vec3::new(to_her.x, 0.0, to_her.z).length();
        let breath = (t * 1.7).sin();
        let hips = feet + up * (0.94 + 0.008 * breath);
        let neck = hips + up * 0.52 + face * (0.012 * breath);
        // the head turns to her when she is near, as far as a neck turns
        let look = if near < 6.0 {
            let want = to_her.x.atan2(to_her.z) - face.x.atan2(face.z);
            want.sin().atan2(want.cos()).clamp(-1.1, 1.1)
        } else {
            0.0
        };
        let (ls, lc) = look.sin_cos();
        let head_face = face * lc + side * ls;
        let head = neck + up * 0.17;
        let ink = [0.93, 0.87, 0.77];
        let mut stick = |a: Vec3, b: Vec3, width: f32, color: [f32; 3], glow: f32| {
            let d = b - a;
            let length = d.length().max(1.0e-3);
            let dir = d * (1.0 / length);
            draws.push(MeshDraw3D {
                mesh: self.cube,
                transform: Transform3D {
                    position: (a + b) * 0.5,
                    rotation: Vec3::new(
                        -dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt()),
                        dir.x.atan2(dir.z),
                        0.0,
                    ),
                    scale: Vec3::new(width, width, length),
                }
                .matrix(),
                material: Material3D::rgb(color[0], color[1], color[2]).with_emission(glow),
                texture: None,
            });
        };
        stick(hips, neck, 0.06, ink, 0.12);
        stick(neck, head - up * 0.1, 0.05, ink, 0.12);
        let shoulder = neck - up * 0.05;
        stick(
            shoulder - side * 0.19,
            shoulder + side * 0.19,
            0.05,
            ink,
            0.12,
        );
        stick(hips - side * 0.11, hips + side * 0.11, 0.05, ink, 0.12);
        for s in [-1.0f32, 1.0] {
            let hip = hips + side * (s * 0.1);
            let knee = hip - up * 0.46 + face * 0.02;
            stick(hip, knee, 0.055, ink, 0.12);
            stick(knee, feet + side * (s * 0.12), 0.05, ink, 0.12);
        }
        // the right hand on the pick's haft, its head on the floor
        let right = shoulder - side * 0.19;
        let elbow = right - up * 0.29 - side * 0.04;
        let hand = elbow - up * 0.26 + face * 0.08;
        stick(right, elbow, 0.05, ink, 0.12);
        stick(elbow, hand, 0.045, ink, 0.12);
        let pick_foot = feet - side * 0.36 + face * 0.3 + up * 0.04;
        stick(hand + up * 0.12, pick_foot, 0.04, [0.45, 0.28, 0.14], 0.0);
        stick(
            pick_foot - face * 0.26,
            pick_foot + face * 0.26 + up * 0.05,
            0.045,
            [0.42, 0.42, 0.45],
            0.0,
        );
        // the left arm: hanging, or waving at her now and then when she is near
        let left = shoulder + side * 0.19;
        let waving = near < 4.5 && (t % 6.0) < 1.7;
        let (elbow, hand) = if waving {
            let elbow = left + side * 0.2 + up * 0.18;
            (elbow, elbow + up * 0.26 + side * (0.08 * (t * 12.0).sin()))
        } else {
            let elbow = left - up * 0.29 + side * 0.04;
            (elbow, elbow - up * 0.26 + face * 0.04)
        };
        stick(left, elbow, 0.05, ink, 0.12);
        stick(elbow, hand, 0.045, ink, 0.12);
        // the head, its helmet, the helmet's lamp
        self.draw(
            draws,
            self.fx_orb,
            head,
            Vec3::new(0.12, 0.13, 0.12),
            0.0,
            ink,
        );
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(0.12);
        }
        self.draw(
            draws,
            self.fx_orb,
            head + up * 0.05,
            Vec3::new(0.145, 0.085, 0.145),
            0.0,
            [0.85, 0.6, 0.22],
        );
        self.draw(
            draws,
            self.fx_orb,
            head + up * 0.035,
            Vec3::new(0.2, 0.014, 0.2),
            0.0,
            [0.8, 0.56, 0.2],
        );
        let lamp = head + up * 0.08 + head_face * 0.13;
        self.draw(
            draws,
            self.fx_orb,
            lamp,
            Vec3::new(0.035, 0.035, 0.035),
            0.0,
            [1.0, 0.85, 0.55],
        );
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(5.0);
        }
        lights.push(PointLight3D::new(
            lamp + head_face * 0.3,
            [1.0, 0.78, 0.45],
            2.2,
            4.0,
        ));
    }

    /// A gate of the hand-made mine while it is shut: a wall of rune light across the way.
    fn draw_mine_gate(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        gate: &super::authored::Gate,
        elapsed: f32,
    ) {
        let pulse = 0.75 + 0.25 * (elapsed * 2.3).sin();
        let across_x = gate.half[0] >= gate.half[1];
        let width = gate.half[0].max(gate.half[1]) * 2.0;
        let yaw = if across_x {
            0.0
        } else {
            std::f32::consts::FRAC_PI_2
        };
        // the veil, and its bars of runes
        self.draw(
            draws,
            self.cube,
            gate.center + Vec3::Y * (gate.height * 0.5),
            Vec3::new(width, gate.height, 0.04),
            yaw,
            [0.75, 0.12, 0.16],
        );
        if let Some(draw) = draws.last_mut() {
            draw.material = draw.material.with_emission(1.6 * pulse);
            draw.material.base_color[3] = 0.55;
        }
        for k in 0..5 {
            let t = (k as f32 + 0.5) / 5.0 - 0.5;
            let along = if across_x { Vec3::X } else { Vec3::Z };
            self.draw(
                draws,
                self.crystal,
                gate.center
                    + along * (t * width)
                    + Vec3::Y * (1.1 + 0.25 * (elapsed * 1.7 + k as f32).sin()),
                Vec3::new(0.09, 0.26, 0.09),
                elapsed * 0.8 + k as f32,
                [1.0, 0.35, 0.3],
            );
            if let Some(draw) = draws.last_mut() {
                draw.material = draw.material.with_emission(4.0 * pulse);
            }
        }
        lights.push(PointLight3D::new(
            gate.center + Vec3::Y * 1.4,
            [1.0, 0.22, 0.2],
            9.0 * pulse,
            7.5,
        ));
    }
    fn draw(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        mesh: MeshHandle,
        position: Vec3,
        scale: Vec3,
        yaw: f32,
        color: [f32; 3],
    ) {
        draws.push(MeshDraw3D {
            mesh,
            transform: Transform3D {
                position,
                rotation: Vec3::new(0.0, yaw, 0.0),
                scale,
            }
            .matrix(),
            material: Material3D::rgb(color[0], color[1], color[2]),
            texture: None,
        });
    }

    fn draw_rotating(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        mesh: MeshHandle,
        position: Vec3,
        scale: Vec3,
        yaw: f32,
        angle: f32,
        color: [f32; 3],
    ) {
        let world = Transform3D {
            position,
            rotation: Vec3::new(0.0, yaw, 0.0),
            scale,
        }
        .matrix();
        draws.push(MeshDraw3D {
            mesh,
            transform: world * Mat4::from_rotation_z(angle),
            material: Material3D::rgb(color[0], color[1], color[2]),
            texture: None,
        });
    }
}

#[cfg(test)]
mod chest_orientation_tests {
    use super::*;

    #[test]
    fn generated_chests_face_their_arrival_portal() {
        let catalog = Catalog::bundled();
        let difficulty = gameplay::Difficulty::default();
        for seed in 0..32 {
            let map = generate(seed, &Config::default(), &catalog, &difficulty)
                .expect("stress seed generates a valid map");
            let entries = room_entry_portals(&map);
            for chest in map
                .spawns
                .iter()
                .filter(|spawn| spawn.kind == gameplay::SpawnKind::Chest)
            {
                let entry = entries[chest.room].expect("chest room is reachable from entrance");
                let yaw = chest_facing_yaw(chest.position, entry);
                let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                let toward_entry = horizontal(entry.position - chest.position);
                let toward_entry = if toward_entry.length_squared() > 0.001 {
                    toward_entry.normalize_or_zero()
                } else {
                    horizontal(entry.outward).normalize_or_zero()
                };
                assert!(
                    forward.dot(toward_entry) > 0.999,
                    "chest in room {} faces away from its entry for seed {seed}",
                    chest.room
                );
            }
        }
    }

    #[test]
    fn chest_contents_rotate_with_the_chest_front() {
        let origin = Vec3::new(2.0, 3.0, 4.0);
        let front = chest_point(
            origin,
            Vec3::new(0.0, 0.0, 1.0),
            std::f32::consts::FRAC_PI_2,
        );
        assert!((front.x - 3.0).abs() < 0.001);
        assert!((front.y - 3.0).abs() < 0.001);
        assert!((front.z - 4.0).abs() < 0.001);
    }
}

#[cfg(test)]
mod room_identity_tests {
    use super::*;

    #[test]
    fn treasure_chambers_get_distinct_readable_identity_without_blocking_entry() {
        let catalog = Catalog::bundled();
        let difficulty = gameplay::Difficulty::default();
        let mut treasure_rooms = 0;
        let mut saw_rest = false;
        let mut saw_event = false;
        let mut saw_combat = false;

        for seed in 0..48 {
            let map = generate(seed, &Config::default(), &catalog, &difficulty)
                .expect("stress seed generates a valid map");
            let entries = room_entry_portals(&map);
            for room in &map.rooms {
                let focus = map
                    .spawns
                    .iter()
                    .find(|spawn| {
                        spawn.room == room.id
                            && matches!(
                                spawn.kind,
                                gameplay::SpawnKind::Chest
                                    | gameplay::SpawnKind::Rest
                                    | gameplay::SpawnKind::Memory
                            )
                    })
                    .map(|spawn| spawn.position);
                let entry = entries.get(room.id).copied().flatten();
                let props = room_identity_plan(room, seed, entry, focus);
                saw_rest |= room.role == Role::Rest;
                saw_event |= room.role == Role::Event;
                saw_combat |= room.role == Role::Encounter;
                // Narrow bridge decks retain only their authored objective;
                // floor dressing would consume the player's only clear lane.
                if room.shape == Shape::Bridge {
                    if room.role == Role::Event {
                        assert!(map.spawns.iter().any(|spawn| {
                            spawn.room == room.id && spawn.kind == gameplay::SpawnKind::Memory
                        }));
                    }
                    continue;
                }

                match room.role {
                    Role::Treasure => {
                        treasure_rooms += 1;
                        assert!(props.is_empty(), "treasuries are dressed by vaults");
                    }
                    Role::Rest => {
                        assert!(props.is_empty(), "sanctuaries are dressed by vaults");
                    }
                    Role::Event => {
                        let gems = props
                            .iter()
                            .filter(|prop| prop.kind == RoomIdentityKind::Gem)
                            .count();
                        assert!(
                            gems >= 3,
                            "seed {seed}, event room {} got {gems} gems: {props:?}",
                            room.id
                        );
                    }
                    Role::Encounter => {
                        assert!(props.iter().any(|prop| {
                            matches!(prop.kind, RoomIdentityKind::Blade | RoomIdentityKind::Axe)
                        }));
                    }
                    _ => {}
                }

                for prop in props
                    .iter()
                    .filter(|prop| prop.kind != RoomIdentityKind::Banner)
                {
                    assert!(room.contains(prop.position.x, prop.position.z, 0.01));
                    assert!(
                        (room.floor_y(prop.position.x, prop.position.z) - prop.position.y).abs()
                            < 0.001
                    );
                    assert!(room.sockets.iter().all(|socket| {
                        horizontal(socket.position - prop.position).length() >= 1.79
                    }));
                    if let (Some(entry), Some(focus)) = (entry, focus) {
                        assert!(segment_distance(prop.position, entry.position, focus) >= 0.94);
                    }
                }
            }
        }

        assert!(
            treasure_rooms > 0,
            "stress seeds must include treasure rooms"
        );
        assert!(saw_rest, "stress seeds must include a rest room");
        assert!(saw_event, "stress seeds must include an event room");
        assert!(saw_combat, "stress seeds must include an encounter room");
    }
}

impl Scene {
    /// The Heart's blood rain: streaks falling all around the player and
    /// rings where drops splash on the floor.
    fn draw_blood_rain(
        &mut self,
        run: &Dungeon,
        camera: Camera3D,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
    ) {
        let rain = run.heart.rain;
        let floor = run.controller.position.y;
        let t = run.elapsed;
        let unit = super::combat::unit;
        let mut drop = Material3D::rgb(0.55, 0.02, 0.03).with_emission(0.5);
        drop.base_color[3] = 0.75 * rain;
        let count = (240.0 * rain) as u32;
        for i in 0..count {
            let x = camera.position.x + unit(0x8a1, i * 4) * 18.0 - 9.0;
            let z = camera.position.z + unit(0x8a1, i * 4 + 1) * 18.0 - 9.0;
            let span = 7.5;
            let fall = (t * 10.0 + unit(0x8a1, i * 4 + 2) * span) % span;
            let y = floor + span - fall;
            let head = Vec3::new(x, y, z);
            let tail = head + Vec3::new(0.05, 0.42, 0.02);
            let d = tail - head;
            let length = d.length();
            let dir = d * (1.0 / length);
            draws.push(MeshDraw3D {
                mesh: self.cube,
                transform: Transform3D {
                    position: (head + tail) * 0.5,
                    rotation: Vec3::new(
                        -dir.y.atan2((dir.x * dir.x + dir.z * dir.z).sqrt()),
                        dir.x.atan2(dir.z),
                        0.0,
                    ),
                    scale: Vec3::new(0.03, 0.03, length),
                }
                .matrix(),
                material: drop,
                texture: None,
            });
        }
        // Splashes: rings that open and fade where drops land.
        for i in 0..(40.0 * rain) as u32 {
            let phase = (t * 1.7 + unit(0x5b1, i * 3) * 1.0) % 1.0;
            let x = camera.position.x + unit(0x5b1, i * 3 + 1) * 12.0 - 6.0;
            let z = camera.position.z + unit(0x5b1, i * 3 + 2) * 12.0 - 6.0;
            let mut splash = Material3D::rgb(0.5, 0.03, 0.04).with_emission(0.3);
            splash.base_color[3] = (1.0 - phase) * 0.8 * rain;
            let r = 0.05 + phase * 0.22;
            draws.push(MeshDraw3D {
                mesh: self.fx_ring,
                transform: Mat4::from_translation(Vec3::new(x, floor + 0.03, z))
                    * Mat4::from_scale(Vec3::new(r, r, r)),
                material: splash,
                texture: None,
            });
        }
        self.stats.triangles += count as usize * 12 + 40 * 96;
        lights.push(PointLight3D::new(
            run.controller.position + Vec3::Y * 3.0,
            [0.9, 0.08, 0.06],
            1.4 * rain,
            9.0,
        ));
    }
}

impl Scene {
    /// One slime: its wobbling body, face, droplets and puddles, in its tint.
    fn draw_slime(
        &mut self,
        slime: &super::slime::Slime,
        body: MeshHandle,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
    ) {
        let (drop, puddle) = (slime.tint.drop(), slime.tint.puddle());
        for particle in &slime.particles {
            let fade = (1.0 - particle.age / particle.life).clamp(0.0, 1.0);
            if particle.puddle {
                draws.push(MeshDraw3D {
                    mesh: self.slime_puddle,
                    transform: Transform3D {
                        position: particle.position,
                        rotation: Vec3::ZERO,
                        scale: Vec3::new(particle.radius, 0.008, particle.radius),
                    }
                    .matrix(),
                    material: Material3D {
                        base_color: [puddle[0], puddle[1], puddle[2], 0.36 * fade],
                        texture: None,
                        occlusion_window: [0.0; 4],
                    },
                    texture: None,
                });
            } else {
                let radius = particle.radius;
                self.draw(
                    draws,
                    self.slime_eye,
                    particle.position,
                    Vec3::new(radius, radius, radius),
                    0.0,
                    [drop[0] * fade, drop[1] * fade, drop[2] * fade],
                );
            }
        }
        if slime.alive() || !slime.burst {
            let deformed = super::slime::deform(&self.slime_base, slime);
            let _ = renderer.update_mesh_vertices(queue, body, &deformed);
            let pose = Transform3D {
                position: slime.position,
                rotation: Vec3::new(0.0, slime.yaw, 0.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
            }
            .matrix();
            let glow = slime.glow;
            draws.push(MeshDraw3D {
                mesh: body,
                transform: pose,
                material: Material3D {
                    base_color: slime.tint.body(glow),
                    texture: None,
                    occlusion_window: [0.0; 4],
                },
                texture: None,
            });
            let inverse = slime.squash.sqrt().recip();
            for side in [-1.0, 1.0] {
                let eye = Vec3::new(
                    side * 0.105 * inverse,
                    0.33 * slime.squash + slime.lift,
                    0.36 * inverse,
                );
                let position =
                    slime.position + Mat4::from_rotation_y(slime.yaw).transform_vector3(eye);
                self.draw(
                    draws,
                    self.slime_eye,
                    position,
                    Vec3::new(0.058 * inverse, 0.074 * slime.squash, 0.042 * inverse),
                    slime.yaw,
                    [0.025, 0.04, 0.09],
                );
            }
            let mouth = slime.position + Vec3::Y * slime.lift;
            draws.push(MeshDraw3D {
                mesh: self.slime_mouth,
                transform: Transform3D {
                    position: mouth,
                    rotation: Vec3::new(0.0, slime.yaw, 0.0),
                    scale: Vec3::new(inverse, slime.squash, inverse),
                }
                .matrix(),
                material: Material3D::rgb(0.025, 0.04, 0.09),
                texture: None,
            });
            self.draw(
                draws,
                self.slime_eye,
                slime.position + Vec3::Y * (0.2 * slime.squash + slime.lift),
                Vec3::new(0.10 * inverse, 0.10 * slime.squash, 0.10 * inverse),
                0.0,
                slime.tint.core(glow),
            );
            lights.push(PointLight3D::new(
                slime.position + Vec3::Y * 0.4,
                slime.tint.light(),
                0.42 + glow * 1.15,
                2.5,
            ));
            self.stats.triangles += self.slime_base.len() * 2;
        }
    }
}

/// The guard-stance loop baked for a weapon (`Guard_<weapon>`).
/// `VELVET_FOE_POSE=clip@seconds`: every foe drawn frozen in that clip (QA captures).
fn qa_pose() -> &'static Option<(String, f32)> {
    static POSE: std::sync::OnceLock<Option<(String, f32)>> = std::sync::OnceLock::new();
    POSE.get_or_init(|| {
        let v = std::env::var("VELVET_FOE_POSE").ok()?;
        let (clip, t) = v.split_once('@').unwrap_or((v.as_str(), "0"));
        Some((clip.to_string(), t.parse().unwrap_or(0.0)))
    })
}

pub(super) fn guard_clip(weapon: &str) -> &'static str {
    match weapon {
        "axe" => "Guard_axe",
        "crossbow" => "Guard_crossbow",
        "staff" => "Guard_staff",
        _ => "Guard_blade",
    }
}
