//! Las conversaciones del mundo: guiones de Markdown que se hablan en la caja de diálogo 3D, sus elecciones y las escenas de VN que piden.
use super::ui_sfx::{self, UiSfx};
use super::*;

impl App {
    /// What a guion gave with `@var objeto_<id> = n` (Nemi's wooden sword in the first act): into
    /// the inventory and the pack, worn at once if its slot is empty. The variable goes back to 0,
    /// so each gift arrives once.
    pub(super) fn grant_guion_items(&mut self) {
        let given: Vec<(String, u32)> = self
            .story
            .values
            .iter()
            .filter(|(_, n)| **n > 0)
            .filter_map(|(k, n)| {
                k.strip_prefix("objeto_")
                    .map(|id| (id.to_owned(), *n as u32))
            })
            .collect();
        if given.is_empty() {
            return;
        }
        for (id, n) in &given {
            self.story.values.insert(format!("objeto_{id}"), 0);
            *self.inventory.entry(id.clone()).or_default() += n;
            eprintln!("[guion] gives {n} × {id}");
        }
        let mut pack = self.backpack.clone();
        pack.reconcile(&self.inventory);
        for (id, _) in &given {
            pack.equip_if_empty(id);
        }
        self.backpack = pack;
        ui_sfx::play(UiSfx::Select);
        self.persist_3d_state(true);
    }

    /// Starts the guion written for `who` (a person) or `en` (a place), if one applies now.
    pub(super) fn start_talk(&mut self, who: Option<&str>, en: Option<&str>) -> bool {
        let lib = guion::library();
        let Some(at) = lib.entry(who, en, &self.story) else {
            return false;
        };
        self.talk_story_before = Some(self.story.clone());
        let talk = guion::Conversation::start(lib, at, &mut self.story);
        eprintln!("[guion] {}", talk.scene_id(lib));
        self.talk = Some(talk);
        self.talk_choice = 0;
        self.talk_line_at = self.elapsed;
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
        self.settle_talk();
        true
    }

    /// Starts the guion scene `id` (`file.scene`), whatever its entry says.
    pub(super) fn start_talk_at(&mut self, id: &str) -> bool {
        let lib = guion::library();
        let Some(at) = lib.find(id, None) else {
            eprintln!("[guion] no scene {id}");
            return false;
        };
        self.talk_story_before = Some(self.story.clone());
        let talk = guion::Conversation::start(lib, at, &mut self.story);
        eprintln!("[guion] {}", talk.scene_id(lib));
        self.talk = Some(talk);
        self.talk_choice = 0;
        self.talk_line_at = self.elapsed;
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
        self.settle_talk();
        true
    }

    /// Characters of the line on show that are out by now (it types itself out).
    pub(super) fn talk_shown(&self, text: &str) -> usize {
        let chars = ((self.elapsed - self.talk_line_at).max(0.0)
            * crate::exploration3d::prefs::text_rate(52.0)) as usize;
        chars.min(text.chars().count())
    }

    /// F: the whole line at once, else the next; or take the chosen option.
    pub(super) fn talk_confirm(&mut self) {
        // A beat of the stage playing (or an item shown off, put away now) takes the key.
        if self.stage_confirm() {
            return;
        }
        let lib = guion::library();
        let shown = self.talk.as_ref().map(|t| t.shown(lib, &self.story));
        match shown {
            Some(guion::Shown::Line { text, .. }) => {
                if self.talk_shown(&text) < text.chars().count() {
                    self.talk_line_at = -1.0e6;
                } else if let Some(talk) = self.talk.as_mut() {
                    talk.advance(lib, &mut self.story);
                    self.talk_line_at = self.elapsed;
                    self.stage_line_passed();
                }
            }
            Some(guion::Shown::Choice { .. }) => {
                let pick = self.talk_choice;
                self.talk_pick(pick);
                return;
            }
            _ => {}
        }
        self.settle_talk();
    }

