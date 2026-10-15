//! El burdel como lugar: lo que el mundo hace con él (dibujarlo, recorrerlo, entrar y salir).
use super::*;

impl RenderState {
    // Inside the brothel: its own rooms, lamps, people and map; the same Queen.
    pub(super) fn render_brothel(&mut self, frame: &Frame, inside: &brothel::State, bob: f32) {
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
            drink,
            ..
        } = *frame;

        self.ensure_brothel_loaded();
        let mut draws = Vec::with_capacity(512);
        if let Some(scene) = self.brothel_scene.as_mut() {
            scene.draws(
                inside,
                &mut self.renderer,
                &self.queue,
                elapsed,
                camera.position - camera.target,
                &mut draws,
            );
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
        if let Some(t) = drink {
            if let Some(hand) = self.queen_hand(outfit, t) {
                elixir::bottle_draws(&self.elixir_bottle, queen_matrix, hand, t, &mut draws);
            }
        }
        if drink.is_none() {
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
        }
        let lights = inside.lights(elapsed);
        let saved = self.renderer.replace_finish(brothel::finish());
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            brothel::mood(environment),
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
        let plan = brothel::plan();
        let level = (plan.view_level(inside.walker), inside.rope_open);
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            &self.target_view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            player,
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
            Some(level),
            false,
        );
    }
}

impl RenderState {
    pub(super) fn ensure_brothel_loaded(&mut self) {
        if self.brothel_load_attempted {
            return;
        }
        self.brothel_load_attempted = true;
        match brothel::Scene::load(&mut self.renderer, &self.device, &self.queue) {
            Ok(scene) => self.brothel_scene = Some(scene),
            Err(error) => eprintln!("[3d] brothel failed to load: {error:#}"),
        }
    }
}

impl App {
    pub(super) fn enter_brothel(&mut self) {
        let inside = brothel::State::enter(self.player, self.player_yaw, self.brothel_rope_open);
        self.player = inside.position();
        self.camera_focus = self.player;
        // Facing into the hall, towards the desk.
        self.player_yaw = -std::f32::consts::FRAC_PI_2;
        self.iso.yaw = std::f32::consts::FRAC_PI_4;
        self.iso.yaw_target = self.iso.yaw;
        self.iso.top_down = false;
        self.iso.elevation = ISO.elevation;
        self.iso.distance = ISO.max_distance * 0.62;
        self.map_open = false;
        self.footprints.clear();
        self.brothel = Some(inside);
        self.achievement_event("visit:burdel");
        self.queen_thought = Some(("Debería hablar con la recepcionista.", self.elapsed + 4.0));
        self.clear_input();
        eprintln!("[3d] entered the brothel");
        // The first night: Rosalía receives her, hooded, on the stage (`burdel.md`, «bienvenida»).
        if self.story.get("burdel_bienvenida") == 0 {
            self.queen_thought = None;
            self.start_talk_at("burdel.bienvenida");
        }
    }

    pub(super) fn leave_brothel(&mut self) {
        if let Some(inside) = self.brothel.take() {
            self.brothel_rope_open = inside.rope_open;
            self.player = inside.return_to;
            self.player_yaw = inside.return_yaw + std::f32::consts::PI;
        }
        self.camera_focus = self.player;
        self.iso.distance = ISO.max_distance;
        self.clear_input();
        eprintln!("[3d] left the brothel");
    }

    /// One frame inside the brothel: walk its floors and stair, open its doors.
    pub(super) fn tick_brothel(&mut self, dt: f32) {
        // The house is built to be seen from its open corner.
        self.iso.yaw_target = std::f32::consts::FRAC_PI_4;
        // A girl taking her up walks her there: no key moves her meanwhile.
        if self.escort_active() {
            self.tick_escort(dt);
            return;
        }
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
        let Some(inside) = self.brothel.as_mut() else {
            return;
        };
        let before = inside.position();
        if direction.length_squared() > 0.0 && !self.shop_open {
            let direction = direction.normalize_or_zero();
            let speed = if self.sprint { 4.6 } else { 2.2 };
            // Small sub-steps keep the stair and the rails exact at a sprint.
            let steps = ((dt * speed) / 0.05).ceil().max(1.0) as usize;
            for _ in 0..steps {
                inside.walk(
                    direction.x * dt * speed / steps as f32,
                    direction.z * dt * speed / steps as f32,
                );
            }
            self.player_yaw = direction.x.atan2(direction.z);
        }
        inside.update(dt);
        let after = inside.position();
        self.player = after;
        self.camera_focus = after;
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
    /// A girl of the house is taking the Queen up to her room.
    pub(super) fn escort_active(&self) -> bool {
        self.brothel.as_ref().is_some_and(|b| b.escort.is_some())
    }

    /// The Queen walks behind her guide; the camera keeps them both in view; on arrival the
    /// girl's guion opens in the room.
    fn tick_escort(&mut self, dt: f32) {
        let Some(inside) = self.brothel.as_mut() else {
            return;
        };
        let before = inside.position();
        let mut yaw = self.player_yaw;
        let arrived = inside.tick_escort(dt, &mut yaw);
        inside.update(dt);
        let after = inside.position();
        let focus = inside.escort_focus(after);
        self.player = after;
        self.player_yaw = yaw;
        let ease = (dt * 5.0).min(1.0);
        self.camera_focus = self.camera_focus + (focus - self.camera_focus) * ease;
        let stepped = Vec3::new(after.x - before.x, 0.0, after.z - before.z).length();
        self.step_sound_distance += stepped;
        if self.step_sound_distance >= 1.05 {
            self.step_sound_distance -= 1.05;
            self.play_step();
        }
        if let Some(id) = arrived {
            eprintln!("[escort] {id} brought her to her room");
            self.start_talk(None, Some(&format!("burdel_{id}")));
        }
    }

    /// Esc while she is being taken up: in the room at once.
    pub(super) fn skip_escort(&mut self) {
        let mut yaw = self.player_yaw;
        if let Some(inside) = self.brothel.as_mut() {
            inside.skip_escort(&mut yaw);
            self.player = inside.position();
            self.camera_focus = self.player;
        }
        self.player_yaw = yaw;
    }

    /// F in the brothel: out, or the venue's book at the desk. Whether that ends the key's handling.
    pub(super) fn interact_brothel(&mut self) -> bool {
        let action = self.brothel.as_mut().map(|inside| inside.interact());
        match action {
            Some(brothel::Action::Leave) => {
                self.leave_brothel();
                self.persist_3d_state(true);
            }
            Some(brothel::Action::OpenVenue) => {
                self.brothel_rope_open = true;
                self.open_venue_book();
            }
            _ => {}
        }
        false
    }
}

/// The brothel as a place.
pub(super) struct BrothelZone;

impl Zone for BrothelZone {
    fn id(&self) -> ZoneId {
        ZoneId::Brothel
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_brothel(dt);
        app.elapsed += dt;
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        app.brothel.as_ref().and_then(|inside| inside.prompt())
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_brothel()
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Brothel
    }
    fn emitters(&self, _app: &App) -> &'static [game_audio::Emitter] {
        brothel::emitters()
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, _) = state.prepare_above_ground(frame);
        if let Some(inside) = frame.brothel_state {
            state.render_brothel(frame, inside, bob);
        }
    }
    fn title(&self, app: &App) -> (&'static str, &'static str) {
        app.outdoors_title()
    }
}
