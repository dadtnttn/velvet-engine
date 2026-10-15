//! Versioned exploration checkpoint shared by the integrated VN/3D game and
//! the optional editor preview. `continue.json` owns story state; this sidecar
//! carries the live transform and world-mode flags across saves and resumes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "demon-queen-3d-session";
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub format: String,
    pub schema: u32,
    pub saved_at: u64,
    pub world: String,
    pub spawn: String,
    pub x: f32,
    pub z: f32,
    /// Her height: the castle's upper floor (0 everywhere else).
    #[serde(default)]
    pub y: f32,
    pub yaw: f32,
    #[serde(default)]
    pub nature_mode: bool,
    #[serde(default)]
    pub village_entered: bool,
    /// Selected wardrobe preset. Kept numeric so old checkpoints remain
    /// readable while the 3D wardrobe grows new authored outfits.
    #[serde(default)]
    pub outfit: u8,
    #[serde(default)]
    pub seen_interactions: BTreeSet<String>,
    /// Lightweight world economy carried by the 3D session.  Older saves do
    /// not contain these fields and receive the starter purse automatically.
    #[serde(default = "default_gold")]
    pub gold: u32,
    #[serde(default)]
    pub inventory: BTreeMap<String, u32>,
    /// Grid layout of the backpack and the worn equipment.
    #[serde(default)]
    pub backpack: Option<crate::exploration3d::backpack::Backpack>,
    #[serde(default)]
    pub shop_stock: BTreeMap<String, u32>,
    #[serde(default)]
    pub venue: crate::exploration3d::venue::Progress,
    #[serde(default)]
    pub dungeon: Option<crate::exploration3d::dungeon::Snapshot>,
    #[serde(default)]
    pub dungeon_active: bool,
    /// Achievements: event counters, what is unlocked, outfits and flags earned.
    #[serde(default)]
    pub achievements: crate::exploration3d::achievements::Progress,
    /// The story's variables and the guion scenes already seen (shared with the visual novel).
    #[serde(default)]
    pub story: crate::exploration3d::guion::Vars,
    /// Saved inside a place of the world (the brothel, the baths, el Peñascal): which and where
    /// (`x`/`z`/`yaw` then hold the way back out).
    #[serde(default)]
    pub inside: Option<Inside>,
    /// Rosalía has taken the rope off the brothel's stair.
    #[serde(default)]
    pub brothel_rope_open: bool,
}

/// Where she stands inside a place: `place` is `burdel`, `termas` or `penascal`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Inside {
    pub place: String,
    pub x: f32,
    pub z: f32,
    #[serde(default)]
    pub yaw: f32,
    /// The brothel's floor.
    #[serde(default)]
    pub level: u8,
    /// The baths' room (none: the valley).
    #[serde(default)]
    pub room: Option<String>,
    /// At the baths: the outfit she came in, waiting in the changing room while she wears the
    /// bikini.
    #[serde(default)]
    pub outfit_before: Option<u8>,
}

impl State {
    pub fn new(world: impl Into<String>, spawn: impl Into<String>, x: f32, z: f32) -> Self {
        Self {
            format: FORMAT.to_string(),
            schema: SCHEMA,
            saved_at: now_unix(),
            world: world.into(),
            spawn: spawn.into(),
            x,
            z,
            y: 0.0,
            yaw: 0.0,
            nature_mode: false,
            village_entered: false,
            outfit: 0,
            seen_interactions: BTreeSet::new(),
            gold: default_gold(),
            inventory: BTreeMap::new(),
            backpack: None,
            shop_stock: BTreeMap::new(),
            venue: Default::default(),
            dungeon: None,
            dungeon_active: false,
            achievements: Default::default(),
            story: Default::default(),
            inside: None,
            brothel_rope_open: false,
        }
    }
}

fn default_gold() -> u32 {
    245
}

