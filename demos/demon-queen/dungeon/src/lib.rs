//! Demon Queen's dungeon: the run (rooms, actors, loot, saves), its first-person scene, combat, HUD
//! and art. The generator is the engine's (velvet-dungeon); this crate is the game around it.
//! Built once for the game's three binaries (demon-queen, demon-queen-3d, dungeon-lab).
#[path = "ambient.rs"]
mod ambient;
#[path = "anim/mod.rs"]
#[allow(dead_code)]
mod anim;
#[path = "architecture.rs"]
mod architecture;
#[path = "art/mod.rs"]
#[allow(dead_code)]
mod art;
#[path = "assets.rs"]
pub mod assets;
#[path = "authored.rs"]
pub mod authored;
#[path = "combat.rs"]
pub mod combat;
#[path = "combat_fx.rs"]
pub mod combat_fx;
#[path = "decor.rs"]
mod decor;
#[path = "dressing.rs"]
mod dressing;
#[path = "enemies.rs"]
mod enemies;
#[path = "finish.rs"]
mod finish;
#[path = "fp.rs"]
pub mod fp;
#[path = "hud.rs"]
pub mod hud;
/// Text in the player's language (shared with the game).
pub mod i18n;
#[path = "incidents.rs"]
mod incidents;
#[path = "mine_art.rs"]
mod mine_art;
#[path = "pickups.rs"]
pub mod pickups;
/// The shaft by the mine's mouth, dug block by block.
pub mod pozo;
#[path = "prompt_icons.rs"]
pub mod prompt_icons;
#[path = "replay.rs"]
#[allow(dead_code)]
pub mod replay;
#[path = "runes.rs"]
mod runes;
#[path = "sanctuary.rs"]
mod sanctuary;
#[path = "save.rs"]
mod save;
#[path = "scene.rs"]
pub mod scene;
#[path = "slime.rs"]
pub mod slime;
#[path = "sound.rs"]
pub mod sound;
/// The Tendero's stall at the mine's mouth.
pub mod stall;
#[path = "streaming.rs"]
mod streaming;
#[path = "surfaces.rs"]
mod surfaces;
#[path = "themes.rs"]
pub mod themes;
#[path = "treasury.rs"]
mod treasury;
#[path = "vaults.rs"]
mod vaults;
#[path = "visual.rs"]
pub mod visual;
#[path = "water.rs"]
pub mod water;
#[allow(unused_imports)]
pub use save::Snapshot;
pub use scene::Scene;
use velvet_dungeon::{
    controller::{Controller, Input},
    gameplay::{Difficulty, SpawnKind},
    *,
};
use velvet_math::Vec3;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Actor {
    pub spawn: usize,
    pub position: Vec3,
    pub health: f32,
    /// Number of successful weapon impacts, used for persistent visual damage.
    #[serde(default)]
    pub damage_hits: u8,
    /// Brief stagger remaining after the latest player hit.
    #[serde(default)]
    pub hit_reaction: f32,
    pub cooldown: f32,
    /// The blow in progress, if any (web `actor.attack`).
    #[serde(default)]
    pub attack: Option<combat::Swing>,
    /// Which of the loadout's attacks comes next; they cycle.
    #[serde(default)]
    pub attack_index: usize,
    /// Facing, turned at a limited rate (local +Z toward the target).
    #[serde(default)]
    pub yaw: f32,
    /// Staggered by a hit: no moves or blows until it runs out.
    #[serde(default)]
    pub stagger: f32,
    /// Seconds since this foe fell (its death animation, then it sinks).
    /// Older saves have none: their dead are long gone.
    #[serde(default = "long_dead")]
    pub death: f32,
    /// A champion past half health: it has roared and fights faster.
    #[serde(default)]
    pub enraged: bool,
    /// Seconds a ranged foe has spent backing away from a pursuer.
    #[serde(default)]
    pub flee: f32,
    /// Seconds left holding its ground: a caster or scout that has fled
    /// long enough stops running and fights where it stands.
    #[serde(default)]
    pub stand: f32,
    /// Player-space location committed to by the current attack.
    #[serde(default)]
    pub attack_target: Vec3,
    /// Allows one coordinated side shot before the scout tries the player's back.
    #[serde(default)]
    pub ambush_opening: bool,
    /// Time spent seeking an ideal lane before taking a safe fallback shot.
    #[serde(default)]
    pub reposition_elapsed: f32,
    pub used: bool,
    /// Persistent open fraction for treasure chest lids.
    #[serde(default)]
    pub chest_open_progress: f32,
    /// True only while the authored lid mesh is moving toward its open pose.
    #[serde(default)]
    pub chest_opening: bool,
    #[serde(skip)]
    pub path: Vec<Vec3>,
    #[serde(skip)]
    pub nav_timer: f32,
    /// How it moves this frame, for its animation.
    #[serde(skip)]
    pub gait: combat::Gait,
    /// Seconds spent trying to walk without getting anywhere.
    #[serde(skip)]
    pub stuck: f32,
    /// Seconds left stuck in the sticky sword's goo: it moves and strikes slower.
    #[serde(default)]
    pub slowed: f32,
    /// Seconds left smouldering from the ember sword.
    #[serde(default)]
    pub burning: f32,
}

fn long_dead() -> f32 {
    60.0
}

const CHEST_OPEN_SECONDS: f32 = 0.92;
const SWORD_ATTACK_COOLDOWN: f32 = 0.05;
const SWORD_REACH: f32 = 1.85;
/// A plain sword blow takes this share of any foe's full health: every
/// enemy needs about ten cuts, so at this stage fleeing is often wiser.
const SWORD_DAMAGE_SHARE: f32 = 0.1;
/// The combo finisher hits harder than the two cuts before it.
const SWORD_FINISHER: f32 = 1.45;
/// The relic sword from the treasuries cuts a little deeper.
pub const RELIC_SWORD_BONUS: f32 = 1.3;
/// How fast a foe in the sticky sword's goo goes (moves, blows).
pub const SLOWED_PACE: f32 = 0.5;
/// Share of a foe's health the ember sword's fire takes each second (it never finishes one off).
pub const BURN_SHARE: f32 = 0.035;

/// What the worn sword adds to each blow: set by the game from the backpack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponFx {
    /// Seconds a struck foe stays slowed.
    pub slow: f32,
    /// Seconds a struck foe smoulders.
    pub burn: f32,
    /// Vitality she takes back with each blow that lands.
    pub leech: f32,
}

impl Default for WeaponFx {
    fn default() -> Self {
        Self {
            slow: 0.0,
            burn: 0.0,
            leech: 0.0,
        }
    }
}

/// Chance that a treasury chest holds the relic sword (until it is found).
const RELIC_SWORD_CHANCE: f32 = 0.3;

impl Actor {
    /// Visual damage tier (0..=3) from the share of health already lost.
    pub fn fracture_stage(&self, maximum_health: f32) -> u8 {
        if !(maximum_health.is_finite() && maximum_health > 0.0) {
            return self.damage_hits.min(3);
        }
        let lost = ((maximum_health - self.health).max(0.0) / maximum_health).min(1.0);
        ((lost * 3.0 - 1e-4).ceil().max(0.0) as u8).min(3)
    }
}

/// Whether this chest holds the relic sword (only ever in a treasury).
pub fn chest_holds_relic_sword(seed: u64, spawn: usize) -> bool {
    rng::Rng::stream(seed, "chest_relic_sword_v1", spawn as u64).chance(RELIC_SWORD_CHANCE)
}

pub fn chest_herb_count(seed: u64, spawn: usize) -> u32 {
    let mut rng = rng::Rng::stream(seed, "chest_herbs_v1", spawn as u64);
    1 + rng.index(2) as u32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyStyle {
    Quick,
    Heavy,
    Mage,
    Ranged,
}

/// How a foe moves and fights. The boss swings its staff (melee), so only
/// the grave mage casts.
pub fn enemy_style(definition: &str, kind: SpawnKind) -> EnemyStyle {
    if kind == SpawnKind::Boss {
        EnemyStyle::Heavy
    } else if definition == "grave_mage" {
        EnemyStyle::Mage
    } else if definition == "bone_scout" {
        EnemyStyle::Ranged
    } else if matches!(definition, "bone_guard" | "sentinel") || matches!(kind, SpawnKind::Miniboss)
    {
        EnemyStyle::Heavy
    } else {
        EnemyStyle::Quick
    }
}

/// How far round a foe looks for a walk to the Queen over a hand-made level's floors.
const FOE_PATH_REACH: f32 = 18.0;
/// How far from its post a foe of a hand-made level follows the Queen.
const FOE_LEASH: f32 = 26.0;

/// A foe's step to `to` (its feet at `to.y`): onto the floor there when that floor lies within a
/// step of its feet, up or down (slopes, stairs, the rock's bumps), never off a ledge. Its feet
/// follow the floor under its middle.
fn foe_step(map: &Blueprint, to: Vec3) -> Option<Vec3> {
    let reach = map.step_height().max(0.32);
    map.capsule_floor(to, 0.4)
        .filter(|y| (y - to.y).abs() <= reach)?;
    let y = map
        .surface(to.x, to.z, to.y, PLAYER_HEIGHT)
        .filter(|y| (y - to.y).abs() <= reach)
        .unwrap_or(to.y);
    Some(Vec3::new(to.x, y, to.z))
}

/// A champion speaking: its words under the scene for a few seconds, its name in its colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Speech {
    pub who: String,
    pub line: &'static str,
    pub color: [f32; 3],
    pub at: f32,
}

/// How long a champion's words stay up.
pub const SPEECH_SECONDS: f32 = 4.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnemyIntent {
    Approach,
    Hold,
    Attack,
    /// Back away (a caster the player got too close to).
    Retreat,
}

const ARCHER_MIN_RANGE: f32 = 4.8;
/// A scout or caster backs away this long before it stands and fights.
const RANGED_FLEE_LIMIT: f32 = 1.8;
/// How long a cornered scout or caster holds its ground.
const RANGED_STAND_SECONDS: f32 = 4.0;
/// A scout walks in until the player is this close, then shoots.
const ARCHER_ENGAGE_RANGE: f32 = 10.4;
const ARCHER_REAR_DOT: f32 = -0.42;
const STALKER_BOSS_GUARD_RADIUS: f32 = 2.35;
const STALKER_SURROUND_RADIUS: f32 = 1.38;
const STALKER_BOSS_ENGAGE_RANGE: f32 = 6.2;

fn encounter_ring_position(
    map: &Blueprint,
    room: usize,
    anchor: Vec3,
    rank: usize,
    count: usize,
    radius: f32,
) -> Option<Vec3> {
    let count = count.max(1);
    let angle = std::f32::consts::TAU * (rank as f32 / count as f32) + 0.31;
    for scale in [1.0, 0.86, 1.14, 0.7] {
        let mut point = anchor + Vec3::new(angle.cos(), 0.0, angle.sin()) * (radius * scale);
        if !map.rooms.get(room)?.contains(point.x, point.z, 0.45) {
            continue;
        }
        let Some(floor) = map.capsule_floor(point, 0.4) else {
            continue;
        };
        if (floor - point.y).abs() > 1.0 {
            continue;
        }
        point.y = floor;
        return Some(point);
    }
    None
}

fn archer_is_behind(player: Vec3, player_forward: Vec3, archer: Vec3) -> bool {
    let forward = horizontal(player_forward).normalize_or_zero();
    let to_archer = horizontal(archer - player).normalize_or_zero();
    forward.length_squared() > 0.001
        && to_archer.length_squared() > 0.001
        && forward.dot(to_archer) <= ARCHER_REAR_DOT
}

fn archer_is_on_flank(player: Vec3, player_forward: Vec3, archer: Vec3) -> bool {
    let forward = horizontal(player_forward).normalize_or_zero();
    let to_archer = horizontal(archer - player).normalize_or_zero();
    forward.length_squared() > 0.001
        && to_archer.length_squared() > 0.001
        && forward.dot(to_archer) <= 0.35
}

fn archer_rear_position(
    map: &Blueprint,
    room: usize,
    player: Vec3,
    player_forward: Vec3,
    side: f32,
) -> Option<Vec3> {
    let mut forward = horizontal(player_forward).normalize_or_zero();
    if forward.length_squared() < 0.001 {
        forward = Vec3::Z;
    }
    let right = Vec3::new(forward.z, 0.0, -forward.x);
    let mut fallback = None;
    for distance in [6.4, 5.8, 7.1, 5.0, 4.3] {
        for lateral in [2.1, 1.35, 2.75, 0.7] {
            let mut point = player - forward * distance + right * (side * lateral);
            let Some(room_bounds) = map.rooms.get(room) else {
                return None;
            };
            if !room_bounds.contains(point.x, point.z, 0.45) {
                continue;
            }
            let Some(floor) = map.capsule_floor(point, 0.38) else {
                continue;
            };
            if (floor - player.y).abs() > 1.0 {
                continue;
            }
            point.y = floor;
            if fallback.is_none() {
                fallback = Some(point);
            }
            if map.line_of_sight(point + Vec3::Y, player + Vec3::Y) {
                return Some(point);
            }
        }
    }
    fallback
}

/// A scout never substitutes the player's position for a failed firing lane.
/// First try the intended rear/flank lane, then a room-safe kiting point around
/// the scout's current side. `None` means hold position instead of charging.
fn archer_standoff_position(
    map: &Blueprint,
    room: usize,
    player: Vec3,
    player_forward: Vec3,
    side: f32,
    current: Vec3,
) -> Option<Vec3> {
    if let Some(point) = archer_rear_position(map, room, player, player_forward, side) {
        return Some(point);
    }
    let room_bounds = map.rooms.get(room)?;
    let mut away = horizontal(current - player).normalize_or_zero();
    if away.length_squared() < 0.001 {
        let forward = horizontal(player_forward).normalize_or_zero();
        away = Vec3::new(forward.z, 0.0, -forward.x) * side.signum();
    }
    let lateral = Vec3::new(-away.z, 0.0, away.x) * side.signum();
    let mut fallback = None;
    for radius in [6.0, 5.2, 7.0, 4.9] {
        for blend in [0.0, 0.38, -0.38, 0.68, -0.68] {
            let direction = (away + lateral * blend).normalize_or_zero();
            let mut point = player + direction * radius;
            if !room_bounds.contains(point.x, point.z, 0.55) {
                continue;
            }
            let Some(floor) = map.capsule_floor(point, 0.4) else {
                continue;
            };
            if (floor - player.y).abs() > 1.0 {
                continue;
            }
            point.y = floor;
            if horizontal(point - player).length() < ARCHER_MIN_RANGE {
                continue;
            }
            fallback.get_or_insert(point);
            if map.line_of_sight(point + Vec3::Y, player + Vec3::Y) {
                return Some(point);
            }
        }
    }
    fallback.or_else(|| {
        (room_bounds.contains(current.x, current.z, 0.4)
            && map.capsule_floor(current, 0.4).is_some())
        .then_some(current)
    })
}

fn archer_intent(
    distance: f32,
    visible: bool,
    attack_angle_ready: bool,
    cooldown: f32,
) -> EnemyIntent {
    // Reload is a commitment. A scout that kites throughout its two-second
    // recovery can keep a melee-only player forever outside sword reach.
    // Holding the lane lets a rushing player punish the shot, while its long
    // windup still telegraphs the next bolt.
    if cooldown > 0.0 && visible && attack_angle_ready {
        EnemyIntent::Hold
    } else if !visible
        || !attack_angle_ready
        || distance > ARCHER_ENGAGE_RANGE
        || distance < ARCHER_MIN_RANGE
    {
        EnemyIntent::Approach
    } else {
        EnemyIntent::Attack
    }
}

fn archer_intent_under_pressure(
    distance: f32,
    visible: bool,
    attack_angle_ready: bool,
    cooldown: f32,
    reposition_elapsed: f32,
) -> EnemyIntent {
    // A scout that has spent two seconds failing to open a lane must commit
    // to a defensive shot when cornered. Its 0.86 s windup is a fair melee
    // punish window; without this it can evade forever beside a room wall.
    if reposition_elapsed >= 2.2
        && distance < ARCHER_MIN_RANGE
        && visible
        && attack_angle_ready
        && cooldown <= 0.0
    {
        EnemyIntent::Attack
    } else {
        archer_intent(distance, visible, attack_angle_ready, cooldown)
    }
}

/// The Heart's powers while its guardian lives: it raises minions and, now
/// and then, rains blood that wears the player's vitality down.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeartState {
    /// Seconds until the next summoning.
    pub summon_in: f32,
    /// Seconds until the next blood rain.
    pub rain_in: f32,
    /// Seconds of rain still falling.
    pub rain_left: f32,
    /// Seconds to the next heartbeat.
    pub beat_in: f32,
    /// How hard it is raining (0..1), eased for the scene and HUD.
    pub rain: f32,
    /// Whether the player is in the Heart's chamber with its guardian alive.
    pub awake: bool,
    /// Seconds to the Heart's next spit of blood at her.
    pub spit_in: f32,
    /// A ring of blood racing out from the Heart over the floor: from where, and how long ago.
    pub wave: Option<(Vec3, f32)>,
    /// The ring already caught her.
    pub wave_hit: bool,
    /// Beats since it woke, for the rings.
    pub beats: u32,
    /// How many of the guardian's turning points (two thirds and a third of its life) have passed.
    pub thresholds: u8,
}

/// How fast the Heart's ring races over the floor, and how far it goes.
pub const HEART_WAVE_SPEED: f32 = 7.5;
const HEART_WAVE_REACH: f32 = 17.0;

/// Materials foes leave behind instead of gold: (inventory id, name). They
/// are carried back to the villa and sold at the shop for very little.
pub const MATERIALS: [(&str, &str); 6] = [
    ("hueso_viejo", "Hueso viejo"),
    ("ceniza_arcana", "Ceniza arcana"),
    ("gelatina_azul", "Gelatina azul"),
    ("gelatina_verde", "Gelatina verde"),
    ("craneo_campeon", "Cráneo del campeón"),
    ("esquirla_corazon", "Esquirla del Corazón"),
];

/// What a fallen skeleton leaves: bones for all of them, arcane ash from
/// casters, and a trophy from each champion.
pub fn skeleton_drops(
    seed: u64,
    spawn: &velvet_dungeon::gameplay::Spawn,
) -> Vec<(&'static str, u32)> {
    let mut rng = rng::Rng::stream(seed, "skeleton_drops_v1", spawn.id as u64);
    let warrior = matches!(spawn.definition.as_str(), "bone_guard" | "sentinel");
    match spawn.kind {
        SpawnKind::Boss => vec![
            ("hueso_viejo", 6),
            ("ceniza_arcana", 3),
            ("esquirla_corazon", 1),
        ],
        SpawnKind::Miniboss => vec![("hueso_viejo", 4), ("craneo_campeon", 1)],
        _ => {
            let bones = if warrior || spawn.kind == SpawnKind::Elite {
                2
            } else {
                1
            } + rng.index(2) as u32;
            let mut out = vec![("hueso_viejo", bones)];
            if spawn.definition == "grave_mage" {
                out.push(("ceniza_arcana", 1 + rng.index(2) as u32));
            }
            out
        }
    }
}

/// Vitality blood rain takes per second.
const BLOOD_RAIN_DRAIN: f32 = 1.6;
/// At most this many summoned minions stand at once.
const MAX_SUMMONS: usize = 7;

