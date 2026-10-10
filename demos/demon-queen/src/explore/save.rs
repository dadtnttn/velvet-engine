//! El guardado del mundo: la instantánea de la sesión 3D, su escritura y el punto de partida que
//! se restaura de ella al volver.
use super::*;

impl App {
    pub(crate) fn snapshot(&self) -> exploration3d_state::State {
        let (player, player_yaw, nature_mode, village_entered) = if let Some(inside) = &self.brothel
        {
            (inside.return_to, inside.return_yaw, true, true)
        } else if let Some(inside) = &self.baths {
            (inside.return_to, inside.return_yaw, true, true)
        } else if self.penascal.is_some() && self.dungeon_return.is_none() {
            // In el Peñascal: saved at the foot of its way, in the wood.
            let (at, yaw) = penascal_return();
            (at, yaw, true, false)
        } else {
            self.dungeon_return
                .map(|saved| {
                    (
                        saved.player,
                        saved.player_yaw,
                        saved.nature_mode,
                        saved.village_entered,
                    )
                })
                .unwrap_or((
                    self.player,
                    self.player_yaw,
                    self.nature_mode,
                    self.village_entered,
                ))
        };
        let mut state = exploration3d_state::State::new(
            self.world_id.clone(),
            self.spawn_id.clone(),
            player.x,
            player.z,
        );
        state.yaw = player_yaw;
        // The castle keeps the floor she is on (its upper floor stands higher).
        if self.castle.is_some()
            && self.brothel.is_none()
            && self.baths.is_none()
            && self.dungeon_return.is_none()
        {
            state.y = player.y;
        }
        state.nature_mode = nature_mode;
        state.village_entered = village_entered;
        state.outfit = self.outfit;
        state.seen_interactions = self.seen_interactions.clone();
        state.achievements = self.achievements.clone();
        // Halfway through a conversation, the story as it was before it (it plays again if cut short).
        state.story = match (&self.talk, &self.talk_story_before) {
            (Some(_), Some(before)) => before.clone(),
            _ => self.story.clone(),
        };
        state.gold = self.gold;
        state.inventory = self.inventory.clone();
        state.backpack = Some(self.backpack.clone());
        state.shop_stock = self.shop_stock.clone();
        state.venue = self.venue.progress.clone();
        state.dungeon = (!self.dungeon.visited.is_empty() || self.dungeon_mode)
            .then(|| self.dungeon.snapshot());
        state.dungeon_active = self.dungeon_mode;
        state
    }

    pub(crate) fn persist_3d_state(&mut self, force: bool) {
        let now = Instant::now();
        if !force && now < self.next_autosave {
            return;
        }
        let mut state = self.snapshot();
        if let Err(error) = exploration3d_state::write_to(&self.session_path, &mut state) {
            eprintln!("[3d] autosave falló: {error:#}");
        } else {
            eprintln!(
                "[3d] autosave: world={} pos={:.2},{:.2}",
                self.world_id, self.player.x, self.player.z
            );
        }
        self.next_autosave = now + Duration::from_secs(1);
    }
}

/// Where and with what a session starts: the saved session's place, bag, story and dungeon when
/// the save is for this world and spawn (old castle spawns and spots no longer free moved to
/// safe ground), the spawn's defaults otherwise.
pub(super) struct Start {
    pub(super) dungeon: dungeon::Dungeon,
    pub(super) dungeon_resume: bool,
    pub(super) player: Vec3,
    pub(super) player_yaw: f32,
    pub(super) nature_mode: bool,
    pub(super) village_entered: bool,
    pub(super) outfit: u8,
    pub(super) seen_interactions: BTreeSet<String>,
    pub(super) achievement_progress: achievements::Progress,
    pub(super) story: guion::Vars,
    pub(super) gold: u32,
    pub(super) inventory: BTreeMap<String, u32>,
    pub(super) backpack: backpack::Backpack,
    pub(super) shop_stock: BTreeMap<String, u32>,
    pub(super) venue: venue::Menu,
}

