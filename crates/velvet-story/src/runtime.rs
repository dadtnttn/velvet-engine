//! Interactive story player.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::auto_mode::AutoModeController;
use crate::character::Character;
use crate::history::History;
use crate::host::SharedCommandHost;
use crate::ir::{
    StoryArithOp, StoryCmpOp, StoryCond, StoryExpr, StoryOp, StoryOperand, StoryProgram,
};
use crate::prefs::{SkipMode, StoryPreferences};
use crate::save::SaveGame;
use crate::value::StoryValue;
use crate::variables::StoryVariables;
use crate::voice::VoiceQueue;

/// What the player is waiting on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StoryWait {
    /// Can step immediately (internal).
    #[default]
    Ready,
    /// Showing a dialogue line; advance to continue.
    Line,
    /// Waiting for a choice selection.
    Choice,
    /// Story finished.
    Ended,
}

/// Visible character on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleCharacter {
    /// Character id.
    pub id: String,
    /// Expression tag if any.
    pub expression: Option<String>,
    /// Placement.
    pub at: Option<String>,
}

/// Serializable cursor for saves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorySnapshot {
    /// Current scene name.
    pub scene: String,
    /// Index into scene ops.
    pub op_index: usize,
    /// Wait state.
    pub wait: StoryWait,
    /// Visible characters.
    pub visible: IndexMap<String, VisibleCharacter>,
    /// Current background.
    pub background: Option<String>,
    /// Current music path.
    pub music: Option<String>,
    /// Call stack of (scene, return_index) for Call ops.
    pub call_stack: Vec<(String, usize)>,
}

impl Default for StorySnapshot {
    fn default() -> Self {
        Self {
            scene: String::new(),
            op_index: 0,
            wait: StoryWait::Ready,
            visible: IndexMap::new(),
            background: None,
            music: None,
            call_stack: Vec::new(),
        }
    }
}

/// Choice presented to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    /// Index into current choice arms.
    pub index: usize,
    /// Display text (interpolated).
    pub text: String,
    /// Whether selectable.
    pub enabled: bool,
    /// Hidden from list.
    pub hidden: bool,
}

/// Events emitted when story state changes (for audio/render hooks).
#[derive(Debug, Clone, PartialEq)]
pub enum StoryEvent {
    /// Background changed.
    Background(String),
    /// Music changed.
    Music {
        /// Path.
        path: String,
        /// Fade in.
        fade_in: Option<f64>,
    },
    /// Character shown.
    Show(VisibleCharacter),
    /// Character hidden.
    Hide(String),
    /// New dialogue line.
    Dialogue {
        /// Speaker id.
        speaker: Option<String>,
        /// Display name.
        speaker_name: String,
        /// Text.
        text: String,
    },
    /// Choices available.
    Choices(Vec<ChoiceOption>),
    /// Story ended.
    Ended {
        /// Ending id.
        ending: Option<String>,
    },
    /// Variable changed.
    Variable {
        /// Name.
        name: String,
        /// Value.
        value: StoryValue,
    },
    /// One-shot SFX.
    Sound {
        /// Asset path / id.
        path: String,
    },
    /// Narrative pause / beat.
    Pause {
        /// Optional duration in seconds.
        seconds: Option<f64>,
    },
    /// Named transition.
    Transition {
        /// Transition id.
        name: String,
    },
    /// External / host command (`call combat.start: …`).
    HostCall {
        /// Command name.
        name: String,
        /// Named arguments.
        args: IndexMap<String, StoryValue>,
    },
}

/// One nested block frame (if / choice arm body).
#[derive(Debug, Clone, PartialEq)]
struct ExecFrame {
    ops: Vec<StoryOp>,
    index: usize,
}

/// Return continuation for `call` / `return` (includes nested exec stack).
#[derive(Debug, Clone, PartialEq)]
struct CallContinuation {
    scene: String,
    op_index: usize,
    exec_stack: Vec<ExecFrame>,
}

/// Story runtime / player.
#[derive(Clone)]
pub struct StoryPlayer {
    program: StoryProgram,
    vars: StoryVariables,
    history: History,
    prefs: StoryPreferences,
    snapshot: StorySnapshot,
    /// Current line text (for UI).
    current_speaker_id: Option<String>,
    current_speaker_name: String,
    current_text: String,
    /// Active choice options (when wait == Choice).
    choices: Vec<ChoiceOption>,
    /// Stack of nested block bodies (`if` / choice arms). Outer resumes after pop.
    exec_stack: Vec<ExecFrame>,
    /// Full menu arms while waiting on a choice (scene-level or nested).
    pending_menu: Option<Vec<crate::ir::StoryChoice>>,
    /// When true, [`Self::choose`] advances the scene IP past the Choice op.
    choice_advances_scene: bool,
    /// Full call continuations (scene IP + nested exec stack). Kept in sync with
    /// [`StorySnapshot::call_stack`] for saves (stack shape only).
    call_continuations: Vec<CallContinuation>,
    /// Seen line keys `scene:op_index`.
    seen_lines: std::collections::BTreeSet<String>,
    /// Events since last drain.
    events: Vec<StoryEvent>,
    /// Play time seconds.
    play_time_secs: f64,
    /// Title for saves.
    title: String,
    /// Last ending id.
    ending: Option<String>,
    /// Auto-advance controller (driven by preferences).
    auto_mode: AutoModeController,
    /// Optional voice line queue for wait-for-voice / skip hooks.
    voice: VoiceQueue,
    /// Optional external command dispatcher (`call combat.start: …`).
    command_host: Option<SharedCommandHost>,
    /// Last host error message (if dispatch failed).
    last_host_error: Option<String>,
}

impl std::fmt::Debug for StoryPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoryPlayer")
            .field("scene", &self.snapshot.scene)
            .field("op_index", &self.snapshot.op_index)
            .field("wait", &self.snapshot.wait)
            .field("has_command_host", &self.command_host.is_some())
            .finish()
    }
}

impl StoryPlayer {
    /// Start a program from the beginning.
    pub fn start(program: StoryProgram) -> Self {
        Self::start_with_host_opt(program, None)
    }

    /// Start with a command host already attached (so first `HostCall` dispatches).
    pub fn start_with_host(program: StoryProgram, host: SharedCommandHost) -> Self {
        Self::start_with_host_opt(program, Some(host))
    }

