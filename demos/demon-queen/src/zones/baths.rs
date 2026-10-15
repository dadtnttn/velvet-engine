//! Las termas como lugar: lo que el mundo hace con ellas (dibujarlas, recorrerlas, entrar y salir).
use super::*;

impl RenderState {
    // At the hot springs: the valley or one of its rooms; the same Queen.
    pub(super) fn render_baths(&mut self, frame: &Frame, inside: &baths::State, bob: f32) {
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

        let mut draws = Vec::with_capacity(4096);
        let moon = environment.celestial.moon_visibility;
        if let Some(scene) = self.baths_scene.as_ref() {
            scene.draws(inside, elapsed, moon, &mut draws);
        }
        // Wading, she stands in the water up to her knees.
        let queen_matrix = Mat4::from_translation(Vec3::new(player.x, player.y + bob, player.z))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
            ));
        let queen_parts = self.queen_parts(outfit);
        for (mesh, material, texture, local) in queen_parts {
            draws.push(MeshDraw3D {
                mesh,
                transform: queen_matrix * local,
                material: material_for_outfit(material, outfit).with_ambient_occlusion(0.28),
                texture,
            });
        }
        if inside.wading() < 0.2 {
            draws.push(MeshDraw3D {
                mesh: self.player_marker,
                transform: Mat4::from_translation(Vec3::new(player.x, player.y + 0.03, player.z)),
                material: Material3D {
                    base_color: [0.95, 0.30, 0.62, 0.9],
                    texture: None,
                    occlusion_window: [0.0; 4],
                },
                texture: None,
            });
        }
        let indoors = inside.room.is_some();
        let lights = inside.lights(elapsed, moon);
        let mood = if indoors {
            baths::room_mood(environment)
        } else {
            village::mood(environment)
        };
        let saved = (
            self.renderer.replace_finish(village::finish()),
            self.renderer.replace_toon_style(village::toon_style()),
        );
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            mood,
            &lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
        self.renderer.replace_finish(saved.0);
        self.renderer.replace_toon_style(saved.1);
        let bubbles = queen_thought_bubble(
            camera,
            player,
            false,
            map_open,
            wardrobe_open,
            queen_thought,
        )
        .into_iter()
        .collect::<Vec<_>>();
        // In a room, the map shows her at its door.
        let map_player = match inside.room.as_deref() {
            Some(id) => baths::plan()
                .hotspots
                .iter()
                .find(|h| h.enter.as_deref() == Some(id))
                .map(|h| Vec3::new(h.x, 0.0, h.z))
                .unwrap_or(player),
            None => player,
        };
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            &self.target_view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            map_player,
            yaw,
            true,
            true,
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
            true,
        );
    }
}

impl RenderState {
    pub(super) fn ensure_baths_loaded(&mut self) {
        if self.baths_load_attempted {
            return;
        }
        self.baths_load_attempted = true;
        match baths::Scene::load(&mut self.renderer, &self.device, &self.queue) {
            Ok(scene) => self.baths_scene = Some(scene),
            Err(error) => eprintln!("[3d] baths failed to load: {error:#}"),
        }
    }
}

impl App {
    pub(super) fn enter_baths(&mut self) {
        let inside = baths::State::enter(self.player, self.player_yaw);
        self.player = inside.position();
        self.camera_focus = self.player;
        self.player_yaw = std::f32::consts::PI;
        self.iso.yaw = std::f32::consts::FRAC_PI_4;
        self.iso.yaw_target = self.iso.yaw;
        self.iso.top_down = false;
        self.iso.elevation = ISO.elevation;
        self.iso.distance = ISO.max_distance;
        self.map_open = false;
        self.footprints.clear();
        self.baths = Some(inside);
        self.change_into_bikini();
        self.queen_thought = Some(("El vapor huele a azufre y a yuzu.", self.elapsed + 4.0));
        self.clear_input();
        eprintln!("[3d] entered the baths");
        self.achievement_event("visit:termas");
        // The first visit: the receptionist and Mugo, on the stage (`termas.md`, «bienvenida»).
        if self.story.get("termas_bienvenida") == 0 {
            self.queen_thought = None;
            self.start_talk_at("termas.bienvenida");
        }
    }

