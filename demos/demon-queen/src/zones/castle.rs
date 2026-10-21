//! El castillo como lugar: lo que el mundo hace con él (dibujarlo, recorrerlo, entrar y salir).
use super::*;

/// Where Nemi waits to train the Queen: in the arena of the patio.
pub(crate) const NEMI_ARENA: (f32, f32) = (-12.0, 30.4);

impl RenderState {
    // Inside the castle: its rooms, its flames, the maids on their rounds; the same Queen.
    pub(super) fn render_castle(&mut self, frame: &Frame, inside: &castle3d::State, bob: f32) {
        let Frame {
            camera,
            player,
            yaw,
            elapsed,
            queen_thought,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            shop_preview_rotation,
            venue,
            environment,
            action_prompt,
            ..
        } = *frame;

        let mut draws = Vec::with_capacity(2048);
        if let Some(scene) = self.castle_scene.as_ref() {
            scene.draws(inside, elapsed, camera.position - camera.target, &mut draws);
        }
        let queen_matrix = Mat4::from_translation(Vec3::new(player.x, player.y + bob, player.z))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
            ));
        for (mesh, material, texture, local) in self.queen_parts(outfit) {
            draws.push(MeshDraw3D {
                mesh,
                transform: queen_matrix * local,
                material: material_for_outfit(material, outfit).with_ambient_occlusion(0.28),
                texture,
            });
        }
        draws.push(MeshDraw3D {
            mesh: self.player_marker,
            transform: Mat4::from_translation(Vec3::new(
                player.x,
                player.y + ROOM_PLAYER_MARKER_Y,
                player.z,
            )),
            material: Material3D {
                base_color: [0.95, 0.30, 0.62, 0.9],
                texture: None,
                occlusion_window: [0.0; 4],
            },
            texture: None,
        });
        // Lira and Nemi on their rounds (they walk; the walk loop is posed above), on the
        // ground floor (under the upper floor's slabs while she is up there).
        let maids = if inside.view_level() == 0 && !inside.keep_walls {
            2
        } else {
            0
        };
        for i in 0..maids {
            let (at, facing) = inside.maid(i);
            let matrix = Mat4::from_translation(at + Vec3::new(0.0, 0.018, 0.0))
                * Mat4::from_rotation_y(facing)
                * Mat4::from_scale(Vec3::new(
                    MAID_WORLD_SCALE,
                    MAID_WORLD_SCALE,
                    MAID_WORLD_SCALE,
                ));
            let (model, meshes) = if i == 0 {
                (Some(&self.maid_model), &self.maid_meshes)
            } else {
                (self.maid_alt_model.as_ref(), &self.maid_alt_meshes)
            };
            let Some(model) = model else { continue };
            for (index, (mesh, material, texture)) in meshes.iter().enumerate() {
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: matrix * model.mesh_transform(index),
                    material: (*material).with_ambient_occlusion(0.28),
                    texture: *texture,
                });
            }
        }
        // Nyxara on her round upstairs (only while the upper floor is in view).
        if inside.view_level() == 1 && !inside.keep_walls {
            if let Some(doc) = self.doctor.as_ref() {
                let d = inside.doctor;
                let clip_pose = |clip: usize, t: f32| {
                    doc.clips
                        .get(clip)
                        .copied()
                        .flatten()
                        .and_then(|c| doc.model.pose(c, t))
                };
                let pose = clip_pose(d.clip, d.since).map(|now| {
                    let k = (d.since / 0.3).clamp(0.0, 1.0);
                    match (k < 1.0 && d.prev != d.clip)
                        .then(|| clip_pose(d.prev, d.since + 1.0))
                        .flatten()
                    {
                        Some(before) => before.blend(&now, k),
                        None => now,
                    }
                });
                if let Some(pose) = pose {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        &doc.model,
                        &doc.meshes,
                        &pose,
                    );
                }
                let level_h = castle3d::plan().level_h;
                let matrix = Mat4::from_translation(d.at + Vec3::new(0.0, level_h + 0.018, 0.0))
                    * Mat4::from_rotation_y(d.facing)
                    * Mat4::from_scale(Vec3::new(
                        MAID_WORLD_SCALE,
                        MAID_WORLD_SCALE,
                        MAID_WORLD_SCALE,
                    ));
                for (index, (mesh, material, texture)) in doc.meshes.iter().enumerate() {
                    draws.push(MeshDraw3D {
                        mesh: *mesh,
                        transform: matrix * doc.model.mesh_transform(index),
                        material: (*material).with_ambient_occlusion(0.28),
                        texture: *texture,
                    });
                }
            }
        }
        if let Some(guide) = frame.mission {
            self.mission_draws(guide, elapsed, &mut draws);
        }
        let mut lights = inside.lights(elapsed);
        if !self.castle_life_tried {
            self.castle_life_tried = true;
            match castle_life::Scene::load(&mut self.renderer, &self.device) {
                Ok(life) => self.castle_life = Some(life),
                Err(error) => eprintln!("[3d] the castle's life failed to build: {error:#}"),
            }
        }
        if let Some(life) = self.castle_life.as_mut() {
            life.draws(
                &mut self.renderer,
                &self.queue,
                inside.life,
                inside.view_level(),
                elapsed,
                camera.position - camera.target,
                &mut draws,
                &mut lights,
            );
        }
        let saved = self.renderer.replace_finish(castle3d::finish());
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            castle3d::mood(environment),
            &lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
        self.renderer.replace_finish(saved);
        let mut bubbles = inside.bubbles(camera);
        bubbles.extend(queen_thought_bubble(
            camera,
            player,
            false,
            map_open,
            wardrobe_open,
            queen_thought,
        ));
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            &self.target_view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            player,
            yaw,
            false,
            false,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            elapsed,
            shop_preview_rotation,
            &bubbles,
            Some(venue),
            action_prompt,
            None,
            false,
        );
    }
}

