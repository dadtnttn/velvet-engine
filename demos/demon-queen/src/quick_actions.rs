//! Native dialogue-footer controls. All input uses the same logical rectangles.
use super::*;

const SKIP_INTERVAL: Duration = Duration::from_millis(220);
const ACTION_WIDTHS: [i32; 4] = [146, 122, 134, 134];

pub(super) fn rects(styles: &Stylesheet) -> [(i32, i32, i32, i32); 4] {
    let dialogue = dialogue_layout(styles);
    let padding = component_style(styles, "dialogue-panel", "panel")
        .number("content-padding-x", 58.0)
        .round() as i32;
    let mut x = dialogue.x + dialogue.width - padding - ACTION_WIDTHS.iter().sum::<i32>() - 24;
    ACTION_WIDTHS.map(|width| {
        let rect = (x, dialogue.y + dialogue.height - 52, width, 36);
        x += width + 8;
        rect
    })
}

impl App {
    fn quick_action_enabled(&self, index: usize) -> bool {
        match index {
            0 => !self.reveal_complete || self.view.as_ref().is_some_and(is_inline_continue),
            1 => self.skipping_dialogue || self.can_skip_dialogue(),
            2 => self.game.is_some(),
            3 => true,
            _ => false,
        }
    }

    fn can_skip_dialogue(&self) -> bool {
        self.screen == Screen::Play
            && !self.ui_hidden
            && self.view.as_ref().is_some_and(|view| {
                is_inline_continue(view) && !view.ending && view.world_entry.is_none()
            })
    }

    pub(super) fn stop_dialogue_skip(&mut self) {
        if self.skipping_dialogue {
            self.skipping_dialogue = false;
            self.needs_paint = true;
        }
    }

    fn activate_quick_action(&mut self, index: usize) {
        if !self.quick_action_enabled(index) {
            return;
        }
        match index {
            0 => {
                self.stop_dialogue_skip();
                self.advance_play();
            }
            1 => {
                self.skipping_dialogue = !self.skipping_dialogue;
                self.next_dialogue_skip = Instant::now() + SKIP_INTERVAL;
                if self.skipping_dialogue {
                    self.reveal_complete = true;
                }
                self.needs_paint = true;
            }
            2 => self.open_pause(pause_menu::Page::Save),
            3 => self.open_pause(pause_menu::Page::Status),
            _ => {}
        }
    }

    pub(super) fn tick_dialogue_skip(&mut self, now: Instant) {
        if !self.skipping_dialogue {
            return;
        }
        if !self.can_skip_dialogue() {
            self.stop_dialogue_skip();
            return;
        }
        if now < self.next_dialogue_skip {
            return;
        }
        // Follow the authored continue edge once; never choose a story branch.
        let previous = self.view.as_ref().map(|view| view.id.clone());
        self.reveal_complete = true;
        self.advance_play();
        self.reveal_complete = true;
        self.next_dialogue_skip = Instant::now() + SKIP_INTERVAL;
        if !self.can_skip_dialogue() || self.view.as_ref().map(|view| &view.id) == previous.as_ref()
        {
            self.stop_dialogue_skip();
        }
        self.needs_paint = true;
    }

    pub(super) fn quick_action_click(&mut self, x: i32, y: i32) -> bool {
        if self.screen != Screen::Play || self.ui_hidden {
            return false;
        }
        if let Some(index) = rects(&self.styles)
            .iter()
            .position(|rect| rect_contains(*rect, x, y))
        {
            self.quick_action_focus = None;
            self.activate_quick_action(index);
            // Disabled controls consume the click too; it must not advance text.
            return true;
        }
        self.quick_action_focus = None;
        false
    }

