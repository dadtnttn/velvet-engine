//! Story host: `style.*` (`.vcss`) + `stakes.*` game commands.
//!
//! The author language (`.vstory`) drives flow; this host mutates shared run state
//! and keeps the active stylesheet in sync for paint + deal motion.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};
use velvet_style::{
    call_style_fn, emit_style_event, parse_stylesheet, JsValue, StyleStoryHost, Stylesheet,
};
use velvet_story_lang::commands::{CommandParam, CommandRegistry, CommandSpec, ParamTy};

use crate::catalog::CardStats;
use crate::game::{Outcome, Run, Screen, BLINDS};

/// Shared game world visible to the window loop and story host.
pub struct StakesWorld {
    pub screen: Screen,
    pub menu_sel: usize,
    pub pause_sel: usize,
    pub result_sel: usize,
    pub result: Option<Outcome>,
    pub run: Option<Run>,
    pub stats: HashMap<String, CardStats>,
    pub deck_ids: Vec<String>,
    pub stylesheet: Stylesheet,
    pub money: i64,
    pub seed: u64,
    pub status: String,
    pub quit: bool,
    /// Story wait phase mirrored for input routing (`title`, `blind`, `play`, …).
    pub wait_phase: Option<String>,
    pub data_root: PathBuf,
    pub meta_chips: i64,
    pub meta_crystals: i64,
    pub meta_mult: f32,
}

impl StakesWorld {
    pub fn new(
        stats: HashMap<String, CardStats>,
        deck_ids: Vec<String>,
        data_root: PathBuf,
    ) -> Self {
        Self {
            screen: Screen::Title,
            menu_sel: 0,
            pause_sel: 0,
            result_sel: 0,
            result: None,
            run: None,
            stats,
            deck_ids,
            stylesheet: Stylesheet::default(),
            money: 0,
            seed: 0xBA_1A_70_01,
            status: "Velvet Arcana".into(),
            quit: false,
            wait_phase: None,
            data_root,
            meta_chips: 12_450,
            meta_crystals: 870,
            meta_mult: 3.2,
        }
    }

    pub fn begin_run(&mut self) {
        self.money = 4;
        self.seed = self.seed.wrapping_add(13);
        self.start_blind(0);
    }

    pub fn start_blind(&mut self, ante: usize) {
        self.seed = self.seed.wrapping_add(1);
        let mut run = Run::start(ante, &self.deck_ids, self.seed, self.money);
        let sheet = self.stylesheet.clone();
        run.rebuild_visuals(true, Some(&sheet));
        self.status = format!("{} / {}", run.blind_name, run.target);
        self.run = Some(run);
        self.screen = Screen::BlindInfo;
        self.result = None;
    }

    pub fn enter_play(&mut self) {
        self.screen = Screen::Play;
        self.status = "1-8 select · P play · D discard".into();
        let sheet = self.stylesheet.clone();
        if let Some(r) = self.run.as_mut() {
            // re-deal motion when entering play from blind info
            r.rebuild_visuals(true, Some(&sheet));
        }
    }

    pub fn apply_outcome(&mut self, o: Outcome) {
        if let Some(r) = &self.run {
            self.money = r.money;
        }
        self.result = Some(o);
        self.screen = Screen::Result;
        self.result_sel = 0;
        self.status = match o {
            Outcome::WinBlind => "¡Ciega superada!".into(),
            Outcome::LoseBlind => "Ciega fallida".into(),
            Outcome::RunClear => "¡Run completa!".into(),
        };
    }

    pub fn to_title(&mut self) {
        self.run = None;
        self.result = None;
        self.screen = Screen::Title;
        self.menu_sel = 0;
    }

    pub fn sync_vars(&self, vars: &mut StoryVariables) {
        vars.set(
            "ui.screen",
            StoryValue::String(self.screen.as_id().into()),
        );
        vars.set("stakes.money", StoryValue::Int(self.money));
        vars.set(
            "stakes.outcome",
            StoryValue::String(
                self.result
                    .map(|o| o.as_str().into())
                    .unwrap_or_else(|| "none".into()),
            ),
        );
        if let Some(r) = &self.run {
            vars.set("stakes.score", StoryValue::Int(r.score));
            vars.set("stakes.target", StoryValue::Int(r.target));
            vars.set("stakes.ante", StoryValue::Int(r.ante as i64));
            vars.set(
                "stakes.blind",
                StoryValue::String(r.blind_name.clone()),
            );
            vars.set("stakes.hands", StoryValue::Int(r.hands_left as i64));
            vars.set(
                "stakes.discards",
                StoryValue::Int(r.discards_left as i64),
            );
        }
        vars.set(
            "style.rules",
            StoryValue::Int(self.stylesheet.rules.len() as i64),
        );
        vars.set(
            "style.keyframes",
            StoryValue::Int(self.stylesheet.keyframes.len() as i64),
        );
        vars.set(
            "style.fns",
            StoryValue::Int(self.stylesheet.script.functions.len() as i64),
        );
    }
}

