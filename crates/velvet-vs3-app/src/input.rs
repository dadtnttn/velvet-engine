use crate::script::InputFrame;
use std::collections::HashSet;
use winit::keyboard::KeyCode;
#[derive(Default)]
pub struct InputState {
    held: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
    pointer: Option<[f64; 2]>,
    mouse_held: bool,
    mouse_pressed: bool,
    mouse_released: bool,
    wheel: f64,
}
impl InputState {
    pub fn press(&mut self, key: KeyCode, repeat: bool) {
        self.held.insert(key);
        if !repeat {
            self.pressed.insert(key);
        }
    }
    pub fn release(&mut self, key: KeyCode) {
        self.held.remove(&key);
        self.released.insert(key);
    }
    pub fn pointer(&mut self, p: Option<[f64; 2]>) {
        self.pointer = p;
    }
    pub fn mouse(&mut self, pressed: bool) {
        self.mouse_held = pressed;
        if pressed {
            self.mouse_pressed = true
        } else {
            self.mouse_released = true
        }
    }
    pub fn wheel(&mut self, delta: f64) {
        self.wheel += delta;
    }
    pub fn take_frame(&mut self) -> InputFrame {
        InputFrame {
            held: self.held.iter().map(key_name).collect(),
            pressed: self.pressed.drain().map(|k| key_name(&k)).collect(),
            released: self.released.drain().map(|k| key_name(&k)).collect(),
            pointer: self.pointer,
            mouse_held: self.mouse_held,
            mouse_pressed: take(&mut self.mouse_pressed),
            mouse_released: take(&mut self.mouse_released),
            wheel: std::mem::take(&mut self.wheel),
        }
    }
}
fn key_name(key: &KeyCode) -> String {
    format!("{key:?}")
}
fn take(value: &mut bool) -> bool {
    let out = *value;
    *value = false;
    out
}
