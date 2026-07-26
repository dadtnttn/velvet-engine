use crate::assets::Assets;
use crate::desktop::DesktopShell;
use crate::input::Position;
use crate::model::{AppKind, Rect};
use crate::windows::{DesktopWindow, WindowManager};

pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

pub struct Renderer {
    pub buffer: Vec<u32>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            buffer: vec![0xFF000000; WIDTH * HEIGHT],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.buffer.fill(color);
    }

    pub fn draw_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && (x as usize) < WIDTH && y >= 0 && (y as usize) < HEIGHT {
            let alpha = (color >> 24) & 0xFF;
            if alpha == 255 {
                self.buffer[y as usize * WIDTH + x as usize] = color;
            } else if alpha > 0 {
                let bg = self.buffer[y as usize * WIDTH + x as usize];
                let bg_r = (bg >> 16) & 0xFF;
                let bg_g = (bg >> 8) & 0xFF;
                let bg_b = bg & 0xFF;

                let fg_r = (color >> 16) & 0xFF;
                let fg_g = (color >> 8) & 0xFF;
                let fg_b = color & 0xFF;

                let a = alpha;
                let inv_a = 255 - a;

                let r = (fg_r * a + bg_r * inv_a) / 255;
                let g = (fg_g * a + bg_g * inv_a) / 255;
                let b = (fg_b * a + bg_b * inv_a) / 255;

                self.buffer[y as usize * WIDTH + x as usize] =
                    0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: u32) {
        for y in rect.y..(rect.y + rect.h as i32) {
            for x in rect.x..(rect.x + rect.w as i32) {
                self.draw_pixel(x, y, color);
            }
        }
    }

    pub fn draw_rect_outline(&mut self, rect: Rect, color: u32) {
        for x in rect.x..(rect.x + rect.w as i32) {
            self.draw_pixel(x, rect.y, color);
            self.draw_pixel(x, rect.y + rect.h as i32 - 1, color);
        }
        for y in rect.y..(rect.y + rect.h as i32) {
            self.draw_pixel(rect.x, y, color);
            self.draw_pixel(rect.x + rect.w as i32 - 1, y, color);
        }
    }

    pub fn draw_texture(&mut self, assets: &Assets, key: &str, dest_x: i32, dest_y: i32) {
        if let Some(tex) = assets.get(key) {
            for y in 0..tex.height {
                for x in 0..tex.width {
                    let pixel = tex.get_pixel(x, y);
                    self.draw_pixel(dest_x + x as i32, dest_y + y as i32, pixel);
                }
            }
        }
    }

    pub fn draw_texture_scaled(
        &mut self,
        assets: &Assets,
        key: &str,
        dest_x: i32,
        dest_y: i32,
        target_w: u32,
        target_h: u32,
    ) {
        if let Some(tex) = assets.get(key) {
            for y in 0..target_h {
                let src_y = (y * tex.height) / target_h;
                for x in 0..target_w {
                    let src_x = (x * tex.width) / target_w;
                    let pixel = tex.get_pixel(src_x, src_y);
                    self.draw_pixel(dest_x + x as i32, dest_y + y as i32, pixel);
                }
            }
        }
    }

    pub fn draw_texture_region(
        &mut self,
        assets: &Assets,
        key: &str,
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        dest_x: i32,
        dest_y: i32,
    ) {
        if let Some(tex) = assets.get(key) {
            for y in 0..src_h {
                for x in 0..src_w {
                    let pixel = tex.get_pixel(src_x + x, src_y + y);
                    self.draw_pixel(dest_x + x as i32, dest_y + y as i32, pixel);
                }
            }
        }
    }

    pub fn draw_text(&mut self, text: &str, x: i32, y: i32, color: u32, scale: usize) {
        let mut cur_x = x;
        let mut cur_y = y;

        for ch in text.chars() {
            if ch == '\n' {
                cur_x = x;
                cur_y += (8 * scale) as i32;
                continue;
            }

            let glyph = get_glyph(ch);
            for (row, bits) in glyph.iter().copied().enumerate() {
                for col in 0..8 {
                    if (bits & (1 << (7 - col))) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                self.draw_pixel(
                                    cur_x + (col * scale + sx) as i32,
                                    cur_y + (row * scale + sy) as i32,
                                    color,
                                );
                            }
                        }
                    }
                }
            }
            cur_x += (6 * scale) as i32;
        }
    }

    pub fn draw_window_frame(
        &mut self,
        _assets: &Assets,
        win: &DesktopWindow,
        mouse_pos: Position,
    ) {
        let r = win.rect;
        let active = win.is_focused;

        // Window background / body
        self.fill_rect(r, 0xFFECE9D8); // Classic XP Dialog background color

        // Outer border
        let border_col = if active { 0xFF0055EA } else { 0xFF7F9DB9 };
        self.draw_rect_outline(r, border_col);

        // Titlebar (Luna blue gradient simulation)
        let titlebar_h = 24;
        for y in 0..titlebar_h {
            let factor = y as f32 / titlebar_h as f32;
            let col = if active {
                let r_c = (0.0 * (1.0 - factor) + 0.0 * factor) as u32;
                let g_c = (88.0 * (1.0 - factor) + 40.0 * factor) as u32;
                let b_c = (238.0 * (1.0 - factor) + 180.0 * factor) as u32;
                0xFF000000 | (r_c << 16) | (g_c << 8) | b_c
            } else {
                let r_c = (120.0 * (1.0 - factor) + 140.0 * factor) as u32;
                let g_c = (140.0 * (1.0 - factor) + 160.0 * factor) as u32;
                let b_c = (170.0 * (1.0 - factor) + 190.0 * factor) as u32;
                0xFF000000 | (r_c << 16) | (g_c << 8) | b_c
            };
            for x in (r.x + 1)..(r.x + r.w as i32 - 1) {
                self.draw_pixel(x, r.y + 1 + y, col);
            }
        }

        // Title text
        self.draw_text(&win.title, r.x + 8, r.y + 6, 0xFFFFFFFF, 1);

        // Close button (Red box with 'X')
        let close_btn_rect = Rect::new(r.x + r.w as i32 - 22, r.y + 4, 16, 16);
        let close_hover = close_btn_rect.contains(mouse_pos.x, mouse_pos.y);
        let close_col = if close_hover { 0xFFE04343 } else { 0xFFC75050 };
        self.fill_rect(close_btn_rect, close_col);
        self.draw_rect_outline(close_btn_rect, 0xFFFFFFFF);
        self.draw_text(
            "X",
            close_btn_rect.x + 5,
            close_btn_rect.y + 4,
            0xFFFFFFFF,
            1,
        );

        // Minimize button ('_')
        let min_btn_rect = Rect::new(r.x + r.w as i32 - 42, r.y + 4, 16, 16);
        let min_hover = min_btn_rect.contains(mouse_pos.x, mouse_pos.y);
        let min_col = if min_hover { 0xFF3C7FB1 } else { 0xFF0055EA };
        self.fill_rect(min_btn_rect, min_col);
        self.draw_rect_outline(min_btn_rect, 0xFFFFFFFF);
        self.draw_text("_", min_btn_rect.x + 5, min_btn_rect.y + 2, 0xFFFFFFFF, 1);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_texture_region_scaled(
        &mut self,
        assets: &Assets,
        key: &str,
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        dest_x: i32,
        dest_y: i32,
        target_w: u32,
        target_h: u32,
    ) {
        if let Some(tex) = assets.get(key) {
            for y in 0..target_h {
                let sy = src_y + (y * src_h) / target_h;
                for x in 0..target_w {
                    let sx = src_x + (x * src_w) / target_w;
                    let pixel = tex.get_pixel(sx, sy);
                    self.draw_pixel(dest_x + x as i32, dest_y + y as i32, pixel);
                }
            }
        }
    }

    pub fn draw_text_wrapped(
        &mut self,
        text: &str,
        x: i32,
        y: i32,
        max_width: u32,
        color: u32,
        scale: usize,
    ) {
        let char_w = (6 * scale) as i32;
        let line_h = (10 * scale) as i32;
        let max_chars_per_line = (max_width as i32 / char_w).max(1) as usize;

        let mut cur_y = y;
        for line in text.split('\n') {
            let words = line.split_whitespace();
            let mut current_line = String::new();

            for word in words {
                if current_line.is_empty() {
                    current_line.push_str(word);
                } else if current_line.len() + 1 + word.len() <= max_chars_per_line {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    self.draw_text(&current_line, x, cur_y, color, scale);
                    cur_y += line_h;
                    current_line = word.to_string();
                }
            }
            if !current_line.is_empty() {
                self.draw_text(&current_line, x, cur_y, color, scale);
                cur_y += line_h;
            }
        }
    }

    pub fn draw_clippy(&mut self, assets: &Assets, state: &str, dialog: &str, anim_time: f32) {
        let clippy_key = match state {
            "think" => "clip_think",
            "read" => "clip_read",
            "weird" => "clip_weird",
            "eyes" => "clip_eyes",
            "listen" => "clip_listen",
            "noted" => "clip_noted",
            _ => "clip_idle",
        };

        let target_size = 100;
        let dest_x = 680;
        let dest_y = 455;

        // Draw animated frame from Clippy sheet
        if let Some(tex) = assets.get(clippy_key) {
            let frame_dim = 240;
            let total_frames = (tex.width / frame_dim).max(1);
            let fps = 15.0;
            let current_frame = ((anim_time * fps) as u32) % total_frames;
            let src_x = current_frame * frame_dim;

            self.draw_texture_region_scaled(
                assets,
                clippy_key,
                src_x,
                0,
                frame_dim,
                frame_dim,
                dest_x,
                dest_y,
                target_size,
                target_size,
            );
        }

        // Draw speech bubble with text wrapping
        if !dialog.is_empty() {
            let bubble_rect = Rect::new(410, 440, 260, 90);
            self.fill_rect(bubble_rect, 0xFFFFFFE1); // Light yellow tooltip background
            self.draw_rect_outline(bubble_rect, 0xFF000000);
            self.draw_text_wrapped(
                dialog,
                bubble_rect.x + 8,
                bubble_rect.y + 8,
                bubble_rect.w - 16,
                0xFF000000,
                1,
            );
        }
    }

    pub fn draw_taskbar(
        &mut self,
        assets: &Assets,
        wm: &WindowManager,
        shell: &DesktopShell,
        _operator_name: &str,
        mouse_pos: Position,
    ) {
        // 1. Windows 7 Aero Glass Taskbar (y=560..600, height 40px)
        for y in 0..40 {
            let py = 560 + y;
            let col = if y == 0 {
                0xFF85C2FF // Top translucent glass shine line
            } else if y == 1 {
                0xBB5096E0
            } else if y < 20 {
                let factor = (y - 1) as f32 / 19.0;
                let r = (15.0 * (1.0 - factor) + 25.0 * factor) as u32;
                let g = (45.0 * (1.0 - factor) + 75.0 * factor) as u32;
                let b = (95.0 * (1.0 - factor) + 145.0 * factor) as u32;
                0xD0000000 | (r << 16) | (g << 8) | b
            } else {
                let factor = (y - 20) as f32 / 20.0;
                let r = (20.0 * (1.0 - factor) + 10.0 * factor) as u32;
                let g = (65.0 * (1.0 - factor) + 35.0 * factor) as u32;
                let b = (135.0 * (1.0 - factor) + 85.0 * factor) as u32;
                0xE0000000 | (r << 16) | (g << 8) | b
            };

            for x in 0..800 {
                self.draw_pixel(x, py, col);
            }
        }

        // 2. Windows 7 Start Orb (48x48 glowing orb at x=4, y=552 overlapping top taskbar edge)
        let orb_rect = Rect::new(4, 552, 48, 48);
        let orb_hover = orb_rect.contains(mouse_pos.x, mouse_pos.y);
        let orb_state_idx = if shell.start_menu_open {
            2 // Pressed
        } else if orb_hover {
            1 // Hover glow
        } else {
            0 // Normal
        };

        if assets.get("win7_start_orb").is_some() {
            self.draw_texture_region_scaled(
                assets,
                "win7_start_orb",
                orb_state_idx * 48,
                0,
                48,
                48,
                4,
                552,
                48,
                48,
            );
        } else {
            // Fallback orb
            let orb_col = if orb_hover { 0xFF00A0FF } else { 0xFF1565C0 };
            self.fill_rect(Rect::new(8, 556, 40, 40), orb_col);
        }

        // 3. Taskbar Window Buttons (Windows 7 Glass Taskbar Icons / Buttons)
        let mut btn_x = 60;
        for win in &wm.windows {
            let active = win.is_focused && !win.is_minimized;
            let btn_rect = Rect::new(btn_x, 564, 120, 32);
            let hover = btn_rect.contains(mouse_pos.x, mouse_pos.y);

            let bg_col = if active {
                0xF020529C
            } else if hover {
                0xA04080DC
            } else {
                0x70204080
            };
            self.fill_rect(btn_rect, bg_col);
            self.draw_rect_outline(btn_rect, 0xA080C0FF);

            // Icon
            let (icon_col, icon_row) = match win.app {
                AppKind::Inbox => (0, 0),
                AppKind::CaseFiles => (1, 0),
                AppKind::PhotoViewer => (2, 0),
                AppKind::TapePlayer => (3, 0),
                AppKind::Classifier => (4, 0),
                AppKind::RecycleBin => (5, 0),
                _ => (0, 0),
            };
            self.draw_texture_region_scaled(
                assets,
                "winicons_48",
                icon_col * 48,
                icon_row * 48,
                48,
                48,
                btn_x + 6,
                572,
                16,
                16,
            );

            let short_title = if win.title.len() > 11 {
                format!("{}..", &win.title[..9])
            } else {
                win.title.clone()
            };
            self.draw_text(&short_title, btn_x + 26, 576, 0xFFFFFFFF, 1);
            btn_x += 126;
        }

        // 4. System Tray / Clock (Windows 7 Aero Glass Tray)
        let tray_rect = Rect::new(710, 560, 90, 40);
        self.fill_rect(tray_rect, 0x60000000);
        self.draw_rect_outline(tray_rect, 0x50FFFFFF);

        // Clock Time ("5:44 AM")
        self.draw_text("5:44 AM", 731, 576, 0xFF001030, 1); // Shadow
        self.draw_text("5:44 AM", 730, 575, 0xFFFFFFFF, 1); // White text

        // Aero Peek rectangle at extreme right (x=790..800)
        self.fill_rect(Rect::new(792, 562, 7, 36), 0x40FFFFFF);
        self.draw_rect_outline(Rect::new(792, 562, 7, 36), 0x80FFFFFF);
    }

    pub fn draw_start_menu(&mut self, assets: &Assets, operator_name: &str, mouse_pos: Position) {
        let menu_rect = Rect::new(0, 180, 280, 380);

        // Windows 7 Start Menu background
        self.fill_rect(Rect::new(0, 180, 170, 340), 0xF5F8FCFF); // Left column (White/Soft blue)
        self.fill_rect(Rect::new(170, 180, 110, 340), 0xE8D5E6F8); // Right column (Translucent Ice Blue)
        self.draw_rect_outline(menu_rect, 0xFF3060B0);

        // Top user profile header banner (Deep Aero Blue)
        for y in 0..40 {
            let py = 180 + y;
            let factor = y as f32 / 40.0;
            let r = (15.0 * (1.0 - factor) + 30.0 * factor) as u32;
            let g = (55.0 * (1.0 - factor) + 95.0 * factor) as u32;
            let b = (145.0 * (1.0 - factor) + 195.0 * factor) as u32;
            let col = 0xF0000000 | (r << 16) | (g << 8) | b;
            for x in 0..280 {
                self.draw_pixel(x, py, col);
            }
        }

        // Classic XP Windows Flag icon & Operator Name
        self.draw_texture_scaled(assets, "windows_logo_small", 10, 186, 28, 28);
        self.draw_text(operator_name, 48, 195, 0xFF001030, 2); // Shadow
        self.draw_text(operator_name, 47, 194, 0xFFFFFFFF, 2); // White text

        // Left Column Items (Programs List)
        let items = [
            ("Inbox", AppKind::Inbox, 0),
            ("Case Files", AppKind::CaseFiles, 1),
            ("Evidence Photo", AppKind::PhotoViewer, 2),
            ("Tape Player", AppKind::TapePlayer, 3),
            ("Classifier", AppKind::Classifier, 4),
            ("Recycle Bin", AppKind::RecycleBin, 5),
        ];

        let mut item_y = 226;
        for (label, _app, icon_idx) in items {
            let item_rect = Rect::new(4, item_y, 162, 36);
            if item_rect.contains(mouse_pos.x, mouse_pos.y) {
                self.fill_rect(item_rect, 0xFF3D82E8);
                self.draw_texture_region_scaled(
                    assets,
                    "winicons_48",
                    icon_idx * 48,
                    0,
                    48,
                    48,
                    10,
                    item_y + 6,
                    24,
                    24,
                );
                self.draw_text(label, 40, item_y + 14, 0xFFFFFFFF, 1);
            } else {
                self.draw_texture_region_scaled(
                    assets,
                    "winicons_48",
                    icon_idx * 48,
                    0,
                    48,
                    48,
                    10,
                    item_y + 6,
                    24,
                    24,
                );
                self.draw_text(label, 40, item_y + 14, 0xFF102040, 1);
            }
            item_y += 40;
        }

        // Right Column System Shortcuts
        let shortcuts = [
            "Documents",
            "Pictures",
            "Music",
            "Computer",
            "Control Panel",
            "Shut Down",
        ];
        let mut sc_y = 232;
        for sc in shortcuts {
            let sc_rect = Rect::new(172, sc_y, 104, 28);
            if sc_rect.contains(mouse_pos.x, mouse_pos.y) {
                self.fill_rect(sc_rect, 0x403D82E8);
                self.draw_text(sc, 178, sc_y + 10, 0xFF1040A0, 1);
            } else {
                self.draw_text(sc, 178, sc_y + 10, 0xFF102040, 1);
            }
            sc_y += 38;
        }

        // Bottom Windows 7 Search Bar ("Search programs and files")
        let search_rect = Rect::new(6, 524, 268, 30);
        self.fill_rect(search_rect, 0xFFFFFFFF);
        self.draw_rect_outline(search_rect, 0xFF7095CD);
        self.draw_text("Search programs and files", 14, 534, 0xFF90A0B0, 1);
        self.draw_text("Q", 256, 534, 0xFF7095CD, 1); // Magnifying glass icon
    }

    pub fn apply_corruption(&mut self, corruption: f32) {
        if corruption <= 0.001 {
            return;
        }

        let amount = (corruption * 20.0) as usize;
        for y in (0..HEIGHT).step_by(4) {
            let shift = (y * 7) % (amount.max(1));
            for x in 0..(WIDTH - shift) {
                let pixel = self.buffer[y * WIDTH + x + shift];
                // RGB static noise/shift
                let r = ((pixel >> 16) & 0xFF) ^ 0x33;
                let g = (pixel >> 8) & 0xFF;
                let b = pixel & 0xFF;
                self.buffer[y * WIDTH + x] = 0xFF000000 | (r << 16) | (g << 8) | b;
            }
        }
    }

    pub fn draw_cursor(&mut self, assets: &Assets, pos: Position, cursor_type: &str) {
        let key = match cursor_type {
            "link" => "cursor_link",
            "ibeam" => "cursor_ibeam",
            "wait" => "cursor_wait",
            "move" => "cursor_move",
            _ => "cursor_arrow",
        };

        self.draw_texture(assets, key, pos.x, pos.y);
    }
}