pub struct Dungeon {
    pub map: Blueprint,
    /// The hand-made mine's art, names, chests and gates, when this run is it (see authored.rs).
    pub authored: Option<std::sync::Arc<authored::Authored>>,
    /// Solid art per room (props, beacons, banners); derived from the map,
    /// shared by rendering and player collision, never saved.
    pub decor: Vec<decor::RoomDecor>,
    pub catalog: Catalog,
    pub controller: Controller,
    pub actors: Vec<Actor>,
    /// Bolts in flight and fading combat effects (not saved).
    pub fx: Vec<combat::Fx>,
    /// Actors past this index were summoned by the Heart (never saved).
    pub base_actors: usize,
    /// The Heart's summoning and blood rain.
    pub heart: HeartState,
    /// Camera shake from big blows, decaying.
    pub shake: f32,
    /// Sounds made since the game last took them.
    pub sounds: Vec<sound::Sound>,
    /// Distance walked since the last footstep, and whether on the ground.
    step_distance: f32,
    was_grounded: bool,
    /// One authored first encounter, regenerated from the seed and saved as
    /// combat state without changing the procedural spawn table.
    pub slime: Option<slime::Slime>,
    /// Blue and green slimes by the trees and mushrooms of other halls.
    pub groves: Vec<slime::Slime>,
    pub health: f32,
    pub potions: u32,
    pub herbs: u32,
    /// Treasure items not yet transferred into the persistent villa inventory.
    pub unbanked_potions: u32,
    pub unbanked_herbs: u32,
    /// Materials gathered on this expedition, carried until the villa.
    pub materials: std::collections::BTreeMap<String, u32>,
    /// The relic sword is in hand (found now or brought from the villa).
    pub relic_sword: bool,
    /// Found on this expedition, not yet carried back to the villa.
    pub unbanked_relic_sword: bool,
    /// What the worn sword adds to a blow (sticky, embers, heart).
    pub weapon_fx: WeaponFx,
    /// A sword is in her hand (the weapon taken off: she punches, at half the damage).
    pub armed: bool,
    /// Her skill with the sword (Nemi's training): every blow × this.
    pub skill_bonus: f32,
    /// The slime coating the sword (its colour), if any.
    pub sword_goo: Option<[f32; 3]>,
    pub gold: u32,
    pub seal: bool,
    pub sigils: [u8; 3],
    pub rune_changed_at: [f32; 3],
    pub rune_start_angle: [f32; 3],
    pub rune_target_angle: [f32; 3],
    pub crest: bool,
    pub won: bool,
    pub opened: std::collections::BTreeSet<usize>,
    pub visited: std::collections::BTreeSet<usize>,
    pub elapsed: f32,
    /// Remaining first-person sword animation time.
    pub sword_swing: f32,
    pub sword_cooldown: f32,
    pub sword_hit_flash: f32,
    sword_hit_pending: bool,
    pub sword_combo: usize,
    pub sword_from: fp::Pose,
    sword_queued: bool,
    sword_hit_stop: f32,
    pub hit_flash: f32,
    pub notice: String,
    pub notice_until: f32,
    pub map_open: bool,
    /// The 3D map's camera: turn, tilt (elevation above the horizon, in
    /// radians) and zoom, plus its own clock (the run's stops while the map
    /// is open).
    pub map_yaw: f32,
    pub map_pitch: f32,
    pub map_zoom: f32,
    pub map_time: f32,
    /// Screen-space shift of the map (right-drag) and the drag in progress.
    pub map_pan: [f32; 2],
    map_drag: Option<MapDrag>,
    map_cursor: Option<[f32; 2]>,
    pub debug: bool,
    pub seed_input: Option<String>,
    pub checkpoint: Vec3,
    pub torch: bool,
    pub pending_exit: bool,
    pub jump_requested: bool,
    pub auto_walk: bool,
    /// How many times she has talked to the mine's people (the line they say next).
    pub npc_talks: usize,
    /// What fallen foes left on the floor, on its way to her.
    pub pickups: Vec<pickups::Pickup>,
    /// Kills one after another (each within a few seconds of the last), and when the last was.
    pub streak: u32,
    pub streak_at: f32,
    /// What she has picked up lately, for the notice, and when.
    loot_note: Vec<(String, u32)>,
    loot_note_at: f32,
    /// The named champions' clocks and the turning points they have passed, by spawn.
    champion_state: std::collections::BTreeMap<usize, (f32, u8)>,
    /// What a champion is saying.
    pub speech: Option<Speech>,
    /// The miner's shaft by the mouth, while she is down it.
    pub pozo: Option<Box<pozo::Pozo>>,
    /// Where the shaft's winch stands in the mine (by the Tendero's stall).
    pub shaft: Option<Vec3>,
    /// Her pick and bag for the shaft (the best she owns; the game fills it from her things).
    pub gear: pozo::Gear,
    /// The gold she keeps at home (the game mirrors it every frame) and what the Tendero took of it.
    pub purse: u32,
    pub purse_spent: u32,
    /// What she bought down here, for the game to put in her things.
    pub bought: Vec<String>,
    /// The Tendero's stall.
    pub stall: stall::Stall,
    /// What the shaft gave her lately: the label, its colour, when.
    pub mine_notes: Vec<(String, [f32; 3], f32)>,
    /// Per champion: what it has said (1 on seeing her, 2 raging, 4 falling), when it taunts next, how many
    /// taunts it has said.
    voices: std::collections::BTreeMap<usize, (u8, f32, usize)>,
}
/// The 3D map opens turned a third of the way round and looking down at
/// about fifty degrees.
/// Where the 3D chart sits on the 1920x1080 canvas (mouse drags start here).
pub const MAP_VIEWPORT: [f32; 4] = [48.0, 140.0, 1240.0, 790.0];
/// The map's tilt runs from looking up at it from below to straight down.
pub const MAP_PITCH_RANGE: (f32, f32) = (-1.45, 1.52);
#[derive(Clone, Copy, PartialEq, Debug)]
enum MapDrag {
    Orbit,
    Pan,
}
const MAP_YAW: f32 = 0.62;
const MAP_PITCH: f32 = 0.92;
impl Dungeon {
    pub fn generate(seed: u64) -> Self {
        Self::try_generate(seed, Config::default())
            .expect("bundled dungeon rules failed validation")
    }
    pub fn try_generate(seed: u64, config: Config) -> Result<Self, String> {
        if seed == authored::MINE_SEED {
            return Self::mine();
        }
        let catalog = Catalog::bundled();
        let map = generate(seed, &config, &catalog, &Difficulty::default())?;
        let mut controller = Controller::new(map.spawn());
        if let Some(s) = map.rooms[0].sockets.first() {
            let d = s.position - map.spawn();
            controller.yaw = (-d.x).atan2(-d.z);
        }
        let slime = slime::Slime::new(&map);
        let decor: Vec<decor::RoomDecor> = map
            .rooms
            .iter()
            .map(|room| decor::room(&map, room))
            .collect();
        let groves = slime::groves(&map, &decor);
        Ok(Self::assemble(
            map, catalog, controller, decor, slime, groves, None,
        ))
    }

    /// The hand-made mine (Mina 2.0), from `data/mina/mina.json`.
    pub fn mine() -> Result<Self, String> {
        let (map, mine, groves) = authored::load()?;
        let mut controller = Controller::new(mine.spawn);
        controller.yaw = mine.spawn_yaw;
        let decor = vec![decor::RoomDecor::default(); map.rooms.len()];
        let mut run = Self::assemble(
            map,
            Catalog::bundled(),
            controller,
            decor,
            None,
            groves,
            Some(std::sync::Arc::new(mine)),
        );
        run.checkpoint = run.controller.position;
        run.notice =
            "Alinea las tres runas, vence al Custodio y abre la garganta del Corazón.".into();
        run.shaft = run.find_shaft();
        Ok(run)
    }

    /// A run on a map: its actors from the map's spawns, a fresh Queen at its way in.
    fn assemble(
        map: Blueprint,
        catalog: Catalog,
        controller: Controller,
        decor: Vec<decor::RoomDecor>,
        slime: Option<slime::Slime>,
        groves: Vec<slime::Slime>,
        authored: Option<std::sync::Arc<authored::Authored>>,
    ) -> Self {
        let actors: Vec<Actor> = map
            .spawns
            .iter()
            .map(|s| Actor {
                spawn: s.id,
                position: s.position,
                health: s.health,
                damage_hits: 0,
                hit_reaction: 0.0,
                cooldown: combat::opening_cooldown(s.id),
                attack: None,
                attack_index: s.id % combat::moves(&s.definition, s.kind, s.id).len(),
                yaw: 0.0,
                stagger: 0.0,
                enraged: false,
                death: 0.0,
                flee: 0.0,
                stand: 0.0,
                attack_target: Vec3::ZERO,
                ambush_opening: false,
                reposition_elapsed: 0.0,
                used: false,
                chest_open_progress: 0.0,
                chest_opening: false,
                path: vec![],
                nav_timer: 0.0,
                gait: combat::Gait::Idle,
                stuck: 0.0,
                slowed: 0.0,
                burning: 0.0,
            })
            .collect();
        let actors_len = actors.len();
        let checkpoint = map.spawn();
        Self {
            authored,
            decor,
            fx: Vec::new(),
            groves,
            base_actors: actors_len,
            heart: HeartState::default(),
            shake: 0.0,
            sounds: Vec::new(),
            step_distance: 0.0,
            was_grounded: true,
            map,
            catalog,
            controller,
            actors,
            slime,
            health: 100.0,
            potions: 3,
            herbs: 0,
            unbanked_potions: 0,
            unbanked_herbs: 0,
            materials: Default::default(),
            relic_sword: false,
            unbanked_relic_sword: false,
            armed: true,
            skill_bonus: 1.0,
            sword_goo: None,
            weapon_fx: WeaponFx::default(),
            gold: 0,
            seal: false,
            sigils: [0; 3],
            rune_changed_at: [-100.0; 3],
            rune_start_angle: [0.0; 3],
            rune_target_angle: [0.0; 3],
            crest: false,
            won: false,
            opened: Default::default(),
            visited: Default::default(),
            elapsed: 0.0,
            sword_swing: 0.0,
            sword_cooldown: 0.0,
            sword_hit_flash: 0.0,
            sword_hit_pending: false,
            sword_combo: 0,
            sword_from: fp::REST,
            sword_queued: false,
            sword_hit_stop: 0.0,
            hit_flash: 0.0,
            notice: "Encuentra el sello antiguo y derrota al Custodio.".into(),
            notice_until: 8.0,
            map_open: false,
            map_yaw: MAP_YAW,
            map_pitch: MAP_PITCH,
            map_zoom: 1.0,
            map_time: 0.0,
            map_pan: [0.0; 2],
            map_drag: None,
            map_cursor: None,
            debug: false,
            seed_input: None,
            checkpoint,
            torch: true,
            pending_exit: false,
            jump_requested: false,
            auto_walk: false,
            npc_talks: 0,
            pickups: Vec::new(),
            streak: 0,
            streak_at: -100.0,
            loot_note: Vec::new(),
            loot_note_at: -100.0,
            champion_state: Default::default(),
            speech: None,
            voices: Default::default(),
            pozo: None,
            shaft: None,
            gear: Default::default(),
            purse: 0,
            purse_spent: 0,
            bought: Vec::new(),
            stall: Default::default(),
            mine_notes: Vec::new(),
        }
    }
    pub fn seed(&self) -> u64 {
        self.map.seed
    }
    /// Make a sound here (`None`: at the Queen).
    pub fn sound(&mut self, sfx: sound::Sfx, at: Option<Vec3>, volume: f32) {
        self.sounds.push(sound::Sound { sfx, at, volume });
    }

    /// The sounds made since the last call.
    pub fn take_sounds(&mut self) -> Vec<sound::Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// Pick up materials and say so.
    fn gather(&mut self, drops: &[(&str, u32)]) -> String {
        let mut parts = Vec::new();
        if drops.iter().any(|(_, n)| *n > 0) {
            self.sound(sound::Sfx::Pickup, None, 0.45);
        }
        for (id, count) in drops.iter().copied().filter(|(_, n)| *n > 0) {
            *self.materials.entry(id.to_string()).or_default() += count;
            let name = MATERIALS
                .iter()
                .find(|(m, _)| *m == id)
                .map_or(id, |(_, name)| *name);
            // In the player's language (the whole message is put together here).
            parts.push(format!("+{count} {}", crate::i18n::tr(name)));
        }
        parts.join(", ")
    }

    /// The materials carried out of the calabozo (once).
    pub fn take_materials(&mut self) -> std::collections::BTreeMap<String, u32> {
        std::mem::take(&mut self.materials)
    }