pub(super) const MAID_WORLD_SCALE: f32 = 2.0;

pub(super) fn maid_idle_comment(agent_index: usize, elapsed: f32) -> &'static str {
    const LIRA: [&str; 4] = [
        "El ala norte está en orden.",
        "Las sábanas están listas.",
        "Debo revisar el agua.",
        "Todo está preparado, Majestad.",
    ];
    const NEMI: [&str; 4] = [
        "¿Quién dejó esto aquí?",
        "Ps ps... Lira, espera.",
        "No se me cayó nada.",
        "La cocina está casi lista.",
    ];
    let slot = (elapsed / 5.5).floor().max(0.0) as usize;
    if agent_index == 0 {
        LIRA[slot % LIRA.len()]
    } else {
        NEMI[slot % NEMI.len()]
    }
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct NpcDialogueLine {
    pub(super) speaker: String,
    pub(super) context: String,
    pub(super) text: String,
}

pub(super) static NPC_DIALOGUE_BANK: OnceLock<Vec<NpcDialogueLine>> = OnceLock::new();

pub(super) fn npc_dialogue_bank() -> &'static [NpcDialogueLine] {
    NPC_DIALOGUE_BANK
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../data/npc_dialogues.json")).unwrap_or_default()
        })
        .as_slice()
}

pub(super) fn authored_maid_line(agent_index: usize, context: &str, elapsed: f32) -> &'static str {
    let speaker = if agent_index == 0 { "Lira" } else { "Nemi" };
    let candidates = npc_dialogue_bank()
        .iter()
        .filter(|line| line.speaker == speaker && line.context == context);
    let count = candidates.count();
    if count == 0 {
        return maid_idle_comment(agent_index, elapsed);
    }
    let slot = ((elapsed / 4.8).floor().max(0.0) as usize + agent_index.wrapping_mul(13)) % count;
    npc_dialogue_bank()
        .iter()
        .filter(|line| line.speaker == speaker && line.context == context)
        .nth(slot)
        .map(|line| line.text.as_str())
        .unwrap_or_else(|| maid_idle_comment(agent_index, elapsed))
}

impl RenderState {
    /// Loads the castle the first time it is wanted; whether it is there to walk in.
    pub(super) fn ensure_castle_loaded(&mut self) -> bool {
        if !self.castle_load_attempted {
            self.castle_load_attempted = true;
            match castle3d::Scene::load(&mut self.renderer, &self.device, &self.queue) {
                Ok(scene) => self.castle_scene = Some(scene),
                Err(error) => eprintln!("[3d] castle failed to load: {error:#}"),
            }
        }
        self.castle_scene.is_some()
    }
}