    pub(super) fn quick_action_key(&mut self, key: KeyCode) -> bool {
        if self.screen != Screen::Play || self.ui_hidden {
            return false;
        }
        if key == KeyCode::KeyS {
            self.activate_quick_action(1);
            return true;
        }
        if key == KeyCode::Tab
            || (self.quick_action_focus.is_some()
                && matches!(key, KeyCode::ArrowLeft | KeyCode::ArrowRight))
        {
            let direction = if key == KeyCode::ArrowLeft { -1 } else { 1 };
            let start = self.quick_action_focus.map(|i| i as i32).unwrap_or(-1);
            for step in 1..=4 {
                let index = (start + direction * step).rem_euclid(4) as usize;
                if self.quick_action_enabled(index) {
                    self.quick_action_focus = Some(index);
                    break;
                }
            }
            self.needs_paint = true;
            return true;
        }
        if matches!(key, KeyCode::Enter | KeyCode::Space) {
            if let Some(index) = self.quick_action_focus {
                self.activate_quick_action(index);
                return true;
            }
        }
        if matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown) {
            self.quick_action_focus = None;
            self.needs_paint = true;
        }
        false
    }

    pub(super) fn paint_quick_actions(&mut self) {
        if self.ui_hidden {
            return;
        }
        let Some(fonts) = &self.fonts else {
            return;
        };
        // Spanish source for every language but English: the canvas translates it as it draws.
        let spanish = self.localizer.language() != Language::English;
        let labels = [
            if !self.reveal_complete {
                if spanish {
                    "Mostrar texto"
                } else {
                    "Show text"
                }
            } else if self.view.as_ref().is_some_and(is_inline_continue) {
                if spanish {
                    "Continuar ›"
                } else {
                    "Continue ›"
                }
            } else {
                if spanish {
                    "Elegir ↑"
                } else {
                    "Choose ↑"
                }
            },
            if self.skipping_dialogue {
                if spanish {
                    "Detener »"
                } else {
                    "Stop »"
                }
            } else {
                if spanish {
                    "Saltar »"
                } else {
                    "Skip »"
                }
            },
            if spanish {
                "Guardar · F5"
            } else {
                "Save · F5"
            },
            if spanish {
                "Menú · Esc"
            } else {
                "Menu · Esc"
            },
        ];
        let accent = rgb_u32(self.ambient_now());
        for (index, rect) in rects(&self.styles).into_iter().enumerate() {
            let enabled = self.quick_action_enabled(index);
            let hovered = enabled && rect_contains(rect, self.mouse.0, self.mouse.1);
            let lit = enabled
                && (hovered
                    || self.quick_action_focus == Some(index)
                    || (index == 1 && self.skipping_dialogue));
            let pressed = hovered && self.mouse_down;
            if !vn_web::classic() {
                let (main, key) = labels[index]
                    .split_once(" · ")
                    .unwrap_or((labels[index], ""));
                vn_web::paint_quick_button(
                    &mut self.canvas,
                    fonts,
                    rect,
                    main,
                    key,
                    enabled,
                    lit,
                    pressed,
                );
                continue;
            }
            let (x, y, w, h) = rect;
            let with_alpha = |alpha: f32| ((alpha * 255.0) as u32) << 24 | accent;
            // A quiet pill: faint glass that takes the speaker's colour when lit.
            let fill = if pressed {
                with_alpha(0.32)
            } else if lit {
                with_alpha(0.20)
            } else {
                0x14ffffff
            };
            self.canvas.round_rect(x, y, w, h, h / 2, fill, fill);
            let border = if lit {
                with_alpha(0.85)
            } else if enabled {
                0x2cffffff
            } else {
                0x14ffffff
            };
            self.canvas.round_outline(x, y, w, h, h / 2, 1.0, border);
            // "Guardar · F5": the action in full colour, its key dimmer.
            let (main, key) = labels[index]
                .split_once(" · ")
                .unwrap_or((labels[index], ""));
            let main_color = if !enabled {
                0x66d9c6d2
            } else if lit {
                0xffffffff
            } else {
                0xfff0e2ec
            };
            let (main_size, key_size, gap) = (16.5, 14.0, 8);
            let main_width = fonts.text_width(main, main_size, true);
            let key_width = if key.is_empty() {
                0
            } else {
                fonts.text_width(key, key_size, false) + gap
            };
            let mut tx = x + (w - main_width - key_width) / 2;
            let ty = y + (h - main_size.round() as i32) / 2 - 2;
            self.canvas
                .text(fonts, main, tx, ty, main_size, main_color, true);
            if !key.is_empty() {
                tx += main_width + gap;
                self.canvas.text(
                    fonts,
                    key,
                    tx,
                    ty + 1,
                    key_size,
                    if enabled { 0xa0d8bccb } else { 0x55d9c6d2 },
                    false,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_save_and_menu_do_not_advance_or_reveal_dialogue() {
        let mut app = App::default();
        app.start_game().unwrap();
        let before = app.view.as_ref().unwrap().id.clone();
        for (index, page) in [(2, pause_menu::Page::Save), (3, pause_menu::Page::Status)] {
            let r = rects(&app.styles)[index];
            app.click_at(r.0 + r.2 / 2, r.1 + r.3 / 2);
            assert_eq!(app.screen, Screen::GameMenu);
            assert_eq!(app.pause.as_ref().unwrap().page, page);
            assert_eq!(app.view.as_ref().unwrap().id, before);
            assert!(!app.reveal_complete);
            app.resume_pause();
        }
    }

    #[test]
    fn skip_stops_on_authored_decision_and_cannot_choose_for_player() {
        let mut app = App::default();
        app.start_game().unwrap();
        app.activate_quick_action(1);
        for _ in 0..128 {
            if !app.skipping_dialogue {
                break;
            }
            app.tick_dialogue_skip(Instant::now() + Duration::from_secs(1));
        }
        assert!(!app.skipping_dialogue);
        assert!(app.reveal_complete);
        assert!(app.view.as_ref().unwrap().choices.len() > 1);
        let decision = app.view.as_ref().unwrap().id.clone();
        app.activate_quick_action(1);
        app.tick_dialogue_skip(Instant::now() + Duration::from_secs(2));
        assert_eq!(app.view.as_ref().unwrap().id, decision);
    }

    #[test]
    fn pause_and_hidden_ui_stop_skip_and_footer_keyboard_opens_menu() {
        let mut app = App::default();
        app.start_game().unwrap();
        app.handle_key(KeyCode::KeyS);
        assert!(app.skipping_dialogue);
        app.open_pause(pause_menu::Page::Status);
        assert!(!app.skipping_dialogue);
        app.resume_pause();
        app.handle_key(KeyCode::KeyS);
        app.set_ui_hidden(true);
        assert!(!app.skipping_dialogue);
        let r = rects(&app.styles)[3];
        assert!(!app.quick_action_click(r.0 + 10, r.1 + 10));
        app.set_ui_hidden(false);
        for _ in 0..4 {
            app.handle_key(KeyCode::Tab);
        }
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.screen, Screen::GameMenu);
    }

    #[test]
    fn footer_hitboxes_stay_inside_dialogue_and_match_letterboxed_pointer() {
        let styles = load_menu_styles();
        let dialogue = dialogue_layout(&styles);
        for rect in rects(&styles) {
            assert!(dialogue_contains(dialogue, rect.0, rect.1));
            assert!(dialogue_contains(
                dialogue,
                rect.0 + rect.2 - 1,
                rect.1 + rect.3 - 1
            ));
            for (width, height) in [(1920, 1080), (1280, 800), (1000, 1000)] {
                let viewport = SoftwareViewport::letterbox(width, height, WIN_W, WIN_H).unwrap();
                let x = viewport.x as f64
                    + (rect.0 + rect.2 / 2) as f64 * viewport.width as f64 / WIN_W as f64;
                let y = viewport.y as f64
                    + (rect.1 + rect.3 / 2) as f64 * viewport.height as f64 / WIN_H as f64;
                let (vx, vy) = viewport.physical_to_virtual(x, y).unwrap();
                assert!(rect_contains(rect, vx as i32, vy as i32));
            }
        }
    }
}
