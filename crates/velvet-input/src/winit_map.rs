//! Map winit key/mouse events into [`crate::InputState`].
//!
//! Enabled when the `winit` feature is on.

use winit::event::{ElementState, MouseButton as WinitMouseButton};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

use crate::binding::{KeyCode, MouseButton};
use crate::state::InputState;

/// Translate a winit physical key to Velvet [`KeyCode`].
pub fn map_key_code(key: WinitKeyCode) -> Option<KeyCode> {
    use WinitKeyCode::*;
    Some(match key {
        KeyA => KeyCode::A,
        KeyB => KeyCode::B,
        KeyC => KeyCode::C,
        KeyD => KeyCode::D,
        KeyE => KeyCode::E,
        KeyF => KeyCode::F,
        KeyG => KeyCode::G,
        KeyH => KeyCode::H,
        KeyI => KeyCode::I,
        KeyJ => KeyCode::J,
        KeyK => KeyCode::K,
        KeyL => KeyCode::L,
        KeyM => KeyCode::M,
        KeyN => KeyCode::N,
        KeyO => KeyCode::O,
        KeyP => KeyCode::P,
        KeyQ => KeyCode::Q,
        KeyR => KeyCode::R,
        KeyS => KeyCode::S,
        KeyT => KeyCode::T,
        KeyU => KeyCode::U,
        KeyV => KeyCode::V,
        KeyW => KeyCode::W,
        KeyX => KeyCode::X,
        KeyY => KeyCode::Y,
        KeyZ => KeyCode::Z,
        Space => KeyCode::Space,
        Enter => KeyCode::Enter,
        Escape => KeyCode::Escape,
        ShiftLeft => KeyCode::ShiftLeft,
        ControlLeft => KeyCode::ControlLeft,
        AltLeft => KeyCode::AltLeft,
        Tab => KeyCode::Tab,
        ArrowUp => KeyCode::ArrowUp,
        ArrowDown => KeyCode::ArrowDown,
        ArrowLeft => KeyCode::ArrowLeft,
        ArrowRight => KeyCode::ArrowRight,
        Digit0 => KeyCode::Digit0,
        Digit1 => KeyCode::Digit1,
        Digit2 => KeyCode::Digit2,
        Digit3 => KeyCode::Digit3,
        Digit4 => KeyCode::Digit4,
        Digit5 => KeyCode::Digit5,
        Digit6 => KeyCode::Digit6,
        Digit7 => KeyCode::Digit7,
        Digit8 => KeyCode::Digit8,
        Digit9 => KeyCode::Digit9,
        F1 => KeyCode::F1,
        F5 => KeyCode::F5,
        F11 => KeyCode::F11,
        _ => return None,
    })
}

/// Map winit mouse button.
pub fn map_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Apply keyboard input to state.
pub fn apply_keyboard(state: &mut InputState, physical: PhysicalKey, element: ElementState) {
    let PhysicalKey::Code(code) = physical else {
        return;
    };
    let Some(key) = map_key_code(code) else {
        return;
    };
    match element {
        ElementState::Pressed => state.key_down(key),
        ElementState::Released => state.key_up(key),
    }
}

/// Apply mouse button.
pub fn apply_mouse_button(state: &mut InputState, button: WinitMouseButton, element: ElementState) {
    let Some(btn) = map_mouse_button(button) else {
        return;
    };
    match element {
        ElementState::Pressed => state.mouse_down(btn),
        ElementState::Released => state.mouse_up(btn),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_wasd() {
        assert_eq!(map_key_code(WinitKeyCode::KeyW), Some(KeyCode::W));
        assert_eq!(map_key_code(WinitKeyCode::Space), Some(KeyCode::Space));
    }
}
