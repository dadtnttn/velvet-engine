//! La Reina se bebe el elixir de Rosalía en el mostrador. En 3D: la cámara corta a un plano cercano
//! en perspectiva, ella se vuelve hacia la sala, saca el frasco, lo destapa y se lo bebe en tres
//! tragos (`elixir::CLIP`, con el frasco en su mano). En 2D, encima: la cortinilla, el rótulo, los
//! «¡pop!», «glup» y «¡ah!», y el sello que despierta. Al acabar vuelve al libro de Rosalía, con la
//! forma del sello abierta para las chicas.
use super::*;

/// Before her hand moves: the wipe, under which the shot cuts (s).
pub(super) const LEAD: f32 = 0.5;
/// After the clip: the seal waking, before a key closes it (s).
pub(super) const AWAKE: f32 = 1.6;

/// The scene while it plays.
pub(super) struct Drink {
    started: f32,
    /// Where she stood and which way she faced (she is put back there after).
    from: Vec3,
    from_yaw: f32,
    from_walker: Option<brothel::Walker>,
    /// Where she drinks and which way she faces.
    spot: Vec3,
    yaw: f32,
    /// Chiquita's comments said so far.
    said: u8,
}

impl App {
    /// The elixir is paid: she drinks it at the desk, turned to the room.
    pub(super) fn start_drink(&mut self) {
        let desk = brothel::plan()
            .hotspots
            .iter()
            .find(|h| h.id == "recepcionista")
            .map(|h| (h.x, h.z));
        let from = self.player;
        let from_yaw = self.player_yaw;
        let from_walker = self.brothel.as_ref().map(|b| b.walker);
        // Rosalía sits behind her desk at the room's back: facing +Z is facing the room. Turned a
        // little to her right, the close shot (from her left) stands in front of the desk.
        let (spot, yaw) = match (desk, self.brothel.as_mut()) {
            (Some((x, z)), Some(inside)) => {
                inside.walker.x = x;
                inside.walker.z = z;
                (inside.position(), -0.61)
            }
            _ => (self.player, self.player_yaw),
        };
        self.player = spot;
        self.player_yaw = yaw;
        self.camera_focus = spot;
        self.shop_open = false;
        self.venue.open = false;
        self.queen_thought = None;
        self.drink = Some(Drink {
            started: self.elapsed,
            from,
            from_yaw,
            from_walker,
            spot,
            yaw,
            said: 0,
        });
        self.clear_input();
    }

    /// Seconds since it began.
    pub(super) fn drink_clock(&self) -> Option<f32> {
        self.drink.as_ref().map(|d| self.elapsed - d.started)
    }

    /// Seconds into her clip (`None` before the shot has cut to her).
    pub(super) fn drink_clip_time(&self) -> Option<f32> {
        let c = self.drink_clock()?;
        (c >= LEAD * 0.5).then(|| (c - LEAD).clamp(0.0, elixir::SECS))
    }

    /// The close shot (from the wipe's middle on).
    pub(super) fn drink_camera(&self) -> Option<Camera3D> {
        let d = self.drink.as_ref()?;
        let c = self.elapsed - d.started;
        (c >= LEAD * 0.5).then(|| elixir::camera(d.spot, d.yaw, c - LEAD))
    }

    /// Each step: she stays put, and Chiquita has her say.
    pub(super) fn tick_drink(&mut self) {
        let Some(c) = self.drink_clock() else {
            return;
        };
        let said = self.drink.as_ref().map_or(0, |d| d.said);
        let lines: [(f32, &str, mascot::Cue); 2] = [
            (
                LEAD + elixir::POP + 0.15,
                "—¿Te lo vas a beber sin preguntar qué lleva? …Claro que sí.",
                mascot::Cue::Scold,
            ),
            // The heat gets to her too: she flies at the screen.
            (
                LEAD + elixir::AH + 0.35,
                "—¿Por qué noto calor YO? ¡Que la del sello eres tú!",
                mascot::Cue::Splat,
            ),
        ];
        if let Some((_, text, cue)) = lines.get(said as usize) {
            if c >= lines[said as usize].0 {
                if let Some(d) = self.drink.as_mut() {
                    d.said += 1;
                }
                self.float_chiquita_cue(text, *cue);
            }
        }
        self.clear_input();
    }

    /// The seal has woken: a key closes it.
    fn drink_ready(&self) -> bool {
        self.drink_clock()
            .is_some_and(|c| c >= LEAD + elixir::SECS + AWAKE)
    }

    /// Keys while she drinks: Esc skips to the seal waking; then F, Space, Enter or Esc go on.
    pub(super) fn drink_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if !pressed || repeat {
            return;
        }
        let ready = self.drink_ready();
        match code {
            KeyCode::KeyF | KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter if ready => {
                self.finish_drink()
            }
            KeyCode::Escape | KeyCode::Backspace => {
                if ready {
                    self.finish_drink();
                } else if let Some(d) = self.drink.as_mut() {
                    let c = self.elapsed - d.started;
                    if c < LEAD + elixir::SECS {
                        d.started = self.elapsed - (LEAD + elixir::SECS);
                    }
                }
            }
            _ => {}
        }
    }

    /// A click: like F once the seal has woken.
    pub(super) fn drink_click(&mut self) {
        if self.drink_ready() {
            self.finish_drink();
        }
    }

    /// Back to the desk and Rosalía's book, the seal awake.
    pub(super) fn finish_drink(&mut self) {
        let Some(d) = self.drink.take() else {
            return;
        };
        if let (Some(inside), Some(walker)) = (self.brothel.as_mut(), d.from_walker) {
            inside.walker = walker;
        }
        self.player = d.from;
        self.player_yaw = d.from_yaw;
        self.camera_focus = d.from;
        self.open_shop();
        let now = (self.elapsed * 1000.0) as u32;
        self.venue.drunk(now);
        super::ero::tally(&mut self.story.values, super::ero::ELIXIR);
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// The 2D layer for the overlay: where her lips, the flask and her belly are on screen.
    pub(super) fn drink_view(&self) -> Option<map_overlay::DrinkView> {
        let d = self.drink.as_ref()?;
        let c = (self.elapsed - d.started).max(0.0);
        let t = (c - LEAD).clamp(0.0, elixir::SECS);
        let camera = self.camera();
        let queen = Mat4::from_translation(d.spot)
            * Mat4::from_rotation_y(d.yaw)
            * Mat4::from_scale(Vec3::new(
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
                QUEEN_WORLD_SCALE,
            ));
        let on_screen = |p: Vec3| {
            hud::project_npc_anchor(camera, queen.transform_point3(p))
                .map(|(x, y)| [x.round() as i32, y.round() as i32])
        };
        let points = self
            .state
            .as_ref()
            .and_then(|s| s.queen_drink_points(self.outfit, t));
        let (lips, belly, flask) = match (points, c >= LEAD * 0.5) {
            (Some((lips, belly, flask)), true) => {
                (on_screen(lips), on_screen(belly), on_screen(flask))
            }
            _ => (None, None, None),
        };
        Some(map_overlay::DrinkView {
            ms: (c * 1000.0) as u32,
            // Not held at the clip's end: its words run out on their own.
            clip_ms: (c >= LEAD).then(|| ((c - LEAD) * 1000.0) as u32),
            awake_ms: (c >= LEAD + elixir::SECS)
                .then(|| ((c - LEAD - elixir::SECS) * 1000.0) as u32),
            ready: self.drink_ready(),
            lips,
            flask,
            belly,
            first: self.venue.progress.elixirs_bought <= 1,
        })
    }
}