    /// The option under the pointer (on the 1920×1080 screen) while a choice is on show: its card as
    /// the stage draws them, centred, one under another from y 330, 88 tall every 110.
    fn talk_option_at(&self, x: f32, y: f32) -> Option<usize> {
        let lib = guion::library();
        let Some(guion::Shown::Choice { options, .. }) =
            self.talk.as_ref().map(|t| t.shown(lib, &self.story))
        else {
            return None;
        };
        if !(100.0..WINDOW_WIDTH as f32 - 100.0).contains(&x) || y < 330.0 {
            return None;
        }
        let k = ((y - 330.0) / 110.0) as usize;
        (y - 330.0 - k as f32 * 110.0 <= 88.0 && k < options.len()).then_some(k)
    }

    /// A left click in a conversation: the option under the pointer, else what F does. A click
    /// beside the options does nothing.
    pub(super) fn talk_click(&mut self) {
        let at = self.shop_pointer_position();
        self.talk_click_at(at);
    }

    /// A click at `at` (the 1920×1080 screen; `None`: off the picture).
    pub(super) fn talk_click_at(&mut self, at: Option<(f32, f32)>) {
        let lib = guion::library();
        let choosing = matches!(
            self.talk.as_ref().map(|t| t.shown(lib, &self.story)),
            Some(guion::Shown::Choice { .. })
        );
        if !choosing {
            self.talk_confirm();
        } else if let Some(k) = at.and_then(|(x, y)| self.talk_option_at(x, y)) {
            self.talk_pick(k);
        }
    }

    /// The option under the pointer is the one lit.
    pub(super) fn talk_hover(&mut self) {
        if let Some(k) = self
            .shop_pointer_position()
            .and_then(|(x, y)| self.talk_option_at(x, y))
        {
            if k != self.talk_choice {
                self.talk_choice = k;
                ui_sfx::play(UiSfx::Move);
            }
        }
    }

    pub(super) fn talk_move(&mut self, by: i32) {
        let lib = guion::library();
        if let Some(guion::Shown::Choice { options, .. }) =
            self.talk.as_ref().map(|t| t.shown(lib, &self.story))
        {
            let n = options.len() as i32;
            let before = self.talk_choice;
            self.talk_choice = (self.talk_choice as i32 + by).rem_euclid(n.max(1)) as usize;
            if self.talk_choice != before {
                ui_sfx::play(UiSfx::Move);
            }
        }
    }

    pub(super) fn talk_pick(&mut self, index: usize) {
        if self.stage_waiting() {
            return;
        }
        let lib = guion::library();
        if let Some(talk) = self.talk.as_mut() {
            if talk.choose(lib, &mut self.story, index) {
                self.talk_choice = 0;
                self.talk_line_at = self.elapsed;
                ui_sfx::play(UiSfx::Select);
            } else {
                self.talk_choice = index;
            }
        }
        self.settle_talk();
    }

    /// Ends the conversation when it is over, or hands it to the visual novel. Chiquita's lines
    /// do not wait in the box: they float on the screen and the conversation goes on.
    pub(super) fn settle_talk(&mut self) {
        let lib = guion::library();
        self.stage_take();
        // On the stage she says her lines herself, up in her corner, looking as they say.
        if self.stage_up() {
            if let Some(guion::Shown::Line {
                who,
                text,
                kind: guion::LineKind::Dialogue,
                mood,
                ..
            }) = self.talk.as_ref().map(|t| t.shown(lib, &self.story))
            {
                if who == "chiquita" {
                    // After the beat on show, if one plays (an item shown off, a reveal).
                    if self.stage_waiting() {
                        self.stage_cue_later();
                    } else {
                        self.mascot_say(mascot::line_cue(mood.as_deref(), &text), &text, 0.0);
                    }
                }
            }
        }
        for _ in 0..32 {
            // On the stage she speaks in the box like everyone else; over the war table too (no
            // world for her to float in there).
            if self.stage_up() || self.war.is_some() {
                break;
            }
            let Some(guion::Shown::Line {
                who,
                text,
                kind,
                mood,
                ..
            }) = self.talk.as_ref().map(|t| t.shown(lib, &self.story))
            else {
                break;
            };
            if who != "chiquita" || kind != guion::LineKind::Dialogue {
                break;
            }
            self.float_chiquita(&text, mood.as_deref() == Some("celosa"));
            if let Some(talk) = self.talk.as_mut() {
                talk.advance(lib, &mut self.story);
            }
            self.talk_line_at = self.elapsed;
        }
        match self.talk.as_ref().map(|t| t.shown(lib, &self.story)) {
            Some(guion::Shown::End) => self.end_talk(),
            Some(guion::Shown::Vn(scene)) => {
                eprintln!("[guion] to the visual novel: {scene}");
                self.pending_guion_vn = Some(scene);
                self.end_talk();
            }
            _ => {}
        }
    }

