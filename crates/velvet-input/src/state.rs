//! Runtime input state and action resolution.

use std::collections::{HashMap, HashSet};

use crate::action::{ActionId, ActionState, ActionValue};
use crate::axis::Axis2d;
use crate::binding::{AxisComponent, Binding, KeyCode, MouseButton, VirtualKey};
use crate::builtin;
use crate::context::{ContextStack, InputContext};

/// Full input manager resource.
#[derive(Debug, Default)]
pub struct InputState {
    /// Keys currently down.
    keys_down: HashSet<KeyCode>,
    /// Keys pressed this frame.
    keys_pressed: HashSet<KeyCode>,
    /// Keys released this frame.
    keys_released: HashSet<KeyCode>,
    /// Mouse buttons down.
    mouse_down: HashSet<MouseButton>,
    /// Mouse pressed.
    mouse_pressed: HashSet<MouseButton>,
    /// Mouse released.
    mouse_released: HashSet<MouseButton>,
    /// Cursor position in window logical pixels.
    pub cursor: (f32, f32),
    /// Cursor delta this frame.
    pub cursor_delta: (f32, f32),
    /// Scroll delta.
    pub scroll: (f32, f32),
    /// Gamepad button states: (pad, button) -> down.
    gamepad_buttons: HashMap<(u32, u32), bool>,
    /// Gamepad axes: (pad, axis) -> value.
    gamepad_axes: HashMap<(u32, u32), f32>,
    /// Context stack.
    pub contexts: ContextStack,
    /// Resolved actions.
    actions: HashMap<ActionId, ActionState>,
    /// Activation threshold.
    pub threshold: f32,
    /// Deadzone for sticks.
    pub deadzone: f32,
}

impl InputState {
    /// Create with default gameplay + UI contexts.
    pub fn with_defaults() -> Self {
        let mut state = Self {
            threshold: 0.5,
            deadzone: 0.2,
            ..Default::default()
        };
        state.contexts.register(default_gameplay_context());
        state.contexts.register(default_ui_context());
        state.contexts.register(default_dialogue_context());
        state.contexts.push("gameplay");
        state
    }

