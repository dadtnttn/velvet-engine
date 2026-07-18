//! Input prelude.

pub use crate::action::{ActionId, ActionMap, ActionState, ActionValue};
pub use crate::axis::{Axis1d, Axis2d};
pub use crate::binding::{Binding, KeyCode, MouseButton, VirtualKey};
pub use crate::builtin;
pub use crate::chord::{ChordDetector, KeyChord};
pub use crate::context::{InputContext, InputContextId};
pub use crate::plugin::InputPlugin;
pub use crate::replay::{InputPlayback, InputRecorder};
pub use crate::state::InputState;
pub use crate::virtual_controls::{VirtualButton, VirtualControls, VirtualStick};
