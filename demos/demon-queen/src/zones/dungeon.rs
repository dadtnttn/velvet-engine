//! La mazmorra como lugar: bajar por la mina, el ratón capturado de la primera persona y volver arriba con el botín.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct DungeonReturnState {
    pub(super) player: Vec3,
    pub(super) player_yaw: f32,
    /// The view as it was above ground.
    pub(super) iso: IsoCamera3D,
    pub(super) nature_mode: bool,
    pub(super) village_entered: bool,
}

impl App {
    pub(crate) fn dungeon_seed(&self) -> u64 {
        self.dungeon.seed()
    }

    pub(crate) fn dungeon_active(&self) -> bool {
        self.dungeon_mode
    }

    pub(super) fn capture_dungeon_mouse(&self) -> bool {
        if let Some(window) = &self.window {
            if !window.has_focus() {
                window.set_cursor_visible(true);
                return false;
            }
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                let center = PhysicalPosition::new(
                    f64::from(size.width) * 0.5,
                    f64::from(size.height) * 0.5,
                );
                if let Err(error) = window.set_cursor_position(center) {
                    eprintln!(
                        "[dungeon] no se pudo centrar el cursor antes de capturarlo: {error}"
                    );
                }
            }
            let grabbed = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            match grabbed {
                Ok(()) => {
                    window.set_cursor_visible(false);
                    true
                }
                Err(error) => {
                    window.set_cursor_visible(true);
                    eprintln!("[dungeon] no se pudo capturar el cursor: {error}");
                    false
                }
            }
        } else {
            false
        }
    }

    pub(super) fn release_dungeon_mouse(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
        self.dungeon_mouse_captured = false;
    }

    /// The way down: not without something to defend herself with.
    pub(super) fn enter_dungeon(&mut self) {
        self.achievement_event("visit:mina");
        if self.dungeon_mode {
            return;
        }
        if !self.has_weapon() {
            self.queen_thought =
                Some(("Necesito algo para defenderme primero.", self.elapsed + 4.2));
            return;
        }
        self.enter_dungeon_unchecked();
    }

    /// Resuming a saved expedition, captures and tests skip the weapon check.
    pub(crate) fn enter_dungeon_unchecked(&mut self) {
        if self.dungeon_mode {
            return;
        }
        self.dungeon_return = Some(DungeonReturnState {
            player: self.player,
            player_yaw: self.player_yaw,
            iso: self.iso,
            nature_mode: self.nature_mode,
            village_entered: self.village_entered,
        });
        self.dungeon_mode = true;
        self.dungeon.pending_exit = false;
        // The relic sword, once found, goes on every expedition unless another
        // weapon was chosen in the equipment screen.
        self.apply_equipment();
        self.dungeon.controller =
            velvet_dungeon::controller::Controller::new(self.dungeon.spawn_position());
        self.dungeon.controller.yaw = self.dungeon.spawn_yaw();
        self.player = self.dungeon.spawn_position();
        self.player_yaw = self.dungeon.spawn_yaw();
        self.iso.yaw = self.dungeon.spawn_yaw();
        self.iso.yaw_target = self.dungeon.spawn_yaw();
        self.dungeon_pitch = 0.0;
        self.camera_focus = self.player;
        self.iso.top_down = false;
        self.map_open = false;
        self.shop_open = false;
        self.wardrobe_open = false;
        self.venue.open = false;
        self.footprints.clear();
        self.footstep_distance = 0.0;
        self.clear_input();
        self.dungeon_mouse_captured = self.capture_dungeon_mouse();
        eprintln!("[3d] entered procedural cave {:016x}", self.dungeon.seed());
    }

    pub(super) fn leave_dungeon(&mut self) {
        if !self.dungeon_mode {
            return;
        }
        let banked_items = self.bank_dungeon_loot();
        self.dungeon_mode = false;
        self.gold = self
            .gold
            .saturating_add(std::mem::take(&mut self.dungeon.gold));
        // Back with loot and gold: the cave counts, and so may what she carries now.
        self.achievements.record("dungeon:return");
        self.dungeon.pending_exit = false;
        if let Some(saved) = self.dungeon_return.take() {
            self.player = saved.player;
            self.player_yaw = saved.player_yaw;
            self.iso = saved.iso;
            self.nature_mode = saved.nature_mode;
            self.village_entered = saved.village_entered;
        }
        self.camera_focus = self.player;
        self.dungeon_pitch = 0.0;
        self.release_dungeon_mouse();
        self.clear_input();
        if banked_items && !self.session_path.as_os_str().is_empty() {
            self.persist_3d_state(true);
        }
        eprintln!("[3d] returned to the previous world from the cave");
        self.check_achievements();
    }
}