    /// Begin frame: clear edges.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.cursor_delta = (0.0, 0.0);
        self.scroll = (0.0, 0.0);
    }

    /// End frame: resolve actions from bindings.
    pub fn end_frame(&mut self) {
        self.resolve_actions();
    }

    /// Key down event.
    pub fn key_down(&mut self, key: KeyCode) {
        if self.keys_down.insert(key) {
            self.keys_pressed.insert(key);
        }
    }

    /// Key up event.
    pub fn key_up(&mut self, key: KeyCode) {
        if self.keys_down.remove(&key) {
            self.keys_released.insert(key);
        }
    }

    /// Mouse down.
    pub fn mouse_down(&mut self, button: MouseButton) {
        if self.mouse_down.insert(button) {
            self.mouse_pressed.insert(button);
        }
    }

    /// Mouse up.
    pub fn mouse_up(&mut self, button: MouseButton) {
        if self.mouse_down.remove(&button) {
            self.mouse_released.insert(button);
        }
    }

    /// Set cursor position, accumulating delta.
    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.cursor_delta.0 += x - self.cursor.0;
        self.cursor_delta.1 += y - self.cursor.1;
        self.cursor = (x, y);
    }

    /// Set gamepad button.
    pub fn set_gamepad_button(&mut self, pad: u32, index: u32, down: bool) {
        self.gamepad_buttons.insert((pad, index), down);
    }

    /// Set gamepad axis.
    pub fn set_gamepad_axis(&mut self, pad: u32, index: u32, value: f32) {
        self.gamepad_axes.insert((pad, index), value);
    }

    /// Whether key is down.
    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Whether key was pressed this frame.
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Whether key was released this frame.
    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Action state.
    pub fn action(&self, id: impl Into<ActionId>) -> ActionState {
        let id = id.into();
        self.actions.get(&id).copied().unwrap_or_default()
    }

    /// Just pressed convenience.
    pub fn just_pressed(&self, id: impl Into<ActionId>) -> bool {
        self.action(id).just_pressed
    }

    /// Just released convenience.
    pub fn just_released(&self, id: impl Into<ActionId>) -> bool {
        self.action(id).just_released
    }

    /// Pressed/active.
    pub fn pressed(&self, id: impl Into<ActionId>) -> bool {
        self.action(id).active
    }

    /// Axis1 for action.
    pub fn axis1(&self, id: impl Into<ActionId>) -> f32 {
        self.action(id).value.map(|v| v.as_axis1()).unwrap_or(0.0)
    }

    /// Axis2 for action (e.g. move).
    pub fn axis2(&self, id: impl Into<ActionId>) -> Axis2d {
        let (x, y) = self
            .action(id)
            .value
            .map(|v| v.as_axis2())
            .unwrap_or((0.0, 0.0));
        Axis2d::new(x, y)
            .with_deadzone(self.deadzone)
            .clamp_length()
    }

    /// Iterate resolved action states.
    pub fn actions_iter(&self) -> impl Iterator<Item = (&ActionId, &ActionState)> + '_ {
        self.actions.iter()
    }

    /// All action ids currently known.
    pub fn action_ids(&self) -> impl Iterator<Item = &ActionId> + '_ {
        self.actions.keys()
    }

    /// Inject a synthetic action value (tests / virtual controls / replay).
    pub fn set_action_value(&mut self, id: impl Into<ActionId>, value: ActionValue) {
        let id = id.into();
        let state = self.actions.entry(id).or_default();
        state.update(value, self.threshold);
    }

    /// Any of the listed actions just pressed.
    pub fn any_just_pressed<I, A>(&self, ids: I) -> bool
    where
        I: IntoIterator<Item = A>,
        A: Into<ActionId>,
    {
        ids.into_iter().any(|id| self.just_pressed(id))
    }

    /// Composite digital pressed for confirm-style multi-binding already resolved.
    pub fn value_bool(&self, id: impl Into<ActionId>) -> bool {
        self.action(id).value.map(|v| v.as_bool()).unwrap_or(false)
    }

    fn control_value(&self, control: &VirtualKey) -> f32 {
        match control {
            VirtualKey::Key { key } => {
                if self.keys_down.contains(key) {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKey::Mouse { button } => {
                if self.mouse_down.contains(button) {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKey::GamepadButton { index } => {
                // pad 0 default
                if *self.gamepad_buttons.get(&(0, *index)).unwrap_or(&false) {
                    1.0
                } else {
                    0.0
                }
            }
            VirtualKey::GamepadAxis { index, sign } => {
                let v = *self.gamepad_axes.get(&(0, *index)).unwrap_or(&0.0);
                let s = f32::from(*sign);
                (v * s).max(0.0)
            }
        }
    }

    fn resolve_actions(&mut self) {
        let bindings: Vec<Binding> = self
            .contexts
            .active_bindings()
            .into_iter()
            .cloned()
            .collect();

        let mut accum: HashMap<ActionId, Acc> = HashMap::new();

        for b in bindings {
            let v = self.control_value(&b.control) * b.scale;
            let entry = accum.entry(b.action.clone()).or_default();
            match b.axis_component {
                Some(AxisComponent::X) => {
                    entry.x += v;
                    entry.is_axis2 = true;
                }
                Some(AxisComponent::Y) => {
                    entry.y += v;
                    entry.is_axis2 = true;
                }
                None => {
                    entry.digital = entry.digital.max(v);
                }
            }
        }

        let mut seen = HashSet::new();
        for (id, acc) in accum {
            let value = if acc.is_axis2 {
                ActionValue::Axis2 {
                    x: acc.x.clamp(-1.0, 1.0),
                    y: acc.y.clamp(-1.0, 1.0),
                }
            } else if acc.digital > 0.0 && acc.digital < 1.0 {
                ActionValue::Axis1(acc.digital.clamp(-1.0, 1.0))
            } else {
                ActionValue::Bool(acc.digital >= self.threshold)
            };
            seen.insert(id.clone());
            let state = self.actions.entry(id).or_default();
            state.update(value, self.threshold);
        }
        // Release actions that received no contributions this frame.
        for (id, state) in self.actions.iter_mut() {
            if !seen.contains(id) {
                state.update(ActionValue::Bool(false), self.threshold);
            }
        }
    }
}

#[derive(Default)]
struct Acc {
    digital: f32,
    x: f32,
    y: f32,
    is_axis2: bool,
}

fn default_gameplay_context() -> InputContext {
    use crate::binding::Binding;
    InputContext::new("gameplay", 10)
        .with_binding(Binding::key(builtin::CONFIRM, KeyCode::Enter))
        .with_binding(Binding::key(builtin::CONFIRM, KeyCode::Space))
        .with_binding(Binding::key(builtin::CANCEL, KeyCode::Escape))
        .with_binding(Binding::key(builtin::ATTACK, KeyCode::Z))
        .with_binding(Binding::key(builtin::INTERACT, KeyCode::E))
        .with_binding(Binding::key(builtin::OPEN_MENU, KeyCode::Tab))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::W,
            AxisComponent::Y,
            1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::S,
            AxisComponent::Y,
            -1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::A,
            AxisComponent::X,
            -1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::D,
            AxisComponent::X,
            1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::ArrowUp,
            AxisComponent::Y,
            1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::ArrowDown,
            AxisComponent::Y,
            -1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::ArrowLeft,
            AxisComponent::X,
            -1.0,
        ))
        .with_binding(Binding::key_axis(
            builtin::MOVE,
            KeyCode::ArrowRight,
            AxisComponent::X,
            1.0,
        ))
}

fn default_ui_context() -> InputContext {
    InputContext::new("ui", 20)
        .with_binding(Binding::key(builtin::CONFIRM, KeyCode::Enter))
        .with_binding(Binding::key(builtin::CANCEL, KeyCode::Escape))
}

fn default_dialogue_context() -> InputContext {
    InputContext::new("dialogue", 30)
        .with_binding(Binding::key(builtin::CONFIRM, KeyCode::Space))
        .with_binding(Binding::key(builtin::CONFIRM, KeyCode::Enter))
        .with_binding(Binding::key(builtin::SKIP_DIALOGUE, KeyCode::ControlLeft))
        .with_binding(Binding::key(builtin::AUTO_DIALOGUE, KeyCode::A))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_axis_from_wasd() {
        let mut input = InputState::with_defaults();
        input.begin_frame();
        input.key_down(KeyCode::W);
        input.key_down(KeyCode::D);
        input.end_frame();
        let m = input.axis2(builtin::MOVE);
        assert!(m.y > 0.5);
        assert!(m.x > 0.5);
    }

    #[test]
    fn just_pressed_edge() {
        let mut input = InputState::with_defaults();
        input.begin_frame();
        input.key_down(KeyCode::Enter);
        input.end_frame();
        assert!(input.just_pressed(builtin::CONFIRM));
        input.begin_frame();
        input.end_frame();
        // key still down but bindings re-resolved; just_pressed should be false
        assert!(input.pressed(builtin::CONFIRM));
        assert!(!input.just_pressed(builtin::CONFIRM));
    }
}
