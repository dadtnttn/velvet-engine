use velvet_input::winit_map::{apply_keyboard, apply_mouse_button};
use velvet_input::{builtin, DeadzoneConfig, InputState, KeyCode};
use velvet_math::Vec2 as EngineVec2;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::PhysicalKey;

use crate::model::{PlayerView, Vec2};
use crate::render::{HEIGHT, WIDTH};

#[derive(Debug, Clone, Copy, Default)]
pub struct UiInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub confirm: bool,
    pub cancel: bool,
    pub pause: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GameplayInput {
    pub movement: Vec2,
    pub aim: Vec2,
    pub fire: bool,
    pub dash: bool,
    pub interact: bool,
    pub reload: bool,
    pub weapon: i64,
    pub pause: bool,
}

impl Default for GameplayInput {
    fn default() -> Self {
        Self {
            movement: Vec2::default(),
            aim: Vec2 { x: 1.0, y: 0.0 },
            fire: false,
            dash: false,
            interact: false,
            reload: false,
            weapon: 0,
            pause: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PadState {
    move_x: f32,
    move_y: f32,
    aim_x: f32,
    aim_y: f32,
    fire: bool,
    dash: bool,
    interact: bool,
    reload: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    start: bool,
    cancel: bool,
}

pub struct Controls {
    state: InputState,
    mouse_left: bool,
    mouse_right_pressed: bool,
    cursor: (f32, f32),
    cursor_moved: bool,
    pad: PadState,
    previous_pad: PadState,
    #[cfg(windows)]
    gilrs: Option<gilrs::Gilrs>,
}

impl Controls {
    pub fn new() -> Self {
        let mut state = InputState::with_defaults();
        state.begin_frame();
        Self {
            state,
            mouse_left: false,
            mouse_right_pressed: false,
            cursor: (WIDTH as f32 * 0.5, HEIGHT as f32 * 0.5),
            cursor_moved: false,
            pad: PadState::default(),
            previous_pad: PadState::default(),
            #[cfg(windows)]
            gilrs: gilrs::Gilrs::new().ok(),
        }
    }

    pub fn keyboard(&mut self, physical: PhysicalKey, element: ElementState) {
        apply_keyboard(&mut self.state, physical, element);
    }

    pub fn mouse_button(&mut self, button: MouseButton, element: ElementState) {
        apply_mouse_button(&mut self.state, button, element);
        if button == MouseButton::Left {
            self.mouse_left = element == ElementState::Pressed;
        }
        if button == MouseButton::Right && element == ElementState::Pressed {
            self.mouse_right_pressed = true;
        }
    }

    pub fn cursor(&mut self, x: f64, y: f64) {
        self.cursor = (x as f32, y as f32);
        self.cursor_moved = true;
        self.state.set_cursor(x as f32, y as f32);
    }

    pub fn poll(&mut self) {
        self.previous_pad = self.pad;
        self.pad = PadState::default();
        #[cfg(windows)]
        if let Some(gilrs) = &mut self.gilrs {
            while gilrs.next_event().is_some() {}
            if let Some((_, gamepad)) = gilrs.gamepads().next() {
                use gilrs::{Axis, Button};
                self.pad.move_x = gamepad.value(Axis::LeftStickX);
                self.pad.move_y = gamepad.value(Axis::LeftStickY);
                self.pad.aim_x = gamepad.value(Axis::RightStickX);
                self.pad.aim_y = gamepad.value(Axis::RightStickY);
                self.pad.fire =
                    gamepad.is_pressed(Button::RightTrigger2) || gamepad.is_pressed(Button::West);
                self.pad.dash =
                    gamepad.is_pressed(Button::LeftTrigger) || gamepad.is_pressed(Button::East);
                self.pad.interact = gamepad.is_pressed(Button::South);
                self.pad.reload = gamepad.is_pressed(Button::North);
                self.pad.up = gamepad.is_pressed(Button::DPadUp);
                self.pad.down = gamepad.is_pressed(Button::DPadDown);
                self.pad.left = gamepad.is_pressed(Button::DPadLeft);
                self.pad.right = gamepad.is_pressed(Button::DPadRight);
                self.pad.start = gamepad.is_pressed(Button::Start);
                self.pad.cancel = gamepad.is_pressed(Button::East);
            }
        }
        self.state.end_frame();
    }

    pub fn ui(&self) -> UiInput {
        UiInput {
            up: self.state.key_just_pressed(KeyCode::W)
                || self.state.key_just_pressed(KeyCode::ArrowUp)
                || rising(self.pad.up, self.previous_pad.up),
            down: self.state.key_just_pressed(KeyCode::S)
                || self.state.key_just_pressed(KeyCode::ArrowDown)
                || rising(self.pad.down, self.previous_pad.down),
            left: self.state.key_just_pressed(KeyCode::A)
                || self.state.key_just_pressed(KeyCode::ArrowLeft)
                || rising(self.pad.left, self.previous_pad.left),
            right: self.state.key_just_pressed(KeyCode::D)
                || self.state.key_just_pressed(KeyCode::ArrowRight)
                || rising(self.pad.right, self.previous_pad.right),
            confirm: self.state.key_just_pressed(KeyCode::Enter)
                || self.state.key_just_pressed(KeyCode::Space)
                || rising(self.pad.interact, self.previous_pad.interact),
            cancel: self.state.key_just_pressed(KeyCode::Escape)
                || rising(self.pad.cancel, self.previous_pad.cancel),
            pause: rising(self.pad.start, self.previous_pad.start),
        }
    }

    pub fn gameplay(&mut self, player: &PlayerView, window: PhysicalSize<u32>) -> GameplayInput {
        let keyboard = self.state.axis2(builtin::MOVE);
        let pad_move =
            DeadzoneConfig::gamepad().apply(EngineVec2::new(self.pad.move_x, self.pad.move_y));
        let movement = if pad_move.length() > 0.05 {
            Vec2 {
                x: pad_move.x,
                y: -pad_move.y,
            }
        } else {
            Vec2 {
                x: keyboard.x,
                y: -keyboard.y,
            }
        };

        let pad_aim = DeadzoneConfig::aim().apply(EngineVec2::new(self.pad.aim_x, self.pad.aim_y));
        let aim = if pad_aim.length() > 0.08 {
            self.cursor_moved = false;
            Vec2 {
                x: pad_aim.x,
                y: -pad_aim.y,
            }
        } else if self.cursor_moved {
            let (x, y) = cursor_to_design(self.cursor, window);
            Vec2 {
                x: x / 1.5 - player.pos.x,
                y: y / 1.5 - player.pos.y,
            }
        } else {
            player.aim
        };

        let weapon = if self.state.key_just_pressed(KeyCode::Digit1)
            || rising(self.pad.left, self.previous_pad.left)
        {
            1
        } else if self.state.key_just_pressed(KeyCode::Digit2)
            || rising(self.pad.down, self.previous_pad.down)
        {
            2
        } else if self.state.key_just_pressed(KeyCode::Digit3)
            || rising(self.pad.right, self.previous_pad.right)
        {
            3
        } else {
            0
        };

        GameplayInput {
            movement,
            aim,
            fire: self.mouse_left || self.pad.fire,
            dash: self.state.key_just_pressed(KeyCode::ShiftLeft)
                || self.state.key_just_pressed(KeyCode::Space)
                || self.mouse_right_pressed
                || rising(self.pad.dash, self.previous_pad.dash),
            interact: self.state.key_just_pressed(KeyCode::E)
                || rising(self.pad.interact, self.previous_pad.interact),
            reload: self.state.key_just_pressed(KeyCode::R)
                || rising(self.pad.reload, self.previous_pad.reload),
            weapon,
            pause: self.state.key_just_pressed(KeyCode::Escape)
                || rising(self.pad.start, self.previous_pad.start),
        }
    }

    pub fn finish_frame(&mut self) {
        self.mouse_right_pressed = false;
        self.state.begin_frame();
    }
}

fn rising(current: bool, previous: bool) -> bool {
    current && !previous
}

fn cursor_to_design(cursor: (f32, f32), window: PhysicalSize<u32>) -> (f32, f32) {
    let scale = (window.width as f32 / WIDTH as f32).min(window.height as f32 / HEIGHT as f32);
    let used_w = WIDTH as f32 * scale;
    let used_h = HEIGHT as f32 * scale;
    let offset_x = (window.width as f32 - used_w) * 0.5;
    let offset_y = (window.height as f32 - used_h) * 0.5;
    (
        ((cursor.0 - offset_x) / scale).clamp(0.0, WIDTH as f32),
        ((cursor.1 - offset_y) / scale).clamp(0.0, HEIGHT as f32),
    )
}
