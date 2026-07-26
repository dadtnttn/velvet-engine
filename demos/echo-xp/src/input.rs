use std::time::{Duration, Instant};
use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default)]
pub struct MouseState {
    pub pos: Position,
    pub left_down: bool,
    pub left_pressed: bool,
    pub left_released: bool,
    pub double_clicked: bool,
    pub drag_start: Option<Position>,
    pub is_dragging: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<Position>,
}

impl MouseState {
    pub fn update_pos(&mut self, x: i32, y: i32) {
        self.pos = Position { x, y };
    }

    pub fn handle_button(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    self.left_down = true;
                    self.left_pressed = true;

                    // Double click check (within 300ms and 5 pixels)
                    let now = Instant::now();
                    if let (Some(last_time), Some(last_pos)) =
                        (self.last_click_time, self.last_click_pos)
                    {
                        if now.duration_since(last_time) < Duration::from_millis(300)
                            && (self.pos.x - last_pos.x).abs() <= 5
                            && (self.pos.y - last_pos.y).abs() <= 5
                        {
                            self.double_clicked = true;
                            self.last_click_time = None;
                            self.last_click_pos = None;
                            return;
                        }
                    }
                    self.last_click_time = Some(now);
                    self.last_click_pos = Some(self.pos);
                }
                ElementState::Released => {
                    self.left_down = false;
                    self.left_released = true;
                    self.is_dragging = false;
                    self.drag_start = None;
                }
            }
        }
    }

    pub fn end_frame(&mut self) {
        self.left_pressed = false;
        self.left_released = false;
        self.double_clicked = false;
    }
}

#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    pub escape: bool,
    pub enter: bool,
    pub tab: bool,
    pub space: bool,
    pub alt: bool,
    pub f4: bool,
    pub f11: bool,
    pub alt_f4: bool,
}

impl KeyboardState {
    pub fn handle_key(&mut self, code: KeyCode, state: ElementState) {
        let is_pressed = state == ElementState::Pressed;
        match code {
            KeyCode::Escape => self.escape = is_pressed,
            KeyCode::Enter | KeyCode::NumpadEnter => self.enter = is_pressed,
            KeyCode::Tab => self.tab = is_pressed,
            KeyCode::Space => self.space = is_pressed,
            KeyCode::AltLeft | KeyCode::AltRight => self.alt = is_pressed,
            KeyCode::F4 => {
                self.f4 = is_pressed;
                if self.alt && is_pressed {
                    self.alt_f4 = true;
                }
            }
            KeyCode::F11 => self.f11 = is_pressed,
            _ => {}
        }
    }

    pub fn end_frame(&mut self) {
        self.escape = false;
        self.enter = false;
        self.tab = false;
        self.space = false;
        self.alt_f4 = false;
        self.f11 = false;
    }
}