/// Composite host: style.* + stakes.*.
pub struct StakesHost {
    pub world: Mutex<StakesWorld>,
    pub style: StyleStoryHost,
}

impl StakesHost {
    pub fn new(world: StakesWorld) -> Self {
        Self {
            world: Mutex::new(world),
            style: StyleStoryHost::new(),
        }
    }

    /// Apply a reparsed stylesheet from live-dev (or tools) without restarting.
    pub fn apply_stylesheet(&self, name: &str, sheet: Stylesheet) -> Result<(), String> {
        let mut w = self.world.lock().map_err(|e| e.to_string())?;
        w.stylesheet = sheet.clone();
        let mut reg = self
            .style
            .registry
            .lock()
            .map_err(|e| e.to_string())?;
        reg.insert(name, sheet);
        Ok(())
    }

    fn load_vcss_into(&self, w: &mut StakesWorld, name: &str, src: &str) -> Result<(), String> {
        let sheet = parse_stylesheet(src).map_err(|e| e.to_string())?;
        w.stylesheet = sheet.clone();
        let mut reg = self
            .style
            .registry
            .lock()
            .map_err(|e| e.to_string())?;
        reg.insert(name, sheet);
        Ok(())
    }

    fn resolve_style_path(&self, w: &StakesWorld, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        let candidates = [
            w.data_root.join(path),
            w.data_root.join("styles").join(path),
            PathBuf::from(path),
        ];
        candidates
            .into_iter()
            .find(|c| c.exists())
            .unwrap_or_else(|| w.data_root.join(path))
    }

    fn sync_sheet_from_style_reg(&self, w: &mut StakesWorld) {
        if let Ok(reg) = self.style.registry.lock() {
            if let Some(active) = &reg.active {
                if let Some(s) = reg.sheets.get(active) {
                    w.stylesheet = s.clone();
                }
            } else if let Some((_, s)) = reg.sheets.last() {
                w.stylesheet = s.clone();
            }
        }
    }
}