impl App {
    /// One frame in the castle: walk its rooms, let the walls drop and rise, the maids go round.
    pub(super) fn tick_castle(&mut self, dt: f32) {
        let (view_forward, view_right) = self.iso.walk_axes();
        let mut direction = Vec3::ZERO;
        if self.move_forward {
            direction += view_forward;
        }
        if self.move_back {
            direction = direction - view_forward;
        }
        if self.move_left {
            direction = direction - view_right;
        }
        if self.move_right {
            direction += view_right;
        }
        let busy = self.shop_open
            || self.wardrobe_open
            || self.map_open
            || self.talk.is_some()
            || self.court.is_some();
        let elapsed = self.elapsed;
        let yaw = self.iso.yaw;
        let life = castle_life::life(&self.story);
        self.tick_routine();
        let Some(inside) = self.castle.as_mut() else {
            return;
        };
        inside.life = life;
        let before = inside.position();
        if direction.length_squared() > 0.0 && !busy {
            let direction = direction.normalize_or_zero();
            let speed = if self.sprint { 4.6 } else { 2.2 };
            let steps = ((dt * speed) / 0.05).ceil().max(1.0) as usize;
            for _ in 0..steps {
                inside.walk(
                    direction.x * dt * speed / steps as f32,
                    direction.z * dt * speed / steps as f32,
                );
            }
            self.player_yaw = direction.x.atan2(direction.z);
        }
        // Nemi waits at the arena in the patio from the moment her gift is due (after the first
        // Council) until she has trained the Queen three times.
        let gift_due =
            self.story.get("consejo_1_hecho") > 0 && self.story.get("espada_regalo") == 0;
        let training = self.story.get("espada_regalo") > 0 && self.story.get("entreno") < 3;
        // …and while she trains the Queen there (the court's training).
        let sparring = matches!(
            self.court.as_ref().map(|c| &c.page),
            Some(court::Page::Training(_))
        );
        let training = gift_due || training || sparring;
        inside.maid_post[1] = training.then_some(NEMI_ARENA);
        inside.held = self
            .talk
            .as_ref()
            .and_then(|t| match t.scene_id(guion::library()) {
                id if id.contains("lira") => Some(0),
                id if id.contains("nemi") => Some(1),
                _ => None,
            });
        inside.update(dt, yaw, elapsed);
        let after = inside.position();
        let focus = inside.focus();
        self.player = after;
        let follow = (dt * 3.5).min(1.0);
        self.camera_focus += (focus - self.camera_focus) * follow;
        self.iso.yaw += (self.iso.yaw_target - self.iso.yaw) * (dt * 6.0).min(1.0);
        let stepped = Vec3::new(after.x - before.x, 0.0, after.z - before.z).length();
        self.step_sound_distance += stepped;
        let stride = if self.sprint { 1.5 } else { 1.05 };
        if self.step_sound_distance >= stride {
            self.step_sound_distance -= stride;
            self.play_step();
        }
    }

    pub(super) fn enter_nature_from_castle(&mut self) {
        self.castle = None;
        // Spawn just beyond the castle keep, on the authored trail. This is
        // close enough to read as walking through the gate, while avoiding a
        // teleport back into the keep's collision footprint.
        let spawn_z = -12.0;
        self.nature_mode = true;
        self.village_entered = false;
        self.player = Vec3::new(nature_trail_center(spawn_z), 0.0, spawn_z);
        self.camera_focus = self.player;
        self.player_yaw = 0.0;
        let view_yaw = std::f32::consts::FRAC_PI_4;
        self.iso.yaw = view_yaw;
        self.iso.yaw_target = view_yaw;
        self.iso.top_down = false;
        self.iso.elevation = ISO.elevation;
        self.iso.distance = ISO.max_distance;
        self.map_open = false;
        self.footprints.clear();
        self.footstep_distance = 0.0;
        self.footstep_side = 1.0;
        self.clear_input();
        eprintln!("[3d] exited castle to outdoor terrain");
        self.achievement_event("visit:bosque");
    }

    pub(super) fn enter_castle_from_nature(&mut self) {
        self.penascal = None;
        // The castle from its prototype: in through the great doors of the entrance hall.
        if self
            .state
            .as_mut()
            .is_some_and(|s| s.ensure_castle_loaded())
        {
            let inside = castle3d::State::at_gate();
            self.nature_mode = false;
            self.village_entered = false;
            self.player = inside.position();
            self.camera_focus = inside.focus();
            self.player_yaw = std::f32::consts::PI;
            self.iso.yaw = std::f32::consts::FRAC_PI_4;
            self.iso.yaw_target = self.iso.yaw;
            self.iso.top_down = false;
            self.iso.elevation = ISO.elevation;
            self.iso.distance = ISO.max_distance;
            self.map_open = false;
            self.footprints.clear();
            self.castle = Some(inside);
            self.clear_input();
            eprintln!("[3d] entered the castle from outdoors");
            return;
        }
        eprintln!("[3d] the castle could not load: she stays outside");
    }
}

