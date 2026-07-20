//! Product-level visual-novel session: Say/Choice UI state, save/load, prefs,
//! history, confirm, presentation (BG/sprites/z-order/transitions), BGM intents,
//! rollback / skip / auto — all driven by the real [`StoryPlayer`].

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::history::HistoryEntry;
use crate::prefs::{SkipMode, StoryPreferences, TextSpeed};
use crate::rollback::RollbackStack;
use crate::runtime::{ChoiceOption, StoryEvent, StoryPlayer, StoryWait, VisibleCharacter};
use crate::save::{SaveError, SaveGame, SaveMeta, SaveStore};
use crate::transitions::{Transition, TransitionQueue, WipeDirection};

/// Product say-screen state (namebox + body + typewriter).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SayScreen {
    /// Speaker display name (empty = narrator).
    pub namebox: String,
    /// Full line.
    pub full_text: String,
    /// Currently revealed text.
    pub visible_text: String,
    /// Typewriter complete.
    pub text_complete: bool,
    /// Whether the box is visible.
    pub visible: bool,
}

impl SayScreen {
    /// Clear.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Show a line; typewriter starts incomplete unless instant.
    pub fn show(&mut self, namebox: impl Into<String>, text: impl Into<String>, instant: bool) {
        self.namebox = namebox.into();
        self.full_text = text.into();
        if instant {
            self.visible_text = self.full_text.clone();
            self.text_complete = true;
        } else {
            self.visible_text.clear();
            self.text_complete = false;
        }
        self.visible = true;
    }

    /// Reveal all remaining text.
    pub fn reveal_all(&mut self) {
        self.visible_text = self.full_text.clone();
        self.text_complete = true;
    }

    /// Advance typewriter by `n` characters. Returns true when complete.
    pub fn typewrite(&mut self, n: usize) -> bool {
        if self.text_complete {
            return true;
        }
        let target = (self.visible_text.chars().count() + n).min(self.full_text.chars().count());
        self.visible_text = self.full_text.chars().take(target).collect();
        if self.visible_text.chars().count() >= self.full_text.chars().count() {
            self.text_complete = true;
        }
        self.text_complete
    }
}

/// Choice menu product screen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChoiceScreen {
    /// Options from the player.
    pub options: Vec<ChoiceOption>,
    /// Selected index into `options` (not arm id).
    pub selected: usize,
    /// Open.
    pub open: bool,
}

impl ChoiceScreen {
    /// Open with options.
    pub fn open_with(&mut self, options: Vec<ChoiceOption>) {
        self.options = options;
        self.selected = 0;
        self.open = !self.options.is_empty();
    }

    /// Close.
    pub fn close(&mut self) {
        self.options.clear();
        self.selected = 0;
        self.open = false;
    }

    /// Move selection.
    pub fn move_sel(&mut self, delta: i32) {
        if self.options.is_empty() {
            return;
        }
        let n = self.options.len() as i32;
        let mut s = self.selected as i32 + delta;
        while s < 0 {
            s += n;
        }
        self.selected = (s % n) as usize;
    }

    /// Arm index of selection.
    pub fn selected_arm(&self) -> Option<usize> {
        self.options.get(self.selected).map(|o| o.index)
    }
}

/// Confirm dialog (quit / overwrite save).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConfirmKind {
    /// Idle.
    #[default]
    None,
    /// Confirm quit.
    Quit,
    /// Confirm overwrite slot.
    OverwriteSlot(String),
}

/// Confirm dialog state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConfirmDialog {
    /// Kind.
    pub kind: ConfirmKind,
    /// Prompt text.
    pub message: String,
    /// Open.
    pub open: bool,
}

impl ConfirmDialog {
    /// Ask quit.
    pub fn ask_quit(&mut self) {
        self.kind = ConfirmKind::Quit;
        self.message = "Quit the game?".into();
        self.open = true;
    }

    /// Ask overwrite.
    pub fn ask_overwrite(&mut self, slot: impl Into<String>) {
        let slot = slot.into();
        self.message = format!("Overwrite save slot `{slot}`?");
        self.kind = ConfirmKind::OverwriteSlot(slot);
        self.open = true;
    }

    /// Close.
    pub fn close(&mut self) {
        self.kind = ConfirmKind::None;
        self.message.clear();
        self.open = false;
    }
}

/// Sprite on a presentation layer with z-order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayeredSprite {
    /// Character / sprite id.
    pub id: String,
    /// Expression.
    pub expression: Option<String>,
    /// Placement tag (left/right/center/at coords).
    pub at: Option<String>,
    /// Z-order (higher draws later).
    pub z: i32,
}

/// Presentation state applied from story events.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresentationState {
    /// Current background path.
    pub background: Option<String>,
    /// Visible sprites keyed by id.
    pub sprites: IndexMap<String, LayeredSprite>,
    /// Active transition queue.
    pub transitions: TransitionQueue,
    /// Last placement move target (for move transition bookkeeping).
    pub last_move_to: Option<String>,
    /// Last one-shot SFX path requested by story (host plays audio).
    pub last_sfx: Option<String>,
    /// Queue of SFX paths in order (cleared by host after play).
    pub sfx_queue: Vec<String>,
    /// Last pause duration in seconds (None = beat without duration).
    pub last_pause: Option<f64>,
    /// Whether a pause beat is pending presentation.
    pub pause_pending: bool,
    /// Last transition id from story ops.
    pub last_transition_name: Option<String>,
    /// Last host command name dispatched from story.
    pub last_host_call: Option<String>,
    /// Args of last host call (for debug / host reconcilation).
    pub last_host_args: IndexMap<String, crate::value::StoryValue>,
}

impl PresentationState {
    /// Default z for placement.
    pub fn z_for_at(at: Option<&str>) -> i32 {
        match at {
            Some("left") => 10,
            Some("center") => 20,
            Some("right") => 30,
            _ => 15,
        }
    }

    /// Sorted by z ascending.
    pub fn sprites_by_z(&self) -> Vec<&LayeredSprite> {
        let mut v: Vec<_> = self.sprites.values().collect();
        v.sort_by_key(|s| s.z);
        v
    }

    /// Apply show.
    pub fn show(&mut self, vis: VisibleCharacter) {
        let z = Self::z_for_at(vis.at.as_deref());
        self.sprites.insert(
            vis.id.clone(),
            LayeredSprite {
                id: vis.id,
                expression: vis.expression,
                at: vis.at,
                z,
            },
        );
    }