    /// Whether the relic sword was found on this expedition (once).
    pub fn take_relic_sword(&mut self) -> bool {
        std::mem::take(&mut self.unbanked_relic_sword)
    }
    pub fn take_acquired_items(&mut self) -> (u32, u32) {
        (
            std::mem::take(&mut self.unbanked_potions),
            std::mem::take(&mut self.unbanked_herbs),
        )
    }
    pub fn rune_angle(&self, index: usize) -> f32 {
        let t = ((self.elapsed - self.rune_changed_at[index]) / 0.82).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        self.rune_start_angle[index]
            + (self.rune_target_angle[index] - self.rune_start_angle[index]) * eased
    }
    pub fn spawn_position(&self) -> Vec3 {
        self.map.spawn()
    }
    pub fn spawn_yaw(&self) -> f32 {
        if let Some(mine) = &self.authored {
            return mine.spawn_yaw;
        }
        let Some(socket) = self.map.rooms[0].sockets.first() else {
            return 0.0;
        };
        let d = socket.position - self.map.spawn();
        (-d.x).atan2(-d.z)
    }
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        self.map.rooms.iter().fold(
            (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ),
            |a, r| {
                (
                    a.0.min(r.center.x - r.half[0]),
                    a.1.max(r.center.x + r.half[0]),
                    a.2.min(r.center.z - r.half[1]),
                    a.3.max(r.center.z + r.half[1]),
                )
            },
        )
    }
    pub fn walkable(&self, x: f32, z: f32) -> bool {
        self.map
            .capsule_floor(Vec3::new(x, self.controller.position.y, z), PLAYER_RADIUS)
            .is_some()
    }
    pub fn message(&mut self, s: impl Into<String>) {
        self.notice = s.into();
        self.notice_until = self.elapsed + 4.0;
    }
    pub fn camera(&self) -> velvet_render::Camera3D {
        let t = self.elapsed;
        let jolt = Vec3::new((t * 53.0).sin(), (t * 61.0).cos(), (t * 47.0).sin())
            * (0.05 * self.shake.min(1.0));
        velvet_render::Camera3D {
            position: self.controller.eye() + jolt,
            target: self.controller.eye() + jolt + self.controller.forward(),
            up: Vec3::Y,
            fov_y_radians: 72f32.to_radians(),
            orthographic_height: None,
            near: 0.055,
            far: 95.0,
        }
    }
    /// The living boss or miniboss the player is facing: (name, health
    /// fraction, enraged), for the HUD's bar.
    pub fn boss_bar(&self) -> Option<(&str, f32, bool)> {
        let player = self.controller.position;
        let current = self.map.region_at(player);
        self.actors.iter().find_map(|actor| {
            let spawn = &self.map.spawns[actor.spawn];
            let named = self
                .authored
                .as_ref()
                .and_then(|mine| mine.champions.get(&spawn.id))
                .map(|c| c.name.as_str());
            let name = match spawn.kind {
                _ if named.is_some() => named.unwrap_or_default(),
                SpawnKind::Boss => "Guardián del Corazón",
                SpawnKind::Miniboss if self.map.rooms[spawn.room].role == Role::Miniboss => {
                    "Custodio del Sello"
                }
                SpawnKind::Miniboss => "Campeón de los Acechadores",
                _ => return None,
            };
            (actor.health > 0.0
                && spawn.room == current
                && horizontal(actor.position - player).length() < 24.0)
                .then(|| {
                    (
                        name,
                        (actor.health / spawn.health).clamp(0.0, 1.0),
                        actor.enraged,
                    )
                })
        })
    }

    /// Break the props and braziers a big blow reaches: they fly apart as
    /// rubble and stop blocking the way.
    fn shatter(&mut self, room: usize, center: Vec3, radius: f32, fx: &mut Vec<combat::Fx>) {
        let Some(decor) = self.decor.get_mut(room) else {
            return;
        };
        let mut seed = center.x.to_bits() ^ center.z.to_bits().rotate_left(7);
        let mut broken = |at: Vec3, size: f32, fx: &mut Vec<combat::Fx>| {
            seed = seed.wrapping_mul(0x9E37_79B9).wrapping_add(1);
            fx.push(combat::Fx::Debris {
                at,
                size,
                seed,
                age: 0.0,
            });
            fx.push(combat::Fx::dust(at + Vec3::Y * 0.1, seed ^ 0xd057));
        };
        decor.props.retain(|prop| {
            let hit = !prop.asset.is_empty()
                && (prop.position.y - center.y).abs() < 2.0
                && horizontal(prop.position - center).length() < radius + prop.radius;
            if hit {
                broken(prop.position, (prop.height * 0.6).clamp(0.5, 1.6), fx);
            }
            !hit
        });
        decor.beacons.retain(|beacon| {
            let hit = horizontal(*beacon - center).length() < radius + 0.4
                && (beacon.y - center.y).abs() < 2.0;
            if hit {
                broken(*beacon, 1.0, fx);
            }
            !hit
        });
    }

    /// The Heart's middle: the modelled one in the hand-made mine, else above its room's centre.
    fn heart_center(&self, room: usize) -> Vec3 {
        match self.authored.as_ref().and_then(|mine| mine.heart) {
            Some((center, _)) => center,
            None => self.map.rooms[room].center + Vec3::Y * 1.5,
        }
    }

    /// The floor under the Heart.
    fn heart_floor(&self, room: usize) -> Vec3 {
        match self.authored.as_ref().and_then(|mine| mine.heart) {
            Some((_, floor)) => floor,
            None => self.map.rooms[room].center,
        }
    }

    /// The Heart spits blood at her: orbs arcing from its body to where she stands and round it,
    /// each mark glowing on the floor before it bursts.
    fn heart_spit(&mut self, room: usize, enraged: bool, fx: &mut Vec<combat::Fx>) {
        let from = self.heart_center(room);
        let count = if enraged { 5 } else { 3 };
        self.orbs_at_her(from, count, 9.0, combat::BOSS_CRIMSON, fx);
        let at = self.heart_center(room);
        self.sound(sound::Sfx::SpellBurst, Some(at), 0.7);
    }

    /// Orbs arcing from `from` to where she stands and round it, each marked on the floor first.
    fn orbs_at_her(
        &mut self,
        from: Vec3,
        count: usize,
        damage: f32,
        color: [f32; 3],
        fx: &mut Vec<combat::Fx>,
    ) {
        let player = self.controller.position;
        let turn = self.elapsed * 1.3;
        for k in 0..count {
            let mut to = if k == 0 {
                player
            } else {
                let a = turn + k as f32 * std::f32::consts::TAU / (count - 1) as f32;
                player + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0
            };
            let Some(y) = self
                .map
                .capsule_floor(to + Vec3::Y * 0.5, 0.2)
                .filter(|y| (y - player.y).abs() < 1.5)
            else {
                continue;
            };
            to.y = y;
            let lag = k as f32 * 0.14;
            let flight = combat::Fx::spell_travel(from, to);
            fx.push(combat::Fx::Telegraph {
                at: to,
                age: -lag,
                life: flight,
                radius: 1.25,
            });
            fx.push(combat::Fx::Spell {
                from,
                to,
                age: -lag,
                damage,
                radius: 1.25,
                color,
            });
        }
    }

    /// The named champions' powers while she is in their room: embers, volleys, the dead
    /// raised, their fury.
    /// Where the shaft's winch stands: a little east of the Tendero's stall, on open ground.
    fn find_shaft(&self) -> Option<Vec3> {
        let npc = self.authored.as_ref()?.npcs.first()?;
        let want = npc.position + Vec3::new(3.6, 0.0, 1.2);
        (0..48).find_map(|k| {
            let r = (k / 12) as f32 * 0.6;
            let a = k as f32 * std::f32::consts::TAU / 12.0;
            let at = want + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
            self.map
                .capsule_floor(at + Vec3::Y * 0.6, 1.1)
                .filter(|y| (y - npc.position.y).abs() < 0.8)
                .map(|y| Vec3::new(at.x, y, at.z))
        })
    }

    /// Whether she stands by the shaft's winch (F takes her down).
    pub fn at_shaft(&self) -> bool {
        self.pozo.is_none()
            && self
                .shaft
                .is_some_and(|s| horizontal(s - self.controller.position).length() < 2.4)
    }

    /// Down the shaft: the cage takes her to its ledge.
    pub fn enter_pozo(&mut self) {
        let came_from = (self.controller.position, self.controller.yaw);
        let seed = self.map.seed ^ 0x504f_5a4f;
        let pozo = pozo::Pozo::new(seed, &self.map, came_from);
        let (at, yaw) = pozo::Pozo::arrival();
        self.controller.position = at;
        self.controller.velocity = Vec3::ZERO;
        self.controller.yaw = yaw;
        self.controller.pitch = -0.25;
        self.pozo = Some(Box::new(pozo));
        self.auto_walk = false;
        self.message(
            "El Pozo del Minero: clic para picar (mantén pulsado), U para que el torno te suba.",
        );
        self.sound(sound::Sfx::ChestCreak, None, 0.6);
    }

    /// Back up and out to the mine by the winch.
    pub fn leave_pozo(&mut self) {
        let Some(pozo) = self.pozo.take() else {
            return;
        };
        let (at, yaw) = pozo.came_from;
        self.controller.position = at;
        self.controller.velocity = Vec3::ZERO;
        self.controller.yaw = yaw;
        self.message(format!(
            "Fuera del pozo. Tu marca: {} m.",
            (pozo.deepest + 1).max(0)
        ));
    }

    /// She stops holding the pick down.
    pub fn release_attack(&mut self) {
        if let Some(pozo) = self.pozo.as_mut() {
            pozo.holding = false;
        }
    }

    /// One step in the shaft: she walks its floors (or the winch hauls her up), and digs.
    fn update_pozo(&mut self, input: Input, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.shake = (self.shake - dt * 1.6).max(0.0);
        let Some(mut pozo) = self.pozo.take() else {
            return;
        };
        if let Some(left) = pozo.hauling.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                pozo.hauling = None;
                let (at, yaw) = pozo::Pozo::arrival();
                self.controller.position = at;
                self.controller.velocity = Vec3::ZERO;
                self.controller.yaw = yaw;
                self.message("El torno te ha subido hasta la boca del pozo.");
            }
        } else {
            let before = self.controller.position;
            self.controller.step(&pozo.map, input, dt);
            let walked = horizontal(self.controller.position - before).length();
            self.step_distance += walked;
            if self.step_distance > 1.6 && self.controller.grounded {
                self.step_distance = 0.0;
                self.sound(sound::Sfx::Footstep, None, 0.35);
            }
        }
        let eye = self.controller.eye();
        let forward = self.controller.forward();
        let gear = self.gear;
        let events = pozo.dig(
            eye,
            forward,
            gear,
            &mut self.materials,
            &mut self.gold,
            self.elapsed,
            dt,
        );
        for event in events {
            match event {
                pozo::Event::Hit { at, rock } => {
                    let c = rock.color();
                    self.fx.push(combat::Fx::Burst {
                        at,
                        color: [c[0] * 0.55, c[1] * 0.5, c[2] * 0.45],
                        count: 8,
                        speed: 1.6,
                        life: 0.35,
                        gravity: 6.0,
                        up: 0.6,
                        length: 0.04,
                        seed: (self.elapsed * 1000.0) as u32,
                        age: 0.0,
                    });
                    self.sound(sound::Sfx::HitBone, Some(at), 0.45);
                }
                pozo::Event::Broke { at, rock } => {
                    let c = rock.color();
                    self.fx.push(combat::Fx::Burst {
                        at,
                        color: [c[0] * 0.6, c[1] * 0.55, c[2] * 0.5],
                        count: 22,
                        speed: 2.6,
                        life: 0.6,
                        gravity: 7.0,
                        up: 0.8,
                        length: 0.07,
                        seed: (self.elapsed * 977.0) as u32,
                        age: 0.0,
                    });
                    self.sound(sound::Sfx::Rubble, Some(at), 0.8);
                    self.shake = self.shake.max(0.12);
                }
                pozo::Event::Got { label, color } => {
                    let shiny = color.iter().copied().fold(0.0f32, f32::max) > 0.8;
                    self.sound(
                        if label.contains("oro") {
                            sound::Sfx::Coins
                        } else if shiny {
                            sound::Sfx::RuneChime
                        } else {
                            sound::Sfx::Pickup
                        },
                        None,
                        0.7,
                    );
                    self.mine_notes.push((label, color, self.elapsed));
                }
                pozo::Event::TooHard { needs } => self.message(format!(
                    "Esta roca es demasiado dura: hace falta un {needs}."
                )),
                pozo::Event::Full => {
                    self.message("La mochila está llena: sube (U) y véndeselo al Tendero.")
                }
            }
        }
        // effects fade
        let player = self.controller.position;
        self.fx.retain_mut(|effect| effect.tick(dt, player).1);
        let now = self.elapsed;
        self.mine_notes.retain(|n| now - n.2 < 3.6);
        self.pozo = Some(pozo);
    }

    /// A champion's name, as its bar shows it.
    pub fn champion_name(&self, spawn: &gameplay::Spawn) -> Option<String> {
        if let Some(c) = self
            .authored
            .as_ref()
            .and_then(|mine| mine.champions.get(&spawn.id))
        {
            return Some(c.name.clone());
        }
        match spawn.kind {
            SpawnKind::Boss => Some("Guardián del Corazón".into()),
            SpawnKind::Miniboss if self.map.rooms[spawn.room].role == Role::Miniboss => {
                Some("Custodio del Sello".into())
            }
            _ => None,
        }
    }

    /// The champions speak: once on seeing her (when they move on her), once when they rage, now and then
    /// while they fight her close, and as they fall.
    fn voices_tick(&mut self) {
        let player = self.controller.position;
        for index in 0..self.actors.len() {
            let (spawn_id, health, enraged, engaged, at) = {
                let a = &self.actors[index];
                (
                    a.spawn,
                    a.health,
                    a.enraged,
                    a.gait != combat::Gait::Idle || a.attack.is_some(),
                    a.position,
                )
            };
            let spawn = &self.map.spawns[spawn_id];
            let Some(model) = authored::champion_model(self.authored.as_deref(), spawn) else {
                continue;
            };
            let Some(lines) = authored::champion_lines(model) else {
                continue;
            };
            let close = horizontal(at - player).length();
            let (said, next, taunts) = self.voices.get(&spawn_id).copied().unwrap_or((0, 0.0, 0));
            let line = if health <= 0.0 {
                (said & 1 != 0 && said & 4 == 0).then(|| (lines.fall, 4u8, true))
            } else if said & 1 == 0 {
                (engaged && close < 16.0).then(|| (lines.wake, 1, true))
            } else if enraged && said & 2 == 0 {
                Some((lines.rage, 2, true))
            } else if close < 9.0
                && self.elapsed > next
                && self
                    .speech
                    .as_ref()
                    .is_none_or(|s| self.elapsed - s.at > SPEECH_SECONDS + 2.0)
            {
                Some((lines.taunts[taunts % lines.taunts.len()], 0, false))
            } else {
                None
            };
            let Some((line, mark, urgent)) = line else {
                continue;
            };
            // a word just said is not cut off, but by one that must be heard
            if !urgent
                && self
                    .speech
                    .as_ref()
                    .is_some_and(|s| self.elapsed - s.at < 1.5)
            {
                continue;
            }
            let who = self.champion_name(spawn).unwrap_or_default();
            self.speech = Some(Speech {
                who,
                line,
                color: authored::champion_color(model),
                at: self.elapsed,
            });
            let gap = 11.0 + (taunts % 3) as f32 * 2.0;
            self.voices.insert(
                spawn_id,
                (
                    said | mark,
                    self.elapsed + gap,
                    taunts + usize::from(mark == 0),
                ),
            );
            if mark == 1 {
                self.sound(sound::Sfx::Roar, Some(at), 0.55);
            }
        }
    }

    fn champions_tick(&mut self, current: usize, dt: f32, fx: &mut Vec<combat::Fx>) {
        let Some(mine) = self.authored.clone() else {
            return;
        };
        let player = self.controller.position;
        for (&id, champion) in &mine.champions {
            let Some(index) = self.actors.iter().position(|a| a.spawn == id) else {
                continue;
            };
            let (health, enraged, at) = {
                let a = &self.actors[index];
                (a.health, a.enraged, a.position)
            };
            let (full, room) = {
                let s = &self.map.spawns[id];
                (s.health.max(1.0), s.room)
            };
            if health <= 0.0 || room != current || horizontal(at - player).length() > 22.0 {
                continue;
            }
            let life = health / full;
            let state = self.champion_state.entry(id).or_insert((4.0, 0));
            state.0 -= dt;
            let mut fire = false;
            let mut turn = false;
            match champion.power {
                authored::Power::Sparks | authored::Power::Volley if state.0 <= 0.0 => {
                    let calm = if champion.power == authored::Power::Sparks {
                        7.0
                    } else {
                        9.0
                    };
                    state.0 = if enraged { calm * 0.7 } else { calm };
                    fire = true;
                }
                authored::Power::Raise => {
                    let points = [0.66f32, 0.33];
                    if (state.1 as usize) < points.len() && life < points[state.1 as usize] {
                        state.1 += 1;
                        turn = true;
                    }
                }
                authored::Power::Fury if state.1 == 0 && life < 0.5 => {
                    state.1 = 1;
                    turn = true;
                }
                _ => {}
            }
            let head = at + Vec3::Y * (champion.size * 0.85);
            match champion.power {
                authored::Power::Sparks if fire => {
                    self.orbs_at_her(head, 4, 8.0, [1.0, 0.42, 0.1], fx);
                    self.sound(sound::Sfx::SpellCharge, Some(head), 0.8);
                }
                authored::Power::Volley if fire => {
                    self.orbs_at_her(head, 5, 7.0, combat::MAGE_VIOLET, fx);
                    self.sound(sound::Sfx::SpellCharge, Some(head), 0.8);
                }
                authored::Power::Raise if turn => {
                    self.summon_around(room, Some(at), 3, fx);
                    self.sound(sound::Sfx::Summon, Some(at), 1.0);
                    self.message(format!("{} alza a los muertos.", champion.name));
                }
                authored::Power::Fury if turn => {
                    self.actors[index].enraged = true;
                    self.summon_around(room, Some(at), 2, fx);
                    self.sound(sound::Sfx::Roar, Some(at), 1.0);
                    self.message(format!(
                        "{} se enfurece y llama a los suyos.",
                        champion.name
                    ));
                }
                _ => {}
            }
        }
    }

    /// What reached her from the floor: into her purse and her bag, with a note of it all.
    fn take_loot(&mut self, taken: &[pickups::Loot]) {
        if self.elapsed - self.loot_note_at > 2.4 {
            self.loot_note.clear();
        }
        self.loot_note_at = self.elapsed;
        let mut coins = false;
        for loot in taken {
            let (label, n) = match *loot {
                pickups::Loot::Gold(n) => {
                    self.gold += n;
                    coins = true;
                    ("oro".to_string(), n)
                }
                pickups::Loot::Material(id, n) => {
                    *self.materials.entry(id.to_string()).or_default() += n;
                    let name = MATERIALS
                        .iter()
                        .find(|(m, _)| *m == id)
                        .map_or(id, |(_, name)| *name);
                    (crate::i18n::tr(name).to_string(), n)
                }
                pickups::Loot::Tonic => {
                    self.potions += 1;
                    self.unbanked_potions += 1;
                    (crate::i18n::tr("tónico").to_string(), 1)
                }
                pickups::Loot::Herbs(n) => {
                    self.herbs += n;
                    self.unbanked_herbs += n;
                    (crate::i18n::tr("hierbas").to_string(), n)
                }
            };
            match self.loot_note.iter_mut().find(|(l, _)| *l == label) {
                Some(entry) => entry.1 += n,
                None => self.loot_note.push((label, n)),
            }
        }
        self.sound(
            if coins {
                sound::Sfx::Coins
            } else {
                sound::Sfx::Pickup
            },
            None,
            0.5,
        );
        let note = self
            .loot_note
            .iter()
            .map(|(label, n)| format!("+{n} {label}"))
            .collect::<Vec<_>>()
            .join(" · ");
        self.message(note);
    }

    /// Raise `count` minions around the Heart's chamber.
    fn heart_summon(&mut self, room: usize, count: usize, fx: &mut Vec<combat::Fx>) {
        self.summon_around(room, None, count, fx);
    }

    /// Raise `count` minions in a room, round `at` (else round the Heart or the room's centre).
    fn summon_around(
        &mut self,
        room: usize,
        at: Option<Vec3>,
        count: usize,
        fx: &mut Vec<combat::Fx>,
    ) {
        let living = self.actors[self.base_actors..]
            .iter()
            .filter(|a| a.health > 0.0)
            .count();
        let count = count.min(MAX_SUMMONS.saturating_sub(living));
        let Some(r) = self.map.rooms.get(room) else {
            return;
        };
        // in the hand-made mine they rise round the Heart, on its chamber's floor
        let authored = self.authored.is_some();
        let center = match at {
            Some(at) => at,
            None if authored => self.heart_floor(room),
            None => r.center,
        };
        let player = self.controller.position;
        let template = self
            .map
            .spawns
            .iter()
            .find(|s| s.kind == SpawnKind::Enemy)
            .cloned();
        let Some(template) = template else { return };
        let turn = self.elapsed * 0.7;
        let mut raised = 0;
        for k in 0..16 {
            if raised == count {
                break;
            }
            let angle = turn + k as f32 * std::f32::consts::TAU / 7.0;
            let reach = if authored { 4.6 } else { 3.2 } + (k % 3) as f32 * 1.1;
            let mut at = center + Vec3::new(angle.cos(), 0.0, angle.sin()) * reach;
            let inside = if authored {
                self.map.room_at(at + Vec3::Y * 0.3) == Some(room)
            } else {
                r.contains(at.x, at.z, 0.8)
            };
            if !inside || horizontal(at - player).length() < 2.2 {
                continue;
            }
            let Some(y) = self
                .map
                .capsule_floor(at, 0.4)
                .filter(|y| (y - center.y).abs() < 1.0)
            else {
                continue;
            };
            at.y = y;
            let id = self.map.spawns.len();
            self.map.spawns.push(velvet_dungeon::gameplay::Spawn {
                id,
                room,
                kind: SpawnKind::Enemy,
                definition: "fallen".into(),
                position: at,
                health: template.health.max(24.0),
                damage: template.damage.max(6.0),
                speed: template.speed,
                gold: 0,
                budget: 0.0,
            });
            let moves = combat::moves("fallen", SpawnKind::Enemy, id);
            self.actors.push(Actor {
                spawn: id,
                position: at,
                health: template.health.max(24.0),
                damage_hits: 0,
                hit_reaction: 0.0,
                // It claws its way out before it fights.
                cooldown: combat::SUMMON_SECONDS + 0.4,
                attack: None,
                attack_index: id % combat::cycle_len(moves),
                yaw: combat::yaw_to(player - at),
                stagger: combat::SUMMON_SECONDS,
                enraged: false,
                death: 0.0,
                flee: 0.0,
                stand: 0.0,
                attack_target: Vec3::ZERO,
                ambush_opening: false,
                reposition_elapsed: 0.0,
                used: false,
                chest_open_progress: 0.0,
                chest_opening: false,
                path: vec![],
                nav_timer: 0.0,
                gait: combat::Gait::Idle,
                stuck: 0.0,
                slowed: 0.0,
                burning: 0.0,
            });
            fx.push(combat::Fx::Summon { at, age: 0.0 });
            raised += 1;
        }
    }

    /// The Heart's powers while its guardian lives and the player is in its
    /// chamber: minions every so often and bouts of blood rain.
    fn heart_tick(&mut self, current: usize, dt: f32, fx: &mut Vec<combat::Fx>) {
        let guardian = self.actors[..self.base_actors].iter().find_map(|actor| {
            let spawn = &self.map.spawns[actor.spawn];
            (spawn.kind == SpawnKind::Boss && actor.health > 0.0).then_some((
                spawn.room,
                actor.enraged,
                actor.health / spawn.health.max(1.0),
            ))
        });
        let awake = guardian
            .filter(|(room, _, _)| *room == current)
            .map(|(room, enraged, _)| (room, enraged));
        let life = guardian.map_or(1.0, |(_, _, life)| life);
        let heart = &mut self.heart;
        let mut spit = false;
        let mut ring = false;
        if let Some((room, enraged)) = awake {
            if !heart.awake {
                // The first beats come soon after the player walks in.
                heart.awake = true;
                heart.summon_in = 9.0;
                heart.rain_in = 15.0;
            }
            heart.summon_in -= dt;
            heart.rain_in -= dt;
            heart.beat_in -= dt;
            heart.spit_in -= dt;
            let beat = heart.beat_in <= 0.0;
            if beat {
                heart.beat_in = if enraged { 0.78 } else { 1.1 };
                heart.beats += 1;
                // Raging, every third beat sends a ring of blood over the floor: jump it.
                ring = enraged && heart.beats % 3 == 0 && heart.wave.is_none();
            }
            if heart.spit_in <= 0.0 {
                heart.spit_in = if enraged { 5.5 } else { 8.5 };
                spit = heart.beats > 2;
            }
            let mut summon = false;
            if heart.summon_in <= 0.0 {
                heart.summon_in = if enraged { 15.0 } else { 22.0 };
                summon = true;
            }
            if heart.rain_in <= 0.0 {
                heart.rain_in = if enraged { 17.0 } else { 25.0 };
                heart.rain_left = 8.0;
                self.message("El Corazón late: llueve sangre. Tu vitalidad se consume.");
            }
            if summon {
                self.heart_summon(room, if enraged { 3 } else { 2 }, fx);
                self.message("El Corazón late y alza a sus siervos.");
            }
            if beat {
                let at = self.heart_center(room);
                self.sound(sound::Sfx::HeartBeat, Some(at), 1.0);
            }
            if spit {
                self.heart_spit(room, enraged, fx);
            }
            if ring {
                let origin = self.heart_floor(room);
                self.heart.wave = Some((origin, 0.0));
                self.heart.wave_hit = false;
                fx.push(combat::Fx::Burst {
                    at: origin + Vec3::Y * 0.4,
                    color: [0.9, 0.05, 0.08],
                    count: 30,
                    speed: 5.0,
                    life: 0.7,
                    gravity: 2.0,
                    up: 0.5,
                    length: 0.18,
                    seed: self.heart.beats,
                    age: 0.0,
                });
            }
            // The guardian's turning points: at two thirds and at a third of its life the Heart
            // raises a whole wave of servants, and the second time it rains blood at once.
            let turning = [0.66f32, 0.33];
            let passed = self.heart.thresholds as usize;
            if passed < turning.len() && life < turning[passed] {
                self.heart.thresholds += 1;
                self.heart_summon(room, 4, fx);
                if passed == 1 {
                    self.heart.rain_left = self.heart.rain_left.max(8.0);
                    self.message(
                        "El guardián se tambalea: el Corazón alza a los suyos y llueve sangre.",
                    );
                } else {
                    self.message("El guardián ruge y el Corazón alza una oleada de siervos.");
                }
                let at = self.heart_center(room);
                self.sound(sound::Sfx::HeartBeat, Some(at), 1.0);
            }
        } else {
            heart.awake = false;
            heart.rain_left = 0.0;
            heart.wave = None;
        }
        // The ring of blood: it catches her if she stands on the floor where it passes.
        if let Some((origin, age)) = self.heart.wave {
            let age = age + dt;
            let radius = age * HEART_WAVE_SPEED;
            if radius > HEART_WAVE_REACH {
                self.heart.wave = None;
            } else {
                self.heart.wave = Some((origin, age));
                let p = self.controller.position;
                let reach = horizontal(p - origin).length();
                if !self.heart.wave_hit
                    && (reach - radius).abs() < 0.6
                    && self.controller.grounded
                    && (p.y - origin.y).abs() < 1.6
                {
                    self.heart.wave_hit = true;
                    self.health -= 14.0;
                    self.hit_flash = 0.3;
                    self.shake = self.shake.max(0.5);
                    self.sound(sound::Sfx::PlayerHurt, None, 0.9);
                }
            }
        }
        let heart = &mut self.heart;
        heart.rain_left = (heart.rain_left - dt).max(0.0);
        let want = if heart.rain_left > 0.0 { 1.0 } else { 0.0 };
        heart.rain += (want - heart.rain) * (1.0 - (-dt * 1.5).exp());
        if heart.rain_left > 0.0 {
            // Slow, steady: never kills outright.
            self.health = (self.health - BLOOD_RAIN_DRAIN * dt).max(1.0);
        }
        // With its guardian gone, what the Heart raised falls with it.
        if guardian.is_none() {
            for actor in &mut self.actors[self.base_actors..] {
                actor.health = 0.0;
            }
        }
    }

    fn resolve_slime_contact(&mut self, previous: Vec3) {
        let near: Vec<Vec3> = self
            .slime
            .iter()
            .chain(&self.groves)
            .filter(|slime| slime.alive())
            .map(|slime| slime.position)
            .filter(|p| {
                horizontal(*p - self.controller.position).length() < slime::CONTACT_DISTANCE
            })
            .collect();
        for slime_position in near {
            self.resolve_contact_with(previous, slime_position);
        }
    }

    fn resolve_contact_with(&mut self, previous: Vec3, slime_position: Vec3) {
        if (self.controller.position.y - slime_position.y).abs() > 1.1 {
            return;
        }
        let offset = horizontal(self.controller.position - slime_position);
        let distance = offset.length();
        if distance >= slime::CONTACT_DISTANCE {
            return;
        }
        let away = if distance > 0.001 {
            offset * (1.0 / distance)
        } else {
            -horizontal(self.controller.forward()).normalize_or_zero()
        };
        let mut candidate =
            self.controller.position + away * (slime::CONTACT_DISTANCE - distance + 0.01);
        if let Some(room) = self.map.room_at(candidate) {
            candidate = decor::resolve(&self.decor[room], candidate, PLAYER_RADIUS);
        }
        let valid_floor = self
            .map
            .capsule_floor(candidate, PLAYER_RADIUS)
            .filter(|floor| (*floor - self.controller.position.y).abs() < 0.32);
        if let Some(floor) = valid_floor
            .filter(|_| horizontal(candidate - slime_position).length() >= slime::CONTACT_DISTANCE)
        {
            candidate.y = floor;
            self.controller.position = candidate;
        } else if horizontal(previous - slime_position).length() >= slime::CONTACT_DISTANCE
            && self.map.capsule_floor(previous, PLAYER_RADIUS).is_some()
        {
            self.controller.position = previous;
        }
        self.controller.velocity.x = 0.0;
        self.controller.velocity.z = 0.0;
    }
    /// Turns, tilts and zooms the 3D map with the held movement keys.
    fn steer_map(&mut self, input: Input, dt: f32) {
        let speed = if input.sprint { 2.4 } else { 1.0 };
        self.map_yaw =
            (self.map_yaw + input.strafe * 1.4 * speed * dt).rem_euclid(std::f32::consts::TAU);
        self.map_pitch = (self.map_pitch + input.forward * 0.8 * speed * dt)
            .clamp(MAP_PITCH_RANGE.0, MAP_PITCH_RANGE.1);
    }
    /// Zoom the 3D map by `steps` notches (positive zooms in).
    pub fn map_zoom_by(&mut self, steps: f32) {
        self.map_zoom = (self.map_zoom * 1.18f32.powf(steps)).clamp(0.6, 3.2);
    }
    pub fn reset_map_view(&mut self) {
        self.map_yaw = MAP_YAW;
        self.map_pitch = MAP_PITCH;
        self.map_zoom = 1.0;
        self.map_pan = [0.0; 2];
    }
    /// A mouse button went down over the map (`pan` = the secondary
    /// button); starts an orbit or a pan if the pointer is on the chart.
    pub fn map_press(&mut self, pan: bool) -> bool {
        let Some([x, y]) = self
            .map_cursor
            .filter(|_| self.map_open && self.seed_input.is_none())
        else {
            return false;
        };
        let [vx, vy, vw, vh] = MAP_VIEWPORT;
        if x < vx || y < vy || x > vx + vw || y > vy + vh {
            return false;
        }
        self.map_drag = Some(if pan { MapDrag::Pan } else { MapDrag::Orbit });
        true
    }
    pub fn map_release(&mut self) {
        self.map_drag = None;
    }
    pub fn map_dragging(&self) -> bool {
        self.map_drag.is_some()
    }
    /// The pointer moved (canvas coordinates; `None` = it left the window).
    pub fn map_pointer(&mut self, at: Option<[f32; 2]>) {
        if let (Some(drag), Some(prev), Some(now)) = (self.map_drag, self.map_cursor, at) {
            let (dx, dy) = (now[0] - prev[0], now[1] - prev[1]);
            match drag {
                MapDrag::Orbit => {
                    self.map_yaw = (self.map_yaw - dx * 0.006).rem_euclid(std::f32::consts::TAU);
                    self.map_pitch =
                        (self.map_pitch + dy * 0.005).clamp(MAP_PITCH_RANGE.0, MAP_PITCH_RANGE.1);
                }
                MapDrag::Pan => {
                    self.map_pan = [
                        (self.map_pan[0] + dx).clamp(-1600.0, 1600.0),
                        (self.map_pan[1] + dy).clamp(-1200.0, 1200.0),
                    ];
                }
            }
        }
        if at.is_none() || !self.map_open {
            self.map_drag = None;
        }
        self.map_cursor = at;
    }
    pub fn update(&mut self, mut input: Input, dt: f32) {
        self.map_time += dt.clamp(0.0, 0.05);
        if self.map_open && self.seed_input.is_none() {
            self.steer_map(input, dt.clamp(0.0, 0.05));
        }
        if self.seed_input.is_some() || self.map_open {
            return;
        }
        if self.stall.open {
            self.elapsed += dt.clamp(0.0, 0.05);
            return;
        }
        if self.pozo.is_some() {
            self.update_pozo(input, dt);
            return;
        }
        if self.auto_walk && input.forward == 0.0 {
            input.forward = 1.0;
        }
        let dt = dt.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.sword_cooldown = (self.sword_cooldown - dt).max(0.0);
        self.sword_hit_flash = (self.sword_hit_flash - dt).max(0.0);
        let stopped = self.sword_hit_stop > 0.0;
        self.sword_hit_stop = (self.sword_hit_stop - dt).max(0.0);
        if self.sword_swing > 0.0 {
            let clip = fp::SWINGS[self.sword_combo];
            let previous = clip.duration - self.sword_swing;
            if !stopped {
                self.sword_swing = (self.sword_swing - dt).max(0.0);
            }
            let now = clip.duration - self.sword_swing;
            if self.sword_hit_pending && previous < clip.strike && now >= clip.strike {
                self.sword_hit_pending = false;
                self.resolve_sword_hit();
            }
            if self.sword_queued && now >= fp::CHAIN_COMMIT && self.sword_combo < 2 {
                self.start_sword_swing(self.sword_combo + 1);
            } else if self.sword_swing <= 0.0 {
                self.sword_queued = false;
                self.sword_cooldown = SWORD_ATTACK_COOLDOWN;
            }
        }
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.shake = (self.shake - dt * 1.6).max(0.0);
        let before = self.controller.position;
        self.controller.step(&self.map, input, dt);
        // Footsteps every stride on the ground; a jump and its landing.
        let grounded = self.controller.grounded;
        if grounded {
            self.step_distance += horizontal(self.controller.position - before).length();
            if self.step_distance > if input.sprint { 1.9 } else { 1.45 } {
                self.step_distance = 0.0;
                self.sound(
                    sound::Sfx::Footstep,
                    None,
                    if input.sprint { 0.34 } else { 0.26 },
                );
            }
        }
        if grounded && !self.was_grounded {
            self.sound(sound::Sfx::Land, None, 0.5);
            self.step_distance = 0.0;
        } else if !grounded && self.was_grounded && self.controller.velocity.y > 0.5 {
            self.sound(sound::Sfx::Jump, None, 0.35);
        }
        self.was_grounded = grounded;
        // Barrels, crates, braziers: the art the player sees is solid.
        if let Some(room) = self.map.room_at(self.controller.position) {
            let pushed = decor::resolve(&self.decor[room], self.controller.position, PLAYER_RADIUS);
            if pushed != self.controller.position {
                if self.map.capsule_floor(pushed, PLAYER_RADIUS).is_some() {
                    self.controller.position = pushed;
                } else {
                    self.controller.position.x = before.x;
                    self.controller.position.z = before.z;
                }
            }
        }
        if let Some(mine) = &self.authored {
            if !(self.seal && self.crest)
                && mine
                    .gates
                    .iter()
                    .any(|g| g.holds(self.controller.position, PLAYER_RADIUS))
            {
                self.controller.position = before;
                self.controller.velocity.x = 0.0;
                self.controller.velocity.z = 0.0;
            }
            // The mine's people stand in the way: she steps round them.
            for npc in &mine.npcs {
                let d = horizontal(self.controller.position - npc.position);
                if d.length() < 0.5 && (self.controller.position.y - npc.position.y).abs() < 1.6 {
                    let out = if d.length() > 1.0e-4 {
                        d.normalize_or_zero()
                    } else {
                        Vec3::Z
                    };
                    self.controller.position.x = npc.position.x + out.x * 0.5;
                    self.controller.position.z = npc.position.z + out.z * 0.5;
                }
            }
        }
        for c in &self.map.connections {
            let secret = c.kind == ConnectionKind::Secret && !self.opened.contains(&c.id);
            let boss = self.map.rooms[c.b].role == Role::Boss && !(self.seal && self.crest);
            if secret || boss {
                let door = if secret { c.start } else { c.end };
                let delta = horizontal(c.end - c.start).normalize_or_zero();
                let p = self.controller.position - door;
                if p.dot(delta).abs() < 0.45
                    && horizontal(p - delta * p.dot(delta)).length() < c.width * 0.5 + PLAYER_RADIUS
                    && p.y.abs() < 2.0
                {
                    self.controller.position = before;
                    self.controller.velocity.x = 0.0;
                    self.controller.velocity.z = 0.0;
                }
            }
        }
        let player = self.controller.position;
        let current = self.map.region_at(player);
        let first_visit = self.visited.insert(current);
        if first_visit {
            let champion = self.actors.iter().any(|actor| {
                actor.health > 0.0
                    && self.map.spawns[actor.spawn].room == current
                    && self.map.spawns[actor.spawn].kind == SpawnKind::Miniboss
                    && self.map.rooms[current].role == Role::Encounter
            });
            let scouts = self
                .map
                .spawns
                .iter()
                .any(|spawn| spawn.room == current && spawn.definition == "bone_scout");
            if champion {
                self.message(
                    "¡Campeón de los Acechadores! Acaba con su guardia para abrirte paso.",
                );
            } else if scouts {
                self.message("¡Emboscada! Ballesteros en los flancos. Sigue moviéndote.");
            }
        }
        if first_visit && self.map.rooms[current].role == Role::Encounter {
            let mut melee_rank = 0usize;
            let mut archer_rank = 0usize;
            for actor in &mut self.actors {
                let spawn = &self.map.spawns[actor.spawn];
                if spawn.room != current || actor.health <= 0.0 {
                    continue;
                }
                let delay = match enemy_style(&spawn.definition, spawn.kind) {
                    EnemyStyle::Ranged => {
                        let delay = 0.28 + archer_rank as f32 * 0.36;
                        archer_rank += 1;
                        actor.ambush_opening = true;
                        delay
                    }
                    EnemyStyle::Quick => {
                        let delay = 0.12 + melee_rank as f32 * 0.16;
                        melee_rank += 1;
                        delay
                    }
                    EnemyStyle::Heavy => {
                        let delay = 0.36 + melee_rank as f32 * 0.14;
                        melee_rank += 1;
                        delay
                    }
                    EnemyStyle::Mage => 0.52,
                };
                actor.cooldown = actor.cooldown.max(delay);
            }
        }
        let room_role = self.map.rooms[current].role;
        let room_center_y = self.map.rooms[current].center.y;
        // A generated cave's rooms are flat: well below one's floor she has fallen into a crack. The
        // hand-made mine's places slope and its floor map has no void: only a fall out of it all.
        let fell = match &self.authored {
            Some(_) => player.y < -40.0,
            None => player.y < room_center_y - 2.5,
        };
        if fell {
            self.health = (self.health - 12.0).max(1.0);
            self.controller = Controller::new(self.checkpoint);
            self.message("Caida en la grieta. Regresas al santuario: -12 vitalidad.");
        }
        let mut encounter_targets = vec![None; self.actors.len()];
        let mut archer_sides = vec![0.0; self.actors.len()];
        if room_role == Role::Encounter {
            let miniboss = self.actors.iter().find_map(|actor| {
                let spawn = &self.map.spawns[actor.spawn];
                (actor.health > 0.0 && spawn.room == current && spawn.kind == SpawnKind::Miniboss)
                    .then_some(actor.position)
            });
            let guarding = miniboss
                .filter(|boss| horizontal(*boss - player).length() > STALKER_BOSS_ENGAGE_RANGE);
            let melee = self
                .actors
                .iter()
                .enumerate()
                .filter_map(|(index, actor)| {
                    let spawn = &self.map.spawns[actor.spawn];
                    (actor.health > 0.0
                        && spawn.room == current
                        && spawn.kind == SpawnKind::Enemy
                        && matches!(
                            enemy_style(&spawn.definition, spawn.kind),
                            EnemyStyle::Quick | EnemyStyle::Heavy
                        ))
                    .then_some(index)
                })
                .collect::<Vec<_>>();
            let anchor = guarding.unwrap_or(player);
            let radius = if guarding.is_some() {
                STALKER_BOSS_GUARD_RADIUS
            } else {
                STALKER_SURROUND_RADIUS
            };
            for (rank, index) in melee.iter().copied().enumerate() {
                encounter_targets[index] =
                    encounter_ring_position(&self.map, current, anchor, rank, melee.len(), radius);
            }

            let scouts = self
                .actors
                .iter()
                .enumerate()
                .filter_map(|(index, actor)| {
                    let spawn = &self.map.spawns[actor.spawn];
                    (actor.health > 0.0
                        && spawn.room == current
                        && spawn.definition == "bone_scout")
                        .then_some(index)
                })
                .collect::<Vec<_>>();
            for (rank, index) in scouts.into_iter().enumerate() {
                archer_sides[index] = if rank % 2 == 0 { -1.0 } else { 1.0 };
            }
        }
        let crowd_positions = self
            .actors
            .iter()
            .filter_map(|actor| {
                let spawn = &self.map.spawns[actor.spawn];
                (actor.health > 0.0
                    && spawn.room == current
                    && matches!(
                        spawn.kind,
                        SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss | SpawnKind::Boss
                    ))
                .then_some((actor.spawn, actor.position))
            })
            .collect::<Vec<_>>();
        let mut damage = 0.0;
        let mut new_fx = Vec::new();
        let mut new_sounds: Vec<sound::Sound> = Vec::new();
        // Scenery big blows break, champions that roared, and screen shake.
        let mut shatters: Vec<(usize, Vec3, f32)> = Vec::new();
        let mut roars: Vec<(SpawnKind, usize)> = Vec::new();
        let mut shake: f32 = 0.0;
        for (actor_index, actor) in self.actors.iter_mut().enumerate() {
            let spawn = &self.map.spawns[actor.spawn];
            actor.hit_reaction = (actor.hit_reaction - dt).max(0.0);
            if spawn.kind == SpawnKind::Chest && actor.chest_opening {
                actor.chest_open_progress =
                    (actor.chest_open_progress + dt / CHEST_OPEN_SECONDS).min(1.0);
                actor.chest_opening = actor.chest_open_progress < 1.0;
            }
            // In the goo everything it does goes at half pace.
            let pace = if actor.slowed > 0.0 { SLOWED_PACE } else { 1.0 };
            actor.slowed = (actor.slowed - dt).max(0.0);
            if actor.burning > 0.0 && actor.health > 0.0 {
                let before = (actor.burning * 2.5).floor();
                actor.burning = (actor.burning - dt).max(0.0);
                // The fire eats at it but leaves the last blow to her.
                let floor = spawn.health * 0.04;
                if actor.health > floor {
                    actor.health = (actor.health
                        - spawn.health * BURN_SHARE * dt / combat::toughness(spawn.kind))
                    .max(floor);
                }
                if (actor.burning * 2.5).floor() != before {
                    let size = authored::foe_size(self.authored.as_deref(), &spawn);
                    self.fx.push(combat::Fx::Burst {
                        at: actor.position + Vec3::Y * (0.9 * size),
                        color: [1.0, 0.52, 0.16],
                        count: 6,
                        speed: 1.1,
                        life: 0.55,
                        gravity: -2.5,
                        up: 0.9,
                        length: 0.05,
                        seed: (actor.spawn as u32).wrapping_mul(2654435761)
                            ^ (actor.burning * 1000.0) as u32,
                        age: 0.0,
                    });
                }
            }
            actor.cooldown = (actor.cooldown - dt * pace).max(0.0);
            actor.stagger = (actor.stagger - dt).max(0.0);
            actor.stand = (actor.stand - dt).max(0.0);
            if spawn.kind == SpawnKind::Trap
                && !actor.used
                && current == spawn.room
                && horizontal(actor.position - player).length() < 0.8
                && actor.cooldown <= 0.0
            {
                damage += spawn.damage;
                actor.cooldown = 1.2;
            }
            let foe = matches!(
                spawn.kind,
                SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss | SpawnKind::Boss
            );
            if foe && actor.health <= 0.0 {
                actor.death = (actor.death + dt).min(60.0);
                continue;
            }
            if actor.health <= 0.0 || !foe {
                continue;
            }
            // Walking bones rattle (softly, and only near the Queen).
            if matches!(actor.gait, combat::Gait::Walk | combat::Gait::Run)
                && horizontal(actor.position - player).length() < 10.0
            {
                let rate = if actor.gait == combat::Gait::Run {
                    3.2
                } else {
                    2.0
                };
                let phase = spawn.id as f32 * 0.37;
                if (self.elapsed * rate + phase).floor()
                    != ((self.elapsed - dt) * rate + phase).floor()
                {
                    new_sounds.push(sound::Sound {
                        sfx: sound::Sfx::BoneRattle,
                        at: Some(actor.position),
                        volume: 0.35,
                    });
                }
            }
            if actor.stagger > 0.0 {
                actor.gait = combat::Gait::Guard;
                continue;
            }
            let size = authored::foe_size(self.authored.as_deref(), &spawn);
            let moves = combat::moves(&spawn.definition, spawn.kind, spawn.id);
            let chest = player + Vec3::Y * 1.15;
            // Half-dead, a champion roars and fights enraged from then on.
            if combat::champion(spawn.kind)
                && !actor.enraged
                && actor.attack.is_none()
                && actor.health < spawn.health * 0.5
            {
                if let Some(index) = combat::roar_index(moves) {
                    actor.enraged = true;
                    actor.attack = Some(combat::Swing {
                        index,
                        t: 0.0,
                        struck: false,
                        cast_point: player,
                        struck_again: false,
                        origin: actor.position,
                    });
                    actor.path.clear();
                }
            }
            // A blow in progress plays out (web `advanceAttack`): the wind-up
            // tracks the player, the swing itself is committed.
            if let Some(mut swing) = actor.attack {
                let mv = moves[swing.index.min(moves.len() - 1)];
                let speed = combat::attack_speed(spawn.kind)
                    * if actor.enraged {
                        combat::ENRAGED_SPEED
                    } else {
                        1.0
                    };
                swing.t += dt * speed * pace;
                if swing.t < mv.strike - 0.15 && mv.kind != combat::MoveKind::Whirl {
                    combat::turn_toward(
                        &mut actor.yaw,
                        player - actor.position,
                        dt,
                        if mv.ranged { 6.0 } else { 4.5 },
                    );
                }
                match mv.kind {
                    // Through the air from take-off to the landing mark.
                    combat::MoveKind::Leap
                        if swing.t > combat::LEAP_TAKEOFF && swing.t < mv.strike =>
                    {
                        let u = ((swing.t - combat::LEAP_TAKEOFF)
                            / (mv.strike - combat::LEAP_TAKEOFF))
                            .clamp(0.0, 1.0);
                        let eased = u * u * (3.0 - 2.0 * u);
                        let mut p = swing.origin.lerp(swing.cast_point, eased);
                        // It lands on its own level, inside its room.
                        let inside = self.map.rooms[spawn.room].contains(p.x, p.z, 0.5);
                        if let Some(y) = self
                            .map
                            .capsule_floor(p, 0.4)
                            .filter(|y| inside && (y - swing.origin.y).abs() < 0.3)
                        {
                            p.y = y;
                            actor.position = p;
                        }
                    }
                    // Spinning, it drifts after the player.
                    combat::MoveKind::Whirl if swing.t > 0.5 && swing.t < 1.45 => {
                        actor.yaw += 13.0 * dt;
                        let p = actor.position
                            + horizontal(player - actor.position).normalize_or_zero() * (1.3 * dt);
                        if let Some(p) = foe_step(&self.map, p) {
                            actor.position = p;
                        }
                    }
                    _ => {}
                }
                let mut lands = Vec::new();
                if !swing.struck && swing.t >= mv.strike {
                    swing.struck = true;
                    lands.push(());
                }
                if mv
                    .repeat
                    .is_some_and(|r| !swing.struck_again && swing.t >= r)
                {
                    swing.struck_again = true;
                    lands.push(());
                }
                for () in lands {
                    let amount = if spawn.damage > 0.0 {
                        spawn.damage
                    } else if mv.ranged {
                        8.0
                    } else {
                        10.0
                    } * mv.damage;
                    let tip = mv.tip_at(actor.position, actor.yaw, size);
                    let seed =
                        (spawn.id as u32).wrapping_mul(7919) ^ (self.elapsed * 1000.0) as u32;
                    let floor = actor.position.y;
                    let level = (player.y - actor.position.y).abs() < 1.4;
                    match mv.kind {
                        combat::MoveKind::Shoot => {
                            // A bolt can be dodged: it hurts only if the player
                            // is still where it aimed when it arrives.
                            new_fx.push(combat::Fx::Bolt {
                                from: tip,
                                to: chest,
                                age: 0.0,
                                arrived: false,
                                damage: amount,
                            });
                            new_fx.push(combat::Fx::Burst {
                                at: tip,
                                color: [0.8, 0.72, 0.6],
                                count: 6,
                                speed: 1.2,
                                life: 0.22,
                                gravity: 1.0,
                                up: 0.6,
                                length: 0.09,
                                seed,
                                age: 0.0,
                            });
                        }
                        combat::MoveKind::Cast => {
                            // The orb leaves the staff now and erupts at the
                            // mark when it lands: step off it to dodge.
                            new_fx.push(combat::Fx::Spell {
                                from: tip,
                                to: swing.cast_point,
                                age: 0.0,
                                damage: amount,
                                radius: combat::CAST_RADIUS,
                                color: combat::MAGE_VIOLET,
                            });
                        }
                        combat::MoveKind::Barrage => {
                            for (k, to) in combat::barrage_points(swing.origin, swing.cast_point)
                                .into_iter()
                                .enumerate()
                            {
                                // Staggered a little so they read as three.
                                let lag = k as f32 * 0.12;
                                new_fx.push(combat::Fx::Spell {
                                    from: tip,
                                    to,
                                    age: -lag,
                                    damage: amount,
                                    radius: 1.3,
                                    color: combat::BOSS_CRIMSON,
                                });
                            }
                        }
                        combat::MoveKind::Roar => {
                            new_fx.push(combat::Fx::Shockwave {
                                at: actor.position,
                                radius: mv.reach * size,
                                age: 0.0,
                            });
                            new_fx.push(combat::Fx::Burst {
                                at: actor.position + Vec3::Y * (1.6 * size),
                                color: [1.0, 0.3, 0.12],
                                count: 26,
                                speed: 4.5,
                                life: 0.7,
                                gravity: 1.0,
                                up: 0.3,
                                length: 0.2,
                                seed,
                                age: 0.0,
                            });
                            shake = shake.max(0.7);
                            roars.push((spawn.kind, spawn.room));
                            new_sounds.push(sound::Sound {
                                sfx: sound::Sfx::Roar,
                                at: Some(actor.position),
                                volume: 1.0,
                            });
                        }
                        combat::MoveKind::Nova => {
                            let radius = mv.reach * size;
                            let hit =
                                level && horizontal(player - actor.position).length() < radius;
                            if hit {
                                damage += amount;
                            }
                            new_fx.push(combat::Fx::Shockwave {
                                at: actor.position,
                                radius,
                                age: 0.0,
                            });
                            for k in 0..6u32 {
                                let a = k as f32 / 6.0 * std::f32::consts::TAU;
                                let at = actor.position
                                    + Vec3::new(a.cos(), 0.05, a.sin()) * (radius * 0.55);
                                new_fx.push(combat::Fx::dust(at, seed ^ k));
                            }
                            new_fx.push(combat::Fx::Burst {
                                at: tip,
                                color: combat::BOSS_CRIMSON,
                                count: 30,
                                speed: 6.0,
                                life: 0.6,
                                gravity: 2.0,
                                up: 0.9,
                                length: 0.2,
                                seed,
                                age: 0.0,
                            });
                            shatters.push((spawn.room, actor.position, radius));
                            shake = shake.max(1.0);
                        }
                        combat::MoveKind::Melee
                        | combat::MoveKind::Leap
                        | combat::MoveKind::Whirl => {
                            new_sounds.push(sound::Sound {
                                sfx: sound::Sfx::EnemySwing,
                                at: Some(actor.position),
                                volume: 0.7,
                            });
                            let toward = horizontal(player - actor.position);
                            let within = toward.length() <= mv.reach * size;
                            let in_arc = combat::wrap(combat::yaw_to(toward) - actor.yaw).abs()
                                <= mv.arc.to_radians() / 2.0;
                            let hit = within && in_arc && level;
                            if hit {
                                damage += amount;
                                new_fx.push(combat::Fx::burst(
                                    chest.lerp(tip, 0.4),
                                    [1.0, 0.78, 0.45],
                                    12,
                                    3.0,
                                    seed,
                                ));
                            }
                            // Heavy blows that reach the floor raise dust (and
                            // shockwaves that break what stands near).
                            if tip.y - floor < 0.55 {
                                new_fx.push(combat::Fx::dust(
                                    Vec3::new(tip.x, floor + 0.05, tip.z),
                                    seed ^ 0x5bd1,
                                ));
                            }
                            if let Some(shock) = mv.shockwave {
                                let ground = Vec3::new(tip.x, floor, tip.z);
                                let radius = shock * size;
                                new_fx.push(combat::Fx::Shockwave {
                                    at: ground,
                                    radius,
                                    age: 0.0,
                                });
                                if !hit && level && horizontal(ground - player).length() < radius {
                                    damage += amount * 0.6;
                                }
                                if combat::champion(spawn.kind) {
                                    shatters.push((spawn.room, ground, radius));
                                    shake = shake.max(0.6);
                                }
                            }
                        }
                    }
                }
                actor.attack = (swing.t < mv.duration).then_some(swing);
                actor.path.clear();
                actor.nav_timer = 0.0;
                continue;
            }
            let delta = horizontal(player - actor.position);
            let distance = delta.length();
            // Out of its room, the Queen is only followed while she is close,
            // in sight and on its level (into the corridor, not the dungeon). The mine's
            // places are lines drawn across its tunnels, not doors: there a foe follows her
            // anywhere near its post while it sees her or she is close.
            let followed = if self.authored.is_some() {
                let sees = self
                    .map
                    .line_of_sight(actor.position + Vec3::Y, player + Vec3::Y);
                distance < 16.0
                    && (sees || distance < 6.0)
                    && (actor.position.y - player.y).abs() < 2.5
                    && horizontal(actor.position - spawn.position).length() < FOE_LEASH
            } else {
                distance < 7.0
                    && (actor.position.y - player.y).abs() < 1.5
                    && self
                        .map
                        .line_of_sight(actor.position + Vec3::Y, player + Vec3::Y)
            };
            if current != spawn.room && !followed {
                actor.gait = combat::Gait::Idle;
                actor.path.clear();
                continue;
            }
            let aware = distance < 22.0 && (actor.position.y - player.y).abs() < 2.7;
            if !aware {
                actor.path.clear();
                actor.gait = combat::Gait::Idle;
                continue;
            }
            let style = enemy_style(&spawn.definition, spawn.kind);
            let visible = self
                .map
                .line_of_sight(actor.position + Vec3::Y, player + Vec3::Y);
            // Face the player at a limited rate; blows start only once aimed.
            let mut facing = actor.yaw;
            let facing_error = combat::turn_toward(&mut facing, delta, dt, 7.5);
            let player_forward = self.controller.forward();
            // Chased too long, a ranged foe stops fleeing and fights: it
            // shoots from where it stands, even at close range.
            let cornered = actor.stand > 0.0;
            let ranged_stand = |cooldown: f32| {
                if visible && cooldown <= 0.0 && facing_error < 0.35 {
                    EnemyIntent::Attack
                } else {
                    EnemyIntent::Hold
                }
            };
            let intent = match style {
                EnemyStyle::Ranged | EnemyStyle::Mage if cornered => ranged_stand(actor.cooldown),
                EnemyStyle::Ranged => {
                    let behind = archer_is_behind(player, player_forward, actor.position);
                    let opening_flank = actor.ambush_opening
                        && archer_is_on_flank(player, player_forward, actor.position);
                    let safe_fallback_lane = actor.reposition_elapsed >= 2.2;
                    match archer_intent_under_pressure(
                        distance,
                        visible,
                        behind || opening_flank || safe_fallback_lane,
                        actor.cooldown,
                        actor.reposition_elapsed,
                    ) {
                        EnemyIntent::Attack if facing_error >= 0.35 => EnemyIntent::Hold,
                        intent => intent,
                    }
                }
                EnemyStyle::Mage => {
                    if distance < 5.5 {
                        EnemyIntent::Retreat
                    } else if distance > 10.5 || !visible {
                        EnemyIntent::Approach
                    } else if distance < 13.5 && actor.cooldown <= 0.0 && facing_error < 0.35 {
                        EnemyIntent::Attack
                    } else {
                        EnemyIntent::Hold
                    }
                }
                EnemyStyle::Quick | EnemyStyle::Heavy => {
                    let cycle = combat::cycle_len(moves);
                    let mut next = moves[actor.attack_index % cycle];
                    // A leap needs room to jump: too close, skip to the next.
                    if next.kind == combat::MoveKind::Leap && distance < 2.6 {
                        actor.attack_index = (actor.attack_index + 1) % cycle;
                        next = moves[actor.attack_index % cycle];
                    }
                    let engage = match next.kind {
                        combat::MoveKind::Leap => 7.5,
                        combat::MoveKind::Barrage => 12.0,
                        combat::MoveKind::Nova => next.reach * size * 0.6,
                        _ => next.reach * size * 0.82,
                    };
                    if distance > engage || !visible {
                        EnemyIntent::Approach
                    } else if distance <= engage + 0.15
                        && actor.cooldown <= 0.0
                        && facing_error < 0.5
                    {
                        EnemyIntent::Attack
                    } else {
                        EnemyIntent::Hold
                    }
                }
            };
            if matches!(style, EnemyStyle::Ranged | EnemyStyle::Mage) {
                let fleeing = !cornered
                    && (intent == EnemyIntent::Retreat
                        || (intent == EnemyIntent::Approach && distance < ARCHER_MIN_RANGE + 0.8));
                if fleeing {
                    actor.flee += dt;
                    if actor.flee > RANGED_FLEE_LIMIT {
                        actor.flee = 0.0;
                        actor.stand = RANGED_STAND_SECONDS;
                    }
                } else if distance > ARCHER_MIN_RANGE + 2.0 {
                    actor.flee = (actor.flee - dt * 0.5).max(0.0);
                }
            }
            match intent {
                EnemyIntent::Attack => {
                    // Web `startAttack`: the next blow of the cycle.
                    actor.yaw = facing;
                    let cycle = combat::cycle_len(moves);
                    let index = actor.attack_index % cycle;
                    actor.attack_index = (index + 1) % cycle;
                    let mv = moves[index];
                    let speed = combat::attack_speed(spawn.kind)
                        * if actor.enraged {
                            combat::ENRAGED_SPEED
                        } else {
                            1.0
                        };
                    // A leap lands just short of the player, in front of them.
                    let target = if mv.kind == combat::MoveKind::Leap {
                        player - horizontal(player - actor.position).normalize_or_zero() * 1.1
                    } else {
                        player
                    };
                    actor.attack = Some(combat::Swing {
                        index,
                        t: 0.0,
                        struck: false,
                        cast_point: target,
                        struck_again: false,
                        origin: actor.position,
                    });
                    actor.attack_target = player;
                    actor.cooldown = combat::cooldown_after(&mv, spawn.kind, spawn.id)
                        * if actor.enraged {
                            1.0 / combat::ENRAGED_SPEED
                        } else {
                            1.0
                        };
                    let tip = mv.tip_at(actor.position, actor.yaw, size);
                    match mv.kind {
                        combat::MoveKind::Cast => {
                            // The mark stays until the orb lands on it.
                            let life =
                                mv.strike / speed + combat::Fx::spell_travel(tip, player) + 0.05;
                            new_fx.push(combat::Fx::Telegraph {
                                at: player,
                                age: 0.0,
                                life,
                                radius: combat::CAST_RADIUS,
                            });
                        }
                        combat::MoveKind::Barrage => {
                            for (k, to) in combat::barrage_points(actor.position, player)
                                .into_iter()
                                .enumerate()
                            {
                                let life = mv.strike / speed
                                    + combat::Fx::spell_travel(tip, to)
                                    + 0.05
                                    + k as f32 * 0.12;
                                new_fx.push(combat::Fx::Telegraph {
                                    at: to,
                                    age: 0.0,
                                    life,
                                    radius: 1.3,
                                });
                            }
                        }
                        combat::MoveKind::Nova => {
                            let life = mv.strike / speed;
                            new_fx.push(combat::Fx::Telegraph {
                                at: actor.position,
                                age: 0.0,
                                life,
                                radius: mv.reach * size,
                            });
                        }
                        combat::MoveKind::Leap => {
                            let life = mv.strike / speed;
                            new_fx.push(combat::Fx::Telegraph {
                                at: target,
                                age: 0.0,
                                life,
                                radius: 2.2 * size,
                            });
                        }
                        _ => {}
                    }
                    if style == EnemyStyle::Ranged {
                        actor.ambush_opening = false;
                        // Once the scout has a clear firing lane, keep that
                        // stance through reloads.
                        actor.reposition_elapsed = actor.reposition_elapsed.max(2.2);
                    }
                    actor.path.clear();
                    actor.nav_timer = 0.0;
                    actor.gait = combat::Gait::Guard;
                }
                EnemyIntent::Hold => {
                    // Guard stance: keep spacing, never walk through the player.
                    actor.yaw = facing;
                    actor.path.clear();
                    actor.nav_timer = 0.0;
                    actor.gait = combat::Gait::Guard;
                }
                EnemyIntent::Retreat => {
                    // A cornered caster backs away at a run.
                    actor.yaw = facing;
                    actor.path.clear();
                    let away = -delta.normalize_or_zero();
                    let p = actor.position + away * (spawn.speed * 1.05 * dt * pace);
                    if let Some(p) = foe_step(&self.map, p) {
                        actor.position = p;
                    }
                    actor.gait = combat::Gait::Run;
                }
                EnemyIntent::Approach => {
                    if style == EnemyStyle::Ranged {
                        actor.reposition_elapsed += dt;
                    }
                    actor.nav_timer -= dt;
                    if actor.nav_timer <= 0.0 {
                        let path_target = if style == EnemyStyle::Ranged {
                            archer_standoff_position(
                                &self.map,
                                spawn.room,
                                player,
                                player_forward,
                                archer_sides[actor_index],
                                actor.position,
                            )
                            .unwrap_or(actor.position)
                        } else if room_role == Role::Encounter {
                            encounter_targets[actor_index].unwrap_or(player)
                        } else {
                            player
                        };
                        if horizontal(path_target - actor.position).length() < 0.32 {
                            actor.path.clear();
                            actor.nav_timer = 0.18;
                            actor.yaw = facing;
                            actor.gait = combat::Gait::Guard;
                            continue;
                        }
                        let refresh = match style {
                            EnemyStyle::Quick => 0.18,
                            EnemyStyle::Ranged => 0.42,
                            _ => 0.24,
                        };
                        // over a hand-made level the floor map knows every barrel and cart: a
                        // walk round them (straight at her when none is found)
                        actor.path = match &self.map.floors {
                            Some(_) => self.map.walk_path(
                                actor.position,
                                path_target,
                                0.4,
                                FOE_PATH_REACH.max(distance + 6.0),
                            ),
                            None => navigation::room_path(
                                &self.map,
                                spawn.room,
                                actor.position,
                                path_target,
                                0.44,
                            ),
                        };
                        actor.nav_timer = refresh;
                    }
                    while actor
                        .path
                        .first()
                        .is_some_and(|p| horizontal(*p - actor.position).length() < 0.35)
                    {
                        actor.path.remove(0);
                    }
                    let goal = actor.path.first().copied().unwrap_or_else(|| {
                        if style == EnemyStyle::Ranged {
                            archer_standoff_position(
                                &self.map,
                                spawn.room,
                                player,
                                player_forward,
                                archer_sides[actor_index],
                                actor.position,
                            )
                            .unwrap_or(actor.position)
                        } else if room_role == Role::Encounter {
                            encounter_targets[actor_index].unwrap_or(player)
                        } else {
                            player
                        }
                    });
                    let travel = horizontal(goal - actor.position).normalize_or_zero();
                    // Web pacing: melee runs from afar and closes at a walk;
                    // casters and scouts walk into range.
                    let (stride, gait) = match style {
                        EnemyStyle::Quick | EnemyStyle::Heavy if distance > 5.0 => {
                            (1.0, combat::Gait::Run)
                        }
                        EnemyStyle::Quick | EnemyStyle::Heavy => (0.72, combat::Gait::Walk),
                        EnemyStyle::Ranged if distance < 5.5 => (1.05, combat::Gait::Run),
                        _ => (0.7, combat::Gait::Walk),
                    };
                    actor.gait = gait;
                    let pace = spawn.speed * stride * pace;
                    let mut steering = travel;
                    // Keep a little personal space so melee rigs don't
                    // occupy one another's capsule and jitter at the player.
                    for (other_spawn, other_position) in &crowd_positions {
                        if *other_spawn == actor.spawn {
                            continue;
                        }
                        let away = horizontal(actor.position - *other_position);
                        let separation = away.length();
                        if separation > 0.001 && separation < 1.05 {
                            steering += away.normalize_or_zero() * ((1.05 - separation) * 1.35);
                        }
                    }
                    let travel = steering.normalize_or_zero();
                    let goal_distance = horizontal(goal - actor.position).length();
                    let step = (dt * pace).min(goal_distance);
                    let mut p = actor.position + travel * step;
                    if style == EnemyStyle::Ranged
                        && horizontal(p - player).length() < ARCHER_MIN_RANGE
                    {
                        let mut outward = horizontal(actor.position - player).normalize_or_zero();
                        if outward.length_squared() < 0.001 {
                            outward = -horizontal(player_forward).normalize_or_zero();
                        }
                        p = actor.position + outward * (dt * pace).max(0.05);
                    }
                    let start = actor.position;
                    // against something: it slides along it rather than stopping dead
                    let d = p - start;
                    if let Some(p) = foe_step(&self.map, p)
                        .or_else(|| foe_step(&self.map, start + Vec3::new(d.x, 0.0, 0.0)))
                        .or_else(|| foe_step(&self.map, start + Vec3::new(0.0, 0.0, d.z)))
                    {
                        actor.position = p;
                    }
                    // Walking into a wall or a corner: find another way, step
                    // aside, and a shooter stands and fights from here.
                    if horizontal(actor.position - start).length() < dt * pace * 0.25 {
                        actor.stuck += dt;
                        // held up, it stands ready instead of walking on the spot
                        actor.gait = combat::Gait::Guard;
                    } else {
                        actor.stuck = 0.0;
                    }
                    if actor.stuck > 1.0 {
                        actor.stuck = 0.0;
                        actor.path.clear();
                        actor.nav_timer = 0.0;
                        if matches!(style, EnemyStyle::Ranged | EnemyStyle::Mage) && visible {
                            actor.stand = RANGED_STAND_SECONDS;
                        }
                        let turn = if spawn.id % 2 == 0 { 1.0 } else { -1.0 };
                        let aside =
                            actor.position + Vec3::new(-travel.z, 0.0, travel.x) * (0.45 * turn);
                        if let Some(aside) = foe_step(&self.map, aside) {
                            actor.position = aside;
                        }
                    }
                    // Face where it walks; with the player in sight and
                    // near, keep facing them instead of moonwalking.
                    let heading = if visible && distance < 5.0 {
                        delta
                    } else {
                        horizontal(goal - actor.position)
                    };
                    combat::turn_toward(&mut actor.yaw, heading, dt, 7.5);
                }
            }
        }
        self.shake = self.shake.max(shake);
        for (room, center, radius) in shatters {
            self.shatter(room, center, radius, &mut new_fx);
        }
        for (kind, room) in roars {
            if kind == SpawnKind::Boss {
                // The Heart answers its guardian's cry.
                self.heart_summon(room, 3, &mut new_fx);
                self.heart.rain_left = self.heart.rain_left.max(8.0);
                self.message("¡El guardián ruge! El Corazón alza siervos y llueve sangre.");
            } else {
                self.message("¡El campeón ruge enfurecido!");
            }
        }
        self.heart_tick(current, dt, &mut new_fx);
        self.champions_tick(current, dt, &mut new_fx);
        self.voices_tick();
        let taken = pickups::update(&mut self.pickups, &self.map, self.controller.position, dt);
        if !taken.is_empty() {
            self.take_loot(&taken);
        }
        // Bolts and orbs land (or miss) and effects fade.
        let mut bursts = Vec::new();
        let mut fx_sounds = Vec::new();
        self.fx.retain_mut(|effect| {
            let (hurt, alive, left) = effect.tick(dt, player);
            if hurt > 0.0 {
                damage += hurt;
                if let combat::Fx::Bolt { to, .. } = effect {
                    bursts.push(combat::Fx::burst(
                        *to,
                        [1.0, 0.55, 0.35],
                        10,
                        2.4,
                        to.x.to_bits() ^ to.z.to_bits(),
                    ));
                    fx_sounds.push(sound::Sound {
                        sfx: sound::Sfx::BoltImpact,
                        at: Some(*to),
                        volume: 0.9,
                    });
                }
            }
            if let combat::Fx::Spell { to, color, .. } = effect {
                if !alive {
                    fx_sounds.push(sound::Sound {
                        sfx: sound::Sfx::SpellBurst,
                        at: Some(*to),
                        volume: 0.9,
                    });
                    bursts.push(combat::Fx::Burst {
                        at: *to + Vec3::Y * 0.3,
                        color: *color,
                        count: 22,
                        speed: 4.2,
                        life: 0.55,
                        gravity: 3.0,
                        up: 0.8,
                        length: 0.14,
                        seed: to.x.to_bits() ^ 0x51ab,
                        age: 0.0,
                    });
                }
            }
            bursts.extend(left);
            alive
        });
        self.fx.extend(bursts);
        self.sounds.extend(fx_sounds);
        // Effects that just began make their sounds.
        for effect in &new_fx {
            let sound = match *effect {
                combat::Fx::Shockwave { at, radius, .. } => {
                    Some((sound::Sfx::Shockwave, at, (0.5 + radius * 0.12).min(1.0)))
                }
                combat::Fx::Telegraph { at, .. } => Some((sound::Sfx::SpellCharge, at, 0.6)),
                combat::Fx::Summon { at, .. } => Some((sound::Sfx::Summon, at, 0.9)),
                combat::Fx::Debris { at, .. } => Some((sound::Sfx::Rubble, at, 0.8)),
                combat::Fx::Bolt { from, .. } => Some((sound::Sfx::CrossbowShot, from, 0.85)),
                _ => None,
            };
            if let Some((sfx, at, volume)) = sound {
                self.sounds.push(sound::Sound {
                    sfx,
                    at: Some(at),
                    volume,
                });
            }
        }
        self.sounds.extend(new_sounds);
        self.fx.extend(new_fx);
        if let Some(slime) = &mut self.slime {
            damage += slime.tick(&self.map, player, current, dt);
        }
        for slime in &mut self.groves {
            if slime.room == current || slime.hop.is_some() || !slime.particles.is_empty() {
                damage += slime.tick(&self.map, player, current, dt);
            }
        }
        let hops: Vec<Vec3> = self
            .slime
            .iter()
            .chain(&self.groves)
            .filter(|s| s.landed && s.alive() && horizontal(s.position - player).length() < 12.0)
            .map(|s| s.position)
            .collect();
        for at in hops {
            self.sound(sound::Sfx::SlimeHop, Some(at), 0.55);
        }
        self.resolve_slime_contact(before);
        if damage > 0.0 {
            self.health -= damage;
            self.hit_flash = 0.25;
            self.sound(sound::Sfx::PlayerHurt, None, (0.5 + damage / 20.0).min(1.0));
        }
        if self.health <= 0.0 {
            self.sound(sound::Sfx::Respawn, None, 0.8);
            self.health = 100.0;
            self.gold /= 2;
            for count in self.materials.values_mut() {
                *count -= *count / 2;
            }
            self.materials.retain(|_, count| *count > 0);
            self.controller = Controller::new(self.checkpoint);
            self.message(
                "Has caido. Regresas al santuario y pierdes la mitad del oro y de los materiales de esta expedicion.",
            );
        }
    }
    fn start_sword_swing(&mut self, combo: usize) {
        self.sword_from = if self.sword_swing > 0.0 {
            let clip = fp::SWINGS[self.sword_combo];
            fp::pose(
                self.sword_combo,
                clip.duration - self.sword_swing,
                self.sword_from,
            )
        } else {
            fp::REST
        };
        self.sword_combo = combo;
        self.sword_swing = fp::SWINGS[combo].duration;
        self.sword_queued = false;
        self.sword_hit_pending = true;
        self.sound(
            sound::Sfx::Swing,
            None,
            if combo == 2 { 0.55 } else { 0.42 },
        );
    }
    pub fn swing_sword(&mut self) {
        if self.seed_input.is_some() || self.map_open || self.stall.open {
            return;
        }
        // in the shaft the pick: held down, it keeps digging
        if let Some(pozo) = self.pozo.as_mut() {
            pozo.holding = true;
            return;
        }
        if self.sword_swing > 0.0 {
            let clip = fp::SWINGS[self.sword_combo];
            let elapsed = clip.duration - self.sword_swing;
            if self.sword_combo < 2 && elapsed > clip.strike && elapsed < clip.chain {
                self.sword_queued = true;
            }
            return;
        }
        if self.sword_cooldown <= 0.0 {
            self.start_sword_swing(0);
        }
    }

    /// Share of a foe's full health the current blow takes.
    fn swing_share(&self) -> f32 {
        SWORD_DAMAGE_SHARE
            * if self.sword_combo == 2 {
                SWORD_FINISHER
            } else {
                1.0
            }
            * if self.relic_sword {
                RELIC_SWORD_BONUS
            } else {
                1.0
            }
            * if self.armed { 1.0 } else { 0.5 }
            * self.skill_bonus.max(0.1)
    }

    fn resolve_sword_hit(&mut self) {
        let share = self.swing_share();
        let eye = self.controller.eye();
        let forward = horizontal(self.controller.forward()).normalize_or_zero();
        let origin = self.controller.position;
        let target = self
            .actors
            .iter()
            .enumerate()
            .filter_map(|(index, actor)| {
                if actor.health <= 0.0 {
                    return None;
                }
                let spawn = &self.map.spawns[actor.spawn];
                let offset = horizontal(actor.position - origin);
                let along = offset.dot(forward);
                let lateral = (offset - forward * along).length();
                let radius = match spawn.kind {
                    SpawnKind::Boss => 0.95,
                    SpawnKind::Miniboss | SpawnKind::Elite => 0.68,
                    _ => 0.48,
                };
                let target_point = actor.position
                    + Vec3::Y
                        * if spawn.kind == SpawnKind::Boss {
                            1.55
                        } else {
                            0.88
                        };
                (along > 0.0
                    && along <= SWORD_REACH + radius
                    && lateral <= radius + along * 0.20
                    && self.map.line_of_sight(eye, target_point))
                .then_some((along + lateral * 0.65, index))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let slime_target = self
            .slime
            .iter()
            .map(|slime| (0, slime))
            .chain(
                self.groves
                    .iter()
                    .enumerate()
                    .map(|(k, slime)| (k + 1, slime)),
            )
            .filter(|(_, slime)| slime.alive())
            .filter_map(|(index, slime)| {
                let offset = horizontal(slime.position - origin);
                let along = offset.dot(forward);
                let lateral = (offset - forward * along).length();
                (along > 0.0
                    && along <= SWORD_REACH + 0.52
                    && lateral <= 0.48 + along * 0.20
                    && self.map.line_of_sight(eye, slime.position + Vec3::Y * 0.34))
                .then_some((along + lateral * 0.65, index))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((score, index)) =
            slime_target.filter(|(score, _)| target.is_none_or(|(other, _)| *score <= other))
        {
            let _ = score;
            // Disjoint borrows: the slime and the map it lands on.
            let slime = match index {
                0 => self.slime.as_mut(),
                k => self.groves.get_mut(k - 1),
            };
            let mut struck = None;
            let killed = slime.and_then(|slime| {
                slime.hit(&self.map, slime::MAX_HEALTH * share, forward);
                struck = Some(slime.position);
                (!slime.alive()).then_some((slime.tint, slime.home))
            });
            if let Some(at) = struck {
                self.sound(sound::Sfx::HitSlime, Some(at), 0.8);
            }
            if killed.is_some() {
                self.sound(sound::Sfx::SlimeDeath, struck, 0.9);
            }
            self.sword_hit_flash = 0.16;
            self.sword_hit_stop = 0.065;
            if let Some((tint, home)) = killed {
                let jelly = match tint {
                    slime::Tint::Blue => "gelatina_azul",
                    slime::Tint::Green => "gelatina_verde",
                };
                let loot = self.gather(&[(jelly, 1 + home.x.to_bits() % 2)]);
                self.message(format!("El slime se dispersa. {loot}."));
            }
            return;
        }
        let target = target.map(|(_, index)| index);
        let Some(index) = target else {
            return;
        };

        let spawn = self.map.spawns[self.actors[index].spawn].clone();
        let stage_before = self.actors[index].fracture_stage(spawn.health);
        let at = self.actors[index].position;
        let actor = &mut self.actors[index];
        actor.health =
            (actor.health - spawn.health * share / combat::toughness(spawn.kind)).max(0.0);
        let stage_after = actor.fracture_stage(spawn.health);
        let dead = actor.health <= 0.0;
        let size = authored::foe_size(self.authored.as_deref(), &spawn);
        let chest_at = at + Vec3::Y * (1.1 * size);
        let seed = (spawn.id as u32).wrapping_mul(2654435761) ^ (self.elapsed * 997.0) as u32;
        let bone = [0.92, 0.86, 0.7];
        if dead {
            // It falls apart: a shower of bone and a clattering collapse.
            self.sound(sound::Sfx::SkeletonDeath, Some(at), 1.0);
            self.fx.push(combat::Fx::Burst {
                at: chest_at,
                color: bone,
                count: 26,
                speed: 3.4,
                life: 0.8,
                gravity: 7.0,
                up: 0.7,
                length: 0.12,
                seed,
                age: 0.0,
            });
        } else if stage_after > stage_before {
            // A new crack: a sharp snap and chips of bone.
            self.sound(sound::Sfx::BoneCrack, Some(at), 0.9);
            self.fx.push(combat::Fx::Burst {
                at: chest_at,
                color: bone,
                count: 14,
                speed: 2.6,
                life: 0.55,
                gravity: 7.0,
                up: 0.6,
                length: 0.1,
                seed,
                age: 0.0,
            });
        } else {
            self.sound(sound::Sfx::HitBone, Some(at), 0.8);
        }
        // What the sword carries: goo, embers, a sip of life.
        let fx = self.weapon_fx;
        if fx.slow > 0.0 && !dead {
            self.actors[index].slowed = fx.slow;
            self.fx.push(combat::Fx::Burst {
                at: chest_at,
                color: [0.45, 0.95, 0.42],
                count: 12,
                speed: 1.6,
                life: 0.7,
                gravity: 6.0,
                up: 0.3,
                length: 0.06,
                seed: seed ^ 0x51ee,
                age: 0.0,
            });
        }
        if fx.burn > 0.0 && !dead {
            self.actors[index].burning = fx.burn;
            self.fx.push(combat::Fx::Burst {
                at: chest_at,
                color: [1.0, 0.55, 0.15],
                count: 14,
                speed: 2.2,
                life: 0.6,
                gravity: -3.0,
                up: 1.0,
                length: 0.07,
                seed: seed ^ 0xf17e,
                age: 0.0,
            });
        }
        if fx.leech > 0.0 && self.health > 0.0 {
            self.health = (self.health + fx.leech).min(100.0);
        }
        let actor = &mut self.actors[index];
        actor.damage_hits = actor.damage_hits.saturating_add(1).min(3);
        actor.hit_reaction = 0.42;
        // A hit staggers and cancels the blow, except a big foe mid-swing.
        let heavy = matches!(spawn.kind, SpawnKind::Boss | SpawnKind::Miniboss);
        if !heavy || actor.attack.is_none() {
            actor.attack = None;
            actor.stagger = if heavy { 0.22 } else { 0.38 };
        }
        actor.path.clear();
        actor.nav_timer = 0.0;
        self.sword_hit_flash = 0.16;
        self.sword_hit_stop = 0.065;
        if actor.health <= 0.0 {
            // What it leaves bursts out of it onto the floor: its materials, gold, the odd tonic
            // and herbs, a champion's own reward. What the Heart raised crumbles to nothing, but
            // still counts for a streak.
            let summoned = index >= self.base_actors;
            let at = self.actors[index].position;
            self.streak = if self.elapsed - self.streak_at < 6.0 {
                self.streak + 1
            } else {
                1
            };
            self.streak_at = self.elapsed;
            let mut loot: Vec<pickups::Loot> = Vec::new();
            if !summoned {
                loot.extend(
                    skeleton_drops(self.map.seed, &spawn)
                        .into_iter()
                        .map(|(id, n)| pickups::Loot::Material(id, n)),
                );
                loot.extend(pickups::extra_loot(self.map.seed, spawn.id, spawn.kind));
            }
            let champion = self
                .authored
                .as_ref()
                .and_then(|mine| mine.champions.get(&spawn.id))
                .cloned();
            if let Some(c) = &champion {
                loot.extend(pickups::coins(c.reward.gold));
                loot.extend((0..c.reward.potions).map(|_| pickups::Loot::Tonic));
                if c.reward.herbs > 0 {
                    loot.push(pickups::Loot::Herbs(c.reward.herbs));
                }
                for (id, n) in &c.reward.materials {
                    if let Some((known, _)) = MATERIALS.iter().find(|(m, _)| *m == id.as_str()) {
                        loot.push(pickups::Loot::Material(known, *n));
                    }
                }
            }
            if self.streak >= 3 {
                loot.extend(pickups::coins(self.streak * 3));
            }
            let seed = self.map.seed
                ^ (spawn.id as u64).rotate_left(17)
                ^ u64::from(self.elapsed.to_bits());
            self.pickups.extend(pickups::burst(at, &loot, seed));
            if spawn.kind == SpawnKind::Miniboss {
                if self.map.rooms[spawn.room].role == Role::Miniboss {
                    self.crest = true;
                    self.message("Custodio derrotado. Obtienes la llave del Corazon.");
                } else if let Some(c) = &champion {
                    self.message(format!("{} ha caído. Recoge su botín.", c.name));
                } else {
                    self.message("Campeon de los Acechadores derrotado.");
                }
            } else if spawn.kind == SpawnKind::Boss {
                self.won = true;
                self.message("El Corazon ha caido. La salida esta abierta.");
            } else if self.streak >= 3 {
                self.message(format!("¡Racha ×{}! Botín extra.", self.streak));
            } else if summoned {
                self.message("El siervo del Corazón se deshace en ceniza.");
            }
        }
    }
    /// Something from the backpack drunk or eaten: `amount` of vitality and a line.
    pub fn cure(&mut self, amount: f32, line: impl Into<String>) {
        self.health = (self.health + amount).min(100.0);
        self.sound(sound::Sfx::Drink, None, 0.7);
        self.message(line);
    }

    /// Whether the expedition still has its own tonics or herbs.
    pub fn has_supplies(&self) -> bool {
        self.potions > 0 || self.herbs > 0
    }

    pub fn heal(&mut self) {
        if self.health >= 100.0 {
            return;
        }
        if self.potions > 0 {
            self.potions -= 1;
            self.unbanked_potions = self.unbanked_potions.saturating_sub(1);
            self.health = (self.health + 45.0).min(100.0);
            self.sound(sound::Sfx::Drink, None, 0.7);
            self.message("Tonico de expedicion: +45 vitalidad.");
        } else if self.herbs > 0 {
            self.herbs -= 1;
            self.unbanked_herbs = self.unbanked_herbs.saturating_sub(1);
            self.health = (self.health + 20.0).min(100.0);
            self.sound(sound::Sfx::Pickup, None, 0.5);
            self.message("Hierbas curativas: +20 vitalidad.");
        }
    }
    pub fn nearby_actor(&self) -> Option<usize> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                a.health <= 0.0
                    && !a.used
                    && self.map.spawns[a.spawn].health == 0.0
                    && self.map.spawns[a.spawn].kind != SpawnKind::Trap
            })
            .filter(|(_, a)| {
                (a.position - self.controller.position).length() < 2.7
                    && self
                        .map
                        .line_of_sight(self.controller.eye(), a.position + Vec3::Y)
            })
            .min_by(|(_, a), (_, b)| {
                (a.position - self.controller.position)
                    .length_squared()
                    .total_cmp(&(b.position - self.controller.position).length_squared())
            })
            .map(|(i, _)| i)
    }
    /// Someone of the mine near enough to talk to, and in front of her.
    pub fn npc_near(&self) -> Option<&authored::Npc> {
        let mine = self.authored.as_ref()?;
        let eye = self.controller.position;
        let forward = Vec3::new(-self.controller.yaw.sin(), 0.0, -self.controller.yaw.cos());
        mine.npcs.iter().find(|npc| {
            let d = horizontal(npc.position - eye);
            d.length() < 2.6
                && (npc.position.y - eye.y).abs() < 1.6
                && d.normalize_or_zero().dot(forward) > 0.55
        })
    }
    /// What F does here, as plain text (logs, tests).
    pub fn prompt(&self) -> String {
        self.interaction().map(|p| p.text()).unwrap_or_default()
    }

    /// What F does here: its icon, verb and what it acts on.
    pub fn interaction(&self) -> Option<prompt_icons::Prompt> {
        use prompt_icons::{Icon, Prompt};
        if self.pozo.is_some() {
            return pozo::Pozo::at_cage(self.controller.position).then(|| {
                Prompt::new(Icon::Leave, "Salir del pozo").detail("Volver a la boca de la mina")
            });
        }
        if let Some(npc) = self.npc_near() {
            if npc.id == "tendero" {
                return Some(
                    Prompt::new(Icon::Shop, "Comerciar").detail("Picos, mochilas y minerales"),
                );
            }
            return Some(Prompt::new(Icon::Talk, "Hablar").detail(npc.name.clone()));
        }
        if self.at_shaft() {
            return Some(
                Prompt::new(Icon::Mine, "Bajar al Pozo del Minero")
                    .detail("Pica hacia abajo: cuanto más hondo, más vale"),
            );
        }
        if let Some(mine) = &self.authored {
            if !(self.seal && self.crest)
                && mine
                    .gates
                    .iter()
                    .any(|g| g.holds(self.controller.position, PLAYER_RADIUS + 1.6))
            {
                return Some(
                    Prompt::new(Icon::Lock, "Umbral sellado")
                        .detail(match (self.seal, self.crest) {
                            (false, false) => "Exige el sello y la llave del Custodio",
                            (true, false) => "Falta la llave del Custodio",
                            _ => "Falta despertar el sello",
                        })
                        .locked(),
                );
            }
        }
        for c in &self.map.connections {
            if c.kind == ConnectionKind::Secret
                && !self.opened.contains(&c.id)
                && (c.start - self.controller.position).length() < 3.0
            {
                return Some(Prompt::new(Icon::Crack, "Inspeccionar").detail("Pared agrietada"));
            }
            if self.map.rooms[c.b].role == Role::Boss
                && !(self.seal && self.crest)
                && (c.end - self.controller.position).length() < 3.2
            {
                return Some(
                    Prompt::new(Icon::Lock, "Umbral sellado")
                        .detail("Exige el sello y la llave del Custodio")
                        .locked(),
                );
            }
        }
        let i = self.nearby_actor()?;
        let s = &self.map.spawns[self.actors[i].spawn];
        Some(match s.kind {
            SpawnKind::Entrance => {
                Prompt::new(Icon::Leave, "Regresar a la superficie").detail("Salida de la mina")
            }
            SpawnKind::Exit => {
                if self.won {
                    Prompt::new(Icon::Leave, "Completar la expedición")
                        .detail("Regresar a la superficie")
                } else {
                    Prompt::new(Icon::Lock, "Salida sellada")
                        .detail("El Corazón la mantiene cerrada")
                        .locked()
                }
            }
            SpawnKind::Chest => {
                let chest = &self.actors[i];
                if chest.chest_opening {
                    Prompt::new(Icon::Chest, "El relicario se está abriendo…").locked()
                } else if chest.chest_open_progress >= 1.0 {
                    Prompt::new(Icon::Loot, "Recoger").detail("Tónico, hierbas y oro")
                } else {
                    Prompt::new(Icon::Chest, "Abrir el relicario")
                }
            }
            SpawnKind::Seal => {
                let index = s
                    .definition
                    .strip_prefix("sigil_")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
                    .min(2);
                Prompt::new(
                    Icon::Rune,
                    format!("Sintonizar la runa {}", runes::stone_name(index)),
                )
                .detail(format!(
                    "Fase actual: {}",
                    runes::phase_name(self.sigils[index])
                ))
            }
            SpawnKind::Rest => Prompt::new(Icon::Rest, "Descansar").detail("Fijar el santuario"),
            SpawnKind::Memory => Prompt::new(Icon::Memory, "Escuchar la memoria"),
            _ => return None,
        })
    }
    pub fn interact(&mut self) {
        if self.seed_input.is_some() || self.map_open {
            return;
        }
        if self.pozo.is_some() {
            if pozo::Pozo::at_cage(self.controller.position) {
                self.leave_pozo();
            }
            return;
        }
        if self.npc_near().is_some_and(|npc| npc.id == "tendero") {
            self.open_stall();
            return;
        }
        if self.at_shaft() {
            self.enter_pozo();
            return;
        }
        if let Some(npc) = self.npc_near() {
            if !npc.lines.is_empty() {
                let line = npc.lines[self.npc_talks % npc.lines.len()].clone();
                let name = npc.name.clone();
                self.npc_talks += 1;
                self.message(format!("{name}: «{line}»"));
            }
            return;
        }
        if let Some(c) = self.map.connections.iter().find(|c| {
            c.kind == ConnectionKind::Secret
                && !self.opened.contains(&c.id)
                && (c.start - self.controller.position).length() < 3.0
        }) {
            let id = c.id;
            self.opened.insert(id);
            self.message("Una corriente de aire revela la camara oculta.");
            return;
        }
        let Some(i) = self.nearby_actor() else {
            return;
        };
        let s = self.map.spawns[self.actors[i].spawn].clone();
        match s.kind {
            SpawnKind::Entrance => self.pending_exit = true,
            SpawnKind::Exit => {
                if self.won {
                    self.pending_exit = true;
                }
            }
            SpawnKind::Chest => {
                let herbs = chest_herb_count(self.seed(), s.id);
                let loot = self
                    .authored
                    .as_ref()
                    .and_then(|mine| mine.loot.get(&s.id).cloned());
                if let Some(loot) = loot {
                    self.open_authored_chest(i, loot);
                    return;
                }
                let chest = &mut self.actors[i];
                if chest.chest_open_progress >= 1.0 {
                    chest.used = true;
                    let at = chest.position;
                    self.sound(sound::Sfx::Coins, Some(at), 0.9);
                    self.gold += s.gold;
                    self.potions += 1;
                    self.herbs += herbs;
                    self.unbanked_potions += 1;
                    self.unbanked_herbs += herbs;
                    let treasury = self.map.rooms[s.room].role == Role::Treasure;
                    if treasury && !self.relic_sword && chest_holds_relic_sword(self.seed(), s.id) {
                        self.sound(sound::Sfx::Relic, None, 0.9);
                        self.relic_sword = true;
                        self.unbanked_relic_sword = true;
                        // It goes to her hand at once.
                        self.armed = true;
                        self.sword_goo = None;
                        self.message(format!(
                            "¡Una espada de piedra entre el oro! La empuñas. +1 tónico, +{} hierbas y +{} oro.",
                            herbs, s.gold
                        ));
                        return;
                    }
                    self.message(format!(
                        "Dentro: +1 tónico, +{} hierbas y +{} oro.",
                        herbs, s.gold
                    ));
                } else if !chest.chest_opening {
                    chest.chest_opening = true;
                    let at = chest.position;
                    self.sound(sound::Sfx::ChestCreak, Some(at), 0.9);
                    self.message("El relicario cede con un crujido; algo brilla dentro.");
                }
            }
            SpawnKind::Seal => {
                let index = s
                    .definition
                    .strip_prefix("sigil_")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
                    .min(2);
                if !self.seal {
                    self.rune_start_angle[index] = self.rune_angle(index);
                    self.rune_target_angle[index] += std::f32::consts::FRAC_PI_2;
                    self.sigils[index] = (self.sigils[index] + 1) % 4;
                    self.rune_changed_at[index] = self.elapsed;
                    self.seal = self.sigils == [1, 2, 3];
                    let at = self.actors[i].position;
                    self.sound(sound::Sfx::RuneChime, Some(at), 0.8);
                    if self.seal {
                        self.sound(sound::Sfx::SealOpen, None, 1.0);
                    }
                }
                if self.seal {
                    self.message("Alba, cénit y ocaso se alinean. El sello ha despertado.");
                } else {
                    self.message(format!(
                        "Runa {}: {}. El sol nace, asciende y descansa.",
                        runes::stone_name(index),
                        runes::phase_name(self.sigils[index])
                    ));
                }
            }
            SpawnKind::Rest => {
                self.health = 100.0;
                // in the hand-made mine, right where the sanctuary's font stands
                self.checkpoint = if self.authored.is_some() {
                    s.position
                } else {
                    self.map.rooms[s.room].center
                };
                self.actors[i].used = true;
                self.potions = self.potions.max(2);
                self.message("Santuario activado. Vitalidad restaurada.");
            }
            SpawnKind::Memory => {
                self.message(
                    "La piedra susurra: de izquierda a derecha, ALBA, CÉNIT y OCASO. Haz girar las espirales.",
                );
            }
            _ => {}
        }
    }
    /// A chest of the hand-made mine: open it, then take what it holds.
    fn open_authored_chest(&mut self, i: usize, loot: authored::Loot) {
        let chest = &mut self.actors[i];
        if chest.chest_opening && chest.chest_open_progress < 1.0 {
            return;
        }
        if chest.chest_open_progress < 1.0 {
            chest.chest_opening = true;
            let at = chest.position;
            self.sound(sound::Sfx::ChestCreak, Some(at), 0.9);
            self.message(if loot.relic || loot.gold >= 60 {
                "La tapa cede despacio: el oro brilla dentro."
            } else {
                "El cofre cede con un crujido; algo brilla dentro."
            });
            return;
        }
        chest.used = true;
        let at = chest.position;
        self.sound(sound::Sfx::Coins, Some(at), 0.9);
        self.gold += loot.gold;
        self.potions += loot.potions;
        self.unbanked_potions += loot.potions;
        self.herbs += loot.herbs;
        self.unbanked_herbs += loot.herbs;
        let mut parts = Vec::new();
        if loot.potions > 0 {
            parts.push(if loot.potions == 1 {
                "+1 tónico".to_string()
            } else {
                format!("+{} tónicos", loot.potions)
            });
        }
        if loot.herbs > 0 {
            parts.push(format!("+{} hierbas", loot.herbs));
        }
        parts.push(format!("+{} oro", loot.gold));
        let drops: Vec<(&str, u32)> = loot
            .materials
            .iter()
            .map(|(id, n)| (id.as_str(), *n))
            .collect();
        let gathered = self.gather(&drops);
        if !gathered.is_empty() {
            parts.push(gathered);
        }
        let list = parts.join(", ");
        if loot.relic && !self.relic_sword {
            self.sound(sound::Sfx::Relic, None, 0.9);
            self.relic_sword = true;
            self.unbanked_relic_sword = true;
            self.armed = true;
            self.sword_goo = None;
            self.message(format!(
                "¡Una espada de piedra entre el oro! La empuñas. {list}."
            ));
        } else {
            self.message(format!("Dentro: {list}."));
        }
    }
    pub fn regenerate(&mut self, text: &str) -> Result<(), String> {
        let seed = if let Some(hex) = text.trim().strip_prefix("0x") {
            u64::from_str_radix(hex, 16)
        } else {
            text.trim().parse::<u64>()
        }
        .map_err(|_| "Seed invalida: usa un entero o 0x hexadecimal.".to_string())?;
        let replacement = Self::try_generate(seed, self.map.config.clone())?;
        *self = replacement;
        Ok(())
    }
    pub fn text_input(&mut self, text: &str) {
        if let Some(input) = &mut self.seed_input {
            for c in text.chars().filter(|c| c.is_ascii_hexdigit() || *c == 'x') {
                if input.len() < 20 {
                    input.push(c);
                }
            }
        }
    }
    pub fn key(
        &mut self,
        code: winit::keyboard::KeyCode,
        pressed: bool,
        repeat: bool,
        text: Option<&str>,
    ) {
        use winit::keyboard::KeyCode::*;
        if !pressed {
            return;
        }
        if let Some(input) = &mut self.seed_input {
            match code {
                Escape => self.seed_input = None,
                Enter => {
                    let value = input.clone();
                    if let Err(e) = self.regenerate(&value) {
                        self.message(e);
                    }
                }
                Backspace => {
                    input.pop();
                }
                _ => {
                    if let Some(text) = text {
                        for c in text.chars().filter(|c| c.is_ascii_hexdigit() || *c == 'x') {
                            if input.len() < 20 {
                                input.push(c);
                            }
                        }
                    }
                }
            }
            return;
        }
        if self.map_open {
            match code {
                KeyE => return self.map_zoom_by(1.0),
                KeyQ => return self.map_zoom_by(-1.0),
                KeyR if !repeat => return self.reset_map_view(),
                _ => {}
            }
        }
        if repeat {
            return;
        }
        if self.stall.open {
            self.stall_key(code);
            return;
        }
        if self.pozo.is_some() {
            match code {
                KeyF => self.interact(),
                KeyU => {
                    if let Some(pozo) = self.pozo.as_mut() {
                        pozo.haul();
                    }
                }
                KeyT => self.torch = !self.torch,
                F3 => self.debug = !self.debug,
                Space => self.jump_requested = true,
                _ => {}
            }
            return;
        }
        match code {
            KeyF => self.interact(),
            KeyG => self.pending_exit = true,
            KeyQ => self.heal(),
            KeyT => self.torch = !self.torch,
            KeyR => self.auto_walk = !self.auto_walk,
            KeyM => {
                self.map_open = !self.map_open;
                self.auto_walk = false;
            }
            F3 => self.debug = !self.debug,
            F2 if self.authored.is_none() => {
                self.seed_input = Some(String::new());
                self.auto_walk = false;
            }
            Space => self.jump_requested = true,
            Escape => {
                self.map_open = false;
                self.debug = false;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod combat_tests {
    use super::*;

    #[test]
    fn jumping_slime_never_pushes_through_the_player_camera() {
        let mut run = Dungeon::generate(42);
        let slime = run.slime.as_ref().expect("entrance slime");
        let center = run.map.rooms[run.map.entrance].center;
        let toward = horizontal(center - slime.position).normalize_or_zero();
        let at = slime.position + toward * 1.55;
        run.controller.position = Vec3::new(
            at.x,
            run.map.rooms[run.map.entrance].floor_y(at.x, at.z),
            at.z,
        );
        for tick in 0..240 {
            run.update(Input::default(), 0.02);
            let slime = run.slime.as_ref().unwrap();
            let distance = horizontal(run.controller.position - slime.position).length();
            assert!(
                distance >= slime::CONTACT_DISTANCE - 0.03,
                "camera overlaps slime on tick {tick}: {distance:.3}m ({:?})",
                slime.mode
            );
            assert!(run
                .map
                .capsule_floor(run.controller.position, PLAYER_RADIUS)
                .is_some());
        }
    }

    fn face_enemy(run: &mut Dungeon, enemy_index: usize, behind_player: bool) {
        let enemy = run.actors[enemy_index].position;
        run.controller.position = if behind_player {
            enemy - Vec3::new(0.0, 0.0, 1.45)
        } else {
            enemy + Vec3::new(0.0, 0.0, 1.45)
        };
        run.controller.yaw = 0.0;
        run.controller.pitch = 0.0;
        for (index, actor) in run.actors.iter_mut().enumerate() {
            if index != enemy_index {
                actor.health = 0.0;
            }
        }
    }

    fn stage_crossbow_test(seed: u64) -> (Dungeon, usize) {
        let mut run = Dungeon::generate(seed);
        let index = run
            .actors
            .iter()
            .position(|actor| run.map.spawns[actor.spawn].definition == "bone_scout")
            .expect("seeded encounters include a crossbow scout");
        let room = run.map.spawns[run.actors[index].spawn].room;
        run.controller.position = run.map.rooms[room].center;
        run.controller.velocity = Vec3::ZERO;
        run.controller.grounded = true;
        run.controller.yaw = 0.0;
        for (other, actor) in run.actors.iter_mut().enumerate() {
            if other != index {
                actor.health = 0.0;
            }
        }
        run.actors[index].position = archer_rear_position(
            &run.map,
            room,
            run.controller.position,
            run.controller.forward(),
            -1.0,
        )
        .expect("encounter room has a reachable rear firing lane");
        run.visited.insert(room);
        (run, index)
    }

    fn bolt_in_flight(run: &Dungeon) -> bool {
        run.fx
            .iter()
            .any(|effect| matches!(effect, combat::Fx::Bolt { arrived: false, .. }))
    }

    #[test]
    fn ballistae_only_commit_to_shots_from_a_visible_rear_lane() {
        let player = Vec3::ZERO;
        let forward = Vec3::Z;
        let side_lane = Vec3::new(6.0, 0.0, 0.0);
        let front_lane = Vec3::new(0.0, 0.0, 6.0);
        assert!(archer_is_on_flank(player, forward, side_lane));
        assert!(!archer_is_behind(player, forward, side_lane));
        assert!(!archer_is_on_flank(player, forward, front_lane));
        assert!(archer_is_behind(player, forward, Vec3::new(0.0, 0.0, -6.0)));
        assert_eq!(
            archer_intent(6.0, true, false, 0.0),
            EnemyIntent::Approach,
            "a clear shot from the front is not the ambush plan"
        );
        assert_eq!(
            archer_intent(6.0, false, true, 0.0),
            EnemyIntent::Approach,
            "archers keep moving until they have line of sight"
        );
        assert_eq!(archer_intent(6.0, true, true, 0.0), EnemyIntent::Attack);
        assert_eq!(archer_intent(6.0, true, true, 0.4), EnemyIntent::Hold);
        assert_eq!(
            archer_intent(2.0, true, true, 0.0),
            EnemyIntent::Approach,
            "a scout creates space instead of standing in melee range"
        );
    }

    #[test]
    fn stalker_minions_get_separate_guard_and_surround_slots() {
        let mut tested = false;
        for seed in 0..32 {
            let map = generate(
                seed,
                &Config::default(),
                &Catalog::bundled(),
                &Difficulty::default(),
            )
            .expect("stress seed generates a valid dungeon");
            let Some(room) = map.rooms.iter().find(|room| room.role == Role::Encounter) else {
                continue;
            };
            let guard = (0..4)
                .filter_map(|rank| {
                    encounter_ring_position(
                        &map,
                        room.id,
                        room.center,
                        rank,
                        4,
                        STALKER_BOSS_GUARD_RADIUS,
                    )
                })
                .collect::<Vec<_>>();
            let surround = (0..4)
                .filter_map(|rank| {
                    encounter_ring_position(
                        &map,
                        room.id,
                        room.center,
                        rank,
                        4,
                        STALKER_SURROUND_RADIUS,
                    )
                })
                .collect::<Vec<_>>();
            if guard.len() < 3 || surround.len() < 3 {
                continue;
            }
            for positions in [&guard, &surround] {
                for (index, position) in positions.iter().enumerate() {
                    for other in &positions[index + 1..] {
                        assert!(horizontal(*position - *other).length() > 0.7);
                    }
                }
            }
            assert!(
                horizontal(guard[0] - room.center).length()
                    > horizontal(surround[0] - room.center).length()
            );
            tested = true;
            break;
        }
        assert!(
            tested,
            "seeded encounter should fit a coordinated formation"
        );
    }

    #[test]
    fn first_stalker_volley_is_staggered_instead_of_firing_as_one_blob() {
        let mut run = Dungeon::generate(42);
        let room = run
            .map
            .rooms
            .iter()
            .find(|room| room.role == Role::Encounter)
            .expect("seed has an encounter room")
            .id;
        run.controller.position = run.map.rooms[room].center;
        run.controller.velocity = Vec3::ZERO;
        run.controller.grounded = true;
        run.update(Input::default(), 0.05);
        let mut cooldowns = run
            .actors
            .iter()
            .filter(|actor| {
                let spawn = &run.map.spawns[actor.spawn];
                actor.health > 0.0 && spawn.room == room && spawn.definition == "bone_scout"
            })
            .map(|actor| actor.cooldown)
            .collect::<Vec<_>>();
        cooldowns.sort_by(f32::total_cmp);
        assert_eq!(cooldowns.len(), 2);
        assert!(cooldowns[0] > 0.0 && cooldowns[1] > cooldowns[0] + 0.1);
        assert!(run.actors.iter().any(|actor| {
            let spawn = &run.map.spawns[actor.spawn];
            spawn.room == room && spawn.definition == "bone_scout" && actor.ambush_opening
        }));
    }

    /// A lone quick melee skeleton `offset` from the player, ready to strike.
    fn stage_melee(seed: u64, offset: Vec3) -> (Dungeon, usize) {
        let mut run = Dungeon::generate(seed);
        let enemy = run
            .actors
            .iter()
            .position(|actor| {
                let spawn = &run.map.spawns[actor.spawn];
                actor.health > 0.0
                    && spawn.kind == SpawnKind::Enemy
                    && enemy_style(&spawn.definition, spawn.kind) == EnemyStyle::Quick
            })
            .expect("seed has a quick melee enemy");
        let room = run.map.spawns[run.actors[enemy].spawn].room;
        for (index, actor) in run.actors.iter_mut().enumerate() {
            if index != enemy {
                actor.health = 0.0;
            }
        }
        let player = run.map.rooms[room].center;
        run.controller.position = player;
        run.controller.velocity = Vec3::ZERO;
        run.controller.grounded = true;
        run.visited.insert(room);
        let mut close = player + offset;
        close.y = run
            .map
            .capsule_floor(close, 0.4)
            .expect("walkable test point");
        run.actors[enemy].position = close;
        run.actors[enemy].yaw = combat::yaw_to(player - close);
        run.actors[enemy].cooldown = 0.0;
        (run, enemy)
    }

    #[test]
    fn close_melee_enemy_stops_in_range_instead_of_jittering_through_player() {
        let (mut run, enemy) = stage_melee(63, Vec3::Z * 0.92);
        let start = run.actors[enemy].position;
        let mut attacked = false;
        for _ in 0..80 {
            run.update(Input::default(), 0.05);
            attacked |= run.actors[enemy].attack.is_some();
        }
        assert!(attacked, "enemy should complete weapon attack cycles");
        assert!(
            horizontal(run.actors[enemy].position - start).length() < 0.08,
            "an enemy already inside attack reach should plant its feet, not orbit or overshoot"
        );
    }

    #[test]
    fn a_blade_skeleton_alternates_its_two_slashes_and_lands_on_the_strike() {
        let (mut run, enemy) = stage_melee(63, Vec3::Z * 1.2);
        let health = run.health;
        let spawn = run.map.spawns[run.actors[enemy].spawn].clone();
        let moves = combat::moves(&spawn.definition, spawn.kind, spawn.id);
        assert_eq!(
            moves.iter().map(|m| m.name).collect::<Vec<_>>(),
            ["Slash_A", "Slash_B"]
        );
        let mut seen = vec![];
        let mut struck_at = None;
        for step in 0..160 {
            run.update(Input::default(), 0.025);
            if let Some(swing) = run.actors[enemy].attack {
                if seen.last() != Some(&swing.index) || swing.t < 0.03 {
                    if seen.last() != Some(&swing.index) {
                        seen.push(swing.index);
                    }
                }
                if struck_at.is_none() && run.health < health {
                    struck_at = Some((step, swing.t, moves[swing.index].strike));
                }
            }
        }
        assert!(
            seen.len() >= 2 && seen[0] != seen[1],
            "attacks cycle: {seen:?}"
        );
        let (_, t, strike) = struck_at.expect("a player in reach and arc takes the blow");
        assert!(
            t >= strike && t < strike + 0.05,
            "damage lands on the strike frame"
        );
    }

    #[test]
    fn a_blow_misses_a_player_outside_its_arc() {
        let (mut run, enemy) = stage_melee(63, Vec3::Z * 1.2);
        // Already mid-swing, facing away from the player.
        run.actors[enemy].yaw = combat::yaw_to(Vec3::Z);
        run.actors[enemy].attack = Some(combat::Swing {
            index: 0,
            t: combat::SLASH_A.strike - 0.1,
            struck: false,
            cast_point: Vec3::ZERO,
            struck_again: false,
            origin: Vec3::ZERO,
        });
        let health = run.health;
        for _ in 0..6 {
            run.update(Input::default(), 0.025);
        }
        assert_eq!(run.health, health, "the committed swing cuts empty air");
    }

    #[test]
    fn crossbow_scout_launches_a_real_delayed_shot() {
        let (mut run, shooter) = stage_crossbow_test(42);
        run.actors[shooter].ambush_opening = true;
        let initial_health = run.health;
        let mut launched = false;
        for _ in 0..80 {
            run.update(Input::default(), 0.05);
            if bolt_in_flight(&run) {
                launched = true;
                break;
            }
        }
        assert!(
            launched,
            "visible scout should launch after its telegraphed windup"
        );
        assert!(!run.actors[shooter].ambush_opening);
        assert_eq!(
            run.health, initial_health,
            "shot must travel before it can hurt"
        );
        for _ in 0..20 {
            run.update(Input::default(), 0.05);
            if run.health < initial_health {
                break;
            }
        }
        assert!(
            run.health < initial_health,
            "a stationary player should be hit by the bolt"
        );
    }

    #[test]
    fn crossbow_scout_holds_its_lane_during_reload_instead_of_rushing() {
        let (mut run, shooter) = stage_crossbow_test(44);
        run.actors[shooter].ambush_opening = true;
        for _ in 0..80 {
            run.update(Input::default(), 0.05);
            if bolt_in_flight(&run) {
                break;
            }
        }
        assert!(bolt_in_flight(&run), "expected first shot");
        let firing_position = run.actors[shooter].position;
        assert!(run.actors[shooter].reposition_elapsed >= 2.2);

        for _ in 0..20 {
            run.update(Input::default(), 0.05);
        }
        assert!(
            horizontal(run.actors[shooter].position - firing_position).length() < 0.06,
            "scout should hold its clear shooting lane while reloading"
        );
    }

    #[test]
    fn crossbow_reload_gives_the_melee_player_a_punish_window() {
        assert_eq!(archer_intent(2.0, true, true, 0.9), EnemyIntent::Hold);
        assert_eq!(archer_intent(2.0, true, true, 0.0), EnemyIntent::Approach);
        assert_eq!(archer_intent(7.0, false, false, 0.0), EnemyIntent::Approach);
        assert_eq!(
            archer_intent_under_pressure(2.0, true, true, 0.0, 1.0),
            EnemyIntent::Approach
        );
        assert_eq!(
            archer_intent_under_pressure(2.0, true, true, 0.0, 2.3),
            EnemyIntent::Attack
        );
    }

    #[test]
    fn crossbow_scout_shot_can_be_dodged_after_launch() {
        let (mut run, shooter) = stage_crossbow_test(43);
        run.actors[shooter].ambush_opening = true;
        for _ in 0..80 {
            run.update(Input::default(), 0.05);
            if bolt_in_flight(&run) {
                break;
            }
        }
        assert!(bolt_in_flight(&run), "expected a bolt in flight");
        run.controller.position.x += 2.0;
        let health = run.health;
        for _ in 0..18 {
            run.update(Input::default(), 0.05);
        }
        assert_eq!(
            run.health, health,
            "moving off the marked line should evade the bolt"
        );
    }

    #[test]
    fn crossbow_scout_keeps_standoff_and_fires_if_a_rear_lane_takes_too_long() {
        let (mut run, shooter) = stage_crossbow_test(43);
        let room = run.map.spawns[run.actors[shooter].spawn].room;
        let player = run.controller.position;
        let forward = run.controller.forward();
        let front = [6.0, 5.5, 5.0, 4.5]
            .into_iter()
            .find_map(|distance| {
                let mut point = player + horizontal(forward) * distance;
                if !run.map.rooms[room].contains(point.x, point.z, 0.45) {
                    return None;
                }
                point.y = run.map.capsule_floor(point, 0.38)?;
                Some(point)
            })
            .expect("encounter has space to stage an archer in front");
        run.actors[shooter].position = front;
        run.actors[shooter].ambush_opening = false;
        run.actors[shooter].cooldown = 0.0;

        let mut launched = false;
        let mut closest = f32::INFINITY;
        for _ in 0..160 {
            run.update(Input::default(), 0.05);
            let actor = &run.actors[shooter];
            let distance = horizontal(actor.position - run.controller.position).length();
            closest = closest.min(distance);
            assert!(
                distance >= ARCHER_MIN_RANGE - 0.16,
                "crossbow scout entered melee range: {distance:.2} m"
            );
            if actor.attack.is_some() {
                assert!(
                    run.map
                        .line_of_sight(actor.position + Vec3::Y, run.controller.position + Vec3::Y),
                    "a safe fallback attack still requires line of sight"
                );
            }
            if bolt_in_flight(&run) {
                launched = true;
                break;
            }
        }
        assert!(
            launched,
            "scout should reposition or take a safe shot within eight seconds"
        );
        assert!(closest >= ARCHER_MIN_RANGE - 0.16);
    }

    #[test]
    fn grave_mage_commits_a_target_then_damages_the_ground_mark() {
        let mut run = Dungeon::generate(42);
        let enemy = run
            .actors
            .iter()
            .position(|actor| {
                actor.health > 0.0 && run.map.spawns[actor.spawn].kind == SpawnKind::Enemy
            })
            .expect("seed has a regular enemy");
        let room_id = run.map.spawns[run.actors[enemy].spawn].room;
        let target = run.map.rooms[room_id].center;
        run.controller.position = target;
        run.visited.insert(room_id);
        for (index, actor) in run.actors.iter_mut().enumerate() {
            if index != enemy
                && matches!(
                    run.map.spawns[actor.spawn].kind,
                    SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss | SpawnKind::Boss
                )
            {
                actor.health = 0.0;
            }
        }
        run.map.spawns[run.actors[enemy].spawn].definition = "grave_mage".into();
        let offset = (0..16)
            .flat_map(|k| [6.0, 7.0, 8.5].map(move |r| (k, r)))
            .map(|(k, r)| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                target + Vec3::new(a.sin(), 0.0, a.cos()) * r
            })
            .find(|p| {
                run.map.rooms[room_id].contains(p.x, p.z, 0.5)
                    && run.map.capsule_floor(*p, 0.4).is_some()
                    && run.map.line_of_sight(*p + Vec3::Y, target + Vec3::Y)
            })
            .expect("room fits a caster at range");
        run.actors[enemy].position = offset;
        run.actors[enemy].position.y = run.map.capsule_floor(offset, 0.4).unwrap();
        run.actors[enemy].cooldown = 0.0;
        run.actors[enemy].attack_index = 0;
        let health = run.health;

        for _ in 0..20 {
            run.update(Input::default(), 0.05);
            if run.actors[enemy].attack.is_some() {
                break;
            }
        }
        assert!(
            run.actors[enemy].attack.is_some(),
            "mage begins a visible cast"
        );
        assert_eq!(run.actors[enemy].attack_target, target);
        assert!(run
            .fx
            .iter()
            .any(|fx| matches!(fx, combat::Fx::Telegraph { .. })));
        let mut orb = false;
        for _ in 0..40 {
            orb |= run
                .fx
                .iter()
                .any(|fx| matches!(fx, combat::Fx::Spell { .. }));
            run.update(Input::default(), 0.05);
        }
        assert!(orb, "the cast flies from the staff as a visible orb");
        assert!(run.health < health, "standing on the crack takes the blast");
        assert!(
            run.fx
                .iter()
                .any(|fx| matches!(fx, combat::Fx::Rupture { .. })),
            "impact leaves a brief rupture"
        );
    }

    #[test]
    fn sword_deals_damage_at_the_slash_impact_frame() {
        let mut run = Dungeon::generate(42);
        let enemy = run
            .actors
            .iter()
            .position(|actor| {
                actor.health > 0.0 && run.map.spawns[actor.spawn].kind == SpawnKind::Enemy
            })
            .expect("seed has a regular enemy");
        face_enemy(&mut run, enemy, false);
        let health = run.actors[enemy].health;

        run.swing_sword();
        assert!(run.sword_swing > 0.0);
        for _ in 0..4 {
            run.update(Input::default(), 0.05);
        }

        assert!((run.actors[enemy].health - health * (1.0 - SWORD_DAMAGE_SHARE)).abs() < 1e-3);
        assert_eq!(run.actors[enemy].damage_hits, 1);
        assert_eq!(run.actors[enemy].fracture_stage(health), 1);
        assert!(run.actors[enemy].hit_reaction > 0.0);
        assert!(run.sword_hit_flash > 0.0);
        assert!(
            run.sword_swing > 0.0,
            "the slash stays visible after impact"
        );
    }

    #[test]
    fn every_foe_takes_about_ten_plain_cuts_whatever_its_health() {
        for kind in [
            SpawnKind::Enemy,
            SpawnKind::Elite,
            SpawnKind::Miniboss,
            SpawnKind::Boss,
        ] {
            let Some((run, index)) = (0..40).find_map(|seed| {
                let run = Dungeon::generate(seed);
                let index = run
                    .actors
                    .iter()
                    .position(|a| a.health > 0.0 && run.map.spawns[a.spawn].kind == kind)?;
                Some((run, index))
            }) else {
                continue;
            };
            let full = run.map.spawns[run.actors[index].spawn].health;
            let cuts = (full / (full * SWORD_DAMAGE_SHARE)).ceil();
            assert!(
                (9.0..=11.0).contains(&cuts),
                "{kind:?} falls in {cuts} cuts"
            );
        }
    }

    #[test]
    fn some_treasury_chests_hold_the_relic_sword_and_it_lasts() {
        let mut found = 0;
        for seed in 0..60u64 {
            let map = Dungeon::generate(seed).map;
            for s in map
                .spawns
                .iter()
                .filter(|s| s.kind == SpawnKind::Chest && map.rooms[s.room].role == Role::Treasure)
            {
                found += usize::from(chest_holds_relic_sword(seed, s.id));
            }
        }
        assert!(
            found > 5,
            "the relic sword should turn up now and then ({found})"
        );
        let mut run = Dungeon::generate(3);
        run.relic_sword = true;
        run.unbanked_relic_sword = true;
        let restored = Dungeon::restore(&run.snapshot()).unwrap();
        assert!(restored.relic_sword && restored.unbanked_relic_sword);
        let mut restored = restored;
        assert!(restored.take_relic_sword());
        assert!(!restored.take_relic_sword());
    }

    #[test]
    fn sword_chain_commits_three_distinct_strikes_without_teleporting_pose() {
        let mut run = Dungeon::generate(42);
        run.swing_sword();
        assert_eq!(run.sword_combo, 0);
        run.update(Input::default(), 0.05);
        run.update(Input::default(), 0.05);
        run.update(Input::default(), 0.05);
        run.update(Input::default(), 0.05);
        run.swing_sword();
        run.update(Input::default(), 0.05);
        assert_eq!(run.sword_combo, 1);
        assert!(run.sword_from.hand.to_array().iter().all(|n| n.is_finite()));
        for _ in 0..4 {
            run.update(Input::default(), 0.05);
        }
        run.swing_sword();
        run.update(Input::default(), 0.05);
        assert_eq!(run.sword_combo, 2);
        assert!((run.swing_share() - SWORD_DAMAGE_SHARE * SWORD_FINISHER).abs() < 1e-6);
        for _ in 0..14 {
            run.update(Input::default(), 0.05);
        }
        assert_eq!(run.sword_swing, 0.0);
        run.swing_sword();
        assert_eq!(run.sword_combo, 0);
    }

    #[test]
    fn sword_does_not_hit_enemies_behind_the_player() {
        let mut run = Dungeon::generate(42);
        let enemy = run
            .actors
            .iter()
            .position(|actor| {
                actor.health > 0.0 && run.map.spawns[actor.spawn].kind == SpawnKind::Enemy
            })
            .expect("seed has a regular enemy");
        face_enemy(&mut run, enemy, true);
        let health = run.actors[enemy].health;

        run.swing_sword();
        for _ in 0..4 {
            run.update(Input::default(), 0.05);
        }

        assert_eq!(run.actors[enemy].health, health);
        assert_eq!(run.sword_hit_flash, 0.0);
    }
}

#[cfg(test)]
mod champion_tests {
    use super::*;

    /// A lone foe of `definition`/`kind` a few metres from the player.
    fn stage(seed: u64, definition: &str, kind: SpawnKind, offset: f32) -> (Dungeon, usize) {
        let mut run = Dungeon::generate(seed);
        let enemy = run
            .actors
            .iter()
            .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Enemy)
            .expect("seed has an enemy");
        let room = run.map.spawns[run.actors[enemy].spawn].room;
        for (index, actor) in run.actors.iter_mut().enumerate() {
            if index != enemy {
                actor.health = 0.0;
            }
        }
        let spawn = run.actors[enemy].spawn;
        run.map.spawns[spawn].definition = definition.into();
        run.map.spawns[spawn].kind = kind;
        let player = run.map.rooms[room].center;
        run.controller.position = player;
        run.controller.velocity = Vec3::ZERO;
        run.controller.grounded = true;
        run.visited.insert(room);
        let at = (0..16)
            .map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                player + Vec3::new(a.sin(), 0.0, a.cos()) * offset
            })
            .find(|p| {
                run.map.rooms[room].contains(p.x, p.z, 0.8)
                    && run
                        .map
                        .capsule_floor(*p, 0.4)
                        .is_some_and(|y| (y - player.y).abs() < 0.2)
                    && run.map.line_of_sight(*p + Vec3::Y, player + Vec3::Y)
            })
            .expect("room fits the foe");
        run.actors[enemy].position = at;
        run.actors[enemy].yaw = combat::yaw_to(player - at);
        run.actors[enemy].cooldown = 0.0;
        (run, enemy)
    }

    #[test]
    fn a_chased_caster_stops_fleeing_and_casts_where_it_stands() {
        let (mut run, mage) = stage(42, "grave_mage", SpawnKind::Enemy, 3.0);
        let mut stood = false;
        let mut cast = false;
        for _ in 0..200 {
            // The player keeps right on its heels.
            let away =
                horizontal(run.controller.position - run.actors[mage].position).normalize_or_zero();
            let follow = run.actors[mage].position + away * 3.0;
            if run.map.capsule_floor(follow, PLAYER_RADIUS).is_some() {
                run.controller.position.x = follow.x;
                run.controller.position.z = follow.z;
            }
            run.update(Input::default(), 0.05);
            stood |= run.actors[mage].stand > 0.0;
            cast |= stood && run.actors[mage].attack.is_some();
        }
        assert!(stood, "a caster chased long enough holds its ground");
        assert!(cast, "and fights from there instead of running forever");
    }

    #[test]
    fn a_champion_roars_at_half_health_and_fights_enraged() {
        let (mut run, boss) = stage(42, "sentinel", SpawnKind::Miniboss, 3.0);
        let spawn = run.actors[boss].spawn;
        run.actors[boss].health = run.map.spawns[spawn].health * 0.45;
        run.update(Input::default(), 0.05);
        let actor = &run.actors[boss];
        assert!(actor.enraged);
        let moves = combat::moves("sentinel", SpawnKind::Miniboss, run.map.spawns[spawn].id);
        assert_eq!(
            actor.attack.map(|s| moves[s.index].kind),
            Some(combat::MoveKind::Roar)
        );
        assert!(
            combat::cycle_len(moves) < moves.len(),
            "the roar is never part of the cycle"
        );
    }

    #[test]
    fn champions_take_several_times_the_cuts() {
        assert_eq!(combat::toughness(SpawnKind::Enemy), 1.0);
        assert!(combat::toughness(SpawnKind::Miniboss) >= 2.0);
        assert!(combat::toughness(SpawnKind::Boss) > combat::toughness(SpawnKind::Miniboss));
    }

    #[test]
    fn the_heart_summons_minions_and_rains_blood() {
        let mut run = Dungeon::generate(42);
        let boss = run
            .actors
            .iter()
            .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Boss)
            .expect("seed has the Heart's guardian");
        let room = run.map.spawns[run.actors[boss].spawn].room;
        run.controller.position = run.map.rooms[room].center;
        run.health = 100.0;
        let base = run.actors.len();
        let mut fx = Vec::new();
        for _ in 0..200 {
            run.heart_tick(room, 0.1, &mut fx);
        }
        assert!(run.actors.len() > base, "the Heart raised minions");
        assert!(run.actors[base..]
            .iter()
            .all(|a| run.map.spawns[a.spawn].room == room));
        assert!(fx.iter().any(|f| matches!(f, combat::Fx::Summon { .. })));
        assert!(
            run.health < 100.0 && run.health > 50.0,
            "the rain wears vitality down slowly: {}",
            run.health
        );
        // The guardian falls: its summons fall with it, and saves skip them.
        run.actors[boss].health = 0.0;
        run.heart_tick(room, 0.1, &mut fx);
        assert!(run.actors[base..].iter().all(|a| a.health <= 0.0));
        assert_eq!(run.snapshot().actors.len(), base);
    }

    #[test]
    fn big_blows_shatter_props_and_braziers_in_reach() {
        let mut run = Dungeon::generate(42);
        let (room, prop) = run
            .decor
            .iter()
            .enumerate()
            .find_map(|(room, decor)| {
                decor
                    .props
                    .iter()
                    .find(|p| !p.asset.is_empty())
                    .map(|p| (room, p.position))
            })
            .expect("some room has props");
        let before = run.decor[room].props.len();
        let mut fx = Vec::new();
        run.shatter(room, prop, 0.5, &mut fx);
        assert!(run.decor[room].props.len() < before);
        assert!(fx.iter().any(|f| matches!(f, combat::Fx::Debris { .. })));
    }

    #[test]
    fn blue_and_green_slimes_doze_by_trees_and_mushrooms() {
        let (mut total, mut blue, mut green) = (0, 0, 0);
        for seed in 0..12 {
            let run = Dungeon::generate(seed);
            for slime in &run.groves {
                assert!(slime.valid(&run.map));
                assert_ne!(slime.room, run.map.entrance);
                let near_tree = run.decor[slime.room].props.iter().any(|p| {
                    (p.asset.starts_with("trunk") || p.asset.contains("mushroom"))
                        && horizontal(p.position - slime.home).length() < p.radius.max(0.3) + 1.2
                });
                assert!(near_tree, "a grove slime sits by a tree or mushroom");
                total += 1;
                match slime.tint {
                    slime::Tint::Blue => blue += 1,
                    slime::Tint::Green => green += 1,
                }
            }
        }
        assert!(
            total > 0 && blue > 0 && green > 0,
            "blue {blue}, green {green}"
        );
    }

    #[test]
    fn fallen_skeletons_leave_bones_not_gold() {
        let (mut run, foe) = stage(42, "lurker", SpawnKind::Enemy, 1.4);
        let gold = run.gold;
        run.actors[foe].health = 0.01;
        run.controller.yaw = {
            let d = run.actors[foe].position - run.controller.position;
            (-d.x).atan2(-d.z)
        };
        run.controller.pitch = 0.0;
        run.swing_sword();
        for _ in 0..30 {
            run.update(Input::default(), 1.0 / 60.0);
        }
        assert!(run.actors[foe].health <= 0.0, "the cut lands");
        // what it leaves lies on the floor a moment, then comes to her
        for _ in 0..900 {
            run.update(Input::default(), 1.0 / 60.0);
        }
        assert!(run.pickups.is_empty(), "everything it left reached her");
        assert!(run.gold >= gold, "foes may pay a little gold now");
        let sounds = run.take_sounds();
        assert!(
            sounds.iter().any(|s| s.sfx == sound::Sfx::Swing),
            "the sword swishes"
        );
        assert!(
            sounds.iter().any(|s| s.sfx == sound::Sfx::SkeletonDeath),
            "the bones clatter down"
        );
        assert!(
            run.actors[foe].death > 0.0,
            "the fall is timed for its animation"
        );
        assert!(run.materials.get("hueso_viejo").copied().unwrap_or(0) >= 1);
        let taken = run.take_materials();
        assert!(taken.contains_key("hueso_viejo") && run.materials.is_empty());
    }

    #[test]
    fn champions_and_casters_leave_their_own_materials() {
        let run = Dungeon::generate(42);
        let mut spawn = run.map.spawns[0].clone();
        for (definition, kind, wanted) in [
            ("grave_mage", SpawnKind::Enemy, "ceniza_arcana"),
            ("sentinel", SpawnKind::Miniboss, "craneo_campeon"),
            ("lurker", SpawnKind::Boss, "esquirla_corazon"),
        ] {
            spawn.definition = definition.into();
            spawn.kind = kind;
            let drops = skeleton_drops(run.map.seed, &spawn);
            assert!(
                drops.iter().any(|(id, n)| *id == wanted && *n > 0),
                "{definition}: {drops:?}"
            );
            assert!(drops
                .iter()
                .all(|(id, _)| MATERIALS.iter().any(|(m, _)| m == id)));
        }
    }
}