impl App {
    /// One step in the castle: its rooms load the first time she is there, then she walks them.
    pub(super) fn tick_castle_zone(&mut self, dt: f32) {
        if self.castle.is_none()
            && self
                .state
                .as_mut()
                .is_some_and(|s| s.ensure_castle_loaded())
        {
            // Where her session left her (a story spawn of this castle, or a saved spot): kept
            // if it is free here, else the bedroom.
            let inside = castle3d::State::restore(self.player);
            self.player = inside.position();
            self.camera_focus = inside.focus();
            self.iso.distance = ISO.max_distance;
            self.castle = Some(inside);
            // The first time she walks it (after the prologue's lessons at the chalkboard),
            // Chiquita, from her corner, goes over the keys once more: «estaré aquí abajo».
            if self.story.get("tutorial_castillo") == 0 {
                self.story.values.insert("tutorial_castillo".to_owned(), 1);
                self.float_chiquita_after(
                    "—Ya estás en casa. WASD para andar; lo que brille, con F.",
                    mascot::Cue::Talk,
                    2.0,
                );
                self.float_chiquita_after(
                    "—La M, el mapa. Esc, el menú y tu estado. Y duerme en tu cama para pasar el día.",
                    mascot::Cue::Talk,
                    9.5,
                );
            }
        }
        if self.castle.is_some() {
            self.tick_castle(dt);
        }
        // Without its scene there is no castle to walk in.
        self.elapsed += dt;
    }
}

impl App {
    /// F in the castle: a guion for this person or place first, then the place's own use. Whether that ends the key's handling.
    pub(super) fn interact_castle(&mut self) -> bool {
        // A guion written for this person or place plays first (the gate, the
        // wardrobe and the bed keep their own use).
        let near = self
            .castle
            .as_ref()
            .and_then(|c| c.nearest())
            .map(|h| h.id.clone());
        // Her own bed: its menu, lying in it.
        if near.as_deref() == Some("cama") {
            self.open_alcove();
            return true;
        }
        // The court's places: the throne's audiences, Lira's accounts, the restored rooms, the
        // portraits, Nyxara's lessons and Nemi's arena.
        if let Some(id) = near.as_deref() {
            if self.interact_court(id) {
                return true;
            }
        }
        if let Some(id) = near.as_deref() {
            let person = matches!(id, "lira" | "nemi");
            let own_use = matches!(id, castle3d::GATE | "vestidor");
            let started = if person {
                self.start_talk(Some(id), None)
            } else if !own_use {
                self.start_talk(None, Some(id))
            } else {
                false
            };
            if started {
                return true;
            }
        }
        let elapsed = self.elapsed;
        let action = self.castle.as_mut().map(|inside| inside.interact(elapsed));
        match action {
            Some(castle3d::Action::Leave) => {
                self.enter_nature_from_castle();
                self.persist_3d_state(true);
            }
            Some(castle3d::Action::Wardrobe) => {
                self.map_open = false;
                self.wardrobe_open = true;
                self.wardrobe_selection = self.outfit;
                self.clear_input();
            }
            Some(castle3d::Action::Sleep) => self.sleep_in_bed(),
            Some(castle3d::Action::Say(text)) => {
                self.queen_thought = Some((text, self.elapsed + 4.5));
            }
            _ => {}
        }
        false
    }
}

impl App {
    /// A scene of the story (one that plays once) waits at this place.
    fn story_scene_at(&self, place: &str) -> bool {
        story_waits(guion::library(), place, &self.story)
    }