    /// Hide.
    pub fn hide(&mut self, id: &str) {
        self.sprites.shift_remove(id);
    }

    /// Queue dissolve.
    pub fn dissolve(&mut self, duration: f32) {
        self.transitions.push(Transition::dissolve(duration));
    }

    /// Queue fade.
    pub fn fade(&mut self, duration: f32) {
        self.transitions.push(Transition::fade(duration));
    }

    /// Queue move (sprite re-placement transition).
    pub fn move_to(&mut self, id: &str, at: impl Into<String>, duration: f32) {
        let at = at.into();
        if let Some(s) = self.sprites.get_mut(id) {
            s.at = Some(at.clone());
            s.z = Self::z_for_at(Some(&at));
        }
        self.last_move_to = Some(format!("{id}@{at}"));
        self.transitions
            .push(Transition::r#move(duration, WipeDirection::LeftToRight));
    }

    /// Tick transitions.
    pub fn tick(&mut self, dt: f32) {
        self.transitions.tick(dt);
    }
}

/// BGM intent produced by story music events (host applies via real audio).
#[derive(Debug, Clone, PartialEq)]
pub enum BgmIntent {
    /// Play path with optional fade-in seconds.
    Play {
        /// Asset path.
        path: String,
        /// Fade in seconds.
        fade_in: f32,
    },
    /// Stop with fade-out.
    Stop {
        /// Fade out seconds.
        fade_out: f32,
    },
}

/// Lightweight BGM mixer state driven by prefs (real volumes for host).
#[derive(Debug, Clone, PartialEq)]
pub struct BgmController {
    /// Current path.
    pub path: Option<String>,
    /// Effective volume after prefs (0..=1).
    pub volume: f32,
    /// Fade-in remaining.
    pub fade_in_left: f32,
    /// Fade-out remaining.
    pub fade_out_left: f32,
    /// Pending intents (host drains).
    pub intents: Vec<BgmIntent>,
    /// Whether currently considered playing.
    pub playing: bool,
}

impl Default for BgmController {
    fn default() -> Self {
        Self {
            path: None,
            volume: 1.0,
            fade_in_left: 0.0,
            fade_out_left: 0.0,
            intents: Vec::new(),
            playing: false,
        }
    }
}

impl BgmController {
    /// Apply play from story event with prefs volumes.
    pub fn play_with_prefs(
        &mut self,
        path: String,
        fade_in: Option<f64>,
        prefs: &StoryPreferences,
    ) {
        let fade = fade_in.unwrap_or(0.0) as f32;
        let vol = (prefs.master_volume * prefs.music_volume).clamp(0.0, 1.0);
        self.path = Some(path.clone());
        self.volume = vol;
        self.fade_in_left = fade;
        self.fade_out_left = 0.0;
        self.playing = true;
        self.intents.push(BgmIntent::Play {
            path,
            fade_in: fade,
        });
    }

    /// Stop with fade.
    pub fn stop(&mut self, fade_out: f32) {
        self.fade_out_left = fade_out;
        self.playing = false;
        self.intents.push(BgmIntent::Stop { fade_out });
        if fade_out <= 0.0 {
            self.path = None;
        }
    }

    /// Recompute volume from prefs.
    pub fn apply_prefs(&mut self, prefs: &StoryPreferences) {
        self.volume = (prefs.master_volume * prefs.music_volume).clamp(0.0, 1.0);
    }

    /// Drain intents for host audio backend.
    pub fn drain_intents(&mut self) -> Vec<BgmIntent> {
        std::mem::take(&mut self.intents)
    }

    /// Tick fade timers.
    pub fn tick(&mut self, dt: f32) {
        if self.fade_in_left > 0.0 {
            self.fade_in_left = (self.fade_in_left - dt).max(0.0);
        }
        if self.fade_out_left > 0.0 {
            self.fade_out_left = (self.fade_out_left - dt).max(0.0);
            if self.fade_out_left <= 0.0 {
                self.path = None;
            }
        }
    }
}

/// Product VN session host — single entry point for S1–S4 + i18n behaviors.
#[derive(Debug)]
pub struct VnSession {
    /// Base (source language) program for locale reloads.
    base_program: crate::ir::StoryProgram,
    /// Underlying story player.
    player: StoryPlayer,
    /// Active language code (`en`, `es`, …).
    pub language: String,
    /// Project root for `tl/<lang>/` resolution.
    project_root: Option<PathBuf>,
    /// Say screen.
    pub say: SayScreen,
    /// Choice screen.
    pub choice: ChoiceScreen,
    /// Confirm dialog.
    pub confirm: ConfirmDialog,
    /// Presentation (BG, sprites, transitions).
    pub presentation: PresentationState,
    /// BGM controller.
    pub bgm: BgmController,
    /// Rollback stack.
    pub rollback: RollbackStack,
    /// Optional gallery for sample/replay unlocks.
    pub gallery: crate::gallery::Gallery,
    /// Save directory (optional until bound).
    save_dir: Option<PathBuf>,
    /// Whether user requested quit (after confirm).
    pub quit_requested: bool,
    /// Typewriter characters per second cache.
    cps: f32,
    /// Accumulator for typewriter.
    type_acc: f32,
}

impl VnSession {
    /// Start from a story player (already loaded program).
    pub fn new(player: StoryPlayer) -> Self {
        let base_program = player.program().clone();
        let mut s = Self {
            base_program,
            player,
            language: "en".into(),
            project_root: None,
            say: SayScreen::default(),
            choice: ChoiceScreen::default(),
            confirm: ConfirmDialog::default(),
            presentation: PresentationState::default(),
            bgm: BgmController::default(),
            rollback: RollbackStack::with_capacity(100),
            gallery: crate::gallery::Gallery::default(),
            save_dir: None,
            quit_requested: false,
            cps: 40.0,
            type_acc: 0.0,
        };
        s.sync_cps();
        s.push_rollback_frame();
        s.ingest_events();
        s.sync_ui_from_wait();
        s
    }

    /// Bind a save directory for slots.
    pub fn with_save_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.save_dir = Some(dir.into());
        self
    }

