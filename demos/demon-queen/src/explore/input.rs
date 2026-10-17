//! La entrada del mundo: los eventos de la ventana (teclado, ratón, foco, tamaño), el ritmo de los fotogramas y lo que cada tecla hace en cada lugar.
use super::ui_sfx::{self, UiSfx};
use super::*;

impl ApplicationHandler for App {
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let (DeviceEvent::MouseMotion { delta }, Some(fight)) = (&event, self.boss.as_mut()) {
            if self.dungeon_mouse_captured && self.window.as_ref().is_some_and(|w| w.has_focus()) {
                let k = crate::exploration3d::prefs::look_scale();
                fight.player.look(delta.0 as f32 * k, delta.1 as f32 * k);
            }
            return;
        }
        if !self.dungeon_mode {
            return;
        }
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.dungeon_mouse_captured
                && self.dungeon.seed_input.is_none()
                && !self.dungeon.map_open
                && self.window.as_ref().is_some_and(|w| w.has_focus())
            {
                let k = crate::exploration3d::prefs::look_scale();
                self.dungeon
                    .controller
                    .look(delta.0 as f32 * k, delta.1 as f32 * k);
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Demon Queen — Cámara Real 3D")
            .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_min_inner_size(winit::dpi::PhysicalSize::new(640, 360));
        let window = Arc::new(event_loop.create_window(attrs).expect("create 3D window"));
        eprintln!("[3d] window created");
        let prepared = pollster::block_on(WindowDisplay::new(window.clone())).and_then(|display| {
            let state =
                RenderState::load(display.gpu.clone(), &self.model_file, self.dungeon.seed())?;
            self.display = Some(display);
            Ok(state)
        });
        match prepared {
            Ok(state) => {
                eprintln!("[3d] state ready");
                self.state = Some(state);
                self.window = Some(window.clone());
                self.last_frame = Instant::now();
                self.next_frame_at = self.last_frame;
                self.persist_3d_state(true);
                // QA: VELVET_WAR_TABLE opens the war table at once (to try its menus with the mouse).
                if std::env::var_os("VELVET_WAR_TABLE").is_some() {
                    self.open_war_table();
                }
                window.request_redraw();
                eprintln!("[3d] first redraw requested");
            }
            Err(error) => {
                eprintln!("[3d] no se pudo iniciar: {error:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.persist_3d_state(true);
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(display) = &mut self.display {
                    display.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(false) => {
                self.clear_input();
                self.bed_scene_button(false);
                if self.dungeon_mode || self.boss.is_some() {
                    self.release_dungeon_mouse();
                }
            }
            WindowEvent::Focused(true) if self.boss.is_some() => {
                self.clear_input();
                self.dungeon_mouse_captured = self.capture_dungeon_mouse();
            }
            WindowEvent::Focused(true) if self.dungeon_mode => {
                self.clear_input();
                if !self.modal_is_open() {
                    self.dungeon_mouse_captured = self.capture_dungeon_mouse();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x as f32, position.y as f32));
                if self.war.is_some() {
                    self.war_pointer();
                }
                if self.talk.is_some() && !self.dungeon_mode {
                    self.talk_hover();
                }
                if self.court_open() && self.talk.is_none() {
                    self.court_hover();
                }
                if self.caricias_open() {
                    self.caricias_pointer();
                }
                if self.mascot_held() {
                    if let Some((x, y)) = self.shop_pointer_position() {
                        self.mascot_drag_to(x, y);
                    }
                }
                if self.dungeon_mode && self.dungeon.map_open {
                    let at = self.shop_pointer_position().map(|(x, y)| [x, y]);
                    self.dungeon.map_pointer(at);
                }
                if self.shop_open {
                    self.update_shop_preview_drag();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor_position = None;
                if self.dungeon_mode {
                    self.dungeon.map_pointer(None);
                }
                if self.shop_preview_dragging {
                    self.shop_preview_drag_anchor = None;
                }
            }
            // In the fight the buttons are the sword and the seal.
            WindowEvent::MouseInput { state, button, .. } if self.boss.is_some() => {
                self.boss_button(button, state == ElementState::Pressed);
            }
            // At the war table the mouse is the general's.
            WindowEvent::MouseInput { state, button, .. } if self.war.is_some() => {
                self.war_button(button, state == ElementState::Pressed);
            }
            WindowEvent::MouseWheel { delta, .. } if self.war.is_some() => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 100.0,
                };
                self.war_wheel(amount);
            }
            // Chiquita can be poked and picked up wherever she is: that click is hers.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if !self.dungeon_mode
                && self
                    .shop_pointer_position()
                    .is_some_and(|(x, y)| self.mascot_hit(x, y)) =>
            {
                if let Some((x, y)) = self.shop_pointer_position() {
                    self.mascot_press(x, y);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } if self.mascot_held() => {
                self.mascot_release();
            }
            WindowEvent::MouseInput { state, button, .. }
                if self.caricias_open()
                    && matches!(button, MouseButton::Left | MouseButton::Right) =>
            {
                self.caricias_button(button == MouseButton::Right, state == ElementState::Pressed);
            }
            WindowEvent::MouseWheel { delta, .. } if self.caricias_open() => {
                let down = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y < 0.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y < 0.0,
                };
                self.caricias_wheel(down);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if self.court_open() && self.talk.is_none() => {
                self.court_click();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if self.alcove_menu_open() && !self.bed_scene_open() && self.talk.is_none() => {
                self.alcove_click();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if self.mage_open() && !self.bed_scene_open() => {
                self.mage_click();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if self.drink.is_some() => {
                if state == ElementState::Pressed {
                    self.drink_click();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if self.bed_scene_open() => {
                self.bed_scene_button(state == ElementState::Pressed);
            }
            WindowEvent::MouseInput { state, button, .. }
                if self.dungeon_mode
                    && self.dungeon.map_open
                    && matches!(
                        button,
                        MouseButton::Left | MouseButton::Right | MouseButton::Middle
                    ) =>
            {
                if state == ElementState::Pressed {
                    // The pointer may not have moved since the map opened.
                    let at = self.shop_pointer_position().map(|(x, y)| [x, y]);
                    self.dungeon.map_pointer(at);
                    self.dungeon.map_press(button != MouseButton::Left);
                } else {
                    self.dungeon.map_release();
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.dungeon_mode && self.dungeon.map_open => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 60.0,
                };
                self.dungeon.map_zoom_by(amount);
            }
            WindowEvent::MouseWheel { delta, .. } if self.shop_open && !self.venue.open => {
                if self
                    .shop_pointer_position()
                    .is_some_and(|(x, y)| (72.0..790.0).contains(&x) && (230.0..984.0).contains(&y))
                {
                    let amount = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32 / 28.0,
                    };
                    if amount.abs() >= 0.5 {
                        self.move_shop_selection(if amount > 0.0 { -1 } else { 1 });
                    }
                }
            }
            // The wardrobe: a click chooses a card, a second one puts it on.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if self.wardrobe_open && !self.dungeon_mode && self.talk.is_none() => {
                self.wardrobe_click();
            }
            // A conversation takes the click as it takes F: the next line, or the option under the
            // pointer. A stage scene's last card goes down with a click too.
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if !self.dungeon_mode
                && !self.shop_open
                && !self.escort_active()
                && (self.talk.is_some()
                    || self.stage.as_ref().is_some_and(|s| s.outro_at.is_some())) =>
            {
                if self.talk.is_some() {
                    self.talk_click();
                } else if self
                    .stage
                    .as_ref()
                    .and_then(|s| s.outro_at)
                    .is_some_and(|t| self.elapsed - t > 0.8)
                {
                    if let Some(stage) = self.stage.as_mut() {
                        stage.unlocks.clear();
                    }
                    self.end_talk();
                }
                self.clear_input();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } if self.shop_preview_dragging => {
                self.shop_preview_dragging = false;
                self.shop_preview_drag_anchor = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.dungeon_mode
                    && !self.shop_open
                    && !self.wardrobe_open
                    && !self.venue.open
                    && !self.escort_active()
                {
                    // The corner minimap opens the full map; its Close button shuts it.
                    let hit = self.shop_pointer_position().and_then(|(x, y)| {
                        map_overlay::map_hit_test(
                            WINDOW_WIDTH as f32,
                            WINDOW_HEIGHT as f32,
                            self.map_open,
                            x,
                            y,
                        )
                    });
                    match hit {
                        Some(map_overlay::MapHit::Close) => {
                            self.map_open = false;
                            return;
                        }
                        Some(map_overlay::MapHit::Expand) => {
                            self.map_open = true;
                            return;
                        }
                        None => {}
                    }
                }
                if self.dungeon_mode {
                    self.dungeon.swing_sword();
                    return;
                }
                if self.shop_open {
                    if self.begin_shop_preview_drag() {
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                        return;
                    }
                    self.handle_shop_mouse_click();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    if self.dungeon_mode && event.state == ElementState::Pressed {
                        if let Some(text) = event.text {
                            self.dungeon.text_input(&text);
                        }
                    }
                    return;
                };
                if self.boss.is_some() {
                    self.boss_key(code, event.state == ElementState::Pressed, event.repeat);
                    return;
                }
                if self.war.is_some() {
                    self.war_key(code, event.state == ElementState::Pressed, event.repeat);
                    return;
                }
                if self.dungeon_mode && matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight) {
                    if event.state == ElementState::Pressed {
                        self.clear_input();
                        self.release_dungeon_mouse();
                    } else if self
                        .window
                        .as_ref()
                        .is_some_and(|window| window.has_focus())
                        && !self.modal_is_open()
                    {
                        self.dungeon_mouse_captured = self.capture_dungeon_mouse();
                    }
                    return;
                }
                if self.dungeon_mode {
                    if self.dungeon.seed_input.is_none() {
                        self.set_key(code, event.state == ElementState::Pressed);
                    }
                    let was_modal = self.dungeon.seed_input.is_some() || self.dungeon.map_open;
                    if code == KeyCode::KeyQ
                        && event.state == ElementState::Pressed
                        && !event.repeat
                        && !was_modal
                    {
                        self.quick_cure();
                        return;
                    }
                    self.dungeon.key(
                        code,
                        event.state == ElementState::Pressed,
                        event.repeat,
                        event.text.as_ref().map(|s| s.as_str()),
                    );
                    let modal = self.dungeon.seed_input.is_some() || self.dungeon.map_open;
                    if modal != was_modal {
                        self.clear_input();
                        if modal {
                            self.release_dungeon_mouse();
                        } else {
                            self.dungeon_mouse_captured = self.capture_dungeon_mouse();
                        }
                    }
                    if self.dungeon.pending_exit {
                        self.leave_dungeon();
                    }
                    return;
                }
                // Being taken up by a girl: Esc (or F, Space, Enter) skips to the room.
                if self.escort_active() {
                    if event.state == ElementState::Pressed
                        && !event.repeat
                        && matches!(
                            code,
                            KeyCode::Escape
                                | KeyCode::Backspace
                                | KeyCode::Space
                                | KeyCode::Enter
                                | KeyCode::KeyF
                        )
                    {
                        self.skip_escort();
                    }
                    self.clear_input();
                    return;
                }
                // The elixir's close shot takes every key while it plays.
                if self.drink.is_some() {
                    self.drink_key(code, event.state == ElementState::Pressed, event.repeat);
                    return;
                }
                // The bed scene takes every key while it plays.
                if self.bed_scene_open() {
                    self.bed_scene_key(code, event.state == ElementState::Pressed, event.repeat);
                    return;
                }
                // A stage scene's outro: F (or Enter, Space, Esc) brings the stage down.
                if self.talk.is_none() && self.stage.as_ref().is_some_and(|s| s.outro_at.is_some())
                {
                    if event.state == ElementState::Pressed && !event.repeat {
                        if matches!(
                            code,
                            KeyCode::KeyF
                                | KeyCode::Space
                                | KeyCode::Enter
                                | KeyCode::NumpadEnter
                                | KeyCode::Escape
                        ) && self
                            .stage
                            .as_ref()
                            .and_then(|s| s.outro_at)
                            .is_some_and(|t| self.elapsed - t > 0.8)
                        {
                            if let Some(stage) = self.stage.as_mut() {
                                stage.unlocks.clear();
                            }
                            self.end_talk();
                        }
                    }
                    self.clear_input();
                    return;
                }
                // A conversation takes the keys: on, choose, or leave it.
                if self.talk.is_some() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        match code {
                            KeyCode::KeyF
                            | KeyCode::Space
                            | KeyCode::Enter
                            | KeyCode::NumpadEnter => self.talk_confirm(),
                            KeyCode::ArrowUp | KeyCode::KeyW => self.talk_move(-1),
                            KeyCode::ArrowDown | KeyCode::KeyS => self.talk_move(1),
                            KeyCode::Digit1 | KeyCode::Numpad1 => self.talk_pick(0),
                            KeyCode::Digit2 | KeyCode::Numpad2 => self.talk_pick(1),
                            KeyCode::Digit3 | KeyCode::Numpad3 => self.talk_pick(2),
                            KeyCode::Digit4 | KeyCode::Numpad4 => self.talk_pick(3),
                            // A stage scene is played through (its end sets its variables).
                            KeyCode::Escape | KeyCode::Backspace if !self.stage_up() => {
                                self.end_talk()
                            }
                            _ => {}
                        }
                    }
                    self.clear_input();
                    return;
                }
                // A night's touch game takes the keys while it is up.
                if self.caricias_open() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.caricias_key(code);
                    }
                    self.clear_input();
                    return;
                }
                // The court takes the keys while it is up.
                if self.court_open() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.court_key(code);
                    }
                    self.clear_input();
                    return;
                }
                // The hypnotist's stall takes the keys while it is open.
                if self.mage_open() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.mage_key(code);
                    }
                    self.clear_input();
                    return;
                }
                // In her bed, its menu takes the keys.
                if self.alcove_menu_open() {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.alcove_key(code);
                    }
                    self.clear_input();
                    return;
                }
                if matches!(code, KeyCode::Escape | KeyCode::Backspace)
                    && event.state == ElementState::Pressed
                {
                    if self.achievements_open {
                        self.achievements_open = false;
                        ui_sfx::play(UiSfx::Close);
                        return;
                    }
                    if self.shop_open {
                        let now = (self.elapsed * 1000.0) as u32;
                        if self.venue.open {
                            if !self.venue.back(now) {
                                self.close_shop();
                            }
                        } else {
                            match self.shop_page {
                                shop_games::Page::Haggle => {
                                    self.haggle_play(shop_games::Tactic::Leave)
                                }
                                shop_games::Page::Fuse => self.show_shelves(shop::Tab::Buy),
                                shop_games::Page::List => self.close_shop(),
                            }
                        }
                        return;
                    }
                    if self.wardrobe_open {
                        self.wardrobe_open = false;
                        self.wardrobe_selection = self.outfit;
                        self.clear_input();
                        ui_sfx::play(UiSfx::Close);
                        return;
                    }
                    self.persist_3d_state(true);
                    event_loop.exit();
                    return;
                }
                if !self.wardrobe_open && !self.shop_open && !self.achievements_open {
                    self.set_key(code, event.state == ElementState::Pressed);
                }
                if event.state == ElementState::Pressed {
                    // The achievements screen takes no other key than its own.
                    if self.achievements_open {
                        if code == KeyCode::KeyL {
                            self.achievements_open = false;
                            ui_sfx::play(UiSfx::Close);
                        }
                        return;
                    }
                    if self.shop_open && self.venue.open {
                        self.venue_key(code);
                        return;
                    }
                    if self.shop_open && self.shop_page == shop_games::Page::Haggle {
                        self.haggle_key(code);
                        return;
                    }
                    if self.shop_open && self.shop_page == shop_games::Page::Fuse {
                        match code {
                            KeyCode::Tab | KeyCode::KeyB => self.toggle_shop_tab(),
                            KeyCode::KeyF => self.close_shop(),
                            _ => self.fuse_key(code),
                        }
                        return;
                    }
                    if self.shop_open {
                        match code {
                            KeyCode::KeyH => self.start_haggle(),
                            KeyCode::ArrowUp | KeyCode::KeyW => self.move_shop_selection(-1),
                            KeyCode::ArrowDown | KeyCode::KeyS => self.move_shop_selection(1),
                            KeyCode::ArrowLeft | KeyCode::PageUp if !self.venue.open => {
                                self.change_shop_page(-1)
                            }
                            KeyCode::ArrowRight | KeyCode::PageDown if !self.venue.open => {
                                self.change_shop_page(1)
                            }
                            KeyCode::Digit1 if !self.venue.open => {
                                self.select_shop_category(shop::Category::All)
                            }
                            KeyCode::Digit2 if !self.venue.open => {
                                self.select_shop_category(shop::Category::Consumables)
                            }
                            KeyCode::Digit3 if !self.venue.open => {
                                self.select_shop_category(shop::Category::Equipment)
                            }
                            KeyCode::Digit4 if !self.venue.open => {
                                self.select_shop_category(shop::Category::Materials)
                            }
                            KeyCode::Digit5 if !self.venue.open => {
                                self.select_shop_category(shop::Category::Special)
                            }
                            KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                            | KeyCode::Tab
                            | KeyCode::KeyB => self.toggle_shop_tab(),
                            KeyCode::Minus | KeyCode::NumpadSubtract => {
                                self.adjust_shop_quantity(-1)
                            }
                            KeyCode::Equal | KeyCode::NumpadAdd => self.adjust_shop_quantity(1),
                            KeyCode::Enter | KeyCode::Space => self.commit_shop_transaction(),
                            KeyCode::KeyF => self.close_shop(),
                            _ => {}
                        }
                        return;
                    }
                    if self.wardrobe_open {
                        match code {
                            KeyCode::ArrowLeft | KeyCode::KeyA => {
                                self.wardrobe_selection =
                                    self.wardrobe_step(self.wardrobe_selection, -1);
                                ui_sfx::play(UiSfx::Move);
                            }
                            KeyCode::ArrowRight | KeyCode::KeyD => {
                                self.wardrobe_selection =
                                    self.wardrobe_step(self.wardrobe_selection, 1);
                                ui_sfx::play(UiSfx::Move);
                            }
                            KeyCode::Digit1
                            | KeyCode::Digit2
                            | KeyCode::Digit3
                            | KeyCode::Digit4
                            | KeyCode::Digit5
                            | KeyCode::Digit6 => {
                                let slot = match code {
                                    KeyCode::Digit1 => 0,
                                    KeyCode::Digit2 => 1,
                                    KeyCode::Digit3 => 2,
                                    KeyCode::Digit4 => 3,
                                    KeyCode::Digit5 => 4,
                                    _ => 5,
                                };
                                if slot < wardrobe_outfit_count()
                                    && self
                                        .achievements
                                        .outfit_available(achievements::catalog(), slot)
                                {
                                    self.wardrobe_selection = slot;
                                }
                            }
                            KeyCode::Enter | KeyCode::Space => self.wardrobe_confirm(),
                            KeyCode::KeyF => {
                                self.wardrobe_open = false;
                                self.wardrobe_selection = self.outfit;
                                self.clear_input();
                                ui_sfx::play(UiSfx::Close);
                            }
                            _ => {}
                        }
                        return;
                    }
                    match code {
                        KeyCode::KeyG
                            if self.brothel.is_none()
                                && self.baths.is_none()
                                && self.penascal.is_none() =>
                        {
                            self.enter_dungeon()
                        }
                        // The followed mission: the next open one.
                        KeyCode::KeyJ if !self.dungeon_mode => self.cycle_mission(),
                        // By Varrak: fight him now (a rehearsal or a rematch).
                        KeyCode::KeyR if self.penascal.is_some() => self.fight_varrak_now(),
                        KeyCode::KeyQ => self.iso.zoom_in(0.4),
                        KeyCode::KeyE => self.iso.zoom_out(0.4),
                        KeyCode::KeyM => {
                            self.map_open = !self.map_open;
                        }
                        KeyCode::KeyL if !self.dungeon_mode => {
                            self.achievements_open = !self.achievements_open;
                            ui_sfx::play(if self.achievements_open {
                                UiSfx::Open
                            } else {
                                UiSfx::Close
                            });
                            self.clear_input();
                        }
                        KeyCode::KeyF => {
                            if self.zone().interact(self) {
                                return;
                            }
                        }
                        KeyCode::KeyZ => self.iso.turn(-1),
                        KeyCode::KeyX => self.iso.turn(1),
                        KeyCode::KeyV => self.iso.toggle_top_down(),
                        KeyCode::KeyB => {
                            let preset = match self.environment_cycle.preset() {
                                EnvironmentPreset3D::Lived => EnvironmentPreset3D::Demonic,
                                EnvironmentPreset3D::Demonic => EnvironmentPreset3D::Lived,
                            };
                            self.environment_cycle.set_preset(preset);
                        }
                        KeyCode::KeyC => {
                            self.environment_cycle_enabled = !self.environment_cycle_enabled;
                        }
                        KeyCode::KeyN => {
                            // Development shortcut: through the castle's door to the wood, or back.
                            if self.nature_mode {
                                self.enter_castle_from_nature();
                            } else {
                                self.enter_nature_from_castle();
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().clamp(0.0, 0.05);
                self.last_frame = now;
                // Exercise real movement, skinning, footprints and rendering without
                // OS input injection. Only enabled explicitly by the preview CLI.
                if self.movement_smoke {
                    if self.elapsed >= 24.0 {
                        eprintln!(
                            "[movement-smoke] PASS pos={:.2},{:.2} footprints={}",
                            self.player.x,
                            self.player.z,
                            self.footprints.len()
                        );
                        event_loop.exit();
                        return;
                    }
                    let leg = (self.elapsed / 3.0) as u32 % 4;
                    self.move_forward = leg == 0;
                    self.move_right = leg == 1;
                    self.move_back = leg == 2;
                    self.move_left = leg == 3;
                    self.sprint = self.elapsed >= 12.0;
                }
                self.tick(dt);
                self.persist_3d_state(false);
                self.render_frame();
                // Updating the native title bar every redraw can invalidate
                // the window on Windows and feed extra redraw events back
                // into the loop. Refresh it only when the half-second FPS
                // sample rolls over (about twice per second).
                if !self.embedded && self.fps_frames == 0 {
                    if let Some(window) = &self.window {
                        let (scene_label, interaction_hint) = self.zone().title(self);
                        let cycle_label = if self.environment_cycle_enabled {
                            "ciclo ON"
                        } else {
                            "ciclo PAUSA"
                        };
                        let camera_label = if self.dungeon_mode {
                            "primera persona [ratón: mirar]"
                        } else if self.iso.top_down {
                            "vista cenital [V]"
                        } else {
                            "vista normal [V]"
                        };
                        window.set_title(&format!(
                            "Demon Queen — {scene_label}  |  {camera_label}  |  ambiente {} [B]  |  {} [C]  |  luna {:.0}%  |  WASD/flechas mover  |  Shift correr  |  Q/E zoom  |  Z/X girar 15°  |  [M] mapa {}  |  {:.0} FPS  |  {}x{}  |  pos {:.1}, {:.1}{}",
                            self.environment_cycle.preset().label(),
                            cycle_label,
                            self.environment_cycle.lunar_phase() * 100.0,
                            if self.map_open { "ampliado" } else { "compacto" },
                            self.fps_display,
                            WINDOW_WIDTH,
                            WINDOW_HEIGHT,
                            self.player.x,
                            self.player.z,
                            interaction_hint
                        ));
                    }
                }
            }
            _ => {}
        }
        if let (Some(display), Some(state)) = (&mut self.display, &self.state) {
            if let Err(error) = display.present_texture(&state.target_view, 1.0) {
                eprintln!("[3d] presentación falló: {error:#}");
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Redraw as fast as the display takes frames: the vsynced present
        // paces the loop at the monitor's refresh (60, 144, 240 Hz...). The
        // short floor only stops a hidden or minimised window, whose present
        // no longer blocks, from spinning a core. The deadline is persistent,
        // so it never adds a second wait after a vsynced present.
        let now = Instant::now();
        // Minimised, nothing is seen: a few frames a second keep the world going.
        let minimised = self
            .window
            .as_ref()
            .is_some_and(|w| w.is_minimized().unwrap_or(false) || w.inner_size().width == 0);
        if now >= self.next_frame_at {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            self.next_frame_at = now
                + if minimised {
                    MINIMISED_FRAME_INTERVAL
                } else {
                    MIN_FRAME_INTERVAL
                };
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_at));
    }
}

/// Shortest time between redraws (a 400 fps ceiling); the display's vsync
/// normally paces frames well before this.
pub(super) const MIN_FRAME_INTERVAL: Duration = Duration::from_micros(2500);

/// Between redraws while the window is minimised (10 a second).
pub(super) const MINIMISED_FRAME_INTERVAL: Duration = Duration::from_millis(100);