    pub(super) fn end_talk(&mut self) {
        self.talk = None;
        self.talk_story_before = None;
        self.talk_choice = 0;
        self.clear_input();
        self.grant_guion_items();
        // A stage scene that unlocked something shows it until F; then the stage comes down.
        if let Some(stage) = self.stage.as_mut() {
            if !stage.unlocks.is_empty() && stage.outro_at.is_none() {
                stage.outro_at = Some(self.elapsed);
                ui_sfx::play(UiSfx::Select);
                return;
            }
            self.stage = None;
            self.persist_3d_state(true);
        }
        // The springs' welcome gives her the house's bikini (`@var termas_bikini`): she puts it
        // on there and then, and at the springs from then on (her clothes wait for the way out).
        if self.story.get("termas_bikini") > 0 && self.achievements.grant_outfit("bikini") {
            self.change_into_bikini();
            self.persist_3d_state(true);
        }
        // A night with Lira or Nyxara that went further (`@var caricias_<girl>`): the touch game.
        for girl in ["lira", "nyxara", "maren"] {
            let var = format!("caricias_{girl}");
            if self.story.get(&var) > 0 {
                self.story.values.insert(var, 0);
                if self.open_caricias(girl) {
                    return;
                }
            }
        }
        // Lira prepares the royal bath (`@var corte_bano`): its scene, on the stage.
        if self.story.get("corte_bano") > 0 {
            self.story.values.insert("corte_bano".to_owned(), 0);
            self.story.values.insert("bano_hoy".to_owned(), 1);
            if self.start_talk_at("corte.bano_lira") {
                return;
            }
        }
        // The bed's guion says «Descansar» with `@var dormir`.
        if self.story.get("dormir") > 0 {
            self.story.values.insert("dormir".to_owned(), 0);
            self.sleep_in_bed();
            return;
        }
        // «Dejarse llevar del todo» plays as the bed scene (`@var escena_cama`).
        // (`+2`: «Dejarse llevar del todo», which starts half-way up).
        let bed = self.story.get("escena_cama");
        if bed > 0 {
            self.story.values.insert("escena_cama".to_owned(), 0);
            self.open_bed_scene(bed >= 2);
            return;
        }
        // Nina's room in the brothel (`burdel.md`, `@var escena_nina`).
        if self.story.get("escena_nina") > 0 {
            self.story.values.insert("escena_nina".to_owned(), 0);
            self.open_scene(&bed_scene::NINA, false);
            return;
        }
        // Alba's and Noa's rooms: their minigames (`@var escena_alba`, `escena_alba_futa`,
        // `escena_noa_futa`); with the seal awake, the elixir is spent.
        for (var, game, elixir) in [
            ("escena_alba", play_scene::Game::Alba, false),
            ("escena_alba_futa", play_scene::Game::AlbaFuta, true),
            ("escena_noa", play_scene::Game::Noa, false),
            ("escena_noa_futa", play_scene::Game::NoaFuta, true),
        ] {
            if self.story.get(var) > 0 {
                self.story.values.insert(var.to_owned(), 0);
                if elixir {
                    self.story.values.insert("cita_sello".to_owned(), 0);
                    self.venue.progress.elixir_active = false;
                }
                self.open_play_scene(game);
                self.persist_3d_state(true);
                return;
            }
        }
        // The same room, the seal taking its shape with Rosalía's elixir (`@var escena_nina_futa`):
        // the elixir is spent.
        if self.story.get("escena_nina_futa") > 0 {
            self.story.values.insert("escena_nina_futa".to_owned(), 0);
            self.story.values.insert("cita_sello".to_owned(), 0);
            self.venue.progress.elixir_active = false;
            self.open_scene(&bed_scene::NINA_FUTA, false);
        }
        self.persist_3d_state(true);
    }

