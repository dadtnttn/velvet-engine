//! Background chunk builder. Room/corridor chunks and vault meshes are pure
//! functions of the blueprint, so worker threads build the ones near the
//! player ahead of time; the render thread only uploads them, at most one
//! prefetched chunk per frame. A chunk needed while a worker is building it
//! waits briefly for that build; one nobody is building yet is built right
//! away on the render thread, so nothing ever pops in late.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use velvet_dungeon::{Blueprint, Catalog, Role, Room, Shape};
use velvet_render::Mesh3D;

/// What the worker builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Job {
    /// Room (`key < rooms`) or corridor (`rooms + connection id`) chunk.
    Chunk(usize),
    /// Treasury / arcane vault detail + glow meshes of one room.
    Vault(usize),
}

pub enum Built {
    Chunk(Vec<Mesh3D>),
    Vault(Mesh3D, Mesh3D),
    /// A themed room's art: detail, glow and its water surfaces.
    /// A themed room: lit detail, glow, the glow that beats with the Heart
    /// (or the arcane runes), and its water.
    Theme(Mesh3D, Mesh3D, Mesh3D, Vec<super::water::Surface>),
}

pub struct Streamer {
    requests: Sender<Job>,
    results: Receiver<(Job, Built)>,
    pending: BTreeSet<Job>,
    ready: BTreeMap<Job, Built>,
}

/// Most CPU meshes held between build and upload (they are large).
const READY_LIMIT: usize = 10;
/// Threads building chunks: enough to keep ahead of a sprinting player
/// while leaving cores for the game and the driver.
const WORKERS: usize = 3;

/// Side of the square cells a room or corridor mesh is cut into: the
/// renderer then skips the parts of a room outside the view.
pub const CHUNK_CELL: f32 = 6.0;

/// A room (`key` < rooms) or corridor mesh, cut into view-cullable pieces.
pub fn chunk_pieces(map: &Blueprint, catalog: &Catalog, key: usize) -> Vec<Mesh3D> {
    let mesh = if key < map.rooms.len() {
        super::visual::room(map, catalog, key)
    } else {
        super::visual::corridor(map, catalog, key - map.rooms.len())
    };
    mesh.split_into_cells(CHUNK_CELL)
}

pub fn build(map: &Blueprint, catalog: &Catalog, job: Job) -> Built {
    match job {
        Job::Chunk(key) => Built::Chunk(chunk_pieces(map, catalog, key)),
        Job::Vault(id) => {
            let room = &map.rooms[id];
            if let Some(plan) = super::art::theme::any(map, room) {
                return theme_art(map, catalog, &plan, id);
            }
            match super::vaults::plan(map, room) {
                Some(vault) => {
                    let (detail, glow) =
                        super::vaults::meshes(&vault, &catalog.biomes[room.biome], map.seed, id);
                    Built::Vault(detail, glow)
                }
                None => Built::Vault(super::visual::empty(), super::visual::empty()),
            }
        }
    }
}

/// Build a themed room's art with the stone of its finish.
pub fn theme_art(
    map: &Blueprint,
    catalog: &Catalog,
    plan: &super::art::plan::Plan,
    id: usize,
) -> Built {
    let room = &map.rooms[id];
    // The treasury keeps its warm sandstone whatever the biome (web
    // `artBiomeIndex`); pieces take the stone of the room's finish.
    let art_biome = if room.role == Role::Treasure {
        0
    } else {
        room.biome
    };
    let biome = super::surfaces::styled_biome(&catalog.biomes[art_biome]);
    let stone = super::themes::finish(map, room, &biome)
        .map(|f| f.stone)
        .unwrap_or(biome.wall);
    let glow = art_glow(map, catalog, room);
    let (mut detail, glow_mesh, ponds) = super::art::theme::build(plan, stone, glow, map.seed, id);
    // Veins, sigils and runes that beat: their lit part joins the detail.
    let (pulse, mut pulse_glow) = super::art::theme::build_pulse(plan, stone, glow, map.seed, id);
    super::art::kit::append(&mut detail, &pulse, &super::art::kit::identity(), [1.0; 3]);
    // The arcane hall's glowing medallion (its obelisks are in the shell).
    if plan.vault == Some(super::art::plan::VaultKind::Arcane) {
        let runes = super::art::vaults::vault_glow_mesh(plan);
        super::art::kit::append(
            &mut pulse_glow,
            &runes,
            &super::art::kit::identity(),
            [0.83; 3],
        );
    }
    Built::Theme(
        detail,
        glow_mesh,
        pulse_glow,
        ponds.iter().map(super::water::from_pond).collect(),
    )
}

impl Streamer {
    pub fn new(map: &Blueprint, catalog: &Catalog) -> Self {
        let (requests, jobs) = channel::<Job>();
        let (done, results) = channel();
        let jobs = std::sync::Arc::new(std::sync::Mutex::new(jobs));
        let shared = std::sync::Arc::new((map.clone(), catalog.clone()));
        let workers = std::thread::available_parallelism()
            .map_or(2, |n| n.get().saturating_sub(2))
            .clamp(1, WORKERS);
        for worker in 0..workers {
            let (jobs, done, shared) = (jobs.clone(), done.clone(), shared.clone());
            std::thread::Builder::new()
                .name(format!("dungeon-chunks-{worker}"))
                .spawn(move || {
                    // Exits when the scene drops the request sender (reseed/quit).
                    loop {
                        let job = match jobs.lock() {
                            Ok(jobs) => jobs.recv(),
                            Err(_) => break,
                        };
                        let Ok(job) = job else { break };
                        let (map, catalog) = &*shared;
                        if done.send((job, build(map, catalog, job))).is_err() {
                            break;
                        }
                    }
                })
                .expect("spawn dungeon chunk builder");
        }
        Self {
            requests,
            results,
            pending: BTreeSet::new(),
            ready: BTreeMap::new(),
        }
    }