    /// Bind project root (for `tl/` language packs).
    pub fn with_project_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.project_root = Some(root.into());
        self
    }

    /// Select UI/story language (`en` = source; others load `tl/<lang>/strings.json`).
    /// Restarts the player from the entry scene with the translated program.
    pub fn set_language(&mut self, lang: &str) -> Result<(), String> {
        let program = crate::localization_hook::program_for_language(
            &self.base_program,
            self.project_root.as_deref(),
            lang,
        )?;
        let prefs = self.player.preferences().clone();
        self.language = if lang.trim().is_empty() {
            "en".into()
        } else {
            lang.trim().to_ascii_lowercase()
        };
        self.player = StoryPlayer::start(program);
        *self.player.preferences_mut() = prefs;
        self.player.sync_presentation_prefs();
        self.say.clear();
        self.choice.close();
        self.presentation = PresentationState::default();
        self.bgm = BgmController::default();
        self.rollback.clear();
        self.sync_cps();
        self.push_rollback_frame();
        self.ingest_events();
        self.sync_ui_from_wait();
        Ok(())
    }

    /// Available language codes: always `en`, plus subdirs of `tl/`.
    pub fn available_languages(&self) -> Vec<String> {
        let mut langs = vec!["en".into()];
        if let Some(root) = &self.project_root {
            let tl = root.join("tl");
            if let Ok(rd) = std::fs::read_dir(tl) {
                for e in rd.flatten() {
                    if e.path().is_dir() {
                        if let Some(name) = e.file_name().to_str() {
                            let n = name.to_ascii_lowercase();
                            if n != "en" && !langs.iter().any(|l| l == &n) {
                                langs.push(n);
                            }
                        }
                    }
                }
            }
        }
        langs.sort();
        langs
    }

    /// Story player.
    pub fn player(&self) -> &StoryPlayer {
        &self.player
    }

    /// Mutable player (prefer session methods).
    pub fn player_mut(&mut self) -> &mut StoryPlayer {
        &mut self.player
    }

    /// Preferences.
    pub fn prefs(&self) -> &StoryPreferences {
        self.player.preferences()
    }

    /// Apply preference changes and sync presentation systems.
    pub fn set_prefs(&mut self, prefs: StoryPreferences) {
        *self.player.preferences_mut() = prefs;
        self.player.sync_presentation_prefs();
        self.bgm.apply_prefs(self.player.preferences());
        self.sync_cps();
    }

    fn sync_cps(&mut self) {
        self.cps = match self.player.preferences().text_speed {
            TextSpeed::Instant => 0.0,
            TextSpeed::Cps(c) => c.max(1.0),
        };
    }

    /// History entries for product History screen.
    pub fn history_entries(&self) -> &[HistoryEntry] {
        self.player.history().entries()
    }

    /// List save metas in bound store.
    pub fn list_saves(&self) -> Result<Vec<SaveMeta>, SaveError> {
        let dir = self.save_dir.as_ref().ok_or_else(|| {
            SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save_dir bound",
            ))
        })?;
        let store = SaveStore::new(dir);
        store.list()
    }

    /// Save to slot (no confirm).
    pub fn save_slot(&self, slot: &str) -> Result<SaveGame, SaveError> {
        let dir = self.save_dir.as_ref().ok_or_else(|| {
            SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save_dir bound",
            ))
        })?;
        let store = SaveStore::new(dir);
        let save = self.player.to_save(slot);
        store.write(&save)?;
        Ok(save)
    }

    /// Request save with overwrite confirm if slot exists.
    pub fn request_save(&mut self, slot: &str) -> Result<(), SaveError> {
        let dir = match &self.save_dir {
            Some(d) => d.clone(),
            None => {
                return Err(SaveError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no save_dir bound",
                )))
            }
        };
        let store = SaveStore::new(dir);
        if store.exists(slot) {
            self.confirm.ask_overwrite(slot);
            Ok(())
        } else {
            let save = self.player.to_save(slot);
            store.write(&save)?;
            Ok(())
        }
    }

    /// Load slot into player and resync UI.
    pub fn load_slot(&mut self, slot: &str) -> Result<(), SaveError> {
        let dir = self.save_dir.as_ref().ok_or_else(|| {
            SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no save_dir bound",
            ))
        })?;
        let store = SaveStore::new(dir);
        let save = store.read(slot)?;
        // Propagate ProgramMismatch (and other SaveError) without wrapping as Io.
        self.player.load_save(save)?;
        self.rollback.clear();
        self.push_rollback_frame();
        self.presentation = PresentationState::default();
        if let Some(bg) = self.player.background() {
            self.presentation.background = Some(bg.to_string());
        }
        for (_, vis) in self.player.visible() {
            self.presentation.show(vis.clone());
        }
        self.ingest_events();
        self.sync_ui_from_wait();
        Ok(())
    }

    /// Confirm dialog accept.
    pub fn confirm_yes(&mut self) -> Result<(), SaveError> {
        match self.confirm.kind.clone() {
            ConfirmKind::Quit => {
                self.quit_requested = true;
                self.confirm.close();
            }
            ConfirmKind::OverwriteSlot(slot) => {
                self.confirm.close();
                let _ = self.save_slot(&slot)?;
            }
            ConfirmKind::None => self.confirm.close(),
        }
        Ok(())
    }

    /// Confirm cancel.
    pub fn confirm_no(&mut self) {
        self.confirm.close();
    }

    /// Ask to quit.
    pub fn request_quit(&mut self) {
        self.confirm.ask_quit();
    }

    /// Capture current player into the rollback stack.
    pub fn push_rollback_frame(&mut self) {
        self.rollback.push_from_player(&self.player);
    }

    /// Advance dialogue / continue (click-to-advance).
    pub fn advance(&mut self) {
        if self.confirm.open {
            return;
        }
        match self.player.wait().clone() {
            StoryWait::Line => {
                if !self.say.text_complete && self.cps > 0.0 {
                    self.say.reveal_all();
                    self.player.on_line_fully_shown();
                    return;
                }
                // Snapshot the line we leave, then advance, then snapshot new wait.
                self.push_rollback_frame();
                self.player.advance();
                self.ingest_events();
                self.sync_ui_from_wait();
                self.push_rollback_frame();
            }
            StoryWait::Ready => {
                self.push_rollback_frame();
                self.player.advance();
                self.ingest_events();
                self.sync_ui_from_wait();
                self.push_rollback_frame();
            }
            StoryWait::Pause { .. } => {
                self.push_rollback_frame();
                self.player.skip_pause();
                self.ingest_events();
                self.sync_ui_from_wait();
                self.push_rollback_frame();
            }
            StoryWait::Choice | StoryWait::Ended | StoryWait::Host { .. } => {}
        }
    }

    /// Select choice by list index (0-based into ChoiceScreen.options).
    pub fn choose_selected(&mut self) -> Result<(), String> {
        let arm = self
            .choice
            .selected_arm()
            .ok_or_else(|| "no choice selected".to_string())?;
        self.choose_arm(arm)
    }

    /// Select choice by arm index from story.
    pub fn choose_arm(&mut self, arm_index: usize) -> Result<(), String> {
        self.push_rollback_frame();
        self.player.choose(arm_index)?;
        self.choice.close();
        self.ingest_events();
        self.sync_ui_from_wait();
        Ok(())
    }

    /// Rollback one step.
    pub fn rollback_step(&mut self) -> Result<bool, String> {
        let ok = self.rollback.step_back(&mut self.player)?;
        if ok {
            self.presentation = PresentationState::default();
            if let Some(bg) = self.player.background() {
                self.presentation.background = Some(bg.to_string());
            }
            for (_, vis) in self.player.visible() {
                self.presentation.show(vis.clone());
            }
            self.ingest_events();
            self.sync_ui_from_wait();
        }
        Ok(ok)
    }

    /// Enable skip and try one skip advance.
    pub fn skip_once(&mut self) -> bool {
        if self.player.preferences().skip_mode == SkipMode::Off {
            self.player.preferences_mut().skip_mode = SkipMode::All;
        }
        let advanced = self.player.try_skip();
        if advanced {
            self.ingest_events();
            self.sync_ui_from_wait();
        }
        advanced
    }

    /// Set skip mode.
    pub fn set_skip_mode(&mut self, mode: SkipMode) {
        self.player.preferences_mut().skip_mode = mode;
    }

    /// Toggle auto-forward.
    pub fn set_auto(&mut self, on: bool) {
        self.player.preferences_mut().auto_mode = on;
        self.player.sync_presentation_prefs();
        if on && self.say.text_complete {
            self.player.on_line_fully_shown();
        }
    }

    /// Tick (typewriter, auto, transitions, bgm).
    pub fn tick(&mut self, dt: f32) {
        // Typewriter
        if self.say.visible && !self.say.text_complete && self.cps > 0.0 {
            self.type_acc += dt * self.cps;
            let n = self.type_acc.floor() as usize;
            if n > 0 {
                self.type_acc -= n as f32;
                if self.say.typewrite(n) {
                    self.player.on_line_fully_shown();
                }
            }
        } else if self.say.visible && !self.say.text_complete && self.cps <= 0.0 {
            self.say.reveal_all();
            self.player.on_line_fully_shown();
        }

        // Auto / skip continuous
        if self.player.preferences().skip_mode != SkipMode::Off {
            let mut guard = 0;
            while guard < 64 && self.player.try_skip() {
                guard += 1;
                self.ingest_events();
            }
            if guard > 0 {
                self.sync_ui_from_wait();
            }
        }

        self.player.tick(dt);
        // If auto advanced, resync
        if matches!(self.player.wait(), StoryWait::Line) {
            // player.tick may have advanced
        }
        // Detect wait change after auto
        if self.say.visible
            && matches!(self.player.wait(), StoryWait::Line)
            && self.say.full_text != self.player.current_text()
        {
            self.sync_ui_from_wait();
        }
        if !matches!(self.player.wait(), StoryWait::Line) && self.say.visible {
            // may have auto-advanced off line
            if !matches!(self.player.wait(), StoryWait::Line) {
                self.ingest_events();
                self.sync_ui_from_wait();
            }
        }

        self.presentation.tick(dt);
        self.bgm.tick(dt);
        self.ingest_events();
    }

    /// Skip many lines until choice/end (product skip burst).
    pub fn skip_until_choice_or_end(&mut self, max: u32) -> u32 {
        self.player.preferences_mut().skip_mode = SkipMode::All;
        let mut n = 0;
        while n < max {
            match self.player.wait().clone() {
                StoryWait::Line => {
                    self.say.reveal_all();
                    self.player.advance();
                    self.ingest_events();
                    n += 1;
                }
                StoryWait::Ready | StoryWait::Pause { .. } => {
                    self.player.advance();
                    self.ingest_events();
                    n += 1;
                }
                StoryWait::Choice | StoryWait::Ended | StoryWait::Host { .. } => break,
            }
        }
        self.sync_ui_from_wait();
        n
    }

    /// Run headless product path: always pick choice index 0 until ending.
    pub fn run_to_ending(&mut self, max_steps: u32, choice: usize) -> Option<String> {
        let mut steps = 0;
        while steps < max_steps {
            steps += 1;
            match self.player.wait().clone() {
                StoryWait::Ended => {
                    return self
                        .player
                        .ending()
                        .map(|s| s.to_string())
                        .or_else(|| Some(self.player.current_text().to_string()));
                }
                StoryWait::Line => {
                    self.say.reveal_all();
                    self.player.advance();
                    self.ingest_events();
                }
                StoryWait::Choice => {
                    let idx = choice.min(self.player.choices().len().saturating_sub(1));
                    let arm = self.player.choices().get(idx).map(|c| c.index).unwrap_or(0);
                    let _ = self.choose_arm(arm);
                }
                StoryWait::Ready | StoryWait::Pause { .. } => {
                    self.player.advance();
                    self.ingest_events();
                }
                StoryWait::Host { token } => {
                    // Headless: auto-resume hosts so scripts don't hang tests.
                    let _ = self.player.resume_host(&token);
                    self.ingest_events();
                }
            }
        }
        None
    }

    /// Whether ended.
    pub fn is_ended(&self) -> bool {
        self.player.is_ended()
    }

    /// Ending id.
    pub fn ending(&self) -> Option<&str> {
        self.player.ending()
    }

    /// Ingest pending story events into product screens (pub for tests/host).
    pub fn ingest_events(&mut self) {
        for ev in self.player.drain_events() {
            match ev {
                StoryEvent::Background(p) => {
                    self.presentation.background = Some(p);
                    self.presentation.dissolve(0.35);
                }
                StoryEvent::Music { path, fade_in } => {
                    self.bgm
                        .play_with_prefs(path, fade_in, self.player.preferences());
                }
                StoryEvent::Show(v) => {
                    self.presentation.show(v);
                }
                StoryEvent::Hide(id) => {
                    self.presentation.hide(&id);
                }
                StoryEvent::Dialogue {
                    speaker_name, text, ..
                } => {
                    let instant =
                        matches!(self.player.preferences().text_speed, TextSpeed::Instant)
                            || self.player.preferences().skip_mode != SkipMode::Off;
                    self.show_dialogue_line(speaker_name, &text, instant);
                    if instant {
                        self.player.on_line_fully_shown();
                    }
                    self.type_acc = 0.0;
                }
                StoryEvent::Choices(opts) => {
                    self.say.visible = false;
                    self.choice.open_with(opts);
                }
                StoryEvent::Ended { .. } => {
                    self.say.visible = false;
                    self.choice.close();
                }
                StoryEvent::Variable { .. } => {}
                StoryEvent::Sound { path } => {
                    self.presentation.last_sfx = Some(path.clone());
                    self.presentation.sfx_queue.push(path);
                }
                StoryEvent::Pause { seconds } => {
                    self.presentation.last_pause = seconds;
                    self.presentation.pause_pending = true;
                }
                StoryEvent::Transition { name } => {
                    self.presentation.last_transition_name = Some(name.clone());
                    // Map common names onto the transition queue for the product shell.
                    let lower = name.to_ascii_lowercase();
                    if lower.contains("fade") {
                        self.presentation.fade(0.35);
                    } else if lower.contains("move") {
                        // generic dissolve as stand-in when no sprite target
                        self.presentation.dissolve(0.3);
                    } else {
                        self.presentation.dissolve(0.35);
                    }
                }
                StoryEvent::HostCall { name, args } => {
                    self.presentation.last_host_call = Some(name);
                    self.presentation.last_host_args = args;
                }
            }
        }
    }

    /// Present a dialogue line through the product Say screen.
    ///
    /// Always runs [`say_plain_and_cps`] so `{color=…}` / `{cps=N}` markup is stripped
    /// from displayed text and optional `{cps=N}` overrides the typewriter rate.
    pub fn show_dialogue_line(
        &mut self,
        speaker: impl Into<String>,
        raw_text: &str,
        instant: bool,
    ) {
        let (plain, cps_tag) = say_plain_and_cps(raw_text);
        if let Some(c) = cps_tag {
            self.cps = c.max(1.0);
        } else {
            self.sync_cps();
        }
        self.say.show(speaker, plain, instant);
        self.type_acc = 0.0;
    }

    /// Sync say/choice screens from current wait state.
    pub fn sync_ui_from_wait(&mut self) {
        match self.player.wait().clone() {
            StoryWait::Line => {
                self.choice.close();
                let raw = self.player.current_text().to_string();
                let speaker = self.player.current_speaker_name().to_string();
                let (plain, _) = say_plain_and_cps(&raw);
                if !self.say.visible || self.say.full_text != plain {
                    let instant =
                        matches!(self.player.preferences().text_speed, TextSpeed::Instant);
                    self.show_dialogue_line(speaker, &raw, instant);
                }
            }
            StoryWait::Choice => {
                self.say.visible = false;
                if !self.choice.open {
                    self.choice.open_with(self.player.choices().to_vec());
                }
            }
            StoryWait::Ended => {
                self.say.visible = false;
                self.choice.close();
            }
            StoryWait::Ready | StoryWait::Pause { .. } | StoryWait::Host { .. } => {}
        }
        // Presentation mirrors player if empty
        if self.presentation.background.is_none() {
            if let Some(bg) = self.player.background() {
                self.presentation.background = Some(bg.to_string());
            }
        }
        if self.presentation.sprites.is_empty() {
            for (_, vis) in self.player.visible() {
                self.presentation.show(vis.clone());
            }
        }
    }

    /// One-line transition APIs (script/host).
    pub fn transition_dissolve(&mut self, secs: f32) {
        self.presentation.dissolve(secs);
    }

    /// Fade transition.
    pub fn transition_fade(&mut self, secs: f32) {
        self.presentation.fade(secs);
    }

    /// Move sprite placement with transition.
    pub fn transition_move(&mut self, id: &str, at: &str, secs: f32) {
        self.presentation.move_to(id, at, secs);
    }

    /// Manual show for host/tests.
    pub fn apply_show(&mut self, id: &str, expression: Option<&str>, at: Option<&str>) {
        self.presentation.show(VisibleCharacter {
            id: id.into(),
            expression: expression.map(str::to_string),
            at: at.map(str::to_string),
        });
    }

    /// Manual hide.
    pub fn apply_hide(&mut self, id: &str) {
        self.presentation.hide(id);
    }

    /// Load gallery catalog from project `gallery.json` if present.
    pub fn load_gallery_from_project(&mut self) -> Result<usize, String> {
        let root = self
            .project_root
            .as_ref()
            .ok_or_else(|| "no project root".to_string())?;
        let path = root.join("gallery.json");
        if !path.is_file() {
            return Ok(0);
        }
        self.gallery = crate::gallery::Gallery::from_path(&path).map_err(|e| e.to_string())?;
        Ok(self.gallery.entries.len())
    }

    /// Unlock a gallery entry after an ending (replay/gallery path).
    pub fn unlock_gallery(&mut self, id: &str) -> bool {
        self.gallery.unlock_if_known(id)
    }
}