/// Where a world starts: its saved session when it is this world's, else fresh (a new dungeon from
/// `seed` when one is given — the one a background load was made for — else from the clock).
pub(super) fn starting_point(
    world_id: &str,
    spawn_id: &str,
    session_path: &Path,
    seed: Option<u64>,
) -> Start {
    let mut seed_hasher = DefaultHasher::new();
    world_id.hash(&mut seed_hasher);
    spawn_id.hash(&mut seed_hasher);
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut seed_hasher);
    let mut dungeon = dungeon::Dungeon::generate(seed.unwrap_or_else(|| seed_hasher.finish()));
    let mut dungeon_resume = false;
    let (spawn_x, spawn_z) = exploration3d_state::default_spawn(spawn_id);
    let mut player = Vec3::new(spawn_x, 0.0, spawn_z);
    let mut player_yaw = 0.0;
    let mut nature_mode = false;
    let mut village_entered = false;
    let mut outfit = 0;
    let mut seen_interactions = BTreeSet::new();
    let mut achievement_progress = achievements::Progress::default();
    let mut story = guion::Vars::default();
    let mut gold = 245;
    let mut inventory = BTreeMap::new();
    let mut backpack = backpack::Backpack::new();
    let mut shop_stock = shop::default_stock();
    let mut venue = venue::Menu::default();
    if let Ok(Some(saved)) = exploration3d_state::load_from(session_path) {
        if saved.world == world_id && saved.spawn == spawn_id {
            if let Some(snapshot) = &saved.dungeon {
                match dungeon::Dungeon::restore(snapshot) {
                    Ok(restored) => {
                        dungeon = restored;
                        dungeon_resume = saved.dungeon_active;
                    }
                    Err(error) => eprintln!("[dungeon] {error}"),
                }
            }
            player = Vec3::new(saved.x, saved.y, saved.z);
            player_yaw = saved.yaw;
            nature_mode = saved.nature_mode;
            village_entered = saved.village_entered;
            outfit = saved.outfit % wardrobe_outfit_count();
            seen_interactions = saved.seen_interactions;
            achievement_progress = saved.achievements;
            story = saved.story;
            gold = saved.gold;
            inventory = saved.inventory;
            if let Some(saved_pack) = saved.backpack {
                backpack = saved_pack;
            }
            venue.progress = saved.venue.clone();
            if !saved.shop_stock.is_empty() {
                shop_stock = saved.shop_stock;
            }
            eprintln!(
                "[3d] checkpoint restaurado: world={} spawn={} pos={:.2},{:.2}",
                saved.world, saved.spawn, saved.x, saved.z
            );
        }
    }
    // A once-only scene a mission still waits on, cut short last time: it plays again.
    for scene in missions::unstick(&mut story) {
        eprintln!("[misiones] {scene} vuelve a estar disponible (se cortó a medias)");
    }
    // A save from before the castle was rebuilt holds the old room's story spawns.
    if !nature_mode && [(0.0, -2.0), (0.0, -7.1)].contains(&(player.x, player.z)) {
        let (x, z) = exploration3d_state::default_spawn(if player.z < -5.0 {
            "castle_gate"
        } else {
            "bedroom_inside"
        });
        player = Vec3::new(x, 0.0, z);
    }
    // An old checkpoint may lie inside a newly moved house or beyond the
    // compact circular boundary. Recover at the authored, unobstructed gate.
    if nature_mode && village_entered && village::blocked(player) {
        // A saved spot that is no longer free (the villa was rebuilt): back to its gate.
        player = village::spawn().0;
    }
    Start {
        dungeon,
        dungeon_resume,
        player,
        player_yaw,
        nature_mode,
        village_entered,
        outfit,
        seen_interactions,
        achievement_progress,
        story,
        gold,
        inventory,
        backpack,
        shop_stock,
        venue,
    }
}