#[cfg(test)]
mod stuck_probe {
    use super::*;

    /// Diagnostic: foes that neither move nor attack for seconds while the
    /// Queen waits in their room.
    #[test]
    #[ignore]
    fn find_stuck_foes() {
        let mut stuck = 0;
        let mut total = 0;
        for seed in 0..12u64 {
            let base = Dungeon::generate(seed);
            let rooms: Vec<usize> = base
                .map
                .rooms
                .iter()
                .filter(|r| {
                    base.actors.iter().any(|a| {
                        let s = &base.map.spawns[a.spawn];
                        s.room == r.id
                            && matches!(
                                s.kind,
                                SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss
                            )
                    })
                })
                .map(|r| r.id)
                .collect();
            for room in rooms {
                let mut run = Dungeon::generate(seed);
                run.controller.position = run.map.rooms[room].center;
                // Optionally wait just outside a door, in the corridor.
                if std::env::var_os("PROBE_CORRIDOR").is_some() {
                    let socket = &run.map.rooms[room].sockets[0];
                    let mut at = socket.position + socket.outward * 1.6;
                    if let Some(y) = run.map.capsule_floor(at, 0.3) {
                        at.y = y;
                        run.controller.position = at;
                    }
                }
                run.visited.insert(room);
                let foes: Vec<usize> = (0..run.actors.len())
                    .filter(|&i| {
                        let s = &run.map.spawns[run.actors[i].spawn];
                        s.room == room
                            && matches!(
                                s.kind,
                                SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss
                            )
                    })
                    .collect();
                let mut still = vec![0.0f32; run.actors.len()];
                let mut last = run.actors.iter().map(|a| a.position).collect::<Vec<_>>();
                let mut worst: Vec<(f32, String)> = vec![(0.0, String::new()); run.actors.len()];
                for _ in 0..(20 * 30) {
                    run.health = 100.0;
                    run.update(Input::default(), 1.0 / 30.0);
                    for &i in &foes {
                        let a = &run.actors[i];
                        if a.health <= 0.0 {
                            continue;
                        }
                        let d = horizontal(a.position - run.controller.position).length();
                        let moved = horizontal(a.position - last[i]).length() > 0.01;
                        if moved || a.attack.is_some() {
                            still[i] = 0.0;
                        } else {
                            still[i] += 1.0 / 30.0;
                        }
                        last[i] = a.position;
                        if still[i] > worst[i].0 {
                            let s = &run.map.spawns[a.spawn];
                            let visible = run.map.line_of_sight(
                                a.position + Vec3::Y,
                                run.controller.position + Vec3::Y,
                            );
                            worst[i] = (still[i], format!("seed {seed} room {room} ({:?}) {} {:?}: d {d:.1} gait {:?} cd {:.1} visible {visible} path {} yaw_err {:.2} stand {:.1}",
                                run.map.rooms[room].role, s.definition, s.kind, a.gait, a.cooldown, a.path.len(),
                                combat::wrap(combat::yaw_to(run.controller.position - a.position) - a.yaw).abs(), a.stand));
                        }
                    }
                }
                for &i in &foes {
                    total += 1;
                    if worst[i].0 > 5.0 {
                        stuck += 1;
                        eprintln!("STUCK {:.1}s: {}", worst[i].0, worst[i].1);
                    }
                }
            }
        }
        eprintln!("{stuck} of {total} foes stuck");
    }
}