    pub(super) fn leave_baths(&mut self) {
        if let Some(inside) = self.baths.take() {
            self.player = inside.return_to;
            self.player_yaw = inside.return_yaw + std::f32::consts::PI;
        }
        self.camera_focus = self.player;
        self.iso.distance = ISO.max_distance;
        self.clear_input();
        // Her own clothes back (out in the bikini too soon, Chiquita will not have it).
        self.left_baths_in_bikini();
        eprintln!("[3d] left the baths");
    }

    /// One frame at the springs: walk the valley (wading slows her) or a room.
    pub(super) fn tick_baths(&mut self, dt: f32) {
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
        let others = self
            .state
            .as_ref()
            .and_then(|s| s.baths_scene.as_ref())
            .map(|scene| scene.obstacles())
            .unwrap_or_default();
        let mugo = self
            .state
            .as_ref()
            .and_then(|s| s.baths_scene.as_ref())
            .and_then(|scene| scene.mugo_report());
        let hour = self.environment_cycle.phase() * 24.0;
        let green = self.outfit % wardrobe_outfit_count() == 1;
        let outfit = self.outfit;
        let relax_clips = self
            .state
            .as_ref()
            .map(
                |s| match s.extra_outfits.iter().find(|e| e.slot == outfit) {
                    Some(e) => e.relax,
                    None if green && s.green_model.is_some() => s.green_relax,
                    None => s.relax,
                },
            )
            .unwrap_or_default();
        let Some(inside) = self.baths.as_mut() else {
            return;
        };
        inside.hour = hour;
        inside.others = others;
        inside.mugo = mugo;
        // Asked to (`@var mugo_guia`), Mugo leads her to the first spot not photographed yet;
        // with all of them taken he goes back to his post.
        let story = &self.story;
        inside.guide = if story.get("mugo_guia") > 0 {
            mugo::next_spot(|id| story.get(&format!("foto_{id}")) > 0)
        } else {
            None
        };
        // Taking it easy: still in a pool, she sits back after a moment; any step gets her up.
        if direction.length_squared() > 0.0 && !self.shop_open {
            inside.still = 0.0;
            inside.relax = baths::Relax::None;
        } else {
            inside.still += dt;
            if inside.relax == baths::Relax::None
                && inside.room.is_none()
                && relax_clips.sit.is_some()
                && inside.wading() > 0.6
                && inside.still > 1.4
            {
                inside.relax = baths::Relax::Soaking;
            }
        }
        let toward = if inside.relax != baths::Relax::None {
            1.0
        } else {
            0.0
        };
        inside.relax_blend += (toward - inside.relax_blend).clamp(-dt / 0.45, dt / 0.45);
        if inside.room.is_some() {
            self.iso.yaw_target = std::f32::consts::FRAC_PI_4;
        }
        let before = inside.position();
        if direction.length_squared() > 0.0 && !self.shop_open {
            let direction = direction.normalize_or_zero();
            let speed = (if self.sprint { 5.0 } else { 2.6 }) * (1.0 - 0.45 * inside.wading());
            let steps = ((dt * speed) / 0.05).ceil().max(1.0) as usize;
            for _ in 0..steps {
                inside.walk(
                    direction.x * dt * speed / steps as f32,
                    direction.z * dt * speed / steps as f32,
                );
            }
            self.player_yaw = direction.x.atan2(direction.z);
        }
        let standing = inside.position();
        // Where her body goes when she relaxes (eased from where she stands).
        let e = inside.relax_blend * inside.relax_blend * (3.0 - 2.0 * inside.relax_blend);
        let after = match inside.relax {
            baths::Relax::Soaking => {
                let level = baths::plan()
                    .pool_at(inside.x, inside.z)
                    .map_or(standing.y, |p| p.level);
                // Her chest a hand under the surface.
                let y = level - relax_clips.sit_chest * QUEEN_WORLD_SCALE + 0.12;
                Vec3::new(standing.x, standing.y + (y - standing.y) * e, standing.z)
            }
            baths::Relax::Lying { x, z, y, yaw } => {
                // Her clip is the spa girl's: lying as the girl lies there, the same turn.
                self.player_yaw = yaw;
                let top = y - relax_clips.lie_low * QUEEN_WORLD_SCALE + 0.05;
                Vec3::new(
                    standing.x + (x - standing.x) * e,
                    standing.y + (top - standing.y) * e,
                    standing.z + (z - standing.z) * e,
                )
            }
            baths::Relax::None => standing,
        };
        self.player = after;
        self.camera_focus = Vec3::new(after.x, standing.y, after.z);
        let stepped = Vec3::new(after.x - before.x, 0.0, after.z - before.z).length();
        self.step_sound_distance += stepped;
        let stride = if self.sprint { 1.5 } else { 1.05 };
        if self.step_sound_distance >= stride {
            self.step_sound_distance -= stride;
            self.play_step();
        }
    }
}