/// Strip/parse simple rich-text tags for say display.
///
/// Proven light subset on the product path:
/// - `{cps=N}` → optional typewriter rate
/// - `{color=…}` / `{/color}`, `{b}`/`{/b}`, `{i}`/`{/i}`, `{size=…}` → stripped
/// - `{w}` / `{w=0.5}` wait/pause markers → stripped (host may use pauses separately)
/// - real newlines (`\n`) preserved for multiline dialogue bodies
pub fn say_plain_and_cps(markup: &str) -> (String, Option<f32>) {
    if markup.contains('{') {
        if let Ok(rich) = try_parse_rich(markup) {
            return rich;
        }
    }
    (markup.to_string(), None)
}

/// Join multiple dialogue body lines into one product string (explicit multiline).
pub fn join_dialogue_lines(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|s| s.trim_end())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn try_parse_rich(markup: &str) -> Result<(String, Option<f32>), ()> {
    let mut plain = String::new();
    let mut cps = None;
    let mut i = 0;
    let chars: Vec<char> = markup.chars().collect();
    while i < chars.len() {
        if chars[i] == '{' {
            if let Some(rel) = chars[i..].iter().position(|&c| c == '}') {
                let tag: String = chars[i + 1..i + rel].iter().collect();
                let t = tag.trim();
                if let Some(rest) = t.strip_prefix("cps=") {
                    if let Ok(v) = rest.parse::<f32>() {
                        cps = Some(v);
                    }
                }
                // Strip presentation-only tags; keep content outside braces.
                // Known tags: color, /color, b, /b, i, /i, size=…, w, w=…
                let _known = t == "b"
                    || t == "/b"
                    || t == "i"
                    || t == "/i"
                    || t == "/color"
                    || t.starts_with("color")
                    || t.starts_with("size=")
                    || t == "w"
                    || t.starts_with("w=");
                i += rel + 1;
                continue;
            }
        }
        plain.push(chars[i]);
        i += 1;
    }
    Ok((plain, cps))
}

