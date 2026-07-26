use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use velvet_script_vs3::{
    compile_bundle, float_val, int, map_val, string_val, Vs3Module, Vs3Session,
};

use crate::apps::case_files::CaseFilesApp;
use crate::apps::classifier::{AnomalyCategory, ClassifierApp};
use crate::apps::inbox::InboxApp;
use crate::apps::photo_viewer::PhotoViewerApp;
use crate::apps::recycle_bin::RecycleBinApp;
use crate::apps::system_dialog::SystemDialogApp;
use crate::apps::tape_player::TapePlayerApp;
use crate::assets::Assets;
use crate::audio::Audio;
use crate::desktop::DesktopShell;
use crate::input::{KeyboardState, MouseState, Position};
use crate::model::{AppKind, FrameView, Rect};
use crate::render::{Renderer, HEIGHT, WIDTH};
use crate::save::{SaveData, SaveStore};
use crate::windows::{DesktopWindow, WindowManager};

const GAME_SOURCES: &[(&str, &str)] = &[
    ("game.vel", include_str!("../data/game.vel")),
    ("module.vel", include_str!("../data/module.vel")),
    ("state.vel", include_str!("../data/state.vel")),
    ("flow.vel", include_str!("../data/flow.vel")),
    ("case001.vel", include_str!("../data/case001.vel")),
    ("anomalies.vel", include_str!("../data/anomalies.vel")),
    ("persistence.vel", include_str!("../data/persistence.vel")),
    ("acceptance.vel", include_str!("../data/acceptance.vel")),
];

pub struct Game {
    pub module: Vs3Module,
    pub session: Vs3Session,
    pub frame: Option<FrameView>,
}

impl Game {
    pub fn compile() -> Result<Self> {
        let module = compile_bundle("game.vel", GAME_SOURCES.iter().copied())?;
        let session = module.session()?;
        Ok(Self {
            module,
            session,
            frame: None,
        })
    }

