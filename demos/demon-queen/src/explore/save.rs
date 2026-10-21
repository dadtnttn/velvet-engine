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
        state.brothel_rope_open =
            self.brothel_rope_open || self.brothel.as_ref().is_some_and(|b| b.rope_open);
        state.war = self
            .war_campaign
            .as_ref()
            .and_then(|c| serde_json::to_value(c).ok());
        state.war_scale = Some(self.war_scale);
        state.inside = if let Some(b) = &self.brothel {
            Some(exploration3d_state::Inside {
                place: "burdel".into(),
                x: b.walker.x,
                z: b.walker.z,
                yaw: self.player_yaw,
                level: b.walker.level,
                room: None,
                outfit_before: None,
            })
        } else if let Some(b) = &self.baths {
            Some(exploration3d_state::Inside {
                place: "termas".into(),
                x: b.x,
                z: b.z,
                yaw: self.player_yaw,
                level: 0,
                room: b.room.clone(),
                outfit_before: self.outfit_before_baths,
            })
        } else if let (Some(p), None) = (&self.penascal, &self.dungeon_return) {
            Some(exploration3d_state::Inside {
                place: "penascal".into(),
                x: p.x,
                z: p.z,
                yaw: self.player_yaw,
                level: 0,
                room: None,
                outfit_before: None,
            })
        } else {
            None
        };
        state
    }

    /// Smoke test: some progress of her own (gold, an item, a story mark) a save must keep.
    #[allow(dead_code)]
    pub(crate) fn smoke_mark_progress(&mut self) {
        self.gold = 1234;
        self.inventory.insert("pocion_reforzada".into(), 2);
        self.story.values.insert("smoke_guardado".into(), 1);
    }

    pub(crate) fn persist_3d_state(&mut self, force: bool) {
        let now = Instant::now();
        if !force && now < self.next_autosave {
            return;
        }
        let mut state = self.snapshot();
        // Every second while she walks: on the save writer's thread (written now when forced,
        // as on leaving).
        let written = if force {
            exploration3d_state::write_to(&self.session_path, &mut state)
        } else {
            exploration3d_state::write_in_background(&self.session_path, &mut state)
        };
        if let Err(error) = written {
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
    /// Back inside the place she was saved in.
    pub(super) inside: Option<exploration3d_state::Inside>,
    pub(super) brothel_rope_open: bool,
    pub(super) war: Option<demon_queen_war::rules::Campaign>,
    pub(super) war_scale: f32,
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
    // The mine is the hand-made one (Mina 2.0) when its data is there; a seed asked for, or a build
    // without it, gets a generated cave.
    let mut dungeon = match seed {
        None if dungeon::authored::available() => {
            dungeon::Dungeon::mine().unwrap_or_else(|error| {
                eprintln!("[mina] {error}: a generated cave instead");
                dungeon::Dungeon::generate(seed_hasher.finish())
            })
        }
        _ => dungeon::Dungeon::generate(seed.unwrap_or_else(|| seed_hasher.finish())),
    };
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
    let mut inside = None;
    let mut brothel_rope_open = false;
    let mut war = None;
    let mut war_scale = 1.0;
    if let Ok(Some(saved)) = exploration3d_state::load_from(session_path) {
        brothel_rope_open = saved.brothel_rope_open;
        // The war table's campaign goes wherever she does.
        war = saved
            .war
            .clone()
            .and_then(|v| serde_json::from_value::<demon_queen_war::rules::Campaign>(v).ok());
        war_scale = saved.war_scale.unwrap_or(1.0).clamp(0.5, 2.0);
        // Hers wherever she is: the purse, the bag, the shop, the book, the achievements, the
        // story, the outfit.
        outfit = saved.outfit % wardrobe_outfit_count();
        seen_interactions = saved.seen_interactions.clone();
        achievement_progress = saved.achievements.clone();
        story = saved.story.clone();
        gold = saved.gold;
        inventory = saved.inventory.clone();
        if let Some(saved_pack) = saved.backpack.clone() {
            backpack = saved_pack;
        }
        venue.progress = saved.venue.clone();
        if !saved.shop_stock.is_empty() {
            shop_stock = saved.shop_stock.clone();
        }
        if saved.world == world_id && saved.spawn == spawn_id {
            // A generated cave saved before the hand-made mine came gives way to it, unless she
            // was saved down there.
            let outdated = saved.dungeon.as_ref().is_some_and(|snapshot| {
                snapshot.seed != dungeon::authored::MINE_SEED
                    && !saved.dungeon_active
                    && seed.is_none()
                    && dungeon::authored::available()
            });
            if let Some(snapshot) = saved.dungeon.as_ref().filter(|_| !outdated) {
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
            // Not in the mine: the place she was in (the mine comes back on its own).
            if !saved.dungeon_active {
                inside = saved.inside.clone();
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
        inside,
        brothel_rope_open,
        war,
        war_scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_file(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dq-save-{name}-{}-{stamp}.json",
            std::process::id()
        ))
    }

    #[test]
    fn what_she_owns_comes_back_wherever_the_world_starts() {
        let path = session_file("own");
        let mut saved = exploration3d_state::State::new("act1_castle", "bedroom_inside", 3.0, 4.0);
        saved.gold = 999;
        saved.inventory.insert("pocion_pequena".into(), 3);
        saved.outfit = 1;
        saved.story.values.insert("consejo_1_hecho".into(), 1);
        exploration3d_state::write_to(&path, &mut saved).unwrap();
        // Another spawn of the same world (the story put her at the gate): her things, not her place.
        let start = starting_point("act1_castle", "castle_gate", &path, Some(7));
        assert_eq!(start.gold, 999);
        assert_eq!(start.inventory.get("pocion_pequena"), Some(&3));
        assert_eq!(start.outfit, 1);
        assert_eq!(start.story.get("consejo_1_hecho"), 1);
        assert_ne!((start.player.x, start.player.z), (3.0, 4.0));
        // The same spawn: her place too.
        let start = starting_point("act1_castle", "bedroom_inside", &path, Some(7));
        assert_eq!((start.player.x, start.player.z), (3.0, 4.0));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_save_in_the_mine_comes_back_in_the_mine() {
        let path = session_file("mine");
        let mut saved = exploration3d_state::State::new("act1_castle", "bedroom_inside", 3.0, 4.0);
        saved.dungeon = Some(dungeon::Dungeon::generate(42).snapshot());
        saved.dungeon_active = true;
        exploration3d_state::write_to(&path, &mut saved).unwrap();
        let start = starting_point("act1_castle", "bedroom_inside", &path, Some(1));
        assert!(start.dungeon_resume);
        std::fs::remove_file(path).unwrap();
    }
}