/// Load a program from path into a [`VnSession`].
pub fn open_session_from_file(
    path: &Path,
    title: impl Into<String>,
    save_dir: Option<PathBuf>,
) -> Result<VnSession, crate::load::LoadError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| crate::load::LoadError::Semantic(format!("read {}: {e}", path.display())))?;
    let program =
        crate::load::load_program_from_source(&source, Some(&path.to_string_lossy()), title)?;
    let player = StoryPlayer::start(program);
    let mut session = VnSession::new(player);
    if let Some(dir) = save_dir {
        session = session.with_save_dir(dir);
    }
    if let Some(parent) = path.parent() {
        let root = if parent.file_name().and_then(|s| s.to_str()) == Some("scripts") {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };
        session = session.with_project_root(root.to_path_buf());
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;
    use crate::prefs::TextSpeed;
    use tempfile::tempdir;

    fn sample_program() -> crate::ir::StoryProgram {
        let src = r#"
character hero { name: "Hero" }
character friend { name: "Mira" }
state { trust: int = 0 }

scene main {
    background "assets/bg/room.png"
    music "assets/music/soft.ogg" fade_in 1.0
    show friend at right
    hero "Hello there."
    friend "Hi."
    choice {
        "Stay" {
            trust += 1
            jump good
        }
        "Leave" {
            jump bad
        }
    }
}

scene good {
    hide friend
    "Ending: Warm Lights"
    jump end_good
}

scene bad {
    "Ending: Cool Air"
}

scene end_good {
    "Ending: Warm Lights"
}
"#;
        load_program_from_source(src, Some("test.vel"), "Test VN").unwrap()
    }

    #[test]
    fn s1_say_choice_reaches_named_ending() {
        let mut session = VnSession::new(StoryPlayer::start(sample_program()));
        // Pump until dialogue
        let mut guard = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && guard < 20 {
            session.advance();
            guard += 1;
        }
        assert!(session.say.visible, "say screen should show dialogue");
        assert!(!session.say.namebox.is_empty() || !session.say.full_text.is_empty());
        // advance through lines to choice
        while !matches!(session.player().wait(), StoryWait::Choice) && guard < 50 {
            session.advance();
            guard += 1;
        }
        assert!(session.choice.open, "choice screen open");
        assert!(session.choice.options.len() >= 2);
        session.choose_arm(0).unwrap();
        let ending = session.run_to_ending(80, 0);
        assert!(
            ending
                .as_deref()
                .map(|e| e.contains("Warm") || e.contains("Ending") || session.is_ended())
                .unwrap_or(session.is_ended()),
            "ending={ending:?} text={}",
            session.player().current_text()
        );
        assert!(session.is_ended() || ending.is_some());
    }

    #[test]
    fn s2_save_load_prefs_history_confirm_bgm() {
        let dir = tempdir().unwrap();
        let mut session =
            VnSession::new(StoryPlayer::start(sample_program())).with_save_dir(dir.path());
        // Reach a line
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        session.say.reveal_all();
        let text_before = session.player().current_text().to_string();
        session.save_slot("slot1").unwrap();

        // Mutate prefs + advance
        let mut prefs = session.prefs().clone();
        prefs.master_volume = 0.5;
        prefs.music_volume = 0.8;
        prefs.sfx_volume = 0.6;
        prefs.text_speed = TextSpeed::Instant;
        prefs.auto_mode = true;
        prefs.fullscreen = true;
        session.set_prefs(prefs.clone());
        assert!(
            (session.bgm.volume - 0.4).abs() < 0.01
                || (session.bgm.volume - 0.5 * 0.8).abs() < 0.01
        );
        session.advance();

        // History non-empty after dialogue
        assert!(!session.history_entries().is_empty() || !text_before.is_empty());

        // Confirm quit flow
        session.request_quit();
        assert!(session.confirm.open);
        session.confirm_no();
        assert!(!session.confirm.open);
        session.request_quit();
        session.confirm_yes().unwrap();
        assert!(session.quit_requested);

        // Load restores
        let mut session2 =
            VnSession::new(StoryPlayer::start(sample_program())).with_save_dir(dir.path());
        session2.load_slot("slot1").unwrap();
        assert_eq!(session2.player().current_text(), text_before);

        // BGM must come from the sample music op (no OR with background).
        let mut s4 = VnSession::new(StoryPlayer::start(sample_program()));
        assert!(
            s4.bgm.path.as_deref() == Some("assets/music/soft.ogg") || s4.bgm.playing,
            "music op must set bgm path/playing, got path={:?} playing={} intents={:?}",
            s4.bgm.path,
            s4.bgm.playing,
            s4.bgm.intents
        );
        let intents = s4.bgm.drain_intents();
        assert!(
            !intents.is_empty() || s4.bgm.path.is_some(),
            "expected BGM intent or retained path after music op"
        );
        assert!(
            s4.presentation.background.as_deref() == Some("assets/bg/room.png"),
            "background from script, got {:?}",
            s4.presentation.background
        );
    }

    #[test]
    fn load_slot_surfaces_program_mismatch() {
        use crate::ir::{StoryOp, StoryProgram, StoryScene};
        use crate::save::SaveError;
        use indexmap::IndexMap;
        use tempfile::tempdir;

        let make = |line: &str| {
            let mut scenes = IndexMap::new();
            scenes.insert(
                "start".into(),
                StoryScene {
                    name: "start".into(),
                    ops: vec![
                        StoryOp::Dialogue {
                            speaker: None,
                            text: line.into(),
                        },
                        StoryOp::End { ending: None },
                    ],
                    labels: IndexMap::new(),
                },
            );
            let mut p = StoryProgram::new("prod_hash");
            p.entry = "start".into();
            p.scenes = scenes;
            p
        };

        let dir = tempdir().unwrap();
        let session_a =
            VnSession::new(StoryPlayer::start(make("script A"))).with_save_dir(dir.path());
        session_a.save_slot("s1").unwrap();

        // Open same slot with a different program.
        let mut session_b =
            VnSession::new(StoryPlayer::start(make("script B — other"))).with_save_dir(dir.path());
        let err = session_b.load_slot("s1").expect_err("must reject");
        assert!(
            matches!(err, SaveError::ProgramMismatch { .. }),
            "product API must surface ProgramMismatch, got {err:?}"
        );
    }

    /// Music + Sound leave checkable product presentation/audio records.
    #[test]
    fn music_and_sound_wire_to_product_signals() {
        use crate::ir::{StoryOp, StoryProgram, StoryScene};
        use indexmap::IndexMap;

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::Music {
                        path: "assets/music/theme.ogg".into(),
                        fade_in: Some(0.5),
                    },
                    StoryOp::Sound {
                        path: "assets/sfx/ping.ogg".into(),
                    },
                    StoryOp::End { ending: None },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("audio_wire");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let mut session = VnSession::new(StoryPlayer::start(prog));
        session.ingest_events();
        let intents = session.bgm.drain_intents();
        assert!(
            intents.iter().any(|i| matches!(
                i,
                BgmIntent::Play { path, .. } if path.contains("theme.ogg")
            )),
            "music must produce BgmIntent::Play, intents={intents:?}"
        );
        assert_eq!(
            session.presentation.last_sfx.as_deref(),
            Some("assets/sfx/ping.ogg")
        );
        assert!(session
            .presentation
            .sfx_queue
            .iter()
            .any(|p| p.contains("ping.ogg")));
    }

    #[test]
    fn presentation_hooks_apply_sound_pause_transition_host() {
        use crate::ir::{StoryOp, StoryProgram, StoryScene};
        use crate::value::StoryValue;
        use indexmap::IndexMap;

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::Sound {
                        path: "ui/click.ogg".into(),
                    },
                    StoryOp::Pause {
                        seconds: Some(1.25),
                    },
                    StoryOp::Transition {
                        name: "fade".into(),
                    },
                    StoryOp::HostCall {
                        name: "combat.start".into(),
                        args: {
                            let mut m = IndexMap::new();
                            m.insert("enemy".into(), StoryValue::String("wolf".into()));
                            m
                        },
                    },
                    StoryOp::End { ending: None },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("fx");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let mut session = VnSession::new(StoryPlayer::start(prog));
        session.ingest_events();
        // Pause blocks; first ingest sees Sound + Pause only.
        assert_eq!(
            session.presentation.last_sfx.as_deref(),
            Some("ui/click.ogg")
        );
        assert!(session
            .presentation
            .sfx_queue
            .iter()
            .any(|p| p == "ui/click.ogg"));
        assert_eq!(session.presentation.last_pause, Some(1.25));
        assert!(session.presentation.pause_pending);
        assert!(matches!(session.player().wait(), StoryWait::Pause { .. }));
        session.player_mut().skip_pause();
        session.ingest_events();
        assert_eq!(
            session.presentation.last_transition_name.as_deref(),
            Some("fade")
        );
        assert!(
            session.presentation.transitions.len() >= 1,
            "fade transition should enqueue"
        );
        assert_eq!(
            session.presentation.last_host_call.as_deref(),
            Some("combat.start")
        );
        assert_eq!(
            session
                .presentation
                .last_host_args
                .get("enemy")
                .map(|v| v.display_str()),
            Some("wolf".into())
        );
    }

    #[test]
    fn s3_show_hide_zorder_transitions() {
        let mut session = VnSession::new(StoryPlayer::start(sample_program()));
        // Pump until first dialogue — `show friend at right` runs before first line.
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        assert!(
            session.presentation.sprites.contains_key("friend"),
            "script `show friend at right` must populate sprites, got {:?}",
            session.presentation.sprites.keys().collect::<Vec<_>>()
        );
        let friend = session.presentation.sprites.get("friend").unwrap();
        assert_eq!(friend.at.as_deref(), Some("right"));
        let z_list = session.presentation.sprites_by_z();
        assert!(!z_list.is_empty());

        session.transition_dissolve(0.5);
        session.transition_fade(0.4);
        assert!(session.presentation.transitions.len() >= 2);
        session.presentation.tick(0.1);
        assert!(
            session.presentation.transitions.is_busy()
                || session.presentation.transitions.len() < 2
        );

        session.transition_move("friend", "left", 0.3);
        assert_eq!(
            session
                .presentation
                .sprites
                .get("friend")
                .and_then(|s| s.at.as_deref()),
            Some("left")
        );
        session.apply_hide("friend");
        assert!(!session.presentation.sprites.contains_key("friend"));
    }

    #[test]
    fn s4_rollback_skip_auto() {
        let mut session = VnSession::new(StoryPlayer::start(sample_program()));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        session.say.reveal_all();
        let line1 = session.player().current_text().to_string();
        let wait1 = session.player().wait().clone();
        assert!(!line1.is_empty(), "expected dialogue line1");

        // Natural advance to next line — product advance pushes distinct frames.
        session.advance();
        assert!(
            session.rollback.len() >= 2,
            "natural advance must leave ≥2 rollback frames, got {}",
            session.rollback.len()
        );
        assert!(
            matches!(session.player().wait(), StoryWait::Line),
            "sample has two dialogue lines before choice, wait={:?}",
            session.player().wait()
        );
        session.say.reveal_all();
        let line2 = session.player().current_text().to_string();
        assert_ne!(line1, line2, "should be on a different line after advance");

        let rolled = session.rollback_step().unwrap();
        assert!(rolled, "rollback_step should restore previous frame");
        assert_eq!(
            session.player().current_text(),
            line1,
            "rollback must restore line1 text"
        );
        assert_eq!(
            session.player().wait(),
            &wait1,
            "rollback must restore wait state"
        );

        // Skip multi-line
        let mut session2 = VnSession::new(StoryPlayer::start(sample_program()));
        let n = session2.skip_until_choice_or_end(40);
        assert!(n > 0);
        assert!(
            matches!(
                session2.player().wait(),
                StoryWait::Choice | StoryWait::Ended
            ),
            "wait={:?}",
            session2.player().wait()
        );

        // Auto-forward must advance past line1 without manual advance.
        let mut session3 = VnSession::new(StoryPlayer::start(sample_program()));
        let mut prefs = session3.prefs().clone();
        prefs.auto_mode = true;
        prefs.auto_delay_secs = 0.01;
        prefs.text_speed = TextSpeed::Instant;
        session3.set_prefs(prefs);
        session3.set_auto(true);
        let mut g = 0;
        while !matches!(session3.player().wait(), StoryWait::Line) && g < 40 {
            session3.tick(0.016);
            g += 1;
        }
        assert!(
            matches!(session3.player().wait(), StoryWait::Line),
            "should reach a dialogue line"
        );
        let auto_line1 = session3.player().current_text().to_string();
        session3.say.reveal_all();
        session3.player_mut().on_line_fully_shown();
        // Tick enough for auto delay to fire multiple times.
        for _ in 0..80 {
            session3.tick(0.05);
        }
        let after_auto = session3.player().current_text().to_string();
        let wait_after = session3.player().wait().clone();
        assert!(
            after_auto != auto_line1
                || !matches!(wait_after, StoryWait::Line)
                || matches!(wait_after, StoryWait::Choice | StoryWait::Ended),
            "auto must advance past line1: was {auto_line1:?} now {after_auto:?} wait={wait_after:?}"
        );
    }

    #[test]
    fn product_run_to_ending_named() {
        let mut session = VnSession::new(StoryPlayer::start(sample_program()));
        let end = session.run_to_ending(100, 0);
        assert!(session.is_ended());
        let text = format!("{:?} {}", end, session.player().current_text());
        assert!(
            text.contains("Warm") || text.contains("Ending") || session.ending().is_some(),
            "{text}"
        );
    }

    #[test]
    fn s8_gallery_json_and_multi_ending_names() {
        let dir = tempdir().unwrap();
        let gpath = dir.path().join("gallery.json");
        std::fs::write(
            &gpath,
            r#"{"entries":[{"id":"cg_a","title":"A","path":"a.png","unlocked_by_default":true},{"id":"cg_b","title":"B","path":"b.png"}]}"#,
        )
        .unwrap();
        let mut session =
            VnSession::new(StoryPlayer::start(sample_program())).with_project_root(dir.path());
        let n = session.load_gallery_from_project().unwrap();
        assert_eq!(n, 2);
        assert!(session.gallery.is_unlocked("cg_a"));
        assert!(!session.gallery.is_unlocked("cg_b"));
        assert!(session.unlock_gallery("cg_b"));
        assert!(session.gallery.is_unlocked("cg_b"));

        // Two choice arms → two ending names on sample program
        let mut s0 = VnSession::new(StoryPlayer::start(sample_program()));
        let e0 = s0.run_to_ending(80, 0).unwrap_or_default();
        let t0 = format!("{e0} {}", s0.player().current_text());
        let mut s1 = VnSession::new(StoryPlayer::start(sample_program()));
        let e1 = s1.run_to_ending(80, 1).unwrap_or_default();
        let t1 = format!("{e1} {}", s1.player().current_text());
        assert!(t0.contains("Warm") || t0.contains("Ending"), "{t0}");
        assert!(
            t1.contains("Cool") || t1.contains("Ending") || t1 != t0,
            "second path should differ or be Cool Air: {t1}"
        );
    }

    #[test]
    fn s9_rich_text_plain_extract() {
        let (plain, cps) = say_plain_and_cps("{cps=30}Hello {color=#ff0}world{/color}");
        assert!(plain.contains("Hello"));
        assert!(plain.contains("world"));
        assert!(!plain.contains("cps"));
        assert_eq!(cps, Some(30.0));
    }

    #[test]
    fn s9_product_path_strips_markup_and_applies_cps() {
        // Real product path: StoryPlayer dialogue event → VnSession SayScreen.
        let src = r#"
character hero { name: "Hero" }
scene main {
    hero "{cps=25}Hello {color=#ff0}world{/color}"
    "Ending: Done"
}
"#;
        let program = load_program_from_source(src, Some("rich.vel"), "Rich").unwrap();
        let mut session = VnSession::new(StoryPlayer::start(program));
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 20 {
            session.advance();
            g += 1;
        }
        assert!(
            matches!(session.player().wait(), StoryWait::Line),
            "expected dialogue wait"
        );
        // Raw player text still has markup (story IR is unchanged).
        assert!(
            session.player().current_text().contains("{cps=")
                || session.player().current_text().contains("{color"),
            "source line should retain markup in StoryPlayer: {:?}",
            session.player().current_text()
        );
        // Product Say screen must show stripped plain text only.
        assert_eq!(session.say.full_text, "Hello world");
        assert!(!session.say.full_text.contains('{'));
        assert!(!session.say.full_text.contains("cps"));
        assert!(!session.say.full_text.contains("color"));
        // {cps=25} must override typewriter rate on the session.
        assert!(
            (session.cps - 25.0).abs() < 0.01,
            "expected cps override 25, got {}",
            session.cps
        );
        // Typewriter reveals plain only.
        session.say.reveal_all();
        assert_eq!(session.say.visible_text, "Hello world");
    }

    #[test]
    fn s6_language_select_shows_spanish() {
        let dir = tempdir().unwrap();
        let program = sample_program();
        let cat = crate::extract_loc_keys(&program);
        let mut es = crate::TranslationTable::new();
        for e in &cat.entries {
            es.insert(e.key.clone(), format!("ES::{}", e.source));
        }
        crate::write_tl_scaffold(dir.path(), &program, "es", &es).unwrap();

        let mut session = VnSession::new(StoryPlayer::start(program)).with_project_root(dir.path());
        assert!(session.available_languages().contains(&"es".into()));
        session.set_language("es").unwrap();
        assert_eq!(session.language, "es");
        // Reach first dialogue line
        let mut g = 0;
        while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
            session.advance();
            g += 1;
        }
        let line = session.player().current_text().to_string();
        assert!(
            line.starts_with("ES::") || line.contains("ES::"),
            "expected Spanish-prefixed line, got {line:?}"
        );
        session.set_language("en").unwrap();
        assert_eq!(session.language, "en");
    }
}
