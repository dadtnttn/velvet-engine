//! What the player did, in the game's logical coordinates: the events in order since the last step, and the state now
//! (where the pointer is, which buttons and keys are held).

use std::collections::HashSet;

use velvet_canvas::Point;
pub use winit::keyboard::KeyCode;

/// A pointer button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// One thing the player did.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// The pointer moved to a point of the picture.
    PointerMoved(Point),
    /// The pointer left the picture (into the bars, or out of the window).
    PointerLeft,
    /// A button went down on a point of the picture (never in the bars).
    PointerDown {
        button: Button,
        at: Point,
    },
    /// A button went up; `at` is held to the picture's edge when the pointer is in the bars, so a drag always ends.
    PointerUp {
        button: Button,
        at: Point,
    },
    /// The wheel turned: positive away from the player, in lines.
    Wheel(f32),
    /// A directional touch swipe (`dx < 0` left, `dx > 0` right; `dy < 0` up, `dy > 0` down).
    Swipe {
        dx: f32,
        dy: f32,
    },
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    /// Text typed (already composed by the system).
    Text(String),
}

/// The input of a step.
#[derive(Debug, Clone, Default)]
pub struct Input {
    events: Vec<InputEvent>,
    pointer: Option<Point>,
    buttons: HashSet<Button>,
    keys: HashSet<KeyCode>,
    last_pressed_at: Option<Point>,
    /// Seconds since the game started (the steps' clock).
    pub time: f32,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// What happened since the last step, in order.
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }

    /// Where the pointer is on the picture, if it is on it.
    pub fn pointer(&self) -> Option<Point> {
        self.pointer
    }

    pub fn button_down(&self, button: Button) -> bool {
        self.buttons.contains(&button)
    }

    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }

    /// True when the key went down since the last step.
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.events.contains(&InputEvent::KeyDown(key))
    }

    /// The wheel's turn since the last step.
    pub fn wheel(&self) -> f32 {
        self.events
            .iter()
            .filter_map(|e| {
                if let InputEvent::Wheel(d) = e {
                    Some(*d)
                } else {
                    None
                }
            })
            .sum()
    }

    /// Records an event (the host does this; tests and bots can too) and updates the state.
    pub fn push(&mut self, event: InputEvent) {
        match &event {
            InputEvent::PointerMoved(p) => self.pointer = Some(*p),
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::PointerDown { button, at } => {
                self.buttons.insert(*button);
                self.last_pressed_at = Some(*at);
                self.pointer = Some(*at);
            }
            InputEvent::PointerUp { button, .. } => {
                self.buttons.remove(button);
                if self.buttons.is_empty() {
                    self.last_pressed_at = None;
                }
            }
            InputEvent::KeyDown(k) => {
                // A held key repeats: only its first press counts as an event.
                if !self.keys.insert(*k) {
                    return;
                }
            }
            InputEvent::KeyUp(k) => {
                self.keys.remove(k);
            }
            InputEvent::Wheel(_) | InputEvent::Swipe { .. } | InputEvent::Text(_) => {}
        }
        self.events.push(event);
    }

    /// Releases held inputs when the window loses focus.
    pub(crate) fn release_all(&mut self) {
        self.events.extend(self.keys.drain().map(InputEvent::KeyUp));
        if let Some(at) = self.pointer.or(self.last_pressed_at) {
            self.events.extend(
                self.buttons
                    .drain()
                    .map(|button| InputEvent::PointerUp { button, at }),
            );
        }
        self.last_pressed_at = None;
    }

    /// Ends a step: the events are done, the state stays.
    pub fn end_step(&mut self, dt: f32) {
        self.events.clear();
        self.time += dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_last_one_step_and_the_state_stays() {
        let mut input = Input::new();
        input.push(InputEvent::PointerDown {
            button: Button::Left,
            at: Point::new(10.0, 20.0),
        });
        input.push(InputEvent::KeyDown(KeyCode::Space));
        assert!(input.button_down(Button::Left));
        assert!(input.key_pressed(KeyCode::Space));
        input.end_step(0.5);
        assert!(input.events().is_empty());
        assert!(input.button_down(Button::Left) && input.key_down(KeyCode::Space));
        assert!(!input.key_pressed(KeyCode::Space));
        assert_eq!(input.time, 0.5);
    }

    #[test]
    fn losing_focus_releases_keys_and_buttons_outside_the_picture() {
        let mut input = Input::new();
        let at = Point::new(10.0, 20.0);
        input.push(InputEvent::KeyDown(KeyCode::ControlLeft));
        input.push(InputEvent::PointerDown {
            button: Button::Left,
            at,
        });
        input.push(InputEvent::PointerLeft);
        input.end_step(0.0);
        input.release_all();
        input.release_all();
        assert_eq!(
            input.events(),
            &[
                InputEvent::KeyUp(KeyCode::ControlLeft),
                InputEvent::PointerUp {
                    button: Button::Left,
                    at
                },
            ],
        );
        assert!(!input.key_down(KeyCode::ControlLeft));
        assert!(!input.button_down(Button::Left));
    }

    #[test]
    fn a_held_key_counts_once_and_the_wheel_adds_up() {
        let mut input = Input::new();
        input.push(InputEvent::KeyDown(KeyCode::KeyA));
        input.push(InputEvent::KeyDown(KeyCode::KeyA));
        assert_eq!(input.events().len(), 1);
        input.push(InputEvent::Wheel(1.0));
        input.push(InputEvent::Wheel(2.0));
        assert_eq!(input.wheel(), 3.0);
    }
}