    /// The dialogue box for the overlay (`None` when no one is talking).
    pub(super) fn dialogue_view(&self) -> Option<map_overlay::DialogueView> {
        let lib = guion::library();
        let talk = self.talk.as_ref()?;
        // The Queen's face wears what she wears: `gorou` in retratos.json is the green dress's (the
        // hypnosis art), `gorou_real` the royal dress's, `gorou_bikini` the bikini's; a mood one of
        // them lacks falls back to that set's normal, a set that is not there to `gorou`.
        let dressed = match self.outfit {
            // The cloak is worn over the royal dress.
            0 | 3 => Some("gorou_real"),
            2 => Some("gorou_bikini"),
            _ => None,
        };
        let portrait = |who: &str, mood: Option<&str>| {
            let faces = guion::portraits();
            dressed
                .filter(|_| who == "gorou")
                .and_then(|key| faces.find(key, mood))
                .or_else(|| faces.find(who, mood))
                .map(|p| map_overlay::PortraitRef {
                    image: p.image,
                    crop: p.crop,
                })
        };
        let accent = |who: &str| -> [u8; 3] {
            // The visual novel's colours (`speaker_tone` in main.rs).
            match who {
                "lira" => [0x6e, 0xb0, 0xff],
                "nemi" => [0xbe, 0x8c, 0xff],
                "gorou" | "gorou_thought" | "reina" => [0xff, 0x6e, 0xa0],
                "chiquita" => [0xd9, 0xa6, 0xff],
                "nina" => [0xe8, 0x84, 0x5c],
                "rosalia" => [0xd8, 0x3c, 0x78],
                "alba" => [0xf0, 0xc0, 0x60],
                "noa" => [0xb8, 0x2a, 0x48],
                "hypnotist" => [0xd6, 0x5c, 0xff],
                "nyxara" => [0x64, 0xe0, 0xc8],
                _ => [0xd9, 0xbd, 0x8a],
            }
        };
        match talk.shown(lib, &self.story) {
            guion::Shown::Line {
                who,
                name,
                text,
                kind,
                mood,
            } => Some(map_overlay::DialogueView {
                // Narration and sounds have no face; a thought shows the Queen thinking.
                portrait: match kind {
                    guion::LineKind::Dialogue if self.on_stage(&who) => None,
                    guion::LineKind::Dialogue => portrait(&who, mood.as_deref()),
                    guion::LineKind::Thought if self.on_stage("gorou") => None,
                    guion::LineKind::Thought => {
                        portrait("gorou", Some(mood.as_deref().unwrap_or("piensa")))
                    }
                    _ => None,
                },
                name: if kind == guion::LineKind::Thought {
                    format!("{name} (piensa)")
                } else {
                    name
                },
                speaker: who.clone(),
                shown: self.talk_shown(&text),
                text,
                kind: match kind {
                    guion::LineKind::Dialogue => 0,
                    guion::LineKind::Narration => 1,
                    guion::LineKind::Sound => 2,
                    guion::LineKind::Thought => 3,
                },
                accent: accent(&who),
                prompt: None,
                choices: Vec::new(),
                selected: 0,
                continue_key: true,
            }),
            guion::Shown::Choice { prompt, options } => Some(map_overlay::DialogueView {
                portrait: if self.on_stage("gorou") {
                    None
                } else {
                    portrait("gorou", Some("piensa"))
                },
                name: "Reina".into(),
                speaker: "gorou".into(),
                text: String::new(),
                shown: 0,
                kind: 0,
                accent: accent("gorou"),
                prompt: Some(prompt.unwrap_or_else(|| "¿Qué hace la Reina?".into())),
                selected: self.talk_choice.min(options.len().saturating_sub(1)),
                choices: options,
                continue_key: false,
            }),
            _ => None,
        }
    }

    /// A guion scene the host should open in the visual novel (its first node's id).
    pub(crate) fn take_guion_vn(&mut self) -> Option<String> {
        self.pending_guion_vn
            .take()
            .map(|scene| guion::vn_start_node(&scene))
    }

    /// The story's variables, for the visual novel when it takes over.
    pub(crate) fn story_values(&self) -> std::collections::BTreeMap<String, i32> {
        self.story.values.clone()
    }

    /// Consumed by the VN host after the player presses F beside the mage.
    pub(crate) fn take_story_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_story_request)
    }
}