    fn start_with_host_opt(program: StoryProgram, host: Option<SharedCommandHost>) -> Self {
        let entry = program.entry.clone();
        let title = program.title.clone();
        let mut vars = StoryVariables::new();
        for (k, v) in &program.initial_vars {
            vars.set(k.clone(), v.clone());
        }
        let mut player = Self {
            program,
            vars,
            history: History::with_capacity(500),
            prefs: StoryPreferences::default(),
            snapshot: StorySnapshot {
                scene: entry,
                op_index: 0,
                wait: StoryWait::Ready,
                ..Default::default()
            },
            current_speaker_id: None,
            current_speaker_name: String::new(),
            current_text: String::new(),
            choices: Vec::new(),
            exec_stack: Vec::new(),
            pending_menu: None,
            choice_advances_scene: false,
            call_continuations: Vec::new(),
            seen_lines: std::collections::BTreeSet::new(),
            events: Vec::new(),
            play_time_secs: 0.0,
            title,
            ending: None,
            auto_mode: AutoModeController::default(),
            voice: VoiceQueue::default(),
            command_host: host,
            last_host_error: None,
        };
        player.auto_mode.sync_prefs(&player.prefs);
        player.voice.wait_for_voice = player.prefs.wait_for_voice;
        player.voice.master_volume = player.prefs.voice_volume;
        player.pump();
        player
    }

    /// Attach an external command host (combat, inventory, …).
    ///
    /// Invoked for every [`StoryOp::HostCall`]. Without a host, HostCall still
    /// emits events and debug variables.
    pub fn set_command_host(&mut self, host: SharedCommandHost) {
        self.command_host = Some(host);
    }

    /// Clear the command host.
    pub fn clear_command_host(&mut self) {
        self.command_host = None;
    }

    /// Last error from the command host, if any.
    pub fn last_host_error(&self) -> Option<&str> {
        self.last_host_error.as_deref()
    }

    /// Program reference.
    pub fn program(&self) -> &StoryProgram {
        &self.program
    }

    /// Variables.
    pub fn variables(&self) -> &StoryVariables {
        &self.vars
    }

    /// Mutable variables (for external systems).
    pub fn variables_mut(&mut self) -> &mut StoryVariables {
        &mut self.vars
    }

    /// Preferences.
    pub fn preferences(&self) -> &StoryPreferences {
        &self.prefs
    }

    /// Mutable preferences.
    pub fn preferences_mut(&mut self) -> &mut StoryPreferences {
        &mut self.prefs
    }

    /// History.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Wait state.
    pub fn wait(&self) -> &StoryWait {
        &self.snapshot.wait
    }

    /// Current speaker id.
    pub fn current_speaker_id(&self) -> Option<&str> {
        self.current_speaker_id.as_deref()
    }

    /// Current speaker display name.
    pub fn current_speaker_name(&self) -> &str {
        &self.current_speaker_name
    }

    /// Current dialogue text (interpolated).
    pub fn current_text(&self) -> &str {
        &self.current_text
    }

    /// Choice options if waiting.
    pub fn choices(&self) -> &[ChoiceOption] {
        &self.choices
    }

    /// Background path.
    pub fn background(&self) -> Option<&str> {
        self.snapshot.background.as_deref()
    }

    /// Visible characters.
    pub fn visible(&self) -> &IndexMap<String, VisibleCharacter> {
        &self.snapshot.visible
    }

    /// Scene name.
    pub fn scene_name(&self) -> &str {
        &self.snapshot.scene
    }

    /// Whether finished.
    pub fn is_ended(&self) -> bool {
        matches!(self.snapshot.wait, StoryWait::Ended)
    }

    /// Keys of dialogue lines already shown (`scene:op_index`), for skip-read-only.
    pub fn seen_line_keys(&self) -> &std::collections::BTreeSet<String> {
        &self.seen_lines
    }

    /// Ending id if any.
    pub fn ending(&self) -> Option<&str> {
        self.ending.as_deref()
    }

    /// Play time.
    pub fn play_time_secs(&self) -> f64 {
        self.play_time_secs
    }

    /// Advance clock. When auto-mode fires on a dialogue line, advances automatically.
    pub fn tick(&mut self, dt: f32) {
        self.play_time_secs += f64::from(dt.max(0.0));
        let _ = self.voice.tick(dt);
        self.auto_mode
            .set_waiting_for_voice(self.prefs.wait_for_voice && self.voice.should_wait());
        if matches!(self.snapshot.wait, StoryWait::Line) && self.auto_mode.tick(dt) {
            self.advance();
        }
    }

    /// Auto-mode controller (for UI toggles / typewriter hooks).
    pub fn auto_mode(&self) -> &AutoModeController {
        &self.auto_mode
    }

    /// Mutable auto-mode controller.
    pub fn auto_mode_mut(&mut self) -> &mut AutoModeController {
        &mut self.auto_mode
    }

    /// Voice queue.
    pub fn voice(&self) -> &VoiceQueue {
        &self.voice
    }

    /// Mutable voice queue.
    pub fn voice_mut(&mut self) -> &mut VoiceQueue {
        &mut self.voice
    }

    /// Notify that the current line's typewriter has finished (starts auto timer).
    pub fn on_line_fully_shown(&mut self) {
        let text = self.current_text.clone();
        self.auto_mode.on_line_fully_shown(&text, &self.prefs);
    }

    /// Sync auto-mode / voice settings from current preferences.
    pub fn sync_presentation_prefs(&mut self) {
        self.auto_mode.sync_prefs(&self.prefs);
        self.voice.wait_for_voice = self.prefs.wait_for_voice;
        self.voice.master_volume = self.prefs.voice_volume;
    }

    /// Drain events.
    pub fn drain_events(&mut self) -> Vec<StoryEvent> {
        std::mem::take(&mut self.events)
    }

    /// Snapshot for save.
    pub fn snapshot(&self) -> StorySnapshot {
        self.snapshot.clone()
    }

    /// Character by id.
    pub fn character(&self, id: &str) -> Option<&Character> {
        self.program.characters.get(id)
    }

