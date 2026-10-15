//! El Peñascal como lugar: subir del bosque por el paso, recorrer la aldea, Varrak en su piedra (o
//! de rodillas, ya despierto) y, al desafiarlo, el combate en primera persona.
use super::*;

impl RenderState {
    /// Loads el Peñascal the first time it is wanted; whether it is there to walk in.
    pub(super) fn ensure_penascal_loaded(&mut self) -> bool {
        if !self.penascal_load_attempted {
            self.penascal_load_attempted = true;
            if penascal3d::available() {
                match penascal3d::Scene::load(&mut self.renderer, &self.device, &self.queue) {
                    Ok(scene) => self.penascal_scene = Some(scene),
                    Err(error) => eprintln!("[3d] the Peñascal failed to load: {error:#}"),
                }
            } else {
                eprintln!("[3d] the Peñascal is not exported yet (demon-queen-penascal-threejs: node tools/export-game.mjs)");
            }
        }
        self.penascal_scene.is_some()
    }

    /// In el Peñascal: the village, Varrak on his stone, the same Queen.
    pub(super) fn render_penascal(&mut self, frame: &Frame, inside: &penascal3d::State, bob: f32) {
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
            varrak_awake,
            mission,
            ..
        } = *frame;

        let mut draws = Vec::with_capacity(2048);
        if let Some(scene) = self.penascal_scene.as_ref() {
            scene.draws(inside, elapsed, camera.position - camera.target, &mut draws);
        }
        // Varrak: slumped on the stone, breathing slow; once awake, on his knees in front of it.
        if let Some(v) = self.penascal_scene.as_ref().and_then(|s| s.varrak.as_ref()) {
            let stone = penascal3d::plan().stone;
            let (clip, t, at) = if varrak_awake {
                (
                    v.kneel,
                    2.5,
                    Vec3::new(stone.x + 0.6, 0.0, stone.z + stone.d / 2.0 + 1.2),
                )
            } else {
                (
                    v.sit,
                    elapsed.rem_euclid(4.0),
                    Vec3::new(stone.x, 0.1, stone.z - 0.1),
                )
            };
            if let Some(pose) = clip.and_then(|c| v.model.pose(c, t)) {
                pose_skinned_meshes(&mut self.renderer, &self.queue, &v.model, &v.meshes, &pose);
            }
            let matrix = Mat4::from_translation(at)
                * Mat4::from_scale(Vec3::new(
                    penascal3d::VARRAK_SCALE,
                    penascal3d::VARRAK_SCALE,
                    penascal3d::VARRAK_SCALE,
                ));
            for (index, (mesh, material, texture)) in v.meshes.iter().enumerate() {
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: matrix * v.model.mesh_transform(index),
                    material: (*material).with_ambient_occlusion(0.28),
                    texture: *texture,
                });
            }
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
        if let Some(guide) = mission {
            self.mission_draws(guide, elapsed, &mut draws);
        }
        let lights = inside.lights(elapsed);
        let saved = self.renderer.replace_finish(penascal3d::finish());
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            penascal3d::mood(environment),
            &lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
        self.renderer.replace_finish(saved);
        let bubbles: Vec<_> = queen_thought_bubble(
            camera,
            player,
            false,
            map_open,
            wardrobe_open,
            queen_thought,
        )
        .into_iter()
        .collect();
        self.map_overlay.penascal = true;
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
        self.map_overlay.penascal = false;
    }
}

/// Where she is saved while in el Peñascal (and stands when she comes down): in the wood, a couple
/// of steps from the foot of its way (so the next F does not climb again), and which way she faces.
pub(super) fn penascal_return() -> (Vec3, f32) {
    let at = dark_forest::exit("penascal")
        .map(|(p, _)| p)
        .unwrap_or(Vec3::new(-23.0, 0.0, -5.0));
    let back = Vec3::new(at.x + 2.2, 0.0, at.z + 0.4);
    if nature_position_blocked(back) {
        (
            Vec3::new(at.x + 2.6, 0.0, at.z),
            std::f32::consts::FRAC_PI_2,
        )
    } else {
        (back, std::f32::consts::FRAC_PI_2)
    }
}