impl StoryCommandHost for StakesHost {
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError> {
        // ── style.* → StyleStoryHost, then mirror sheet into world ──
        if name.starts_with("style.") {
            // Resolve relative paths against data_root
            let mut args = args.clone();
            if let Some(path) = args.get("path").map(|v| v.display_str()) {
                if let Ok(w) = self.world.lock() {
                    let resolved = self.resolve_style_path(&w, &path);
                    args.insert(
                        "path".into(),
                        StoryValue::String(resolved.to_string_lossy().into()),
                    );
                }
            }
            let out = self.style.call(name, &args, vars)?;
            if let Ok(mut w) = self.world.lock() {
                self.sync_sheet_from_style_reg(&mut w);
                w.sync_vars(vars);
            }
            return Ok(out);
        }

        let mut w = self
            .world
            .lock()
            .map_err(|e| StoryCommandError::new(e.to_string()))?;

        match name {
            "stakes.boot" => {
                // Prefer on-disk casino.vcss; fallback embedded.
                const EMBEDDED: &str =
                    include_str!("../data/styles/casino.vcss");
                let path = w.data_root.join("styles/casino.vcss");
                let src = std::fs::read_to_string(&path).unwrap_or_else(|_| EMBEDDED.into());
                self.load_vcss_into(&mut w, "casino", &src)
                    .map_err(StoryCommandError::new)?;
                // Also notify style registry active
                vars.set("style.active", StoryValue::String("casino".into()));
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.set_screen" => {
                let id = arg_str(args, "id").unwrap_or_else(|| "title".into());
                if let Some(s) = Screen::from_id(&id) {
                    w.screen = s;
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.wait" => {
                let phase = arg_str(args, "phase").unwrap_or_else(|| "input".into());
                w.wait_phase = Some(phase.clone());
                w.sync_vars(vars);
                vars.set("stakes.phase", StoryValue::String(phase.clone()));
                Ok(CommandOutcome::Wait { token: phase })
            }
            "stakes.begin_run" => {
                w.begin_run();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.start_blind" => {
                let ante = arg_i64(args, "ante").unwrap_or(0) as usize;
                w.start_blind(ante);
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.enter_play" => {
                w.enter_play();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.deal" => {
                // Explicit deal from .vcss @script
                let sheet = w.stylesheet.clone();
                let n = w
                    .run
                    .as_ref()
                    .map(|r| r.zones.hand.len())
                    .unwrap_or(0) as f32;
                if let Some(r) = w.run.as_mut() {
                    r.rebuild_visuals(true, Some(&sheet));
                }
                if let Ok(run) = call_style_fn(&sheet, "dealHand", &[JsValue::num(n)]) {
                    vars.set(
                        "style.actions",
                        StoryValue::Int(run.actions.len() as i64),
                    );
                    vars.set(
                        "style.timelines",
                        StoryValue::Int(run.timelines.len() as i64),
                    );
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.emit_style" => {
                let event = arg_str(args, "event").unwrap_or_else(|| "menu.open".into());
                if let Ok(run) = emit_style_event(&w.stylesheet, &event, &[]) {
                    vars.set(
                        "style.actions",
                        StoryValue::Int(run.actions.len() as i64),
                    );
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.to_title" => {
                w.to_title();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.quit" => {
                w.quit = true;
                w.sync_vars(vars);
                Ok(CommandOutcome::End {
                    ending: Some("quit".into()),
                })
            }
            "stakes.sync" => {
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            _ => {
                // unknown: ignore so story can keep flowing in demos
                Ok(CommandOutcome::Continue)
            }
        }
    }
}

fn arg_str(args: &IndexMap<String, StoryValue>, key: &str) -> Option<String> {
    args.get(key).map(|v| v.display_str())
}

fn arg_i64(args: &IndexMap<String, StoryValue>, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

/// Register `stakes.*` (and ensure style.*) for the story-lang checker / lowerer.
pub fn register_stakes_commands(reg: &mut CommandRegistry) {
    let stakes = [
        (
            "stakes.boot",
            "Carga .vcss casino y prepara el mundo del demo.",
            vec![],
        ),
        (
            "stakes.set_screen",
            "Cambia pantalla UI (title, play, …).",
            vec![("id", ParamTy::Ident, true)],
        ),
        (
            "stakes.wait",
            "Pausa el .vstory hasta input del jugador (token=phase).",
            vec![("phase", ParamTy::Ident, true)],
        ),
        (
            "stakes.begin_run",
            "Empieza una run (ante 0).",
            vec![],
        ),
        (
            "stakes.start_blind",
            "Prepara ciega por ante.",
            vec![("ante", ParamTy::Int, false)],
        ),
        (
            "stakes.enter_play",
            "Entra a la mesa de juego y aplica deal .vcss.",
            vec![],
        ),
        (
            "stakes.deal",
            "Ejecuta dealHand del @script .vcss sobre la mano.",
            vec![],
        ),
        (
            "stakes.emit_style",
            "Dispara on(event) del .vcss.",
            vec![("event", ParamTy::Text, true)],
        ),
        (
            "stakes.to_title",
            "Vuelve al menú título.",
            vec![],
        ),
        (
            "stakes.quit",
            "Cierra el juego.",
            vec![],
        ),
        (
            "stakes.sync",
            "Sincroniza variables de historia con el mundo.",
            vec![],
        ),
    ];
    for (name, desc, params) in stakes {
        let params: Vec<CommandParam> = params
            .into_iter()
            .map(|(n, ty, req)| CommandParam {
                name: n.into(),
                ty,
                required: req,
                default: None,
                doc: String::new(),
            })
            .collect();
        let required: Vec<String> = params
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect();
        reg.register(CommandSpec {
            name: name.into(),
            category: "stakes".into(),
            description: desc.into(),
            params,
            required,
            snippet: format!("call {name}:\n"),
            error_help: format!("call {name}:"),
        });
    }
    // Note: style.* already in builtin()
    let _ = BLINDS.len(); // keep import used if needed later
}