    /// F on one of the court's places. Whether it took the key (else the place's guion or its own
    /// use, as before).
    pub(super) fn interact_court(&mut self, id: &str) -> bool {
        match id {
            "trono" => self.open_audience(),
            "mesa-guerra" => {
                // a story scene at the table (Act 1's councils) plays before the war's own menu
                if self.story_scene_at("mesa-guerra") {
                    return self.start_talk(None, Some("mesa-guerra"));
                }
                self.open_war_table();
                true
            }
            "despensa" => self.open_works(),
            "armero" => {
                if self.story.get("obra_armeria") > 0 {
                    self.open_armory();
                    true
                } else {
                    self.float_chiquita_cue(
                        "—Lanzas oxidadas y escudos sin correas. Si Lira restaura la armería (las obras, en la despensa), aquí habrá espadas que comprar.",
                        mascot::Cue::Talk,
                    );
                    true
                }
            }
            "botiquin" => self.use_cabinet(),
            "hogar" => self.use_kitchen(),
            "literas" => self.use_barracks(),
            "retratos" => {
                self.gallery_pending = true;
                ui_sfx::play(ui_sfx::UiSfx::Open);
                true
            }
            "arena" => self.open_training(),
            "nyxara" => self.open_lessons(),
            "catalejo" => self.open_telescope(),
            "estudio" => self.open_codex(),
            // the red sofas of the great hall: whom to date (the vanity keeps its mirror)
            "sofa" => self.open_dates(),
            _ => false,
        }
    }
}

impl App {
    /// Sleeping: to the morning if it is night, to the night otherwise. A new day lets the body
    /// be explored again (`explorado_hoy`).
    pub(super) fn sleep_in_bed(&mut self) {
        let phase = self.environment_cycle.phase();
        let night = !(0.2..=0.8).contains(&phase);
        self.environment_cycle
            .set_phase(if night { 0.3 } else { 0.95 });
        let thought = if night {
            "Buenos días. He dormido de maravilla."
        } else {
            "Una siesta... y ya es de noche."
        };
        self.queen_thought = Some((thought, self.elapsed + 4.5));
        self.story.values.insert("explorado_hoy".to_owned(), 0);
        // The court's daily uses come back (and a guard's morning brings the village's thanks).
        self.court_new_day(night);
        // A new day: the shopkeeper dares one more favour.
        self.story
            .values
            .insert(play_scene::tienda::TODAY.to_owned(), 0);
        self.night_after_sleep();
        self.persist_3d_state(true);
    }
}

/// The castle as a place.
pub(super) struct CastleZone;

impl Zone for CastleZone {
    fn id(&self) -> ZoneId {
        ZoneId::Castle
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_castle_zone(dt);
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        app.castle.as_ref().and_then(|inside| inside.prompt())
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_castle()
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Castle
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, _) = state.prepare_above_ground(frame);
        if let Some(inside) = frame.castle_state {
            if state.castle_scene.is_some() {
                state.render_castle(frame, inside, bob);
            }
        }
        // Without its scene there is nothing to draw.
    }
    fn title(&self, _app: &App) -> (&'static str, &'static str) {
        ("Cámara Real 3D [N: naturaleza]", "")
    }
}

/// Nyxara, the castle's doctor (`tools/doctor_blender.py`): her model, its clips in
/// `castle3d::DOCTOR_CLIPS` order, and its meshes.
pub(super) struct DoctorAssets {
    pub(super) model: GltfModel,
    pub(super) clips: Vec<Option<usize>>,
    pub(super) meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
}

/// Load her, if her model is there.
pub(super) fn load_doctor(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white_texture: &TextureData3D,
) -> Option<DoctorAssets> {
    let file = runtime_3d_root().join("characters").join("nyxara.glb");
    if !file.exists() {
        eprintln!("[3d] the doctor's model is missing: {}", file.display());
        return None;
    }
    let model = match GltfModel::load(&file) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[3d] the doctor's model failed: {e:#}");
            return None;
        }
    };
    let clips = castle3d::DOCTOR_CLIPS
        .iter()
        .map(|c| model.animation_index(c))
        .collect();
    let mut meshes = Vec::with_capacity(model.meshes.len());
    for mesh in &model.meshes {
        let handle = renderer.upload_skinned_mesh(device, mesh).ok()?;
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        let texture = upload_texture_pair(
            renderer,
            device,
            queue,
            albedo,
            normal,
            model_texture_cache,
            white_texture,
        )
        .ok()?;
        meshes.push((handle, mesh.material, texture));
    }
    eprintln!("[3d] doctor meshes={} clips={:?}", meshes.len(), clips);
    Some(DoctorAssets {
        model,
        clips,
        meshes,
    })
}

/// Lira and Nemi, the castle's maids: their models, walks and meshes.
pub(super) struct MaidAssets {
    pub(super) maid_model: GltfModel,
    pub(super) maid_walk_animation: Option<usize>,
    pub(super) maid_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    pub(super) maid_alt_model: Option<GltfModel>,
    pub(super) maid_alt_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    pub(super) maid_alt_walk_animation: Option<usize>,
}