    /// Collect finished builds without blocking.
    pub fn poll(&mut self) {
        while let Ok((job, built)) = self.results.try_recv() {
            self.pending.remove(&job);
            self.ready.insert(job, built);
        }
    }

    /// Take a finished build if a worker already produced it.
    pub fn take(&mut self, job: Job) -> Option<Built> {
        self.poll();
        self.ready.remove(&job)
    }

    /// Take a build, waiting up to `patience` if a worker is on it already:
    /// finishing a build in flight beats starting the same one from scratch.
    pub fn take_or_wait(&mut self, job: Job, patience: std::time::Duration) -> Option<Built> {
        let deadline = std::time::Instant::now() + patience;
        loop {
            if let Some(built) = self.take(job) {
                return Some(built);
            }
            if !self.pending.contains(&job) {
                return None;
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match self.results.recv_timeout(left) {
                Ok((done, built)) => {
                    self.pending.remove(&done);
                    self.ready.insert(done, built);
                }
                Err(_) => return None,
            }
        }
    }

    /// Queue jobs (nearest first) that are neither resident nor in flight,
    /// keeping the number of large CPU meshes bounded.
    pub fn prefetch(
        &mut self,
        jobs: impl IntoIterator<Item = Job>,
        resident: impl Fn(Job) -> bool,
    ) {
        for job in jobs {
            if self.pending.len() + self.ready.len() >= READY_LIMIT {
                break;
            }
            if resident(job) || self.pending.contains(&job) || self.ready.contains_key(&job) {
                continue;
            }
            if self.requests.send(job).is_ok() {
                self.pending.insert(job);
            }
        }
    }

    /// One finished, not-yet-resident build to pre-upload this frame.
    pub fn next_ready(&mut self, resident: impl Fn(Job) -> bool) -> Option<(Job, Built)> {
        self.poll();
        let stale: Vec<Job> = self
            .ready
            .keys()
            .copied()
            .filter(|job| resident(*job))
            .collect();
        for job in stale {
            self.ready.remove(&job);
        }
        let job = *self.ready.keys().next()?;
        self.ready.remove(&job).map(|built| (job, built))
    }
}

/// Rooms ordered by graph distance from `start`, up to `depth` hops, with
/// the corridor keys that join them: the prefetch horizon around the player.
pub fn horizon(map: &Blueprint, start: usize, depth: usize) -> Vec<Job> {
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([(start, 0)]);
    let mut jobs = Vec::new();
    while let Some((room, d)) = queue.pop_front() {
        jobs.push(Job::Chunk(room));
        if matches!(
            map.rooms[room].role,
            velvet_dungeon::Role::Treasure
                | velvet_dungeon::Role::Secret
                | velvet_dungeon::Role::Rest
        ) || super::themes::of(map, &map.rooms[room]).is_some_and(super::art::theme::ported)
        {
            jobs.push(Job::Vault(room));
        }
        if d >= depth {
            continue;
        }
        for (next, c) in map.neighbors(room) {
            jobs.push(Job::Chunk(map.rooms.len() + c.id));
            if seen.insert(next) {
                queue.push_back((next, d + 1));
            }
        }
    }
    let mut unique = BTreeSet::new();
    jobs.retain(|job| unique.insert(*job));
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Config};

    #[test]
    fn background_builds_match_synchronous_meshes() {
        let catalog = Catalog::bundled();
        let map = generate(42, &Config::default(), &catalog, &Difficulty::default()).unwrap();
        let mut streamer = Streamer::new(&map, &catalog);
        let jobs = horizon(&map, map.entrance, 2);
        assert!(jobs.len() > 3);
        streamer.prefetch(jobs.iter().copied(), |_| false);
        let first = jobs[0];
        let started = std::time::Instant::now();
        let built = loop {
            if let Some(built) = streamer.take(first) {
                break built;
            }
            assert!(started.elapsed().as_secs() < 20, "worker never answered");
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        let (Built::Chunk(a), Built::Chunk(b)) = (built, build(&map, &catalog, first)) else {
            panic!("room job must yield a chunk");
        };
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.indices, b.indices);
            assert_eq!(a.vertices.len(), b.vertices.len());
            assert!(a
                .vertices
                .iter()
                .zip(&b.vertices)
                .all(|(x, y)| x.position == y.position && x.color == y.color));
        }
    }
}

/// The glow colour of a room (web `roomMood(...).glow`).
pub fn art_glow(map: &Blueprint, catalog: &Catalog, room: &Room) -> [f32; 3] {
    let base = super::surfaces::mood(&catalog.biomes[room.biome]);
    match room.role {
        Role::Treasure if room.shape != Shape::Bridge => {
            if room.treasury == Some(velvet_dungeon::Treasury::Grand) {
                [1.0, 0.62, 0.2]
            } else {
                [1.0, 0.45, 0.1]
            }
        }
        Role::Secret if room.shape != Shape::Bridge => [0.62, 0.32, 1.0],
        _ => {
            super::themes::of(map, room)
                .map_or(base, |theme| super::themes::mood(theme, base))
                .glow
        }
    }
}