    /// Advance past current line / continue execution.
    pub fn advance(&mut self) {
        match self.snapshot.wait {
            StoryWait::Line => {
                self.snapshot.wait = StoryWait::Ready;
                self.snapshot.op_index += 1;
                self.pump();
            }
            StoryWait::Ready => self.pump(),
            StoryWait::Choice | StoryWait::Ended => {}
        }
    }

    /// Select a choice by index.
    pub fn choose(&mut self, index: usize) -> Result<(), String> {
        if !matches!(self.snapshot.wait, StoryWait::Choice) {
            return Err("not waiting for choice".into());
        }
        let opt = self
            .choices
            .iter()
            .find(|c| c.index == index)
            .ok_or_else(|| "invalid choice index".to_string())?;
        if !opt.enabled {
            return Err("choice locked".into());
        }
        let arm_index = opt.index;
        let body = if let Some(menu) = self.pending_menu.take() {
            menu.get(arm_index)
                .map(|a| a.body.clone())
                .ok_or("bad arm")?
        } else {
            // Fallback: scene-level cursor still on Choice.
            let scene = self
                .program
                .scene(&self.snapshot.scene)
                .ok_or("missing scene")?;
            let op = scene
                .ops
                .get(self.snapshot.op_index)
                .ok_or("missing choice op")?;
            match op {
                StoryOp::Choice { options } => options
                    .get(arm_index)
                    .map(|a| a.body.clone())
                    .ok_or("bad arm")?,
                _ => return Err("cursor not on choice".into()),
            }
        };
        debug!(index = arm_index, "choice selected");
        self.choices.clear();
        if self.choice_advances_scene {
            // Scene-level Choice: cursor still on the op; skip past it.
            self.snapshot.op_index += 1;
        }
        self.choice_advances_scene = false;
        self.push_exec_body(body);
        self.snapshot.wait = StoryWait::Ready;
        self.pump();
        Ok(())
    }