#[cfg(test)]
mod map_view_tests {
    use super::*;
    use velvet_dungeon::controller::Input;

    #[test]
    fn the_map_turns_tilts_and_zooms_within_limits_and_resets() {
        let mut run = Dungeon::generate(42);
        run.map_open = true;
        let start = (run.map_yaw, run.map_pitch);
        run.update(
            Input {
                strafe: 1.0,
                forward: 1.0,
                ..Default::default()
            },
            0.05,
        );
        assert!(
            run.map_yaw != start.0 && run.map_pitch > start.1,
            "held keys must steer the view"
        );
        for _ in 0..400 {
            run.update(
                Input {
                    forward: 1.0,
                    ..Default::default()
                },
                0.05,
            );
            run.map_zoom_by(1.0);
        }
        assert!(run.map_pitch <= MAP_PITCH_RANGE.1 && run.map_zoom <= 3.2);
        for _ in 0..400 {
            run.update(
                Input {
                    forward: -1.0,
                    sprint: true,
                    ..Default::default()
                },
                0.05,
            );
            run.map_zoom_by(-1.0);
        }
        assert!(run.map_pitch >= MAP_PITCH_RANGE.0 && run.map_zoom >= 0.6);
        assert!((0.0..std::f32::consts::TAU).contains(&run.map_yaw));
        run.reset_map_view();
        assert_eq!(
            (run.map_yaw, run.map_pitch, run.map_zoom),
            (MAP_YAW, MAP_PITCH, 1.0)
        );
    }