/// Load it: lira and Nemi, the castle's maids: their models, walks and meshes.
pub(super) fn load_maids(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white_texture: &TextureData3D,
) -> Result<MaidAssets> {
    let maid_file = maid_model_path();
    let maid_model = GltfModel::load(&maid_file)
        .with_context(|| format!("load authored maid GLB: {}", maid_file.display()))?;
    if maid_model.meshes.is_empty() {
        anyhow::bail!("Maid GLB contains no mesh primitives");
    }
    let maid_walk_animation = maid_model.animation_index("Maid_Walk");
    eprintln!(
        "[3d] maid meshes={} vertices={} animations={:?} walk={maid_walk_animation:?}",
        maid_model.meshes.len(),
        maid_model
            .meshes
            .iter()
            .map(|mesh| mesh.vertices.len())
            .sum::<usize>(),
        maid_model.animations
    );
    let mut maid_meshes = Vec::with_capacity(maid_model.meshes.len());
    for mesh in &maid_model.meshes {
        let handle = renderer.upload_skinned_mesh(&device, mesh)?;
        let albedo_data = mesh
            .texture_index
            .and_then(|index| maid_model.textures.get(index));
        let normal_data = mesh
            .normal_texture_index
            .and_then(|index| maid_model.textures.get(index));
        let texture = upload_texture_pair(
            renderer,
            &device,
            &queue,
            albedo_data,
            normal_data,
            model_texture_cache,
            &white_texture,
        )?;
        maid_meshes.push((handle, mesh.material, texture));
    }
    let maid_alt_file = maid_alt_model_path();
    let (maid_alt_model, maid_alt_meshes, maid_alt_walk_animation) = if maid_alt_file.exists() {
        let model = GltfModel::load(&maid_alt_file)
            .with_context(|| format!("load alternate maid GLB: {}", maid_alt_file.display()))?;
        let walk_animation = model.animation_index("Maid_Walk");
        eprintln!(
            "[3d] alternate maid meshes={} vertices={} animations={:?} walk={walk_animation:?}",
            model.meshes.len(),
            model
                .meshes
                .iter()
                .map(|mesh| mesh.vertices.len())
                .sum::<usize>(),
            model.animations
        );
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for mesh in &model.meshes {
            let handle = renderer.upload_skinned_mesh(&device, mesh)?;
            let albedo_data = mesh
                .texture_index
                .and_then(|index| model.textures.get(index));
            let normal_data = mesh
                .normal_texture_index
                .and_then(|index| model.textures.get(index));
            let texture = upload_texture_pair(
                renderer,
                &device,
                &queue,
                albedo_data,
                normal_data,
                model_texture_cache,
                &white_texture,
            )?;
            meshes.push((handle, mesh.material, texture));
        }
        (Some(model), meshes, walk_animation)
    } else {
        eprintln!(
            "[3d] alternate maid asset missing; skipping {}",
            maid_alt_file.display()
        );
        (None, Vec::new(), None)
    };
    Ok(MaidAssets {
        maid_model,
        maid_walk_animation,
        maid_meshes,
        maid_alt_model,
        maid_alt_meshes,
        maid_alt_walk_animation,
    })
}

/// Whether the scene a place would play now is one of the story's (it plays once), not the place's
/// everyday words.
fn story_waits(lib: &guion::Library, place: &str, story: &guion::Vars) -> bool {
    lib.entry(None, Some(place), story)
        .is_some_and(|(f, s)| lib.guiones[f].scenes[s].attrs.una_vez)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_war_table_lets_act_1_councils_play_before_its_menu() {
        let lib = guion::library();
        let mut v = guion::Vars::default();
        // the first council waits at the table: F plays it, not the war's menu
        let (f, s) = lib
            .entry(None, Some("mesa-guerra"), &v)
            .expect("a scene at the table");
        assert!(story_waits(lib, "mesa-guerra", &v));
        assert!(
            lib.guiones[f].scenes[s].id.ends_with("consejo_1"),
            "{}",
            lib.guiones[f].scenes[s].id
        );
        // once seen, the table's own words are no story: the menu opens
        v.seen.insert(lib.guiones[f].scenes[s].id.clone());
        assert!(!story_waits(lib, "mesa-guerra", &v));
        // Varrak seen: the second council waits there
        v.values.insert("varrak_visto".into(), 1);
        assert!(story_waits(lib, "mesa-guerra", &v));
    }
}