impl App {
    /// F at the springs: through a door, out to the valley, or her thought. Whether that ends the key's handling.
    pub(super) fn interact_baths(&mut self) -> bool {
        // A photo where Mugo has brought her (its stage scene shows the picture), or a word with him.
        let photo = self.baths.as_ref().and_then(|inside| inside.photo_here());
        if let Some(spot) = photo {
            let dusk = !(6.5..16.5).contains(&(self.environment_cycle.phase() * 24.0));
            self.story
                .values
                .insert("hora_tarde".to_owned(), i32::from(dusk));
            self.start_talk(
                None,
                Some(&format!("termas_foto_{}", mugo::PHOTO_SPOTS[spot].id)),
            );
            return false;
        }
        if self.baths.as_ref().is_some_and(|inside| inside.mugo_near()) {
            self.start_talk(None, Some("termas_mugo"));
            return false;
        }
        let action = self.baths.as_mut().map(|inside| inside.interact());
        match action {
            Some(baths::Action::Leave) => {
                self.leave_baths();
                self.persist_3d_state(true);
            }
            Some(baths::Action::Say(text)) => {
                self.queen_thought = Some((text, self.elapsed + 4.5));
            }
            _ => {
                if let Some(room) = self.baths.as_ref().and_then(|b| b.room.clone()) {
                    let event = format!("visit:termas:{room}");
                    // Just walked in from the valley (not standing up inside).
                    if self.player.x < 200.0 {
                        self.achievement_event(&event);
                    }
                }
                if let Some(inside) = &self.baths {
                    self.player = inside.position();
                    self.camera_focus = self.player;
                    self.iso.distance = if inside.room.is_some() {
                        ISO.max_distance * 0.62
                    } else {
                        ISO.max_distance
                    };
                }
            }
        }
        false
    }
}

/// The hot springs as a place.
pub(super) struct BathsZone;

impl Zone for BathsZone {
    fn id(&self) -> ZoneId {
        ZoneId::Baths
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_baths(dt);
        app.elapsed += dt;
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        app.baths.as_ref().and_then(|inside| inside.prompt())
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_baths()
    }
    fn soundtrack(&self, app: &App) -> soundtrack::Zone {
        // The springs open from the villa: they keep its music (or the wood's).
        if app.village_entered {
            soundtrack::Zone::Village
        } else {
            soundtrack::Zone::Forest
        }
    }
    fn emitters(&self, app: &App) -> &'static [game_audio::Emitter] {
        app.baths
            .as_ref()
            .map_or(&[], |b| baths::emitters(b.room.as_deref()))
    }
    fn springs(&self, app: &App) -> f32 {
        match &app.baths {
            Some(b) if b.room.is_none() => 1.0,
            Some(_) => 0.25,
            None => 0.0,
        }
    }
    fn ground(&self, app: &App) -> game_audio::Ground {
        // The ground underfoot decides: water, boards, stone, grass.
        match &app.baths {
            Some(b) if b.room.is_some() => game_audio::Ground::Wood,
            Some(b) => baths::plan().ground_kind(b.x, b.z),
            None => game_audio::Ground::of(self.soundtrack(app)),
        }
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, _) = state.prepare_above_ground(frame);
        if let Some(inside) = frame.baths_state {
            state.render_baths(frame, inside, bob);
        }
    }
    fn title(&self, app: &App) -> (&'static str, &'static str) {
        app.outdoors_title()
    }
}