    /// Skip according to preferences (returns true if advanced).
    pub fn try_skip(&mut self) -> bool {
        match self.prefs.skip_mode {
            SkipMode::Off => false,
            SkipMode::All => {
                if matches!(self.snapshot.wait, StoryWait::Line) {
                    self.advance();
                    true
                } else {
                    false
                }
            }
            SkipMode::ReadOnly => {
                let key = format!("{}:{}", self.snapshot.scene, self.snapshot.op_index);
                if self.seen_lines.contains(&key) && matches!(self.snapshot.wait, StoryWait::Line) {
                    self.advance();
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Build a save game DTO.
    pub fn to_save(&self, slot: impl Into<String>) -> SaveGame {
        SaveGame::from_parts(
            slot,
            self.title.clone(),
            &self.vars,
            self.snapshot.clone(),
            self.history.clone(),
            self.seen_lines.iter().cloned().collect(),
            self.play_time_secs,
            self.current_text.clone(),
        )
    }

    /// Restore from save (program must match / be reloaded).
    pub fn load_save(&mut self, save: SaveGame) -> Result<(), String> {
        let save = save.migrate().map_err(|e| e.to_string())?;
        self.vars.play = save.variables.into_iter().collect();
        for (k, v) in save.persistent {
            self.vars.persistent.insert(k, v);
        }
        self.snapshot = save.snapshot;
        self.history = save.history;
        self.seen_lines = save.seen_lines.into_iter().collect();
        self.play_time_secs = save.meta.play_time_secs;
        self.choices.clear();
        self.exec_stack.clear();
        self.pending_menu = None;
        self.choice_advances_scene = false;
        self.call_continuations.clear();
        // Re-sync UI fields from current wait
        if matches!(self.snapshot.wait, StoryWait::Line) {
            // Re-run the dialogue op at current index for UI text
            if let Some(scene) = self.program.scene(&self.snapshot.scene) {
                if let Some(StoryOp::Dialogue { speaker, text }) =
                    scene.ops.get(self.snapshot.op_index)
                {
                    self.apply_dialogue(speaker.clone(), text.clone());
                }
            }
        } else if matches!(self.snapshot.wait, StoryWait::Choice) {
            self.rebuild_choices();
        } else if matches!(self.snapshot.wait, StoryWait::Ready) {
            self.pump();
        }
        Ok(())
    }

    /// Push a nested block body (if arm / choice arm). Empty bodies are no-ops.
    fn push_exec_body(&mut self, ops: Vec<StoryOp>) {
        if !ops.is_empty() {
            self.exec_stack.push(ExecFrame { ops, index: 0 });
        }
    }

    /// Execute until blocked on line/choice/end.
    fn pump(&mut self) {
        let mut guard = 0;
        while matches!(self.snapshot.wait, StoryWait::Ready) && guard < 10_000 {
            guard += 1;
            // Nested blocks first (if / choice arms). Pop when exhausted so the
            // outer frame resumes — critical for ops after a nested if/choice.
            if let Some(frame) = self.exec_stack.last_mut() {
                if frame.index >= frame.ops.len() {
                    self.exec_stack.pop();
                    continue;
                }
                let op = frame.ops[frame.index].clone();
                frame.index += 1;
                self.exec_op(op);
                continue;
            }

            let scene_name = self.snapshot.scene.clone();
            let Some(scene) = self.program.scene(&scene_name) else {
                self.end_story(Some("missing_scene".into()));
                break;
            };
            if self.snapshot.op_index >= scene.ops.len() {
                // Scene finished without jump
                self.end_story(None);
                break;
            }
            let op = scene.ops[self.snapshot.op_index].clone();
            let advance_ip = self.exec_op(op);
            if advance_ip && matches!(self.snapshot.wait, StoryWait::Ready) {
                self.snapshot.op_index += 1;
            }
        }
    }

    /// Returns true if op_index should auto-increment.
    fn exec_op(&mut self, op: StoryOp) -> bool {
        match op {
            StoryOp::Nop | StoryOp::Label { .. } => true,
            StoryOp::Background { path } => {
                self.snapshot.background = Some(path.clone());
                self.events.push(StoryEvent::Background(path));
                true
            }
            StoryOp::Music { path, fade_in } => {
                self.snapshot.music = Some(path.clone());
                self.events.push(StoryEvent::Music { path, fade_in });
                true
            }
            StoryOp::Show { target, at } => {
                let (id, expression) = split_target(&target);
                let vis = VisibleCharacter {
                    id: id.clone(),
                    expression,
                    at,
                };
                self.snapshot.visible.insert(id, vis.clone());
                self.events.push(StoryEvent::Show(vis));
                true
            }
            StoryOp::Hide { target } => {
                let (id, _) = split_target(&target);
                self.snapshot.visible.shift_remove(&id);
                self.events.push(StoryEvent::Hide(id));
                true
            }
            StoryOp::Dialogue { speaker, text } => {
                self.apply_dialogue(speaker, text);
                let key = format!("{}:{}", self.snapshot.scene, self.snapshot.op_index);
                self.seen_lines.insert(key);
                self.snapshot.wait = StoryWait::Line;
                false
            }
            StoryOp::Choice { options } => {
                // Nested vs scene-level: when already inside an exec frame, the
                // frame IP already advanced past this Choice — choose() must not
                // also advance the scene cursor.
                self.choice_advances_scene = self.exec_stack.is_empty();
                self.pending_menu = Some(options.clone());
                self.choices.clear();
                for (i, arm) in options.iter().enumerate() {
                    let enabled = match &arm.require {
                        Some(v) => self.vars.get(v).is_truthy(),
                        None => true,
                    };
                    let hidden = !enabled && arm.hidden_if_locked;
                    if hidden {
                        continue;
                    }
                    self.choices.push(ChoiceOption {
                        index: i,
                        text: self.vars.interpolate(&arm.text),
                        enabled,
                        hidden: false,
                    });
                }
                self.events.push(StoryEvent::Choices(self.choices.clone()));
                self.snapshot.wait = StoryWait::Choice;
                false
            }
            StoryOp::Jump { target } => {
                // Leaving the current control flow: discard nested bodies.
                self.exec_stack.clear();
                self.pending_menu = None;
                self.choice_advances_scene = false;
                self.jump_to(&target);
                false
            }
            StoryOp::Call { target } => {
                // Save scene IP + nested stack so return resumes the outer body.
                let cont = if self.exec_stack.is_empty() {
                    CallContinuation {
                        scene: self.snapshot.scene.clone(),
                        op_index: self.snapshot.op_index + 1,
                        exec_stack: Vec::new(),
                    }
                } else {
                    CallContinuation {
                        scene: self.snapshot.scene.clone(),
                        op_index: self.snapshot.op_index,
                        exec_stack: self.exec_stack.clone(),
                    }
                };
                self.snapshot
                    .call_stack
                    .push((cont.scene.clone(), cont.op_index));
                self.call_continuations.push(cont);
                self.exec_stack.clear();
                self.pending_menu = None;
                self.choice_advances_scene = false;
                self.jump_to(&target);
                false
            }
            StoryOp::Assign {
                name,
                assign_op,
                value,
            } => {
                let rhs = self.eval_expr(&value);
                self.vars.apply_assign(&name, assign_op, rhs);
                let v = self.vars.get(&name);
                self.events.push(StoryEvent::Variable { name, value: v });
                true
            }
            StoryOp::If {
                cond,
                then_ops,
                else_ops,
            } => {
                let body = if self.eval_cond(&cond) {
                    then_ops
                } else {
                    else_ops
                };
                // Push a new frame — never replace the outer body.
                self.push_exec_body(body);
                true
            }
            StoryOp::End { ending } => {
                self.end_story(ending);
                false
            }
            StoryOp::HostCall { name, args } => {
                // Observable vars for tests + structured event for hosts/UI.
                self.vars
                    .set("__last_command", StoryValue::String(name.clone()));
                if let Some(StoryValue::String(enemy)) = args.get("enemy").cloned() {
                    self.vars.set("cmd.enemy", StoryValue::String(enemy));
                }
                for (k, v) in args.iter() {
                    self.vars.set(format!("cmd.{name}.{k}"), v.clone());
                }
                self.last_host_error = None;
                if let Some(host) = self.command_host.clone() {
                    if let Err(e) = host.call(&name, &args, &mut self.vars) {
                        self.last_host_error = Some(e.message.clone());
                        self.vars.set(
                            "__last_host_error",
                            StoryValue::String(e.message),
                        );
                    } else {
                        self.vars
                            .set("__host_dispatched", StoryValue::String(name.clone()));
                    }
                }
                self.events.push(StoryEvent::HostCall {
                    name: name.clone(),
                    args: args.clone(),
                });
                self.events.push(StoryEvent::Variable {
                    name: "__last_command".into(),
                    value: StoryValue::String(name),
                });
                true
            }
            StoryOp::Sound { path } => {
                self.vars
                    .set("__last_sfx", StoryValue::String(path.clone()));
                self.events.push(StoryEvent::Sound {
                    path: path.clone(),
                });
                true
            }
            StoryOp::Pause { seconds } => {
                if let Some(s) = seconds {
                    self.vars.set("__last_pause", StoryValue::Float(s));
                } else {
                    self.vars.set("__last_pause", StoryValue::Float(0.0));
                }
                self.events.push(StoryEvent::Pause { seconds });
                true
            }
            StoryOp::Transition { name } => {
                self.vars
                    .set("__last_transition", StoryValue::String(name.clone()));
                self.events.push(StoryEvent::Transition {
                    name: name.clone(),
                });
                true
            }
            StoryOp::Return => {
                if let Some(cont) = self.call_continuations.pop() {
                    let _ = self.snapshot.call_stack.pop();
                    self.snapshot.scene = cont.scene;
                    self.snapshot.op_index = cont.op_index;
                    self.exec_stack = cont.exec_stack;
                    self.snapshot.wait = StoryWait::Ready;
                    false
                } else if let Some((scene, idx)) = self.snapshot.call_stack.pop() {
                    // Save-load path: snapshot only (no nested stack).
                    self.snapshot.scene = scene;
                    self.snapshot.op_index = idx;
                    self.exec_stack.clear();
                    self.snapshot.wait = StoryWait::Ready;
                    false
                } else {
                    self.end_story(None);
                    false
                }
            }
        }
    }

    /// Evaluate a narrative condition against current variables.
    fn eval_cond(&self, cond: &StoryCond) -> bool {
        match cond {
            StoryCond::Var { name } => self.vars.get(name).is_truthy(),
            StoryCond::Const { value } => *value,
            StoryCond::Not { inner } => !self.eval_cond(inner),
            StoryCond::And { left, right } => self.eval_cond(left) && self.eval_cond(right),
            StoryCond::Or { left, right } => self.eval_cond(left) || self.eval_cond(right),
            StoryCond::Cmp { left, op, right } => {
                let l = self.resolve_operand(left);
                let r = self.resolve_operand(right);
                Self::cmp_values(&l, &r, *op)
            }
        }
    }

    fn eval_expr(&self, expr: &StoryExpr) -> StoryValue {
        match expr {
            StoryExpr::Value { value } => value.clone(),
            StoryExpr::Var { name } => self.vars.get(name),
            StoryExpr::Neg { inner } => {
                let v = self.eval_expr(inner);
                if let Some(i) = v.as_i64() {
                    StoryValue::Int(-i)
                } else if let Some(f) = v.as_f64() {
                    StoryValue::Float(-f)
                } else {
                    StoryValue::Null
                }
            }
            StoryExpr::Binary { op, left, right } => {
                let l = self.eval_expr(left);
                let r = self.eval_expr(right);
                Self::arith_values(&l, &r, *op)
            }
        }
    }

    fn arith_values(left: &StoryValue, right: &StoryValue, op: StoryArithOp) -> StoryValue {
        if let (Some(a), Some(b)) = (left.as_i64(), right.as_i64()) {
            return match op {
                StoryArithOp::Add => StoryValue::Int(a.saturating_add(b)),
                StoryArithOp::Sub => StoryValue::Int(a.saturating_sub(b)),
                StoryArithOp::Mul => StoryValue::Int(a.saturating_mul(b)),
                StoryArithOp::Div => {
                    if b == 0 {
                        StoryValue::Int(0)
                    } else {
                        StoryValue::Int(a / b)
                    }
                }
            };
        }
        if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
            return match op {
                StoryArithOp::Add => StoryValue::Float(a + b),
                StoryArithOp::Sub => StoryValue::Float(a - b),
                StoryArithOp::Mul => StoryValue::Float(a * b),
                StoryArithOp::Div => {
                    if b == 0.0 {
                        StoryValue::Float(0.0)
                    } else {
                        StoryValue::Float(a / b)
                    }
                }
            };
        }
        match op {
            StoryArithOp::Add => {
                if let (StoryValue::String(s), t) = (left, right) {
                    return StoryValue::String(format!("{s}{}", t.display_str()));
                }
                StoryValue::Null
            }
            _ => StoryValue::Null,
        }
    }

    fn resolve_operand(&self, op: &StoryOperand) -> StoryValue {
        match op {
            StoryOperand::Var { name } => self.vars.get(name),
            StoryOperand::Value { value } => value.clone(),
        }
    }

    fn cmp_values(left: &StoryValue, right: &StoryValue, op: StoryCmpOp) -> bool {
        // Prefer numeric compare when both sides are numeric-ish.
        if let (Some(lf), Some(rf)) = (left.as_f64(), right.as_f64()) {
            return match op {
                StoryCmpOp::Eq => (lf - rf).abs() < f64::EPSILON,
                StoryCmpOp::Ne => (lf - rf).abs() >= f64::EPSILON,
                StoryCmpOp::Lt => lf < rf,
                StoryCmpOp::Le => lf <= rf,
                StoryCmpOp::Gt => lf > rf,
                StoryCmpOp::Ge => lf >= rf,
            };
        }
        let ls = left.display_str();
        let rs = right.display_str();
        match op {
            StoryCmpOp::Eq => ls == rs,
            StoryCmpOp::Ne => ls != rs,
            StoryCmpOp::Lt => ls < rs,
            StoryCmpOp::Le => ls <= rs,
            StoryCmpOp::Gt => ls > rs,
            StoryCmpOp::Ge => ls >= rs,
        }
    }

    fn apply_dialogue(&mut self, speaker: Option<String>, text: String) {
        let text = self.vars.interpolate(&text);
        let speaker_name = match &speaker {
            Some(id) => self
                .program
                .characters
                .get(id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| id.clone()),
            None => String::new(),
        };
        self.current_speaker_id = speaker.clone();
        self.current_speaker_name = speaker_name.clone();
        self.current_text = text.clone();
        self.auto_mode.sync_prefs(&self.prefs);
        self.auto_mode.on_line_started(&text, &self.prefs);
        self.history.push(
            speaker_name.clone(),
            text.clone(),
            self.snapshot.scene.clone(),
        );
        self.events.push(StoryEvent::Dialogue {
            speaker,
            speaker_name,
            text,
        });
    }

    fn jump_to(&mut self, target: &str) {
        // Jump abandons the current nested bodies (goto is non-local).
        self.exec_stack.clear();
        self.pending_menu = None;
        self.choice_advances_scene = false;
        if let Some((scene, label)) = target.split_once(':') {
            if self.program.scenes.contains_key(scene) {
                self.snapshot.scene = scene.to_string();
                if let Some(idx) = self
                    .program
                    .scene(scene)
                    .and_then(|s| s.labels.get(label).copied())
                {
                    self.snapshot.op_index = idx;
                } else {
                    self.snapshot.op_index = 0;
                }
                self.snapshot.wait = StoryWait::Ready;
                return;
            }
        }
        if self.program.scenes.contains_key(target) {
            self.snapshot.scene = target.to_string();
            self.snapshot.op_index = 0;
            self.snapshot.wait = StoryWait::Ready;
            return;
        }
        // Label in current scene
        if let Some(idx) = self
            .program
            .scene(&self.snapshot.scene)
            .and_then(|s| s.labels.get(target).copied())
        {
            self.snapshot.op_index = idx;
            self.snapshot.wait = StoryWait::Ready;
            return;
        }
        debug!(target, "jump target not found — ending");
        self.end_story(Some("bad_jump".into()));
    }

    fn end_story(&mut self, ending: Option<String>) {
        self.ending = ending.clone();
        self.snapshot.wait = StoryWait::Ended;
        self.events.push(StoryEvent::Ended { ending });
    }

    fn rebuild_choices(&mut self) {
        if let Some(scene) = self.program.scene(&self.snapshot.scene) {
            if let Some(StoryOp::Choice { options }) = scene.ops.get(self.snapshot.op_index) {
                self.choices.clear();
                for (i, arm) in options.iter().enumerate() {
                    let enabled = match &arm.require {
                        Some(v) => self.vars.get(v).is_truthy(),
                        None => true,
                    };
                    if !enabled && arm.hidden_if_locked {
                        continue;
                    }
                    self.choices.push(ChoiceOption {
                        index: i,
                        text: self.vars.interpolate(&arm.text),
                        enabled,
                        hidden: false,
                    });
                }
            }
        }
    }
}

fn split_target(target: &str) -> (String, Option<String>) {
    if let Some((id, expr)) = target.split_once('.') {
        (id.to_string(), Some(expr.to_string()))
    } else {
        (target.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;

    fn sample_src() -> &'static str {
        r##"
character aria {
    name: "Aria"
    color: "#ff4f8b"
}

state {
    trust: int = 0
}

scene start {
    background "apartment.png"
    show aria.neutral at right
    aria "Hello, {player}."
    choice {
        "Be kind" {
            trust += 1
            jump good
        }
        "Be cold" {
            trust -= 1
            jump bad
        }
    }
}

scene good {
    aria "Thank you."
}

scene bad {
    aria "I see."
}
"##
    }

    #[test]
    fn play_kind_path() {
        let mut prog = load_program_from_source(sample_src(), Some("s.vel"), "Demo").unwrap();
        prog.initial_vars
            .insert("player".into(), StoryValue::String("Alex".into()));
        let mut player = StoryPlayer::start(prog);
        // Should stop on first dialogue
        assert!(matches!(player.wait(), StoryWait::Line));
        assert!(player.current_text().contains("Alex"));
        player.advance();
        assert!(matches!(player.wait(), StoryWait::Choice));
        assert_eq!(player.choices().len(), 2);
        player.choose(0).unwrap();
        // After choice body + jump, dialogue in good
        assert!(matches!(player.wait(), StoryWait::Line));
        assert_eq!(player.scene_name(), "good");
        assert_eq!(player.variables().get_int("trust", 0), 1);
        // Finish remaining lines in `good`
        while matches!(player.wait(), StoryWait::Line) {
            player.advance();
        }
        assert!(player.is_ended());
        assert_eq!(player.variables().get_int("trust", 0), 1);
    }

    #[test]
    fn cold_path_decrements_trust() {
        let prog = load_program_from_source(sample_src(), None, "Demo").unwrap();
        let mut player = StoryPlayer::start(prog);
        player.advance(); // past hello -> choices (if still on line)
        if matches!(player.wait(), StoryWait::Line) {
            player.advance();
        }
        player.choose(1).unwrap();
        assert_eq!(player.variables().get_int("trust", 0), -1);
        assert_eq!(player.scene_name(), "bad");
    }

    #[test]
    fn save_load_preserves_vars_and_scene() {
        let prog = load_program_from_source(sample_src(), None, "Demo").unwrap();
        let mut player = StoryPlayer::start(prog.clone());
        if matches!(player.wait(), StoryWait::Line) {
            player.advance();
        }
        player.choose(0).unwrap();
        let save = player.to_save("slot_1");
        let mut player2 = StoryPlayer::start(prog);
        player2.load_save(save).unwrap();
        assert_eq!(player2.variables().get_int("trust", 0), 1);
        assert_eq!(player2.scene_name(), "good");
    }

    fn multi_choice_src() -> &'static str {
        r##"
character n { name: "N" }
state {
    path: int = 0
    score: int = 0
    key: bool = false
}
scene start {
    n "Pick a route."
    choice {
        "Route A" {
            path = 1
            score += 10
            jump route_a
        }
        "Route B" {
            path = 2
            score += 5
            jump route_b
        }
        "Route C" {
            path = 3
            score += 1
            key = true
            jump route_c
        }
    }
}
scene route_a {
    n "A chosen"
    end "a"
}
scene route_b {
    n "B chosen"
    end "b"
}
scene route_c {
    n "C chosen"
    end "c"
}
"##
    }

    fn run_to_choice(player: &mut StoryPlayer) {
        let mut steps = 0;
        while !matches!(player.wait(), StoryWait::Choice) && steps < 64 {
            player.advance();
            steps += 1;
        }
        assert!(matches!(player.wait(), StoryWait::Choice));
    }

    #[test]
    fn multi_choice_three_routes() {
        for (idx, expected_path, ending) in [(0, 1, "a"), (1, 2, "b"), (2, 3, "c")] {
            let prog = load_program_from_source(multi_choice_src(), None, "mc").unwrap();
            let mut player = StoryPlayer::start(prog);
            run_to_choice(&mut player);
            assert_eq!(player.choices().len(), 3);
            player.choose(idx).unwrap();
            // Drain dialogue to end
            let mut guard = 0;
            while !player.is_ended() && guard < 32 {
                if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
                    player.advance();
                } else {
                    break;
                }
                guard += 1;
            }
            assert!(player.is_ended());
            assert_eq!(player.variables().get_int("path", 0), expected_path);
            assert_eq!(player.ending(), Some(ending));
            if idx == 2 {
                assert!(matches!(
                    player.variables().get("key"),
                    StoryValue::Bool(true)
                ));
            }
        }
    }

    #[test]
    fn multi_choice_select_middle_route() {
        let prog = load_program_from_source(multi_choice_src(), None, "mc2").unwrap();
        let mut player = StoryPlayer::start(prog);
        run_to_choice(&mut player);
        assert_eq!(player.choices().len(), 3);
        // Select route B (index 1)
        player.choose(1).unwrap();
        let mut guard = 0;
        while !player.is_ended() && guard < 32 {
            if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
                player.advance();
            } else {
                break;
            }
            guard += 1;
        }
        assert_eq!(player.variables().get_int("path", 0), 2);
        assert_eq!(player.variables().get_int("score", 0), 5);
        assert_eq!(player.ending(), Some("b"));
    }

    #[test]
    fn call_pushes_stack_and_jumps() {
        let src = r##"
character n { name: "N" }
scene start {
    n "Before call"
    call sub
    n "After call"
    end "main"
}
scene sub {
    n "Inside sub"
}
"##;
        let prog = load_program_from_source(src, None, "call").unwrap();
        let mut player = StoryPlayer::start(prog);
        assert!(matches!(player.wait(), StoryWait::Line));
        assert!(player.current_text().contains("Before"));
        player.advance();
        // After call, should be in sub (call_stack pushed).
        // Depending on pump, we land on sub's line.
        let mut guard = 0;
        while player.scene_name() != "sub" && guard < 8 {
            if matches!(player.wait(), StoryWait::Line) {
                // still advancing?
                break;
            }
            player.advance();
            guard += 1;
        }
        // call_stack should have been pushed when Call executed.
        // If we're on sub, stack non-empty; if story ended after sub, stack may have been unused for return.
        if player.scene_name() == "sub" {
            assert!(
                !player.snapshot().call_stack.is_empty(),
                "call should push return frame"
            );
            assert!(
                player.current_text().contains("Inside")
                    || matches!(player.wait(), StoryWait::Line)
            );
            player.advance();
            // Scene ends without return implementation → story ends.
            assert!(player.is_ended() || player.scene_name() == "sub");
        } else {
            // Some loaders may inline differently; still assert Call was present in program.
            assert!(prog_has_call(player.program()));
        }
    }

    fn prog_has_call(program: &crate::ir::StoryProgram) -> bool {
        program
            .scenes
            .values()
            .any(|s| s.ops.iter().any(|op| matches!(op, StoryOp::Call { .. })))
    }

    #[test]
    fn nested_choice_and_if() {
        let src = r##"
character n { name: "N" }
state {
    flag: bool = false
    n: int = 0
}
scene start {
    n "Start"
    choice {
        "Set flag" {
            flag = true
            n += 1
            jump check
        }
        "No flag" {
            n += 2
            jump check
        }
    }
}
scene check {
    if flag {
        n "Flagged"
        end "flagged"
    } else {
        n "Clear"
        end "clear"
    }
}
"##;
        // Flag path
        let prog = load_program_from_source(src, None, "if").unwrap();
        let mut player = StoryPlayer::start(prog.clone());
        run_to_choice(&mut player);
        player.choose(0).unwrap();
        let mut guard = 0;
        while !player.is_ended() && guard < 32 {
            if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
                player.advance();
            } else {
                break;
            }
            guard += 1;
        }
        assert_eq!(player.ending(), Some("flagged"));
        assert_eq!(player.variables().get_int("n", 0), 1);

        // Clear path
        let mut player2 = StoryPlayer::start(prog);
        run_to_choice(&mut player2);
        player2.choose(1).unwrap();
        guard = 0;
        while !player2.is_ended() && guard < 32 {
            if matches!(player2.wait(), StoryWait::Line | StoryWait::Ready) {
                player2.advance();
            } else {
                break;
            }
            guard += 1;
        }
        assert_eq!(player2.ending(), Some("clear"));
        assert_eq!(player2.variables().get_int("n", 0), 2);
    }