    pub fn new_game(&mut self, seed: i64) -> Result<()> {
        self.session = self.module.session()?;
        let value = self.session.call("game.new_game", &[int(seed)])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    pub fn load_game(&mut self, save: &SaveData) -> Result<()> {
        self.session = self.module.session()?;
        let value = self.session.call("game.load_game", &[save.to_vs3()])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    pub fn tick(&mut self, dt: f32) -> Result<()> {
        let input = map_val([]);
        let value = self
            .session
            .call("game.tick", &[float_val(dt as f64), input])?;
        self.frame = Some(FrameView::parse(&value)?);
        Ok(())
    }

    pub fn call(&mut self, func: &str, args: &[velvet_script_vs3::Value]) -> Result<FrameView> {
        let qualified = format!("game.{func}");
        let value = self.session.call(&qualified, args)?;
        let frame = FrameView::parse(&value)?;
        self.frame = Some(frame.clone());
        Ok(frame)
    }

    pub fn export_save(&mut self) -> Result<SaveData> {
        let value = self.session.call("game.export_save", &[])?;
        SaveData::from_vs3(&value)
    }
}

pub struct App {
    pub game: Game,
    pub assets: Assets,
    pub renderer: Renderer,
    pub wm: WindowManager,
    pub shell: DesktopShell,
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub audio: Audio,
    pub saves: SaveStore,
    pub inbox_app: InboxApp,
    pub case_app: CaseFilesApp,
    pub photo_app: PhotoViewerApp,
    pub tape_app: TapePlayerApp,
    pub classifier_app: ClassifierApp,
    pub recycle_app: RecycleBinApp,
    pub active_dialog: Option<SystemDialogApp>,
    pub boot_progress: f32,
    pub clippy_anim_time: f32,
}

impl App {
    pub fn new() -> Result<Self> {
        let mut game = Game::compile()?;
        game.new_game(1407)?;
        let assets = Assets::load()?;
        let renderer = Renderer::new();
        let wm = WindowManager::new();
        let shell = DesktopShell::new();
        let mouse = MouseState::default();
        let keyboard = KeyboardState::default();
        let saves = SaveStore::load();
        let audio = Audio::new(&saves.data().settings);

        Ok(Self {
            game,
            assets,
            renderer,
            wm,
            shell,
            mouse,
            keyboard,
            audio,
            saves,
            inbox_app: InboxApp::new(),
            case_app: CaseFilesApp::new(),
            photo_app: PhotoViewerApp::new(),
            tape_app: TapePlayerApp::new(),
            classifier_app: ClassifierApp::new(),
            recycle_app: RecycleBinApp::new(),
            active_dialog: None,
            boot_progress: 0.0,
            clippy_anim_time: 0.0,
        })
    }

    pub fn update(&mut self, dt: f32) -> Result<()> {
        let phase = self
            .game
            .frame
            .as_ref()
            .map(|f| f.phase.clone())
            .unwrap_or_else(|| "boot".to_string());

        if phase == "boot" {
            self.boot_progress += dt * 0.4;
            if self.boot_progress >= 1.0 || self.keyboard.enter {
                self.game.call("complete_boot", &[])?;
                self.audio
                    .play_sound("startup", &self.saves.data().settings);
            }
        }

        if let Some(frame) = &self.game.frame {
            self.audio
                .play_events(&frame.events, &self.saves.data().settings);
        }

        // Tape Player tick update
        self.tape_app.tick(dt);
        if self.tape_app.playing {
            let progress = self.tape_app.position / self.tape_app.duration;
            self.game.call(
                "listen_tape",
                &[string_val("tape_c17"), float_val(progress as f64)],
            )?;
        }

        // Handle Mouse Clicks & Dragging
        if self.mouse.left_pressed {
            self.handle_click();
        }

        if self.mouse.double_clicked {
            self.handle_double_click();
        }

        self.clippy_anim_time += dt;
        self.game.tick(dt)?;
        self.mouse.end_frame();
        self.keyboard.end_frame();
        Ok(())
    }

    fn handle_click(&mut self) {
        let m_pos = self.mouse.pos;

        // Check modal dialog button click
        if self.active_dialog.is_some() {
            let ok_btn = Rect::new(350, 320, 100, 30);
            if ok_btn.contains(m_pos.x, m_pos.y) {
                self.active_dialog = None;
                return;
            }
        }

        // Taskbar Start Orb click
        let start_rect = Rect::new(4, 552, 48, 48);
        if start_rect.contains(m_pos.x, m_pos.y) {
            self.shell.start_menu_open = !self.shell.start_menu_open;
            return;
        }

        // Start Menu item click
        if self.shell.start_menu_open {
            let menu_rect = Rect::new(0, 180, 280, 380);
            if menu_rect.contains(m_pos.x, m_pos.y) {
                let items = [
                    (AppKind::Inbox, "Inbox", 230),
                    (AppKind::CaseFiles, "Case Files", 272),
                    (AppKind::PhotoViewer, "Evidence Photo", 314),
                    (AppKind::TapePlayer, "Tape Player", 356),
                    (AppKind::Classifier, "Classifier", 398),
                    (AppKind::RecycleBin, "Recycle Bin", 440),
                ];
                for (app, title, y) in items {
                    let item_rect = Rect::new(6, y, 158, 36);
                    if item_rect.contains(m_pos.x, m_pos.y) {
                        self.open_app(app, title);
                        self.shell.start_menu_open = false;
                        return;
                    }
                }
            } else {
                self.shell.start_menu_open = false;
            }
        }

        // Check open window clicks (top z-order first)
        let mut sorted_wins = self.wm.windows.clone();
        sorted_wins.sort_by_key(|w| w.z_index);
        sorted_wins.reverse();

        for win in sorted_wins {
            if win.is_minimized {
                continue;
            }
            if win.rect.contains(m_pos.x, m_pos.y) {
                self.wm.focus_window(win.id);

                // Close button click
                let close_btn =
                    Rect::new(win.rect.x + win.rect.w as i32 - 22, win.rect.y + 4, 16, 16);
                if close_btn.contains(m_pos.x, m_pos.y) {
                    self.wm.close_window(win.id);
                    return;
                }

                // Minimize button click
                let min_btn =
                    Rect::new(win.rect.x + win.rect.w as i32 - 42, win.rect.y + 4, 16, 16);
                if min_btn.contains(m_pos.x, m_pos.y) {
                    self.wm.toggle_minimize(win.id);
                    return;
                }

                // App-specific internal clicks
                self.handle_app_click(win.app, win.rect, m_pos);
                return;
            }
        }

        // Desktop icon selection click
        let mut hit_icon = false;
        for icon in &self.shell.icons {
            if icon.rect.contains(m_pos.x, m_pos.y) {
                self.shell.select_icon(Some(icon.id));
                hit_icon = true;
                break;
            }
        }
        if !hit_icon {
            self.shell.select_icon(None);
        }
    }

    fn handle_double_click(&mut self) {
        let m_pos = self.mouse.pos;
        for icon in &self.shell.icons {
            if icon.rect.contains(m_pos.x, m_pos.y) {
                self.open_app(icon.app, icon.label);
                return;
            }
        }
    }

    pub fn open_app(&mut self, app: AppKind, title: &str) {
        let rect = match app {
            AppKind::Inbox => Rect::new(100, 50, 600, 440),
            AppKind::CaseFiles => Rect::new(120, 60, 560, 420),
            AppKind::PhotoViewer => Rect::new(150, 80, 480, 400),
            AppKind::TapePlayer => Rect::new(180, 100, 440, 320),
            AppKind::Classifier => Rect::new(140, 70, 520, 450),
            AppKind::RecycleBin => Rect::new(200, 120, 400, 300),
            AppKind::SystemDialog => Rect::new(250, 200, 300, 160),
            AppKind::OperatorRecord => Rect::new(160, 90, 480, 380),
        };

        self.wm.open_window(app, title, rect);

        let app_id = match app {
            AppKind::Inbox => "inbox",
            AppKind::CaseFiles => "case_files",
            AppKind::PhotoViewer => "photo_viewer",
            AppKind::TapePlayer => "tape_player",
            AppKind::Classifier => "classifier",
            AppKind::RecycleBin => "recycle_bin",
            _ => "",
        };

        if !app_id.is_empty() {
            let _ = self.game.call("open_app", &[string_val(app_id)]);
        }
    }

    fn handle_app_click(&mut self, app: AppKind, win_rect: Rect, pos: Position) {
        match app {
            AppKind::Inbox => {
                let _ = self.game.call("read_mail", &[string_val("mail_001")]);
            }
            AppKind::CaseFiles => {
                let _ = self
                    .game
                    .call("read_case_file", &[string_val("CASE_001_MARA_V")]);
            }
            AppKind::PhotoViewer => {
                let _ = self
                    .game
                    .call("inspect_photo", &[string_val("photo_mara_01")]);
            }
            AppKind::TapePlayer => {
                let play_btn =
                    Rect::new(win_rect.x + 20, win_rect.y + win_rect.h as i32 - 50, 70, 30);
                let pause_btn = Rect::new(
                    win_rect.x + 100,
                    win_rect.y + win_rect.h as i32 - 50,
                    70,
                    30,
                );
                if play_btn.contains(pos.x, pos.y) {
                    self.tape_app.play();
                } else if pause_btn.contains(pos.x, pos.y) {
                    self.tape_app.pause();
                }
            }
            AppKind::Classifier => {
                // Check submit classification button click
                let submit_btn = Rect::new(
                    win_rect.x + 20,
                    win_rect.y + win_rect.h as i32 - 50,
                    200,
                    35,
                );
                if submit_btn.contains(pos.x, pos.y) {
                    if let Some(cat) = self.classifier_app.selected_category {
                        let _ = self
                            .game
                            .call("submit_classification", &[string_val(cat.vs3_string())]);
                    }
                }
            }
            AppKind::RecycleBin => {
                let restore_btn = Rect::new(
                    win_rect.x + 20,
                    win_rect.y + win_rect.h as i32 - 50,
                    160,
                    30,
                );
                if restore_btn.contains(pos.x, pos.y) {
                    let _ = self
                        .game
                        .call("restore_deleted_file", &[string_val("ELISA_V_2004.tmp")]);
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self) {
        self.renderer.clear(0xFF000000);

        let phase = self
            .game
            .frame
            .as_ref()
            .map(|f| f.phase.clone())
            .unwrap_or_else(|| "boot".to_string());

        let operator_name = self
            .game
            .frame
            .as_ref()
            .map(|f| f.operator_name.clone())
            .unwrap_or_else(|| "OPERATOR".to_string());

        if phase == "boot" {
            self.renderer
                .draw_texture_scaled(&self.assets, "boot_screen", 0, 0, 800, 600);
            let bar_w = (self.boot_progress * 300.0) as u32;
            self.renderer
                .fill_rect(Rect::new(250, 420, bar_w, 16), 0xFF5A7EDC);
            self.renderer
                .draw_rect_outline(Rect::new(250, 420, 300, 16), 0xFFFFFFFF);
            self.renderer.draw_text(
                "ECHO//XP BIOS v2.04 — Initializing system...",
                230,
                460,
                0xFFFFFFFF,
                1,
            );
            return;
        }

        if phase == "login" {
            self.renderer
                .fill_rect(Rect::new(0, 0, 800, 600), 0xFF5A7EDC); // Blue XP login screen
            let box_rect = Rect::new(250, 200, 300, 200);
            self.renderer.fill_rect(box_rect, 0xFF0055EA);
            self.renderer.draw_rect_outline(box_rect, 0xFFFFFFFF);
            self.renderer
                .draw_text("ECHO//XP Professional", 270, 220, 0xFFFFFFFF, 2);
            self.renderer
                .draw_text("To begin, click your user name", 270, 250, 0xFFCCCCCC, 1);

            let user_btn = Rect::new(270, 290, 260, 50);
            let hover = user_btn.contains(self.mouse.pos.x, self.mouse.pos.y);
            let col = if hover { 0xFF1C52CE } else { 0xFF0E3088 };
            self.renderer.fill_rect(user_btn, col);
            self.renderer.draw_rect_outline(user_btn, 0xFFFFFFFF);
            self.renderer
                .draw_text(&operator_name, 320, 308, 0xFFFFFFFF, 2);

            if self.mouse.left_pressed && hover {
                let _ = self.game.call("login", &[]);
                self.audio.play_sound("logon", &self.saves.data().settings);
            }
            self.renderer
                .draw_cursor(&self.assets, self.mouse.pos, "arrow");
            return;
        }

        // Draw Wallpaper
        self.renderer.draw_texture(&self.assets, "bliss", 0, 0);

        // Draw Desktop Icons
        for icon in &self.shell.icons {
            if icon.selected {
                self.renderer.fill_rect(icon.rect, 0x66316AC5);
            }

            // Draw Icon 48x48 from atlas
            let (icon_col, icon_row) = match icon.app {
                AppKind::Inbox => (0, 0),
                AppKind::CaseFiles => (1, 0),
                AppKind::PhotoViewer => (2, 0),
                AppKind::TapePlayer => (3, 0),
                AppKind::Classifier => (4, 0),
                AppKind::RecycleBin => (5, 0),
                _ => (0, 0),
            };
            self.renderer.draw_texture_region(
                &self.assets,
                "winicons_48",
                icon_col * 48,
                icon_row * 48,
                48,
                48,
                icon.rect.x + 8,
                icon.rect.y + 4,
            );

            // Label
            self.renderer
                .draw_text(icon.label, icon.rect.x + 2, icon.rect.y + 54, 0xFFFFFFFF, 1);
        }

        // Render Open Windows (sorted by z_index)
        let mut sorted_wins = self.wm.windows.clone();
        sorted_wins.sort_by_key(|w| w.z_index);

        for win in sorted_wins {
            if win.is_minimized {
                continue;
            }
            self.renderer
                .draw_window_frame(&self.assets, &win, self.mouse.pos);
            self.render_app_content(&win);
        }

        // Render Clippy Assistant
        if let Some(frame) = &self.game.frame {
            self.renderer.draw_clippy(
                &self.assets,
                &frame.clippy_state,
                &frame.clippy_dialog,
                self.clippy_anim_time,
            );
        }

        // Render Start Menu if open
        if self.shell.start_menu_open {
            self.renderer
                .draw_start_menu(&self.assets, &operator_name, self.mouse.pos);
        }

        // Render Taskbar
        self.renderer.draw_taskbar(
            &self.assets,
            &self.wm,
            &self.shell,
            &operator_name,
            self.mouse.pos,
        );

        // Render Modal System Dialog if present
        if let Some(dialog) = &self.active_dialog {
            let dlg_rect = Rect::new(250, 180, 300, 180);
            self.renderer.fill_rect(dlg_rect, 0xFFECE9D8);
            self.renderer.draw_rect_outline(dlg_rect, 0xFF0055EA);
            let title_rect = Rect::new(251, 181, 298, 24);
            self.renderer.fill_rect(title_rect, 0xFF0055EA);
            self.renderer
                .draw_text(&dialog.title, 260, 187, 0xFFFFFFFF, 1);
            self.renderer
                .draw_text(&dialog.message, 270, 230, 0xFF000000, 1);

            let ok_btn = Rect::new(350, 320, 100, 30);
            self.renderer.fill_rect(ok_btn, 0xFF388E3C);
            self.renderer.draw_rect_outline(ok_btn, 0xFFFFFFFF);
            self.renderer.draw_text("OK", 390, 328, 0xFFFFFFFF, 1);
        }

        // Corruption / CRT scanlines effect if system compromised
        if let Some(frame) = &self.game.frame {
            if frame.system_corruption > 0.0 {
                self.renderer.apply_corruption(frame.system_corruption);
            }
        }

        // Draw Cursor
        self.renderer
            .draw_cursor(&self.assets, self.mouse.pos, "arrow");
    }

    fn render_app_content(&mut self, win: &DesktopWindow) {
        let r = win.rect;
        let content_rect = Rect::new(r.x + 4, r.y + 28, r.w - 8, r.h - 32);

        match win.app {
            AppKind::Inbox => {
                self.renderer.fill_rect(content_rect, 0xFFFFFFFF);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                if let Some(mail) = self.inbox_app.selected_mail() {
                    self.renderer.draw_text(
                        &format!("From: {}", mail.from),
                        r.x + 10,
                        r.y + 35,
                        0xFF000000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Subject: {}", mail.subject),
                        r.x + 10,
                        r.y + 50,
                        0xFF000000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Date: {}", mail.date),
                        r.x + 10,
                        r.y + 65,
                        0xFF000000,
                        1,
                    );
                    self.renderer
                        .draw_rect_outline(Rect::new(r.x + 10, r.y + 80, r.w - 20, 2), 0xFF7F9DB9);

                    self.renderer
                        .draw_text(&mail.body, r.x + 10, r.y + 90, 0xFF000000, 1);
                }
            }
            AppKind::CaseFiles => {
                self.renderer.fill_rect(content_rect, 0xFFFAFAFA);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                if let Some(file) = self.case_app.selected_file() {
                    self.renderer
                        .draw_text(&file.title, r.x + 10, r.y + 35, 0xFF800000, 2);
                    self.renderer.draw_text(
                        &format!("Subject: {} ({})", file.subject_name, file.subject_id),
                        r.x + 10,
                        r.y + 65,
                        0xFF000000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Age: {} | Status: {}", file.registered_age, file.status),
                        r.x + 10,
                        r.y + 80,
                        0xFF000000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Incident Date: {}", file.incident_date),
                        r.x + 10,
                        r.y + 95,
                        0xFF000000,
                        1,
                    );

                    self.renderer
                        .draw_text("OFFICIAL RECORD:", r.x + 10, r.y + 120, 0xFF000080, 1);
                    self.renderer.draw_text(
                        &file.official_record,
                        r.x + 10,
                        r.y + 135,
                        0xFF000000,
                        1,
                    );

                    self.renderer.draw_text(
                        "CONTRADICTIONS / DISCREPANCIES:",
                        r.x + 10,
                        r.y + 180,
                        0xFF800000,
                        1,
                    );
                    self.renderer.draw_text(
                        &file.discrepancies,
                        r.x + 10,
                        r.y + 195,
                        0xFF800000,
                        1,
                    );
                }
            }
            AppKind::PhotoViewer => {
                self.renderer.fill_rect(content_rect, 0xFF202020);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                // Simulated Photo evidence composition
                let photo_rect = Rect::new(r.x + 30, r.y + 40, r.w - 60, r.h - 100);
                self.renderer.fill_rect(photo_rect, 0xFF404040);
                self.renderer.draw_rect_outline(photo_rect, 0xFFFFFFFF);

                // Draw Mara figure
                self.renderer
                    .fill_rect(Rect::new(r.x + 180, r.y + 120, 40, 100), 0xFFE0C0A0);
                self.renderer
                    .draw_text("MARA", r.x + 180, r.y + 100, 0xFFFFFFFF, 1);

                // Draw Doppelgänger / sister anomalous silhouette if evidence progressing
                if let Some(frame) = &self.game.frame {
                    if frame.photo_inspected || frame.anomaly_level >= 2 {
                        self.renderer
                            .fill_rect(Rect::new(r.x + 240, r.y + 120, 40, 100), 0x88600000);
                        self.renderer
                            .draw_text("ELISA?", r.x + 240, r.y + 100, 0xFFFF0000, 1);
                    }
                }

                self.renderer.draw_text(
                    "EVIDENCE PHOTO #01 — Residence C-17 (2004)",
                    r.x + 20,
                    r.y + r.h as i32 - 40,
                    0xFFFFFFFF,
                    1,
                );
            }
            AppKind::TapePlayer => {
                self.renderer.fill_rect(content_rect, 0xFFECE9D8);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                self.renderer.draw_text(
                    &format!("TAPE: {}", self.tape_app.tape_name),
                    r.x + 15,
                    r.y + 35,
                    0xFF000000,
                    1,
                );

                // Progress Bar
                let progress_bar = Rect::new(r.x + 15, r.y + 55, r.w - 30, 20);
                self.renderer.fill_rect(progress_bar, 0xFFFFFFFF);
                self.renderer.draw_rect_outline(progress_bar, 0xFF000000);
                let fill_w =
                    ((self.tape_app.position / self.tape_app.duration) * (r.w - 30) as f32) as u32;
                self.renderer
                    .fill_rect(Rect::new(r.x + 15, r.y + 55, fill_w, 20), 0xFF0055EA);

                // Play & Pause buttons
                let play_btn = Rect::new(r.x + 20, r.y + r.h as i32 - 50, 70, 30);
                self.renderer.fill_rect(play_btn, 0xFF388E3C);
                self.renderer.draw_rect_outline(play_btn, 0xFFFFFFFF);
                self.renderer
                    .draw_text("PLAY", r.x + 35, r.y + r.h as i32 - 42, 0xFFFFFFFF, 1);

                let pause_btn = Rect::new(r.x + 100, r.y + r.h as i32 - 50, 70, 30);
                self.renderer.fill_rect(pause_btn, 0xFFC75050);
                self.renderer.draw_rect_outline(pause_btn, 0xFFFFFFFF);
                self.renderer
                    .draw_text("PAUSE", r.x + 110, r.y + r.h as i32 - 42, 0xFFFFFFFF, 1);

                // Live transcript
                if let Some(line) = self.tape_app.current_transcript() {
                    self.renderer.draw_text(
                        &format!("{}: {}", line.speaker, line.text),
                        r.x + 15,
                        r.y + 90,
                        0xFF000080,
                        1,
                    );
                }
            }
            AppKind::Classifier => {
                self.renderer.fill_rect(content_rect, 0xFFF0F0F0);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                self.renderer.draw_text(
                    "DISCREPANCY CLASSIFICATION SYSTEM",
                    r.x + 15,
                    r.y + 35,
                    0xFF000000,
                    2,
                );

                // Evidence Checklist
                let can_classify = self
                    .game
                    .frame
                    .as_ref()
                    .map(|f| f.can_classify)
                    .unwrap_or(false);
                let mail_ok = self
                    .game
                    .frame
                    .as_ref()
                    .map(|f| f.mail_read)
                    .unwrap_or(false);
                let file_ok = self
                    .game
                    .frame
                    .as_ref()
                    .map(|f| f.case_file_read)
                    .unwrap_or(false);
                let photo_ok = self
                    .game
                    .frame
                    .as_ref()
                    .map(|f| f.photo_inspected)
                    .unwrap_or(false);
                let tape_ok = self
                    .game
                    .frame
                    .as_ref()
                    .map(|f| f.tape_listened)
                    .unwrap_or(false);

                self.renderer.draw_text(
                    &format!(
                        "[{}] Assignment Email Read",
                        if mail_ok { "X" } else { " " }
                    ),
                    r.x + 15,
                    r.y + 70,
                    0xFF000000,
                    1,
                );
                self.renderer.draw_text(
                    &format!("[{}] Case File Inspected", if file_ok { "X" } else { " " }),
                    r.x + 15,
                    r.y + 85,
                    0xFF000000,
                    1,
                );
                self.renderer.draw_text(
                    &format!(
                        "[{}] Photo Evidence Examined",
                        if photo_ok { "X" } else { " " }
                    ),
                    r.x + 15,
                    r.y + 100,
                    0xFF000000,
                    1,
                );
                self.renderer.draw_text(
                    &format!(
                        "[{}] Recovered Audio Listened",
                        if tape_ok { "X" } else { " " }
                    ),
                    r.x + 15,
                    r.y + 115,
                    0xFF000000,
                    1,
                );

                // Category Selection
                self.renderer.draw_text(
                    "SELECT CLASSIFICATION:",
                    r.x + 15,
                    r.y + 140,
                    0xFF000080,
                    1,
                );
                let mut cat_y = 160;
                for cat in AnomalyCategory::ALL {
                    let is_sel = self.classifier_app.selected_category == Some(cat);
                    let prefix = if is_sel { "(X)" } else { "( )" };
                    let col = if is_sel { 0xFF0055EA } else { 0xFF000000 };
                    self.renderer.draw_text(
                        &format!("{prefix} {}", cat.label()),
                        r.x + 20,
                        cat_y,
                        col,
                        1,
                    );
                    cat_y += 20;
                }

                // Submit Button
                let submit_btn = Rect::new(r.x + 20, r.y + r.h as i32 - 50, 200, 35);
                let btn_col = if can_classify { 0xFF0055EA } else { 0xFF7F9DB9 };
                self.renderer.fill_rect(submit_btn, btn_col);
                self.renderer.draw_rect_outline(submit_btn, 0xFFFFFFFF);
                self.renderer.draw_text(
                    "SUBMIT VERDICT",
                    r.x + 45,
                    r.y + r.h as i32 - 40,
                    0xFFFFFFFF,
                    1,
                );
            }
            AppKind::RecycleBin => {
                self.renderer.fill_rect(content_rect, 0xFFFFFFFF);
                self.renderer.draw_rect_outline(content_rect, 0xFF7F9DB9);

                self.renderer
                    .draw_text("RECYCLE BIN CONTENTS:", r.x + 15, r.y + 35, 0xFF000000, 1);

                if let Some(item) = self.recycle_app.items.first() {
                    self.renderer.draw_text(
                        &format!("File: {}", item.name),
                        r.x + 20,
                        r.y + 60,
                        0xFF800000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Original: {}", item.original_location),
                        r.x + 20,
                        r.y + 80,
                        0xFF000000,
                        1,
                    );
                    self.renderer.draw_text(
                        &format!("Date Deleted: {}", item.date_deleted),
                        r.x + 20,
                        r.y + 100,
                        0xFF000000,
                        1,
                    );

                    let restore_btn = Rect::new(r.x + 20, r.y + r.h as i32 - 50, 160, 30);
                    self.renderer.fill_rect(restore_btn, 0xFF388E3C);
                    self.renderer.draw_rect_outline(restore_btn, 0xFFFFFFFF);
                    self.renderer.draw_text(
                        "RESTORE FILE",
                        r.x + 35,
                        r.y + r.h as i32 - 42,
                        0xFFFFFFFF,
                        1,
                    );
                }
            }
            _ => {}
        }
    }

    pub fn capture_screen(&mut self, output_path: &Path) -> Result<()> {
        self.render();

        let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(WIDTH as u32, HEIGHT as u32);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let argb = self.renderer.buffer[y * WIDTH + x];
                let a = ((argb >> 24) & 0xFF) as u8;
                let r = ((argb >> 16) & 0xFF) as u8;
                let g = ((argb >> 8) & 0xFF) as u8;
                let b = (argb & 0xFF) as u8;
                img.put_pixel(x as u32, y as u32, Rgba([r, g, b, a]));
            }
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        img.save(output_path)
            .with_context(|| format!("Failed saving screenshot to {:?}", output_path))?;
        Ok(())
    }
}
