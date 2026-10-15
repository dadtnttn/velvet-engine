//! Opt-in real-window integration test, always using an isolated save folder.
//! --smoke-unified <report.json>: VN gate → fade → map → movement → resize →
//! pause → slots → gallery → resume → VN/3D load → title → disk Continue.
//! No production save, settings or gallery is touched.
//! With VELVET_SMOKE_CYCLES=n it then goes from the visual novel to the 3D world and back n
//! times (loading the two slots it saved) and records the process's private memory each time:
//! the whole 3D world is built and dropped every round, so what it fails to free shows.
use super::*;
use serde_json::{json, Value};

pub(super) struct SmokeTest {
    report: PathBuf,
    stage: u8,
    started: Instant,
    stage_started: Instant,
    window_id: Option<WindowId>,
    expected: Option<exploration3d_state::State>,
    spawn: Option<exploration3d_state::State>,
    vn_node: String,
    events: Vec<Value>,
    /// VN ↔ 3D rounds still to go (VELVET_SMOKE_CYCLES), and how many were done.
    cycles: usize,
    cycle: usize,
    /// Seconds to stay in the world instead (VELVET_SMOKE_IDLE), the memory every ten.
    idle: u64,
    /// Seconds to stay in the world changing the native resolution and the graphics settings
    /// over and over (VELVET_SMOKE_GFX); it fails when no frame is shown for 3 s.
    gfx: u64,
    /// The frames shown when that last went up, and when.
    shown: (u64, Instant),
    step: u64,
}

/// The process's private memory in MB (what it has committed; 0 off Windows).
fn private_mb() -> u64 {
    #[cfg(windows)]
    // SAFETY: the counters are a plain struct the call fills in, sized by `cb`.
    unsafe {
        use windows_sys::Win32::System::ProcessStatus::{
            K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        if K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters as *mut PROCESS_MEMORY_COUNTERS_EX as *mut _,
            counters.cb,
        ) != 0
        {
            return (counters.PrivateUsage / (1024 * 1024)) as u64;
        }
    }
    0
}

