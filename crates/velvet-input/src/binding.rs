//! Physical bindings mapped to actions.

use serde::{Deserialize, Serialize};

use crate::action::ActionId;

/// Logical key codes (device-independent names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCode {
    /// A
    A,
    /// B
    B,
    /// C
    C,
    /// D
    D,
    /// E
    E,
    /// F
    F,
    /// G
    G,
    /// H
    H,
    /// I
    I,
    /// J
    J,
    /// K
    K,
    /// L
    L,
    /// M
    M,
    /// N
    N,
    /// O
    O,
    /// P
    P,
    /// Q
    Q,
    /// R
    R,
    /// S
    S,
    /// T
    T,
    /// U
    U,
    /// V
    V,
    /// W
    W,
    /// X
    X,
    /// Y
    Y,
    /// Z
    Z,
    /// Space
    Space,
    /// Enter
    Enter,
    /// Escape
    Escape,
    /// Shift left
    ShiftLeft,
    /// Control left
    ControlLeft,
    /// Alt left
    AltLeft,
    /// Tab
    Tab,
    /// Arrow up
    ArrowUp,
    /// Arrow down
    ArrowDown,
    /// Arrow left
    ArrowLeft,
    /// Arrow right
    ArrowRight,
    /// Digit 0
    Digit0,
    /// Digit 1
    Digit1,
    /// Digit 2
    Digit2,
    /// Digit 3
    Digit3,
    /// Digit 4
    Digit4,
    /// Digit 5
    Digit5,
    /// Digit 6
    Digit6,
    /// Digit 7
    Digit7,
    /// Digit 8
    Digit8,
    /// Digit 9
    Digit9,
    /// F1
    F1,
    /// F5
    F5,
    /// F11
    F11,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    /// Left
    Left,
    /// Right
    Right,
    /// Middle
    Middle,
}

/// Virtual control (keyboard/mouse/gamepad abstract).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VirtualKey {
    /// Keyboard key.
    Key {
        /// Key code.
        key: KeyCode,
    },
    /// Mouse button.
    Mouse {
        /// Button.
        button: MouseButton,
    },
    /// Gamepad button index (backend-defined).
    GamepadButton {
        /// Button index.
        index: u32,
    },
    /// Gamepad axis contributing positively or negatively.
    GamepadAxis {
        /// Axis index.
        index: u32,
        /// Sign: +1 or -1 contribution.
        sign: i8,
    },
}

/// Binding of a virtual key to an action with optional scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// Target action.
    pub action: ActionId,
    /// Physical control.
    pub control: VirtualKey,
    /// Scale for analog contribution.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// For composite 2D: which component this binding feeds (`x` or `y`).
    #[serde(default)]
    pub axis_component: Option<AxisComponent>,
}

fn default_scale() -> f32 {
    1.0
}

/// Which axis component a digital binding contributes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisComponent {
    /// X
    X,
    /// Y
    Y,
}

impl Binding {
    /// Key binding helper.
    pub fn key(action: impl Into<ActionId>, key: KeyCode) -> Self {
        Self {
            action: action.into(),
            control: VirtualKey::Key { key },
            scale: 1.0,
            axis_component: None,
        }
    }

    /// Key binding that feeds a move axis component.
    pub fn key_axis(
        action: impl Into<ActionId>,
        key: KeyCode,
        component: AxisComponent,
        scale: f32,
    ) -> Self {
        Self {
            action: action.into(),
            control: VirtualKey::Key { key },
            scale,
            axis_component: Some(component),
        }
    }
}