/// The folder that holds the game's `data/` and `assets/`. A portable build reads only its own,
/// beside the executable, never the project it was built from (on another machine that would not
/// exist, and here it would hide what the portable lacks). Any other build reads the one beside the
/// executable when it carries the 3D models (a copied release), else the project's.
pub fn game_dir() -> PathBuf {
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    if cfg!(feature = "portable") {
        return beside.unwrap_or_default();
    }
    beside
        .filter(|dir| dir.join("data/assets/3d/kaykit_dungeon").is_dir())
        .unwrap_or_else(source_root)
}

/// The game's `data/` folder (see [`game_dir`]).
pub fn data_dir() -> PathBuf {
    game_dir().join("data")
}

/// The project this build was made from, where a development build reads its data when it is not
/// beside the executable. A portable build has no project: it reads only what is beside it.
#[cfg(feature = "portable")]
pub fn source_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

#[cfg(not(feature = "portable"))]
pub fn source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn path() -> PathBuf {
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SAVES") {
        return PathBuf::from(path).join("exploration-3d.json");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("saves")
            .join("exploration-3d.json");
    }
    source_root().join("data/saves/exploration-3d.json")
}

/// Where a story spawn puts her in the castle: just inside its gate, or in the bedroom (the start
/// of the story, `bedroom_inside` and its old alias `bedroom_door`, and any other id).
pub fn default_spawn(spawn: &str) -> (f32, f32) {
    use crate::exploration3d::castle3d;
    match spawn {
        "castle_gate" | "castle_entry" => castle3d::gate_spawn(),
        _ => castle3d::bedroom_spawn(),
    }
}

#[allow(dead_code)]
pub fn load() -> Result<Option<State>> {
    load_from(&path())
}

pub fn load_from(file_path: &Path) -> Result<Option<State>> {
    let Ok(bytes) = fs::read(file_path) else {
        return Ok(None);
    };
    let state: State = serde_json::from_slice(&bytes)
        .with_context(|| format!("estado 3D corrupto en {}", file_path.display()))?;
    if state.format != FORMAT || state.schema != SCHEMA {
        anyhow::bail!(
            "estado 3D incompatible en {}: formato={} schema={}",
            file_path.display(),
            state.format,
            state.schema
        );
    }
    Ok(Some(state))
}

#[allow(dead_code)]
pub fn write(state: &mut State) -> Result<()> {
    write_to(&path(), state)
}

/// The checkpoint saved every second while she walks: written by the save writer, not here
/// (nothing is written when only the time of saving changed).
pub fn write_in_background(file_path: &Path, state: &mut State) -> Result<()> {
    state.format = FORMAT.to_string();
    state.schema = SCHEMA;
    state.saved_at = 0;
    let key = background::key_of(&serde_json::to_vec(state)?);
    state.saved_at = now_unix();
    let bytes = serde_json::to_vec_pretty(state)?;
    background::write(file_path.to_path_buf(), bytes, false, key);
    Ok(())
}