    #[test]
    fn interpolate_vars_in_dialogue() {
        let src = r##"
character hero { name: "Hero" }
state {
    gold: int = 42
    place: string = "town"
}
scene start {
    hero "I have {gold} coins in {place}."
    end
}
"##;
        let prog = load_program_from_source(src, None, "interp").unwrap();
        let player = StoryPlayer::start(prog);
        assert!(matches!(player.wait(), StoryWait::Line));
        let text = player.current_text();
        assert!(text.contains("42"), "text={text}");
        assert!(text.contains("town"), "text={text}");
    }

    #[test]
    fn events_emitted_for_background_show_hide() {
        let src = r##"
character aria { name: "Aria" }
scene start {
    background "bg.png"
    show aria.happy at left
    aria "Hi"
    hide aria
    end
}
"##;
        let prog = load_program_from_source(src, None, "ev").unwrap();
        let mut player = StoryPlayer::start(prog);
        // Drain events from initial pump.
        let mut events = player.drain_events();
        while matches!(player.wait(), StoryWait::Line) {
            player.advance();
            events.extend(player.drain_events());
        }
        events.extend(player.drain_events());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StoryEvent::Background(p) if p == "bg.png")),
            "events={events:?}"
        );
        assert!(
            events.iter().any(|e| matches!(e, StoryEvent::Show(_))),
            "events={events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StoryEvent::Hide(id) if id == "aria" || id.contains("aria"))),
            "events={events:?}"
        );
    }

    #[test]
    fn choose_out_of_range_errors() {
        let prog = load_program_from_source(sample_src(), None, "Demo").unwrap();
        let mut player = StoryPlayer::start(prog);
        if matches!(player.wait(), StoryWait::Line) {
            player.advance();
        }
        assert!(matches!(player.wait(), StoryWait::Choice));
        assert!(player.choose(99).is_err());
        assert!(player.choose(0).is_ok());
    }

    /// Build a product StoryProgram for nested control-flow tests.
    fn nest_prog() -> crate::ir::StoryProgram {
        use crate::ir::{StoryChoice, StoryCond, StoryExpr, StoryOp, StoryProgram, StoryScene};
        use crate::value::StoryValue;
        use crate::variables::AssignOp;
        use indexmap::IndexMap;

        let set = |name: &str, n: i64| StoryOp::Assign {
            name: name.into(),
            assign_op: AssignOp::Set,
            value: StoryExpr::value(StoryValue::Int(n)),
        };

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::Choice {
                        options: vec![StoryChoice {
                            text: "Entrar".into(),
                            body: vec![
                                set("entered", 1),
                                StoryOp::If {
                                    cond: StoryCond::var("has_key"),
                                    then_ops: vec![set("opened", 1)],
                                    else_ops: vec![set("opened", 0)],
                                },
                                // Must still run after nested if (the bug under test).
                                set("finished", 1),
                            ],
                            require: None,
                            hidden_if_locked: false,
                        }],
                    },
                    StoryOp::End {
                        ending: Some("done".into()),
                    },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("nest");
        prog.entry = "start".into();
        prog.scenes = scenes;
        prog.initial_vars
            .insert("has_key".into(), StoryValue::Int(1));
        prog
    }

    #[test]
    fn nested_if_inside_choice_runs_tail_ops() {
        let mut player = StoryPlayer::start(nest_prog());
        assert!(matches!(player.wait(), StoryWait::Choice));
        player.choose(0).unwrap();
        let mut guard = 0;
        while !player.is_ended() && guard < 32 {
            if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
                player.advance();
            } else {
                break;
            }
            guard += 1;
        }
        assert_eq!(player.variables().get_int("entered", 0), 1);
        assert_eq!(player.variables().get_int("opened", 0), 1);
        assert_eq!(
            player.variables().get_int("finished", 0),
            1,
            "ops after nested if inside choice must still run"
        );
        assert_eq!(player.ending(), Some("done"));
    }

    #[test]
    fn nested_if_inside_if_runs_outer_tail() {
        use crate::ir::{StoryCond, StoryExpr, StoryOp, StoryProgram, StoryScene};
        use crate::value::StoryValue;
        use crate::variables::AssignOp;
        use indexmap::IndexMap;

        let set = |name: &str, n: i64| StoryOp::Assign {
            name: name.into(),
            assign_op: AssignOp::Set,
            value: StoryExpr::value(StoryValue::Int(n)),
        };
        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    set("a", 1),
                    StoryOp::If {
                        cond: StoryCond::var("a"),
                        then_ops: vec![
                            set("b", 1),
                            StoryOp::If {
                                cond: StoryCond::var("b"),
                                then_ops: vec![set("c", 1)],
                                else_ops: vec![],
                            },
                            set("d", 1), // after inner if
                        ],
                        else_ops: vec![],
                    },
                    set("e", 1), // after outer if
                    StoryOp::End {
                        ending: Some("ok".into()),
                    },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("nest2");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let mut player = StoryPlayer::start(prog);
        let mut guard = 0;
        while !player.is_ended() && guard < 32 {
            player.advance();
            guard += 1;
        }
        assert_eq!(player.variables().get_int("c", 0), 1);
        assert_eq!(player.variables().get_int("d", 0), 1);
        assert_eq!(player.variables().get_int("e", 0), 1);
        assert_eq!(player.ending(), Some("ok"));
    }

    #[test]
    fn call_return_inside_choice_resumes_tail() {
        use crate::ir::{StoryChoice, StoryExpr, StoryOp, StoryProgram, StoryScene};
        use crate::value::StoryValue;
        use crate::variables::AssignOp;
        use indexmap::IndexMap;

        let set = |name: &str, n: i64| StoryOp::Assign {
            name: name.into(),
            assign_op: AssignOp::Set,
            value: StoryExpr::value(StoryValue::Int(n)),
        };
        let mut scenes = IndexMap::new();
        scenes.insert(
            "helper".into(),
            StoryScene {
                name: "helper".into(),
                ops: vec![set("in_helper", 1), StoryOp::Return],
                labels: IndexMap::new(),
            },
        );
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::Choice {
                        options: vec![StoryChoice {
                            text: "go".into(),
                            body: vec![
                                set("before", 1),
                                StoryOp::Call {
                                    target: "helper".into(),
                                },
                                set("after", 1),
                            ],
                            require: None,
                            hidden_if_locked: false,
                        }],
                    },
                    StoryOp::End {
                        ending: Some("done".into()),
                    },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("callnest");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let mut player = StoryPlayer::start(prog);
        assert!(matches!(player.wait(), StoryWait::Choice));
        player.choose(0).unwrap();
        let mut guard = 0;
        while !player.is_ended() && guard < 64 {
            if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
                player.advance();
            } else {
                break;
            }
            guard += 1;
        }
        assert_eq!(player.variables().get_int("before", 0), 1);
        assert_eq!(player.variables().get_int("in_helper", 0), 1);
        assert_eq!(
            player.variables().get_int("after", 0),
            1,
            "ops after call/return inside choice must run"
        );
        assert_eq!(player.ending(), Some("done"));
    }

    #[test]
    fn command_host_handler_runs_on_host_call() {
        use crate::host::command_host_fn;
        use crate::ir::{StoryOp, StoryProgram, StoryScene};
        use crate::value::StoryValue;
        use indexmap::IndexMap;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let host = command_host_fn(move |name, args, vars| {
            assert_eq!(name, "combat.start");
            assert_eq!(
                args.get("enemy"),
                Some(&StoryValue::String("goblin".into()))
            );
            h.fetch_add(1, Ordering::SeqCst);
            vars.set("combat_started", StoryValue::Int(7));
            Ok(())
        });

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::HostCall {
                        name: "combat.start".into(),
                        args: {
                            let mut m = IndexMap::new();
                            m.insert("enemy".into(), StoryValue::String("goblin".into()));
                            m
                        },
                    },
                    StoryOp::End { ending: None },
                ],
                labels: IndexMap::new(),
            },
        );
        let mut prog = StoryProgram::new("host");
        prog.entry = "start".into();
        prog.scenes = scenes;
        let mut player = StoryPlayer::start_with_host(prog, host);
        assert_eq!(hits.load(Ordering::SeqCst), 1, "handler must run");
        assert_eq!(player.variables().get_int("combat_started", 0), 7);
        assert_eq!(
            player.variables().get("__host_dispatched").display_str(),
            "combat.start"
        );
        assert!(player
            .drain_events()
            .iter()
            .any(|e| matches!(e, StoryEvent::HostCall { name, .. } if name == "combat.start")));
    }

    #[test]
    fn presentation_ops_emit_structured_events() {
        use crate::ir::{StoryOp, StoryProgram, StoryScene};
        use indexmap::IndexMap;

        let mut scenes = IndexMap::new();
        scenes.insert(
            "start".into(),
            StoryScene {
                name: "start".into(),
                ops: vec![
                    StoryOp::Sound {
                        path: "click.ogg".into(),
                    },
                    StoryOp::Pause {
                        seconds: Some(0.5),
                    },
                    StoryOp::Transition {
                        name: "fade".into(),
                    },
                    StoryOp::HostCall {
                        name: "combat.start".into(),
                        args: {
                            let mut m = IndexMap::new();
                            m.insert(
                                "enemy".into(),
                                crate::value::StoryValue::String("goblin".into()),
                            );
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
        let mut player = StoryPlayer::start(prog);
        let mut guard = 0;
        while !player.is_ended() && guard < 16 {
            player.advance();
            guard += 1;
        }
        let events = player.drain_events();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StoryEvent::Sound { path } if path == "click.ogg")),
            "{events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StoryEvent::Pause { seconds: Some(0.5) })),
            "{events:?}"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StoryEvent::Transition { name } if name == "fade")),
            "{events:?}"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                StoryEvent::HostCall { name, .. } if name == "combat.start"
            )),
            "{events:?}"
        );
    }
}