impl SmokeTest {
    pub fn new(report: PathBuf) -> Result<Self> {
        let report = std::path::absolute(report)?;
        let root = report
            .parent()
            .ok_or_else(|| anyhow!("smoke report requires a directory"))?;
        std::fs::create_dir_all(root)?;
        std::env::set_var("DEMON_QUEEN_SAVES", root.join("test-saves"));
        std::env::set_var("DEMON_QUEEN_GALLERY", root.join("test-gallery.json"));
        std::env::set_var("DEMON_QUEEN_SETTINGS", root.join("test-settings.json"));
        let now = Instant::now();
        Ok(Self {
            report,
            stage: 0,
            started: now,
            stage_started: now,
            window_id: None,
            expected: None,
            spawn: None,
            vn_node: String::new(),
            events: Vec::new(),
            cycles: std::env::var("VELVET_SMOKE_CYCLES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            cycle: 0,
            idle: std::env::var("VELVET_SMOKE_IDLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            gfx: std::env::var("VELVET_SMOKE_GFX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            shown: (0, now),
            step: 0,
        })
    }
    fn next(&mut self) {
        self.stage += 1;
        self.stage_started = Instant::now();
    }
    fn record(&mut self, host: &WinitApp, label: &str) -> Result<()> {
        let window = host
            .app
            .window
            .as_ref()
            .ok_or_else(|| anyhow!("missing window"))?;
        if let Some(id) = self.window_id {
            anyhow::ensure!(id == window.id(), "native window changed");
        }
        self.window_id = Some(window.id());
        let size = window.inner_size();
        self.events.push(
            json!({"event": label, "elapsed_ms": self.started.elapsed().as_millis(),
            "pid": std::process::id(), "window_id": format!("{:?}", window.id()),
            "width": size.width, "height": size.height,
            "screen": format!("{:?}", host.app.screen), "world_ready": host.world_ready,
            "presented_frames": host.app.display.as_ref().map(|d| d.presented_frames),
            "private_mb": private_mb(),
            "prefetch_held": exploration3d::prefetch::held() }),
        );
        self.write("running", None)
    }
    fn write(&self, status: &str, error: Option<String>) -> Result<()> {
        let report = json!({"status": status, "stage": self.stage, "events": self.events,
            "expected_checkpoint": self.expected, "error": error });
        std::fs::write(&self.report, serde_json::to_vec_pretty(&report)?)?;
        Ok(())
    }
    pub fn fail(&self, error: &anyhow::Error) {
        let _ = self.write("failed", Some(format!("{error:#}")));
    }
    pub fn tick(&mut self, host: &mut WinitApp, event_loop: &ActiveEventLoop) -> Result<()> {
        anyhow::ensure!(
            self.started.elapsed()
                < Duration::from_secs(180 + 40 * self.cycles as u64 + self.idle + self.gfx),
            "unified smoke timed out at stage {}",
            self.stage
        );
        let elapsed = self.stage_started.elapsed();
        let Some(window) = host.app.window.as_ref().cloned() else {
            return Ok(());
        };
        match self.stage {
            0 if host
                .app
                .display
                .as_ref()
                .is_some_and(|d| d.presented_frames > 0) =>
            {
                anyhow::ensure!(
                    host.app.screen == Screen::Title && host.world.is_none(),
                    "game must start with VN"
                );
                self.record(host, "initial_vn")?;
                let _ = window.request_inner_size(PhysicalSize::new(1280, 800));
                self.next();
            }
            1 if elapsed > Duration::from_millis(600)
                && window.inner_size() == PhysicalSize::new(1280, 800) =>
            {
                let mut game = game::Game::load()?;
                game.new_game()?;
                host.app.enter_game(game)?;
                self.vn_node = host.app.view.as_ref().expect("VN").id.clone();
                host.app.reveal_complete = true;
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("quick-actions-vn.png"),
                    &host.app.canvas,
                    "dialogue-actions",
                )?;
                let save = quick_actions::rects(&host.app.styles)[2];
                host.app.click_at(save.0 + save.2 / 2, save.1 + save.3 / 2);
                anyhow::ensure!(
                    host.app.screen == Screen::GameMenu && !host.app.quit,
                    "footer Save left gameplay"
                );
                anyhow::ensure!(
                    host.app.pause.as_ref().unwrap().page == pause_menu::Page::Save,
                    "footer Save opened wrong page"
                );
                host.app.save_menu_slot(0);
                anyhow::ensure!(
                    host.app.game.as_ref().unwrap().save_slots()[0].is_some(),
                    "VN slot not written"
                );
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-vn-save.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                self.record(host, "vn_paused_and_saved")?;
                host.menu_back();
                anyhow::ensure!(host.app.screen == Screen::Play, "VN did not resume");
                let skip = quick_actions::rects(&host.app.styles)[1];
                host.app.click_at(skip.0 + skip.2 / 2, skip.1 + skip.3 / 2);
                anyhow::ensure!(host.app.skipping_dialogue, "footer Skip did not start");
                self.stage = 20;
                self.stage_started = Instant::now();
            }
            20 if !host.app.skipping_dialogue => {
                anyhow::ensure!(
                    host.app
                        .view
                        .as_ref()
                        .is_some_and(|view| view.choices.len() > 1),
                    "Skip failed to stop at a decision"
                );
                anyhow::ensure!(host.app.reveal_complete, "decision text hidden after Skip");
                host.app.mouse = (-1, -1);
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("quick-actions-decision.png"),
                    &host.app.canvas,
                    "dialogue-actions",
                )?;
                self.record(host, "footer_skip_stopped_at_decision")?;
                let mut game = host.app.game.take().unwrap();
                for step in 0..256 {
                    let view = game.view()?;
                    if view.world_entry.is_some() {
                        break;
                    }
                    let index = view
                        .choices
                        .iter()
                        .position(|choice| choice.enabled)
                        .ok_or_else(|| anyhow!("dead end {}", view.id))?;
                    game.decide(index)?;
                    anyhow::ensure!(step < 255, "prologue did not converge");
                }
                host.app.enter_game(game)?;
                host.app.reveal_complete = true;
                self.stage = 2;
                self.stage_started = Instant::now();
            }
            2 if elapsed > Duration::from_millis(800) => {
                anyhow::ensure!(
                    host.app.screen == Screen::Play && host.app.pending_3d.is_none(),
                    "last VN dialogue skipped"
                );
                self.record(host, "last_vn_dialogue")?;
                save_canvas(
                    &self.report.with_file_name("vn-end.png"),
                    &host.app.canvas,
                    "unified-smoke",
                )?;
                host.app.advance_play();
                anyhow::ensure!(
                    host.app.pending_3d.is_some() && !host.app.quit,
                    "VN did not request embedded 3D"
                );
                self.next();
            }
            3 if host.world_ready && host.transition.is_none() => {
                if self.gfx > 0 {
                    self.record(host, "gfx_0")?;
                    self.stage = 41;
                    self.stage_started = Instant::now();
                    self.shown = (0, Instant::now());
                    return Ok(());
                }
                // Only staying in the world: walking about, the memory every ten seconds.
                if self.idle > 0 {
                    self.record(host, "idle_0")?;
                    self.stage = 40;
                    self.stage_started = Instant::now();
                    return Ok(());
                }
                // Only the rounds: save the world in slot 1 (the VN is in slot 0) and go.
                if self.cycles > 0 {
                    self.record(host, "world_for_rounds")?;
                    host.menu_back();
                    host.app.set_menu_page(pause_menu::Page::Save);
                    host.app.save_menu_slot(1);
                    host.menu_back();
                    anyhow::ensure!(
                        host.app.screen == Screen::Explore && host.world.is_some(),
                        "resume after the save lost the world"
                    );
                    self.stage = 30;
                    self.stage_started = Instant::now();
                    return Ok(());
                }
                anyhow::ensure!(
                    window.inner_size() == PhysicalSize::new(1280, 800),
                    "transition resized the window"
                );
                self.record(host, "map_same_window_same_size")?;
                let world = host.world.as_mut().expect("world ready");
                world.capture(&self.report.with_file_name("world.png"))?;
                self.spawn = Some(world.snapshot());
                world.set_key(KeyCode::KeyW, true);
                self.next();
            }
            4 if elapsed > Duration::from_millis(600) => {
                let world = host.world.as_mut().expect("world ready");
                world.clear_input();
                let saved = world.snapshot();
                let spawn = self.spawn.as_ref().expect("initial position");
                anyhow::ensure!(
                    (saved.x - spawn.x).abs() + (saved.z - spawn.z).abs() > 0.05,
                    "3D movement failed"
                );
                self.expected = Some(saved);
                host.checkpoint_world(true);
                self.record(host, "movement_autosaved")?;
                let _ = window.request_inner_size(PhysicalSize::new(1000, 1000));
                self.next();
            }
            5 if elapsed > Duration::from_millis(800)
                && window.inner_size() == PhysicalSize::new(1000, 1000) =>
            {
                self.record(host, "resized_letterbox")?;
                host.menu_back();
                anyhow::ensure!(
                    host.app.screen == Screen::GameMenu && host.world_ready,
                    "Escape unloaded world"
                );
                self.next();
            }
            6 if elapsed > Duration::from_millis(800) => {
                let saved = host.world.as_ref().expect("world retained").snapshot();
                let expected = self.expected.as_ref().unwrap();
                anyhow::ensure!(
                    saved.x == expected.x && saved.z == expected.z,
                    "world moved while paused"
                );
                host.app.set_menu_page(pause_menu::Page::Status);
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-3d-status.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                host.app.set_menu_page(pause_menu::Page::Save);
                // Her own progress goes in with the save: gold, an item, a story mark.
                host.world.as_mut().unwrap().smoke_mark_progress();
                host.checkpoint_world(true);
                host.app.save_menu_slot(1);
                let slots = host.app.game.as_ref().unwrap().save_slots();
                anyhow::ensure!(
                    slots[1]
                        .as_ref()
                        .is_some_and(|s| s.is_world && s.error.is_none()),
                    "3D slot missing"
                );
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-3d-save.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                host.app.open_gallery(Screen::GameMenu);
                host.menu_back();
                anyhow::ensure!(
                    host.app.screen == Screen::GameMenu && host.world.is_some(),
                    "gallery lost pause session"
                );
                host.app.set_menu_page(pause_menu::Page::History);
                for _ in 0..4 {
                    host.app.menu_scroll(true);
                }
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-history.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                host.app.set_menu_page(pause_menu::Page::Settings);
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-settings.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                host.app.ask_menu_confirmation(pause_menu::Confirm::Title);
                host.app.paint();
                save_canvas(
                    &self.report.with_file_name("menu-confirm.png"),
                    &host.app.canvas,
                    "menu-smoke",
                )?;
                host.menu_back();
                anyhow::ensure!(host.app.screen == Screen::GameMenu, "cancel closed menu");
                self.record(host, "3d_paused_frozen_saved_gallery_back")?;
                host.menu_back();
                anyhow::ensure!(
                    host.app.screen == Screen::Explore && host.world.is_some(),
                    "resume rebuilt world"
                );
                self.next();
            }
            7 if elapsed > Duration::from_millis(450) => {
                host.open_game_menu(pause_menu::Page::Load);
                host.app.ask_menu_confirmation(pause_menu::Confirm::Load(0));
                host.app.confirm_menu_action();
                host.process_menu_action();
                anyhow::ensure!(
                    host.app.screen == Screen::Play && host.world.is_none(),
                    "VN load retained old map"
                );
                anyhow::ensure!(
                    host.app.view.as_ref().unwrap().id == self.vn_node,
                    "VN slot lost node"
                );
                anyhow::ensure!(
                    !exploration3d_state::path().exists(),
                    "VN load kept stale world sidecar"
                );
                self.record(host, "manual_vn_slot_restored_after_3d")?;
                self.next();
            }
            8 if elapsed > Duration::from_millis(500) => {
                host.open_game_menu(pause_menu::Page::Load);
                host.app.ask_menu_confirmation(pause_menu::Confirm::Load(1));
                host.app.confirm_menu_action();
                host.process_menu_action();
                anyhow::ensure!(
                    host.app.pending_3d.is_some(),
                    "3D slot did not restore phase"
                );
                self.next();
            }
            9 if host.world_ready && host.transition.is_none() => {
                let saved = host.world.as_ref().unwrap().snapshot();
                let expected = self.expected.as_ref().unwrap();
                anyhow::ensure!(
                    (saved.x - expected.x).abs() < 0.003
                        && (saved.z - expected.z).abs() < 0.003
                        && saved.nature_mode == expected.nature_mode
                        && saved.village_entered == expected.village_entered,
                    "manual 3D slot lost transform or flags"
                );
                anyhow::ensure!(
                    saved.gold == 1234
                        && saved.inventory.get("pocion_reforzada") == Some(&2)
                        && saved.story.get("smoke_guardado") == 1,
                    "manual 3D slot lost her gold, bag or story (gold {})",
                    saved.gold
                );
                self.record(host, "manual_3d_slot_restored_after_vn")?;
                if self.cycles > 0 {
                    host.menu_back();
                    self.stage = 30;
                    self.stage_started = Instant::now();
                    return Ok(());
                }
                host.menu_back();
                host.app.ask_menu_confirmation(pause_menu::Confirm::Title);
                host.app.confirm_menu_action();
                host.process_menu_action();
                self.next();
            }
            // VN ↔ 3D rounds: to the visual novel (the world dropped), back to the world (built
            // anew), a walk in it; the memory after each.
            30 if elapsed > Duration::from_millis(1500) => {
                if let Some(world) = host.world.as_mut() {
                    world.clear_input();
                }
                self.record(host, &format!("round_{}_world", self.cycle))?;
                host.open_game_menu(pause_menu::Page::Load);
                host.app.ask_menu_confirmation(pause_menu::Confirm::Load(0));
                host.app.confirm_menu_action();
                host.process_menu_action();
                anyhow::ensure!(host.world.is_none(), "VN load kept the world");
                self.next();
            }
            31 if elapsed > Duration::from_millis(900) => {
                self.record(host, &format!("round_{}_vn", self.cycle))?;
                host.open_game_menu(pause_menu::Page::Load);
                host.app.ask_menu_confirmation(pause_menu::Confirm::Load(1));
                host.app.confirm_menu_action();
                host.process_menu_action();
                anyhow::ensure!(host.app.pending_3d.is_some(), "3D slot did not restore");
                self.next();
            }
            40 => {
                let seconds = elapsed.as_secs();
                // Minimised from 20 s to 60 s (what a player does with another window).
                window.set_minimized((20..60).contains(&seconds));
                if let Some(world) = host.world.as_mut() {
                    // Turn about now and then, walking on.
                    world.set_key(KeyCode::KeyW, true);
                    world.set_key(KeyCode::KeyA, (elapsed.as_millis() / 1500) % 3 == 0);
                }
                if seconds >= (self.cycle as u64 + 1) * 10 {
                    self.cycle += 1;
                    self.record(host, &format!("idle_{}", self.cycle * 10))?;
                }
                if seconds >= self.idle {
                    self.record(host, "idle_done")?;
                    self.write("passed", None)?;
                    event_loop.exit();
                }
            }
            41 => {
                let shown = host.app.display.as_ref().map_or(0, |d| d.presented_frames);
                if shown != self.shown.0 {
                    self.shown = (shown, Instant::now());
                }
                if window.is_minimized() == Some(true) {
                    self.shown.1 = Instant::now();
                }
                if self.shown.1.elapsed() > Duration::from_secs(3) {
                    self.record(host, "gfx_stall")?;
                    anyhow::bail!("no frame shown for 3 s at step {}", self.step);
                }
                // VELVET_SMOKE_SHOTS=1 (with the zones): standing at each place's arrival, a
                // picture of it (zone-<step>.png beside the report), to compare two builds.
                let shots = std::env::var("VELVET_SMOKE_SHOTS").is_ok_and(|v| v == "1");
                if let Some(world) = host.world.as_mut().filter(|_| !shots) {
                    world.set_key(KeyCode::KeyW, true);
                    world.set_key(KeyCode::KeyA, (elapsed.as_millis() / 1500) % 3 == 0);
                }
                // Every 0.7 s another change, as a player going through the settings would;
                // with VELVET_SMOKE_ZONES=1 the place changes instead every 2.1 s (native on).
                let step = elapsed.as_millis() as u64 / 700;
                let zones = std::env::var("VELVET_SMOKE_ZONES").is_ok_and(|v| v == "1");
                if zones && step != self.step {
                    self.step = step;
                    if step % 3 == 0 {
                        if let Some(world) = host.world.as_mut() {
                            let place = world.qa_next_zone();
                            eprintln!("[smoke-gfx] step {step} at {} ms: shown {shown}, to {place}", elapsed.as_millis());
                        }
                    }
                    if shots && step % 3 == 2 {
                        if let Some(world) = host.world.as_mut() {
                            world.capture(&self.report.with_file_name(format!("zone-{step:03}.png")))?;
                        }
                    }
                }
                if !zones && step != self.step {
                    self.step = step;
                    if step % 5 == 0 {
                        exploration3d::prefs::set_native_resolution(!exploration3d::prefs::native_resolution());
                    }
                    let mut g = exploration3d::prefs::graphics();
                    match step % 7 {
                        0 => g.shadows = (g.shadows + 1) % 4,
                        1 => g.bloom = (g.bloom + 1) % 3,
                        2 => g.render_scale = [50, 67, 85, 100][(step / 7 % 4) as usize],
                        3 => g.upscaler = (g.upscaler + 1) % 2,
                        4 => g.reflections = !g.reflections,
                        5 => g.lod = (g.lod + 1) % 3,
                        _ => g.prepass = !g.prepass,
                    }
                    g.preset = 4;
                    exploration3d::prefs::set_graphics(g);
                    eprintln!(
                        "[smoke-gfx] step {step} at {} ms: shown {shown}, native {}, {g:?}",
                        elapsed.as_millis(),
                        exploration3d::prefs::native_resolution()
                    );
                    if step % 10 == 0 {
                        self.record(host, &format!("gfx_{step}"))?;
                    }
                }
                if elapsed.as_secs() >= self.gfx {
                    self.record(host, "gfx_done")?;
                    self.write("passed", None)?;
                    event_loop.exit();
                }
            }
            32 if host.world_ready && host.transition.is_none() => {
                if let Some(world) = host.world.as_mut() {
                    world.set_key(KeyCode::KeyW, true);
                }
                self.cycle += 1;
                if self.cycle < self.cycles {
                    self.stage = 30;
                    self.stage_started = Instant::now();
                } else {
                    self.record(host, "rounds_done")?;
                    self.write("passed", None)?;
                    event_loop.exit();
                }
            }
            10 if elapsed > Duration::from_millis(800) => {
                anyhow::ensure!(
                    host.app.screen == Screen::Title && host.world.is_none(),
                    "confirmed title action did not return to title"
                );
                self.record(host, "title_same_window")?;
                host.app.continue_game()?;
                anyhow::ensure!(
                    host.app.pending_3d.is_some(),
                    "Continue did not restore map phase"
                );
                self.next();
            }
            11 if host.world_ready && host.transition.is_none() => {
                let saved = host.world.as_ref().expect("world ready").snapshot();
                let expected = self.expected.as_ref().expect("saved checkpoint");
                anyhow::ensure!(
                    (saved.x - expected.x).abs() < 0.003
                        && (saved.z - expected.z).abs() < 0.003
                        && saved.nature_mode == expected.nature_mode
                        && saved.village_entered == expected.village_entered,
                    "Continue lost 3D checkpoint: {saved:?} vs {expected:?}"
                );
                anyhow::ensure!(
                    window.inner_size() == PhysicalSize::new(1000, 1000),
                    "Continue changed window size"
                );
                host.world
                    .as_ref()
                    .expect("world ready")
                    .capture(&self.report.with_file_name("resume.png"))?;
                self.record(host, "disk_continue_restored_same_window")?;
                self.write("passed", None)?;
                self.next();
                event_loop.exit();
            }
            _ => {}
        }
        // The test drives title screens too, which otherwise sleep indefinitely.
        if host.world.is_none() || host.app.screen != Screen::Explore {
            host.app.needs_paint = true;
            window.request_redraw();
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ));
        }
        Ok(())
    }
}