    #[test]
    fn the_run_stays_frozen_while_the_map_is_open() {
        let mut run = Dungeon::generate(42);
        run.map_open = true;
        let (at, clock) = (run.controller.position, run.elapsed);
        run.update(
            Input {
                forward: 1.0,
                ..Default::default()
            },
            0.05,
        );
        assert_eq!(run.controller.position, at);
        assert_eq!(run.elapsed, clock);
        assert!(
            run.map_time > 0.0,
            "the map keeps its own clock for its pulses"
        );
    }

    #[test]
    fn map_keys_do_not_heal_or_walk_while_the_map_is_open() {
        use winit::keyboard::KeyCode;
        let mut run = Dungeon::generate(42);
        run.health = 50.0;
        run.potions = 3;
        run.map_open = true;
        run.key(KeyCode::KeyQ, true, false, None);
        run.key(KeyCode::KeyR, true, false, None);
        assert_eq!(run.health, 50.0);
        assert_eq!(run.potions, 3);
        assert!(!run.auto_walk);
    }

    #[test]
    fn dragging_orbits_from_below_and_right_drag_pans() {
        let mut run = Dungeon::generate(42);
        run.map_pointer(Some([600.0, 500.0]));
        assert!(!run.map_press(false), "the map must be open to drag it");
        run.map_open = true;
        run.map_pointer(Some([1500.0, 1000.0]));
        assert!(
            !run.map_press(false),
            "presses outside the chart do not drag"
        );
        run.map_pointer(Some([600.0, 500.0]));
        assert!(run.map_press(false));
        let yaw = run.map_yaw;
        run.map_pointer(Some([700.0, 300.0]));
        assert!(run.map_yaw != yaw);
        assert!(run.map_pitch < MAP_PITCH, "dragging up looks from lower");
        run.map_pointer(Some([700.0, -2000.0]));
        assert!(
            run.map_pitch < 0.0 && run.map_pitch >= MAP_PITCH_RANGE.0,
            "it can go under the map"
        );
        run.map_release();
        run.map_pointer(Some([700.0, 300.0]));
        assert!(run.map_press(true));
        run.map_pointer(Some([720.0, 320.0]));
        assert_eq!(run.map_pan, [20.0, 20.0]);
        run.map_pointer(None);
        assert!(!run.map_dragging());
        run.reset_map_view();
        assert_eq!(run.map_pan, [0.0; 2]);
    }
}