pub fn write_to(file_path: &Path, state: &mut State) -> Result<()> {
    // Written now: whatever the save writer still holds for it is older.
    background::flush();
    background::forget(file_path);
    state.format = FORMAT.to_string();
    state.schema = SCHEMA;
    state.saved_at = now_unix();
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("no se pudo crear {}", parent.display()))?;
    }
    let temporary = file_path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(state)?;
    fs::write(&temporary, bytes)
        .with_context(|| format!("no se pudo escribir {}", temporary.display()))?;
    match fs::rename(&temporary, file_path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows refuses replacing an existing destination with rename.
            // Remove only this exact sidecar, then retry the atomic-ish move.
            let _ = fs::remove_file(file_path);
            fs::rename(&temporary, file_path)
                .with_context(|| format!("no se pudo publicar {}", file_path.display()))
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_in_the_background_keep_the_newest_and_skip_what_did_not_change() {
        let dir = std::env::temp_dir().join(format!("dq-bg-save-{}", std::process::id()));
        let file = dir.join("s.json");
        for text in ["one", "two", "three"] {
            background::write(file.clone(), text.as_bytes().to_vec(), false, background::key_of(text.as_bytes()));
        }
        background::flush();
        assert_eq!(fs::read_to_string(&file).unwrap(), "three");
        // The same content again (say, only the time of saving changed): not written.
        fs::write(&file, "touched by hand").unwrap();
        background::write(file.clone(), b"three".to_vec(), false, background::key_of(b"three"));
        background::flush();
        assert_eq!(fs::read_to_string(&file).unwrap(), "touched by hand");
        // A synchronous write after queued ones: the queued (older) ones never land after it.
        let mut state = State::new("castle", "spawn", 1.0, 2.0);
        background::write(file.clone(), b"old".to_vec(), false, background::key_of(b"old"));
        write_to(&file, &mut state).unwrap();
        background::flush();
        assert!(fs::read_to_string(&file).unwrap().contains("\"castle\""));
        assert!(background::take_error().is_none());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_story_starts_in_the_castles_bedroom_on_free_ground() {
        use crate::exploration3d::castle3d;
        let (x, z) = default_spawn("bedroom_inside");
        assert_eq!((x, z), castle3d::bedroom_spawn());
        assert_eq!(default_spawn("bedroom_door"), (x, z));
        assert!(
            !castle3d::plan().blocked(x, z, 0.3, 0),
            "the bedroom spawn is free"
        );
        let (gx, gz) = default_spawn("castle_gate");
        assert!(
            !castle3d::plan().blocked(gx, gz, 0.3, 0),
            "the gate spawn is free"
        );
    }

    #[test]
    fn round_trip_is_versioned() {
        let root =
            std::env::temp_dir().join(format!("demon-queen-3d-state-{}", std::process::id()));
        let file = root.join("session.json");
        let mut state = State::new("act1_castle", "bedroom_door", 1.25, -3.5);
        state.yaw = 0.75;
        state.nature_mode = true;
        state.village_entered = true;
        state.outfit = 2;
        state.seen_interactions.insert("castle_gate".into());
        state.gold = 112;
        state.inventory.insert("pocion_pequena".into(), 3);
        state.shop_stock.insert("espada_de_madera".into(), 2);
        write_to(&file, &mut state).expect("write state");
        let restored = load_from(&file).expect("load state").expect("state exists");
        assert_eq!(restored.world, "act1_castle");
        assert_eq!(restored.spawn, "bedroom_door");
        assert_eq!(restored.x, 1.25);
        assert_eq!(restored.z, -3.5);
        assert_eq!(restored.yaw, 0.75);
        assert!(restored.nature_mode && restored.village_entered);
        assert_eq!(restored.outfit, 2);
        assert!(restored.seen_interactions.contains("castle_gate"));
        assert_eq!(restored.gold, 112);
        assert_eq!(restored.inventory.get("pocion_pequena"), Some(&3));
        assert_eq!(restored.shop_stock.get("espada_de_madera"), Some(&2));
        let _ = fs::remove_dir_all(root);
    }
}

/// Saves written off the thread that draws. The world and the story save every second while she
/// walks; writing (and syncing) a file there cost a frame of 60–85 ms each time, a hitch every
/// second on any machine. Here the caller serializes (cheap) and a writer thread puts it on disk:
/// the newest bytes of a path replace any older ones still waiting, a path whose content did not
/// change is not written again, and [`background::flush`] waits until everything is on disk (on
/// leaving the world, before reading a file that is being written, and when the game ends).
pub mod background {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::path::{Path, PathBuf};
    use std::sync::{Condvar, Mutex, OnceLock};

    /// What a job writes: its bytes, or how to make them (on the writer's thread).
    enum Content {
        Bytes(Vec<u8>),
        Make(Box<dyn FnOnce() -> Vec<u8> + Send>),
    }

    struct Job {
        path: PathBuf,
        content: Content,
        sync: bool,
        key: u64,
    }

    #[derive(Default)]
    struct Queue {
        pending: Vec<Job>,
        busy: bool,
        /// Each path's content as last written (its key), to skip writing it again.
        written: HashMap<PathBuf, u64>,
        error: Option<String>,
    }

    fn shared() -> &'static (Mutex<Queue>, Condvar) {
        static SHARED: OnceLock<(Mutex<Queue>, Condvar)> = OnceLock::new();
        SHARED.get_or_init(|| {
            std::thread::Builder::new()
                .name("save-writer".into())
                .spawn(worker)
                .expect("the save writer starts");
            (Mutex::new(Queue::default()), Condvar::new())
        })
    }

    /// A key for a content: equal contents, equal keys (the caller leaves out what changes
    /// without mattering, such as the time of saving).
    pub fn key_of(bytes: &[u8]) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    /// Writes `bytes` to `path` soon, on the writer thread (`sync`: flushed to the disk itself).
    /// `key` names the content (see [`key_of`]): the same key as last written, nothing is done.
    pub fn write(path: PathBuf, bytes: Vec<u8>, sync: bool, key: u64) {
        queue(path, Content::Bytes(bytes), sync, key);
    }

    /// As [`write`], the bytes made by `make` on the writer's thread (when it is costly).
    pub fn write_with(path: PathBuf, sync: bool, key: u64, make: Box<dyn FnOnce() -> Vec<u8> + Send>) {
        queue(path, Content::Make(make), sync, key);
    }

    fn queue(path: PathBuf, content: Content, sync: bool, key: u64) {
        let (lock, wake) = shared();
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        let waiting = queue.pending.iter().position(|job| job.path == path);
        if waiting.is_none() && queue.written.get(&path) == Some(&key) {
            return;
        }
        if let Some(at) = waiting {
            queue.pending.remove(at);
        }
        queue.pending.push(Job { path, content, sync, key });
        wake.notify_all();
    }

    /// Waits until every save asked for is on disk.
    pub fn flush() {
        let (lock, wake) = shared();
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        while !queue.pending.is_empty() || queue.busy {
            queue = wake.wait(queue).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// The last write that failed, once.
    pub fn take_error() -> Option<String> {
        let (lock, _) = shared();
        lock.lock().unwrap_or_else(|e| e.into_inner()).error.take()
    }

    /// A path written now on this thread: anything still waiting for it is dropped first (it
    /// is older), and its content remembered.
    pub fn forget(path: &Path) {
        let (lock, _) = shared();
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        queue.pending.retain(|job| job.path != path);
        queue.written.remove(path);
    }

    fn worker() {
        let (lock, wake) = shared();
        loop {
            let job = {
                let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
                while queue.pending.is_empty() {
                    queue = wake.wait(queue).unwrap_or_else(|e| e.into_inner());
                }
                queue.busy = true;
                queue.pending.remove(0)
            };
            let bytes = match job.content {
                Content::Bytes(bytes) => bytes,
                Content::Make(make) => make(),
            };
            let result = if bytes.is_empty() {
                Err(std::io::Error::other("nothing to write"))
            } else {
                write_now(&job.path, &bytes, job.sync)
            };
            let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
            queue.busy = false;
            match result {
                Ok(()) => {
                    queue.written.insert(job.path, job.key);
                }
                Err(error) => {
                    queue.written.remove(&job.path);
                    queue.error = Some(format!("{}: {error}", job.path.display()));
                }
            }
            wake.notify_all();
        }
    }

    /// Written whole beside the file, then put in its place (a crash midway leaves the old one).
    pub fn write_now(path: &Path, bytes: &[u8], sync: bool) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        let mut output = std::fs::File::create(&temporary)?;
        output.write_all(bytes)?;
        if sync {
            output.sync_all()?;
        }
        drop(output);
        match std::fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_) => {
                // An old Windows refusing to replace: remove only this file and move again.
                let _ = std::fs::remove_file(path);
                std::fs::rename(&temporary, path)
            }
        }
    }
}