impl App {
    /// Up the way from the wood: she stands at the gorge's far end, the plaza ahead.
    pub(super) fn enter_penascal_from_forest(&mut self) {
        if !self
            .state
            .as_mut()
            .is_some_and(|s| s.ensure_penascal_loaded())
        {
            self.queen_thought = Some((
                "El camino sube entre peñas… pero hoy no llego.",
                self.elapsed + 4.0,
            ));
            return;
        }
        let inside = penascal3d::State::way_in();
        self.enter_penascal(inside);
        // The village seen once: the act 1 mission's step (and an achievement's event).
        self.story.values.insert("penascal_visto".to_owned(), 1);
        self.achievement_event("visit:penascal");
        eprintln!("[3d] up to el Peñascal from the forest");
    }

    /// Back in the plaza, by the stone (after the fight).
    pub(super) fn enter_penascal_by_the_stone(&mut self) {
        if !self
            .state
            .as_mut()
            .is_some_and(|s| s.ensure_penascal_loaded())
        {
            return;
        }
        self.enter_penascal(penascal3d::State::by_the_stone());
    }

    /// Back inside the place a game was saved in: through its door as usual (the brothel's and
    /// the baths' welcome play again if they were not seen), then where she stood.
    pub(crate) fn resume_inside(&mut self, inside: &exploration3d_state::Inside) {
        let back = self.player;
        match inside.place.as_str() {
            "burdel" => {
                self.enter_brothel();
                if let Some(b) = self.brothel.as_mut() {
                    let walker = brothel::Walker {
                        x: inside.x,
                        z: inside.z,
                        level: inside.level,
                        on_stair: false,
                    };
                    if !brothel::plan().blocked(walker.x, walker.z, walker.level) {
                        b.walker = walker;
                    }
                    b.return_to = back;
                    self.player = b.position();
                }
            }
            "termas" => {
                self.enter_baths();
                if let Some(b) = self.baths.as_mut() {
                    b.room = inside.room.clone();
                    b.x = inside.x;
                    b.z = inside.z;
                    b.return_to = back;
                    self.player = b.position();
                }
                // What she came in still waits in the changing room.
                if inside.outfit_before.is_some() {
                    self.outfit_before_baths = inside.outfit_before;
                }
            }
            "penascal" => {
                self.enter_penascal(penascal3d::State::restore(Vec3::new(
                    inside.x, 0.0, inside.z,
                )));
            }
            _ => return,
        }
        self.player_yaw = inside.yaw;
        self.camera_focus = self.player;
        eprintln!(
            "[3d] de vuelta dentro: {} {:.2},{:.2}",
            inside.place, inside.x, inside.z
        );
    }

    fn enter_penascal(&mut self, inside: penascal3d::State) {
        self.nature_mode = true;
        self.village_entered = false;
        self.castle = None;
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
        self.penascal = Some(inside);
        self.clear_input();
    }

    /// Down the gorge to the wood: she stands on the way, at its foot.
    pub(super) fn leave_penascal_to_forest(&mut self) {
        self.penascal = None;
        self.nature_mode = true;
        self.village_entered = false;
        let (back, yaw) = penascal_return();
        self.player = back;
        self.camera_focus = back;
        self.player_yaw = yaw;
        self.iso.yaw = std::f32::consts::FRAC_PI_4;
        self.iso.yaw_target = self.iso.yaw;
        self.map_open = false;
        self.footprints.clear();
        self.clear_input();
        eprintln!("[3d] down from el Peñascal to the forest");
    }

    /// One frame in el Peñascal: walk its places, let the cliffs drop and rise.
    pub(super) fn tick_penascal(&mut self, dt: f32) {
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
        let busy = self.shop_open || self.wardrobe_open || self.map_open || self.talk.is_some();
        let yaw = self.iso.yaw;
        let sprint = self.sprint;
        let Some(inside) = self.penascal.as_mut() else {
            self.elapsed += dt;
            return;
        };
        let before = inside.position();
        if direction.length_squared() > 0.0 && !busy {
            let direction = direction.normalize_or_zero();
            let speed = if sprint { 4.6 } else { 2.2 };
            let steps = ((dt * speed) / 0.05).ceil().max(1.0) as usize;
            for _ in 0..steps {
                inside.walk(
                    direction.x * dt * speed / steps as f32,
                    direction.z * dt * speed / steps as f32,
                );
            }
            self.player_yaw = direction.x.atan2(direction.z);
        }
        inside.update(dt, yaw);
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
        self.elapsed += dt;
    }

