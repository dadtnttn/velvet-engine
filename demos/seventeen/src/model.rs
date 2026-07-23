use anyhow::{anyhow, bail, Context, Result};
use velvet_script_vs3::Value;

#[derive(Debug, Clone, Copy, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerView {
    pub pos: Vec2,
    pub aim: Vec2,
    pub velocity: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub weapon: String,
    pub reload_timer: f32,
    pub dash_cooldown: f32,
    pub hit_flash: f32,
    pub invulnerable: f32,
}

#[derive(Debug, Clone, Default)]
pub struct EnemyView {
    pub kind: String,
    pub pos: Vec2,
    pub aim: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub alive: bool,
    pub hit_flash: f32,
    pub phase: i64,
}

#[derive(Debug, Clone, Default)]
pub struct BulletView {
    pub owner: String,
    pub pos: Vec2,
    pub velocity: Vec2,
    pub radius: f32,
    pub alive: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RectView {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct PickupView {
    pub kind: String,
    pub pos: Vec2,
    pub index: i64,
    pub active: bool,
    pub pulse: f32,
}

#[derive(Debug, Clone, Default)]
pub struct EventView {
    pub kind: String,
    pub pos: Vec2,
    pub power: f32,
}

#[derive(Debug, Clone, Default)]
pub struct FrameView {
    pub phase: String,
    pub room: i64,
    pub room_name: String,
    pub room_time: f32,
    pub deaths: i64,
    pub score: i64,
    pub player: PlayerView,
    pub enemies: Vec<EnemyView>,
    pub bullets: Vec<BulletView>,
    pub obstacles: Vec<RectView>,
    pub hazards: Vec<RectView>,
    pub pickups: Vec<PickupView>,
    pub events: Vec<EventView>,
    pub memories: [bool; 3],
    pub memory_count: i64,
    pub message: String,
    pub speaker: String,
    pub prompt: String,
    pub memory_text: String,
    pub room_clear: bool,
    pub door_open: bool,
    pub ammo: i64,
    pub magazine: i64,
    pub boss_hp: f32,
    pub boss_max: f32,
    pub distortion: f32,
    pub death_timer: f32,
    pub ending_variant: String,
    pub save_revision: i64,
}

impl FrameView {
    pub fn parse(value: &Value) -> Result<Self> {
        Ok(Self {
            phase: string(value, "phase")?,
            room: integer(value, "room")?,
            room_name: string(value, "room_name")?,
            room_time: number(value, "room_time")?,
            deaths: integer(value, "deaths")?,
            score: integer(value, "score")?,
            player: parse_player(&field(value, "player")?)?,
            enemies: list(value, "enemies", parse_enemy)?,
            bullets: list(value, "bullets", parse_bullet)?,
            obstacles: list(value, "obstacles", parse_rect)?,
            hazards: list(value, "hazards", parse_rect)?,
            pickups: list(value, "pickups", parse_pickup)?,
            events: list(value, "events", parse_event)?,
            memories: parse_memories(&field(value, "memories")?)?,
            memory_count: integer(value, "memory_count")?,
            message: string(value, "message")?,
            speaker: string(value, "speaker")?,
            prompt: string(value, "prompt")?,
            memory_text: string(value, "memory_text")?,
            room_clear: boolean(value, "room_clear")?,
            door_open: boolean(value, "door_open")?,
            ammo: integer(value, "ammo")?,
            magazine: integer(value, "magazine")?,
            boss_hp: number(value, "boss_hp")?,
            boss_max: number(value, "boss_max")?,
            distortion: number(value, "distortion")?,
            death_timer: number(value, "death_timer")?,
            ending_variant: string(value, "ending_variant")?,
            save_revision: integer(value, "save_revision")?,
        })
    }
}

fn parse_player(value: &Value) -> Result<PlayerView> {
    Ok(PlayerView {
        pos: vec2(value, "pos")?,
        aim: vec2(value, "aim")?,
        velocity: vec2(value, "velocity")?,
        hp: number(value, "hp")?,
        max_hp: number(value, "max_hp")?,
        weapon: string(value, "weapon")?,
        reload_timer: number(value, "reload_timer")?,
        dash_cooldown: number(value, "dash_cooldown")?,
        hit_flash: number(value, "hit_flash")?,
        invulnerable: number(value, "invulnerable")?,
    })
}

fn parse_enemy(value: &Value) -> Result<EnemyView> {
    Ok(EnemyView {
        kind: string(value, "kind")?,
        pos: vec2(value, "pos")?,
        aim: vec2(value, "aim")?,
        hp: number(value, "hp")?,
        max_hp: number(value, "max_hp")?,
        alive: boolean(value, "alive")?,
        hit_flash: number(value, "hit_flash")?,
        phase: integer(value, "phase")?,
    })
}

fn parse_bullet(value: &Value) -> Result<BulletView> {
    Ok(BulletView {
        owner: string(value, "owner")?,
        pos: vec2(value, "pos")?,
        velocity: vec2(value, "velocity")?,
        radius: number(value, "radius")?,
        alive: boolean(value, "alive")?,
    })
}

fn parse_rect(value: &Value) -> Result<RectView> {
    Ok(RectView {
        x: number(value, "x")?,
        y: number(value, "y")?,
        w: number(value, "w")?,
        h: number(value, "h")?,
        kind: optional_string(value, "kind"),
    })
}

fn parse_pickup(value: &Value) -> Result<PickupView> {
    Ok(PickupView {
        kind: string(value, "kind")?,
        pos: vec2(value, "pos")?,
        index: integer(value, "index")?,
        active: boolean(value, "active")?,
        pulse: number(value, "pulse")?,
    })
}

fn parse_event(value: &Value) -> Result<EventView> {
    Ok(EventView {
        kind: string(value, "kind")?,
        pos: vec2(value, "pos")?,
        power: number(value, "power")?,
    })
}

fn parse_memories(value: &Value) -> Result<[bool; 3]> {
    let items = value.list_items().map_err(|error| anyhow!(error))?;
    if items.len() != 3 {
        bail!("memories must contain three values");
    }
    Ok([
        items[0].is_truthy(),
        items[1].is_truthy(),
        items[2].is_truthy(),
    ])
}

fn list<T>(root: &Value, key: &str, parse: fn(&Value) -> Result<T>) -> Result<Vec<T>> {
    field(root, key)?
        .list_items()
        .map_err(|error| anyhow!(error))?
        .iter()
        .map(parse)
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("snapshot field `{key}`"))
}

fn field(value: &Value, key: &str) -> Result<Value> {
    value
        .map_get(key)
        .map_err(|error| anyhow!(error))?
        .ok_or_else(|| anyhow!("missing VS3 map field `{key}`"))
}

fn string(value: &Value, key: &str) -> Result<String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("`{key}` is not a string"))
}

fn optional_string(value: &Value, key: &str) -> String {
    field(value, key)
        .ok()
        .and_then(|item| item.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn integer(value: &Value, key: &str) -> Result<i64> {
    field(value, key)?
        .as_i64()
        .ok_or_else(|| anyhow!("`{key}` is not an integer"))
}

fn number(value: &Value, key: &str) -> Result<f32> {
    field(value, key)?
        .as_f64()
        .map(|number| number as f32)
        .ok_or_else(|| anyhow!("`{key}` is not numeric"))
}

fn boolean(value: &Value, key: &str) -> Result<bool> {
    let value = field(value, key)?;
    match value {
        Value::Bool(flag) => Ok(flag),
        _ => Err(anyhow!("`{key}` is not a boolean")),
    }
}

fn vec2(value: &Value, key: &str) -> Result<Vec2> {
    match field(value, key)? {
        Value::Vec2([x, y]) => Ok(Vec2 {
            x: x as f32,
            y: y as f32,
        }),
        _ => Err(anyhow!("`{key}` is not a vec2")),
    }
}
