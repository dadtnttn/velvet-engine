//! # velvet-input
//!
//! Action-based input: keyboard/mouse/gamepad are mapped to named actions and axes.
//! Game code should query actions, never scancodes directly.

#![deny(missing_docs)]

mod action;
mod action_buffer;
mod axis;
mod binding;
mod chord;
mod context;
mod deadzone;
mod gesture;
mod plugin;
mod replay;
mod state;
mod virtual_controls;

#[cfg(feature = "winit")]
pub mod winit_map;

pub mod prelude;

pub use action::{ActionId, ActionMap, ActionState, ActionValue};
pub use action_buffer::{
    match_first_special, ActionBuffer, ActionBufferFrame, MotionStep, SpecialMove,
};
pub use axis::{Axis1d, Axis2d};
pub use binding::{Binding, KeyCode, MouseButton, VirtualKey};
pub use chord::{ChordDetector, KeyChord};
pub use context::{InputContext, InputContextId};
pub use deadzone::{
    apply_deadzone, snap_8way, snap_cardinal, DeadzoneConfig, DeadzoneShape, StickFilter,
    StickPipeline,
};
pub use gesture::{
    PointerId, PointerSample, SwipeConfig, SwipeDetector, SwipeDirection, SwipeGesture, TapDetector,
};
pub use plugin::InputPlugin;
pub use replay::{ActionFrame, InputPlayback, InputRecorder};
pub use state::InputState;
pub use virtual_controls::{VirtualButton, VirtualControls, VirtualStick};

/// Built-in action names recommended by the engine.
pub mod builtin {
    /// Confirm / accept.
    pub const CONFIRM: &str = "confirm";
    /// Cancel / back.
    pub const CANCEL: &str = "cancel";
    /// Move up.
    pub const MOVE_UP: &str = "move_up";
    /// Move down.
    pub const MOVE_DOWN: &str = "move_down";
    /// Move left.
    pub const MOVE_LEFT: &str = "move_left";
    /// Move right.
    pub const MOVE_RIGHT: &str = "move_right";
    /// Primary attack.
    pub const ATTACK: &str = "attack";
    /// Interact.
    pub const INTERACT: &str = "interact";
    /// Open menu.
    pub const OPEN_MENU: &str = "open_menu";
    /// Skip dialogue.
    pub const SKIP_DIALOGUE: &str = "skip_dialogue";
    /// Toggle auto dialogue.
    pub const AUTO_DIALOGUE: &str = "auto_dialogue";
    /// Composite move axis2.
    pub const MOVE: &str = "move";
}
