//! Keys that only exist on some platforms: Android's back button and gamepad buttons.
//!
//! winit 0.30 on Android delivers the back button as a `KeyboardInput` whose physical key is
//! `Unidentified(NativeKeyCode::Android(4))` and whose logical key is `BrowserBack`, so a game
//! that matches `KeyCode::Escape` never sees it: check [`is_back_key`] (or use
//! [`crate::Lifecycle`], which turns it into [`crate::PlatformEvent::Back`]).
//!
//! Gamepads: gilrs has no Android backend (it builds, and finds no gamepads). On Android a
//! gamepad's buttons arrive as key events with Android key codes ([`gamepad_button`] names them)
//! and its D-pad as the arrow keys; winit 0.30 does not forward analog sticks or triggers.

use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey, NativeKeyCode, PhysicalKey};

/// Android's `KeyEvent.KEYCODE_BACK`.
pub const ANDROID_KEYCODE_BACK: u32 = 4;

/// The Android key code of a key event, when winit had no name for it.
pub fn android_keycode(event: &KeyEvent) -> Option<u32> {
    match event.physical_key {
        PhysicalKey::Unidentified(NativeKeyCode::Android(code)) => Some(code),
        _ => None,
    }
}

/// Whether the key event is the back button (Android's back key or gesture, or a keyboard's
/// "browser back" key).
pub fn is_back_key(event: &KeyEvent) -> bool {
    android_keycode(event) == Some(ANDROID_KEYCODE_BACK)
        || event.logical_key == Key::Named(NamedKey::BrowserBack)
}

/// A gamepad button, named by position (Xbox layout: South is A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    /// Bottom face button (A on Xbox pads, cross on PlayStation).
    South,
    /// Right face button (B, circle).
    East,
    /// Left face button (X, square).
    West,
    /// Top face button (Y, triangle).
    North,
    /// Extra face button C (some pads).
    C,
    /// Extra face button Z (some pads).
    Z,
    /// Left shoulder (L1).
    LeftShoulder,
    /// Right shoulder (R1).
    RightShoulder,
    /// Left trigger pressed as a button (L2).
    LeftTrigger,
    /// Right trigger pressed as a button (R2).
    RightTrigger,
    /// Left stick pressed (L3).
    LeftStick,
    /// Right stick pressed (R3).
    RightStick,
    /// Start / menu.
    Start,
    /// Select / back / view.
    Select,
    /// The pad's home or mode button.
    Mode,
}

impl GamepadButton {
    /// The button an Android key code stands for (`KEYCODE_BUTTON_*`).
    pub fn from_android_keycode(code: u32) -> Option<Self> {
        Some(match code {
            96 => Self::South,
            97 => Self::East,
            98 => Self::C,
            99 => Self::West,
            100 => Self::North,
            101 => Self::Z,
            102 => Self::LeftShoulder,
            103 => Self::RightShoulder,
            104 => Self::LeftTrigger,
            105 => Self::RightTrigger,
            106 => Self::LeftStick,
            107 => Self::RightStick,
            108 => Self::Start,
            109 => Self::Select,
            110 => Self::Mode,
            _ => return None,
        })
    }
}

/// The gamepad button of a key event (Android), if it is one.
pub fn gamepad_button(event: &KeyEvent) -> Option<GamepadButton> {
    android_keycode(event).and_then(GamepadButton::from_android_keycode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_gamepad_codes() {
        assert_eq!(
            GamepadButton::from_android_keycode(96),
            Some(GamepadButton::South)
        );
        assert_eq!(
            GamepadButton::from_android_keycode(108),
            Some(GamepadButton::Start)
        );
        assert_eq!(
            GamepadButton::from_android_keycode(ANDROID_KEYCODE_BACK),
            None
        );
        assert_eq!(GamepadButton::from_android_keycode(111), None);
    }
}