// 8x8 Pixel font character bitmaps
fn get_glyph(ch: char) -> [u8; 8] {
    match ch {
        'A' => [0x18, 0x24, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x00],
        'B' => [0x7C, 0x42, 0x42, 0x7C, 0x42, 0x42, 0x7C, 0x00],
        'C' => [0x3C, 0x42, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00],
        'D' => [0x78, 0x44, 0x42, 0x42, 0x42, 0x44, 0x78, 0x00],
        'E' => [0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x7E, 0x00],
        'F' => [0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40, 0x00],
        'G' => [0x3C, 0x42, 0x40, 0x4E, 0x42, 0x42, 0x3C, 0x00],
        'H' => [0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x00],
        'I' => [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        'J' => [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x4C, 0x38, 0x00],
        'K' => [0x42, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0x00],
        'L' => [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00],
        'M' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        'N' => [0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x00],
        'O' => [0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00],
        'P' => [0x7C, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x00],
        'Q' => [0x3C, 0x42, 0x42, 0x42, 0x4A, 0x44, 0x3A, 0x00],
        'R' => [0x7C, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x00],
        'S' => [0x3C, 0x42, 0x40, 0x3C, 0x02, 0x42, 0x3C, 0x00],
        'T' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        'U' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00],
        'V' => [0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00],
        'W' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        'X' => [0x42, 0x24, 0x18, 0x18, 0x18, 0x24, 0x42, 0x00],
        'Y' => [0x42, 0x42, 0x24, 0x18, 0x18, 0x18, 0x18, 0x00],
        'Z' => [0x7E, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7E, 0x00],
        'a' => [0x00, 0x00, 0x3C, 0x02, 0x3E, 0x46, 0x3A, 0x00],
        'b' => [0x40, 0x40, 0x5C, 0x62, 0x62, 0x62, 0x5C, 0x00],
        'c' => [0x00, 0x00, 0x3C, 0x40, 0x40, 0x40, 0x3C, 0x00],
        'd' => [0x02, 0x02, 0x3A, 0x46, 0x46, 0x46, 0x3A, 0x00],
        'e' => [0x00, 0x00, 0x3C, 0x42, 0x7E, 0x40, 0x3C, 0x00],
        'f' => [0x0C, 0x12, 0x10, 0x38, 0x10, 0x10, 0x10, 0x00],
        'g' => [0x00, 0x00, 0x3A, 0x46, 0x46, 0x3E, 0x02, 0x3C],
        'h' => [0x40, 0x40, 0x5C, 0x62, 0x62, 0x62, 0x62, 0x00],
        'i' => [0x18, 0x00, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        'j' => [0x06, 0x00, 0x06, 0x06, 0x06, 0x06, 0x46, 0x3C],
        'k' => [0x40, 0x40, 0x46, 0x4C, 0x50, 0x4C, 0x46, 0x00],
        'l' => [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        'm' => [0x00, 0x00, 0x6C, 0x92, 0x92, 0x92, 0x92, 0x00],
        'n' => [0x00, 0x00, 0x5C, 0x62, 0x62, 0x62, 0x62, 0x00],
        'o' => [0x00, 0x00, 0x3C, 0x42, 0x42, 0x42, 0x3C, 0x00],
        'p' => [0x00, 0x00, 0x5C, 0x62, 0x62, 0x7C, 0x40, 0x40],
        'q' => [0x00, 0x00, 0x3A, 0x46, 0x46, 0x3E, 0x02, 0x02],
        'r' => [0x00, 0x00, 0x54, 0x58, 0x50, 0x50, 0x50, 0x00],
        's' => [0x00, 0x00, 0x3E, 0x40, 0x3C, 0x02, 0x7C, 0x00],
        't' => [0x10, 0x10, 0x38, 0x10, 0x10, 0x12, 0x0C, 0x00],
        'u' => [0x00, 0x00, 0x42, 0x42, 0x42, 0x46, 0x3A, 0x00],
        'v' => [0x00, 0x00, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00],
        'w' => [0x00, 0x00, 0x42, 0x42, 0x5A, 0x66, 0x42, 0x00],
        'x' => [0x00, 0x00, 0x42, 0x24, 0x18, 0x24, 0x42, 0x00],
        'y' => [0x00, 0x00, 0x42, 0x42, 0x3E, 0x02, 0x3C, 0x00],
        'z' => [0x00, 0x00, 0x7E, 0x08, 0x10, 0x20, 0x7E, 0x00],
        '0' => [0x3C, 0x46, 0x4A, 0x52, 0x62, 0x3C, 0x00, 0x00],
        '1' => [0x18, 0x28, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00],
        '2' => [0x3C, 0x42, 0x04, 0x18, 0x20, 0x7E, 0x00, 0x00],
        '3' => [0x3C, 0x42, 0x0C, 0x02, 0x42, 0x3C, 0x00, 0x00],
        '4' => [0x08, 0x18, 0x28, 0x48, 0x7E, 0x08, 0x00, 0x00],
        '5' => [0x7E, 0x40, 0x7C, 0x02, 0x42, 0x3C, 0x00, 0x00],
        '6' => [0x3C, 0x40, 0x7C, 0x42, 0x42, 0x3C, 0x00, 0x00],
        '7' => [0x7E, 0x02, 0x04, 0x08, 0x10, 0x10, 0x00, 0x00],
        '8' => [0x3C, 0x42, 0x3C, 0x42, 0x42, 0x3C, 0x00, 0x00],
        '9' => [0x3C, 0x42, 0x3E, 0x02, 0x02, 0x3C, 0x00, 0x00],
        ':' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00],
        '/' => [0x00, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00],
        '?' => [0x3C, 0x42, 0x04, 0x08, 0x00, 0x08, 0x00, 0x00],
        '!' => [0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00, 0x00],
        '[' => [0x3C, 0x20, 0x20, 0x20, 0x20, 0x20, 0x3C, 0x00],
        ']' => [0x3C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x3C, 0x00],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    }
}