    /// Whether Varrak has woken (after the fight).
    pub(super) fn varrak_awake(&self) -> bool {
        self.story.get("varrak_despierto") > 0 || self.story.get("varrak_vencido") > 0
    }

    /// F in el Peñascal: down to the wood, the chief, or a look at something.
    pub(super) fn interact_penascal(&mut self) -> bool {
        let action = self.penascal.as_ref().map(|inside| inside.interact());
        match action {
            Some(penascal3d::Action::Leave) => {
                self.leave_penascal_to_forest();
                self.persist_3d_state(true);
                true
            }
            Some(penascal3d::Action::Varrak) => {
                self.challenge_varrak();
                true
            }
            Some(penascal3d::Action::Say(text)) => {
                self.queen_thought = Some((text, self.elapsed + 5.0));
                true
            }
            _ => false,
        }
    }

    /// R by Varrak: the fight at once, whatever the story says. Before the story reaches him it
    /// is a rehearsal (winning it changes nothing); once he is awake, a rematch.
    pub(super) fn fight_varrak_now(&mut self) {
        if !self
            .penascal
            .as_ref()
            .and_then(|p| p.nearest())
            .is_some_and(|h| h.id == "varrak")
        {
            return;
        }
        self.boss_practice = self.story.get("consejo_2_hecho") == 0 && !self.varrak_awake();
        self.start_boss_fight();
    }

    /// The chief on his stone: not before the Council has decided (she does not even know who he
    /// is); then the fight; once he is awake, a word with him.
    pub(super) fn challenge_varrak(&mut self) {
        if self.varrak_awake() {
            self.queen_thought = Some((
                "«Tenemos una partida pendiente, Majestad.» Respira como un fuelle roto, pero sonríe.",
                self.elapsed + 5.0,
            ));
            return;
        }
        if self.story.get("consejo_2_hecho") == 0 {
            self.float_chiquita_after(
                "—Todavía no. Ni siquiera sabes quién es. Y yo no estoy lista para que lo sepas.",
                mascot::Cue::Scold,
                0.2,
            );
            self.queen_thought = Some((
                "Un orco enorme. No parpadea. No le gusto.",
                self.elapsed + 4.5,
            ));
            return;
        }
        // His arrival (the guion's narration) the first time, then the fight in first person (its
        // plaza read while the arrival is told).
        if let Some(state) = self.state.as_mut() {
            state.ensure_boss_loaded();
        }
        if self.story.seen("acto1.varrak_llega") {
            self.start_boss_fight();
            return;
        }
        self.pending_boss_fight = true;
        if !self.start_talk_at("acto1.varrak_llega") {
            self.pending_boss_fight = false;
            self.start_boss_fight();
        }
    }
}

/// El Peñascal as a place.
pub(super) struct PenascalZone;

impl Zone for PenascalZone {
    fn id(&self) -> ZoneId {
        ZoneId::Penascal
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_penascal(dt);
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        let awake = app.varrak_awake();
        let inside = app.penascal.as_ref()?;
        let prompt = inside.prompt(awake)?;
        // By Varrak, always a second button: fight him now (a rehearsal before the story gets
        // there, a rematch once he is awake).
        Some(if inside.nearest().is_some_and(|h| h.id == "varrak") {
            prompt.also(
                "R",
                if awake {
                    "Revancha"
                } else if app.story.get("consejo_2_hecho") == 0 {
                    "Luchar (ensayo)"
                } else {
                    "Luchar ya"
                },
            )
        } else {
            prompt
        })
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_penascal()
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Forest
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, _) = state.prepare_above_ground(frame);
        if let Some(inside) = frame.penascal_state {
            // Loaded on first sight (a game saved in el Peñascal starts in it).
            if state.ensure_penascal_loaded() {
                state.render_penascal(frame, inside, bob);
            }
        }
    }
    fn title(&self, _app: &App) -> (&'static str, &'static str) {
        ("El Peñascal", "")
    }
}