impl App {
    /// One step down there: the first-person controller, the dungeon's sounds from her eyes, and
    /// the way back up when she takes it.
    pub(super) fn tick_dungeon(&mut self, dt: f32) {
        if self
            .window
            .as_ref()
            .is_some_and(|window| !window.has_focus())
        {
            return;
        }
        let input = velvet_dungeon::controller::Input {
            forward: self.move_forward as u8 as f32 - self.move_back as u8 as f32,
            strafe: self.move_right as u8 as f32 - self.move_left as u8 as f32,
            sprint: self.sprint,
            jump: std::mem::take(&mut self.dungeon.jump_requested),
        };
        self.dungeon.update(input, dt);
        // The dungeon's sounds, heard from the Queen's eyes.
        let sounds = self.dungeon.take_sounds();
        self.poll_audio_boot();
        if !self.audio_tried {
            self.audio_tried = true;
            self.audio = game_audio::GameAudio::new();
        }
        if let Some(audio) = &mut self.audio {
            let camera = self.dungeon.camera();
            let forward = camera.target - camera.position;
            for sound in &sounds {
                audio.play(sound, camera.position, forward);
            }
            let scene = game_audio::Scene {
                emitters: &[],
                zone: soundtrack::Zone::Cave,
                night: 0.0,
                listener: camera.position,
                forward,
                paused: false,
                springs: 0.0,
            };
            audio.update_scene(scene, self.dungeon.heart.rain, dt);
        }
        self.player = self.dungeon.controller.position;
        self.iso.yaw_target = self.dungeon.controller.yaw;
        self.iso.yaw = self.iso.yaw_target;
        self.dungeon_pitch = self.dungeon.controller.pitch;
        self.camera_focus = self.player;
        self.elapsed += dt;
        if self.dungeon.pending_exit {
            self.leave_dungeon();
        }
    }
}

impl RenderState {
    /// Down there: the dungeon's own scene, seen through her eyes.
    pub(super) fn render_dungeon(&mut self, frame: &Frame) {
        let Frame {
            camera,
            dungeon_run,
            ..
        } = *frame;
        if let Some(run) = dungeon_run {
            self.dungeon_scene.render(
                run,
                &mut self.renderer,
                &self.device,
                &self.queue,
                &self.target_view,
                (WINDOW_WIDTH, WINDOW_HEIGHT),
                camera,
            );
        }
    }
}

/// The dungeon as a place.
pub(super) struct DungeonZone;

impl Zone for DungeonZone {
    fn id(&self) -> ZoneId {
        ZoneId::Dungeon
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.tick_dungeon(dt);
    }
    fn prompt(&self, _app: &App) -> Option<Prompt> {
        None
    }
    fn interact(&self, _app: &mut App) -> bool {
        // Down there the keys are the dungeon's own (the input handler gives them to it first).
        false
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Cave
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        state.render_dungeon(frame);
    }
    fn title(&self, _app: &App) -> (&'static str, &'static str) {
        (
            "Mazmorra procedural [G: volver]",
            "  |  [G] Volver al mundo anterior",
        )
    }
}
