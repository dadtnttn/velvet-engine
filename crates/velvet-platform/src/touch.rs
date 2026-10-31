//! Touch input: the fingers on the screen, mouse events for games written for the mouse, and a
//! few gestures (tap, long press, drag, pinch).
//!
//! winit reports each finger as `WindowEvent::Touch` (Android, and touch screens on desktop) and
//! never turns touches into mouse events. Positions are window (physical) pixels, the same space
//! as `CursorMoved`.
//!
//! ```ignore
//! fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
//!     // A mouse-driven game: the first finger moves the cursor and holds the left button.
//!     for mouse in self.touch_mouse.translate(&event) {
//!         self.handle(el, id, mouse);
//!     }
//!     self.handle(el, id, event);
//! }
//! ```

use std::time::{Duration, Instant};

use winit::dpi::PhysicalPosition;
use winit::event::{DeviceId, ElementState, MouseButton, Touch, TouchPhase, WindowEvent};

/// One finger on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finger {
    /// The finger's id while it touches (winit's `Touch::id`).
    pub id: u64,
    /// Where it first touched.
    pub start: PhysicalPosition<f64>,
    /// Where it is now.
    pub position: PhysicalPosition<f64>,
    /// Where it was at the event before.
    pub previous: PhysicalPosition<f64>,
    /// When it touched.
    pub pressed_at: Instant,
    /// When its last event (press, move or release) came.
    pub changed_at: Instant,
    /// The farthest it has been from `start`, in pixels.
    pub travel: f64,
}

/// What one touch event did to a finger.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchChange {
    /// A finger touched the screen.
    Pressed(Finger),
    /// A finger moved.
    Moved(Finger),
    /// A finger left the screen.
    Released(Finger),
    /// The system took the touch away (an edge gesture, a dialog): treat it as never finished.
    Cancelled(Finger),
}

impl TouchChange {
    /// The finger concerned.
    pub fn finger(&self) -> &Finger {
        match self {
            Self::Pressed(f) | Self::Moved(f) | Self::Released(f) | Self::Cancelled(f) => f,
        }
    }
}

/// The fingers on the screen, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Touches {
    fingers: Vec<Finger>,
}

impl Touches {
    /// No fingers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the fingers with a winit touch event.
    pub fn handle(&mut self, touch: &Touch) -> Option<TouchChange> {
        self.handle_at(touch, Instant::now())
    }

    /// [`Touches::handle`] with the event's time given (tests, replays).
    pub fn handle_at(&mut self, touch: &Touch, now: Instant) -> Option<TouchChange> {
        let at = touch.location;
        let index = self.fingers.iter().position(|f| f.id == touch.id);
        match touch.phase {
            TouchPhase::Started => {
                if let Some(i) = index {
                    self.fingers.remove(i);
                }
                let finger = Finger {
                    id: touch.id,
                    start: at,
                    position: at,
                    previous: at,
                    pressed_at: now,
                    changed_at: now,
                    travel: 0.0,
                };
                self.fingers.push(finger);
                Some(TouchChange::Pressed(finger))
            }
            TouchPhase::Moved => {
                let finger = &mut self.fingers[index?];
                finger.previous = finger.position;
                finger.position = at;
                finger.changed_at = now;
                finger.travel = finger.travel.max(distance(finger.start, at));
                Some(TouchChange::Moved(*finger))
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let mut finger = self.fingers.remove(index?);
                finger.previous = finger.position;
                finger.position = at;
                finger.changed_at = now;
                finger.travel = finger.travel.max(distance(finger.start, at));
                Some(if touch.phase == TouchPhase::Ended {
                    TouchChange::Released(finger)
                } else {
                    TouchChange::Cancelled(finger)
                })
            }
        }
    }

    /// The fingers on the screen, oldest first.
    pub fn fingers(&self) -> &[Finger] {
        &self.fingers
    }

    /// How many fingers touch.
    pub fn len(&self) -> usize {
        self.fingers.len()
    }

    /// No finger touches.
    pub fn is_empty(&self) -> bool {
        self.fingers.is_empty()
    }

    /// The oldest finger still down.
    pub fn primary(&self) -> Option<&Finger> {
        self.fingers.first()
    }

    /// A finger by id.
    pub fn get(&self, id: u64) -> Option<&Finger> {
        self.fingers.iter().find(|f| f.id == id)
    }

    /// Forgets every finger (call on suspend: touches in flight never end).
    pub fn clear(&mut self) {
        self.fingers.clear();
    }
}

/// Turns touches into mouse events, so a game written for the mouse works on a touch screen:
/// the first finger moves the cursor and holds the left button; other fingers are ignored (see
/// [`Gestures`] for them). A tap is a click, a drag is a drag with the button held.
#[derive(Debug, Clone)]
pub struct TouchMouse {
    primary: Option<u64>,
    device: Option<DeviceId>,
    inside: bool,
    leave_after_release: bool,
}

impl Default for TouchMouse {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchMouse {
    /// The first finger as the mouse; after each release the cursor leaves the window, so hover
    /// highlights do not stay where the finger was.
    pub fn new() -> Self {
        Self {
            primary: None,
            device: None,
            inside: false,
            leave_after_release: true,
        }
    }

    /// Whether the cursor leaves the window after each release (default true).
    pub fn leave_after_release(mut self, leave: bool) -> Self {
        self.leave_after_release = leave;
        self
    }

    /// Whether the emulated left button is held.
    pub fn is_pressed(&self) -> bool {
        self.primary.is_some()
    }

    /// The mouse events a window event stands for: for a touch of the first finger, cursor and
    /// left-button events; nothing for any other event. A cancelled touch moves the cursor out of
    /// the window before releasing, so a click-on-release UI does not act on it.
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<WindowEvent> {
        let WindowEvent::Touch(touch) = event else {
            return Vec::new();
        };
        let device_id = touch.device_id;
        self.device = Some(device_id);
        let moved = WindowEvent::CursorMoved {
            device_id,
            position: touch.location,
        };
        let button = |state| WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
        };
        let mut out = Vec::new();
        match touch.phase {
            TouchPhase::Started if self.primary.is_none() => {
                self.primary = Some(touch.id);
                if !self.inside {
                    self.inside = true;
                    out.push(WindowEvent::CursorEntered { device_id });
                }
                out.push(moved);
                out.push(button(ElementState::Pressed));
            }
            TouchPhase::Moved if self.primary == Some(touch.id) => out.push(moved),
            TouchPhase::Ended if self.primary == Some(touch.id) => {
                self.primary = None;
                out.push(moved);
                out.push(button(ElementState::Released));
                if self.leave_after_release {
                    self.inside = false;
                    out.push(WindowEvent::CursorLeft { device_id });
                }
            }
            TouchPhase::Cancelled if self.primary == Some(touch.id) => {
                self.primary = None;
                self.inside = false;
                out.push(WindowEvent::CursorLeft { device_id });
                out.push(button(ElementState::Released));
            }
            _ => {}
        }
        out
    }

    /// Releases the button if a finger holds it (call on suspend, when touches in flight never
    /// end). The cursor leaves the window first.
    pub fn reset(&mut self) -> Vec<WindowEvent> {
        let (Some(_), Some(device_id)) = (self.primary.take(), self.device) else {
            return Vec::new();
        };
        self.inside = false;
        vec![
            WindowEvent::CursorLeft { device_id },
            WindowEvent::MouseInput {
                device_id,
                state: ElementState::Released,
                button: MouseButton::Left,
            },
        ]
    }
}

/// Thresholds of the gestures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureConfig {
    /// How far (pixels) a finger may move and still tap or long-press.
    pub slop: f64,
    /// The longest a tap may last.
    pub tap_time: Duration,
    /// How long a finger must stay still for a long press.
    pub long_press_time: Duration,
}

impl GestureConfig {
    /// Thresholds for a screen of this scale factor (`Window::scale_factor()`; on Android the
    /// density / 160): 10 dp of slop, 300 ms taps, 500 ms long presses.
    pub fn for_scale_factor(scale_factor: f64) -> Self {
        Self {
            slop: 10.0 * scale_factor.max(0.5),
            tap_time: Duration::from_millis(300),
            long_press_time: Duration::from_millis(500),
        }
    }
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self::for_scale_factor(1.0)
    }
}

/// A gesture recognised from the fingers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gesture {
    /// One or more fingers touched and lifted quickly without moving (a two-finger tap is a
    /// common "back" or "right click").
    Tap {
        /// Where the first finger touched.
        position: PhysicalPosition<f64>,
        /// How many fingers took part.
        fingers: usize,
    },
    /// One finger held still.
    LongPress {
        /// Where.
        position: PhysicalPosition<f64>,
    },
    /// Fingers moved together (past the slop): one finger drags, two pan or scroll.
    Drag {
        /// Movement since the last drag event, in pixels.
        delta: (f64, f64),
        /// How many fingers.
        fingers: usize,
    },
    /// Two fingers moved apart or together.
    Pinch {
        /// The point between the two fingers.
        center: PhysicalPosition<f64>,
        /// Distance now over distance at the last pinch event (multiply a zoom by it).
        scale: f64,
    },
}

#[derive(Debug, Clone, Copy)]
struct Session {
    started: Instant,
    first: PhysicalPosition<f64>,
    max_fingers: usize,
    moved: bool,
    long_pressed: bool,
    cancelled: bool,
}

/// Recognises [`Gesture`]s from the [`TouchChange`]s of a [`Touches`].
#[derive(Debug, Clone, Default)]
pub struct Gestures {
    config: GestureConfig,
    session: Option<Session>,
    /// Distance and centre of the two pinching fingers at the last event.
    pinch: Option<(f64, PhysicalPosition<f64>)>,
}

impl Gestures {
    /// Gestures with these thresholds.
    pub fn new(config: GestureConfig) -> Self {
        Self {
            config,
            session: None,
            pinch: None,
        }
    }

    /// The thresholds.
    pub fn config(&self) -> GestureConfig {
        self.config
    }

    /// Feeds a change (from [`Touches::handle`]) with the fingers as they are after it.
    pub fn on_change(&mut self, change: &TouchChange, touches: &Touches) -> Vec<Gesture> {
        let mut out = Vec::new();
        let finger = change.finger();
        match change {
            TouchChange::Pressed(_) => {
                let session = self.session.get_or_insert(Session {
                    started: finger.pressed_at,
                    first: finger.start,
                    max_fingers: 0,
                    moved: false,
                    long_pressed: false,
                    cancelled: false,
                });
                session.max_fingers = session.max_fingers.max(touches.len());
                self.pinch = pair(touches);
            }
            TouchChange::Moved(_) => {
                let slop = self.config.slop;
                if let Some(session) = &mut self.session {
                    session.moved |= finger.travel > slop;
                }
                let moved = self.session.is_some_and(|s| s.moved);
                if touches.len() >= 2 {
                    if let (Some((last_distance, last_center)), Some((distance, center))) =
                        (self.pinch, pair(touches))
                    {
                        if moved && last_distance > 0.0 {
                            let scale = distance / last_distance;
                            if (scale - 1.0).abs() > 1e-6 {
                                out.push(Gesture::Pinch { center, scale });
                            }
                            let delta = (center.x - last_center.x, center.y - last_center.y);
                            if delta != (0.0, 0.0) {
                                out.push(Gesture::Drag {
                                    delta,
                                    fingers: touches.len(),
                                });
                            }
                        }
                        self.pinch = Some((distance, center));
                    }
                } else if moved {
                    let delta = (
                        finger.position.x - finger.previous.x,
                        finger.position.y - finger.previous.y,
                    );
                    if delta != (0.0, 0.0) {
                        out.push(Gesture::Drag { delta, fingers: 1 });
                    }
                }
            }
            TouchChange::Released(_) | TouchChange::Cancelled(_) => {
                if let Some(session) = &mut self.session {
                    session.cancelled |= matches!(change, TouchChange::Cancelled(_));
                    session.moved |= finger.travel > self.config.slop;
                }
                self.pinch = pair(touches);
                if touches.is_empty() {
                    if let Some(session) = self.session.take() {
                        let quick = finger.changed_at.saturating_duration_since(session.started)
                            <= self.config.tap_time;
                        if quick && !session.moved && !session.long_pressed && !session.cancelled {
                            out.push(Gesture::Tap {
                                position: session.first,
                                fingers: session.max_fingers,
                            });
                        }
                    }
                }
            }
        }
        out
    }

    /// Call once a frame: reports a long press when one finger has stayed still long enough.
    pub fn poll(&mut self, touches: &Touches, now: Instant) -> Option<Gesture> {
        let session = self.session.as_mut()?;
        let finger = touches.primary()?;
        if touches.len() == 1
            && !session.moved
            && !session.long_pressed
            && session.max_fingers == 1
            && finger.travel <= self.config.slop
            && now.duration_since(finger.pressed_at) >= self.config.long_press_time
        {
            session.long_pressed = true;
            return Some(Gesture::LongPress {
                position: finger.position,
            });
        }
        None
    }

    /// Forgets the gesture in progress (call on suspend).
    pub fn clear(&mut self) {
        self.session = None;
        self.pinch = None;
    }
}

fn distance(a: PhysicalPosition<f64>, b: PhysicalPosition<f64>) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Distance and centre of the two oldest fingers.
fn pair(touches: &Touches) -> Option<(f64, PhysicalPosition<f64>)> {
    let [a, b, ..] = touches.fingers() else {
        return None;
    };
    Some((
        distance(a.position, b.position),
        PhysicalPosition::new(
            (a.position.x + b.position.x) / 2.0,
            (a.position.y + b.position.y) / 2.0,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id,
        }
    }

    #[test]
    fn the_first_finger_is_the_left_button() {
        let mut mouse = TouchMouse::new();
        let down = mouse.translate(&WindowEvent::Touch(touch(
            7,
            TouchPhase::Started,
            10.0,
            20.0,
        )));
        assert!(matches!(down[0], WindowEvent::CursorEntered { .. }));
        assert!(matches!(
            down[1],
            WindowEvent::CursorMoved { position, .. } if position == PhysicalPosition::new(10.0, 20.0)
        ));
        assert!(matches!(
            down[2],
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            }
        ));
        assert!(mouse.is_pressed());
        // A second finger is not the mouse.
        assert!(mouse
            .translate(&WindowEvent::Touch(touch(
                8,
                TouchPhase::Started,
                99.0,
                99.0
            )))
            .is_empty());
        assert!(mouse
            .translate(&WindowEvent::Touch(touch(8, TouchPhase::Moved, 98.0, 98.0)))
            .is_empty());
        let drag = mouse.translate(&WindowEvent::Touch(touch(7, TouchPhase::Moved, 30.0, 20.0)));
        assert_eq!(drag.len(), 1);
        let up = mouse.translate(&WindowEvent::Touch(touch(7, TouchPhase::Ended, 31.0, 21.0)));
        assert!(matches!(
            up[1],
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            }
        ));
        assert!(matches!(up[2], WindowEvent::CursorLeft { .. }));
        assert!(!mouse.is_pressed());
        assert!(mouse.translate(&WindowEvent::Focused(true)).is_empty());
    }

    #[test]
    fn a_cancelled_touch_releases_outside_and_reset_releases() {
        let mut mouse = TouchMouse::new().leave_after_release(false);
        mouse.translate(&WindowEvent::Touch(touch(1, TouchPhase::Started, 5.0, 5.0)));
        let cancel = mouse.translate(&WindowEvent::Touch(touch(
            1,
            TouchPhase::Cancelled,
            5.0,
            5.0,
        )));
        assert!(matches!(cancel[0], WindowEvent::CursorLeft { .. }));
        assert!(matches!(
            cancel[1],
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            }
        ));
        mouse.translate(&WindowEvent::Touch(touch(2, TouchPhase::Started, 5.0, 5.0)));
        assert_eq!(mouse.reset().len(), 2);
        assert!(mouse.reset().is_empty());
    }

    #[test]
    fn taps_drags_pinches_and_long_presses() {
        let t0 = Instant::now();
        let mut touches = Touches::new();
        let mut gestures = Gestures::new(GestureConfig::for_scale_factor(1.0));
        let mut feed = |touches: &mut Touches, t: Touch, at: Duration| {
            let change = touches.handle_at(&t, t0 + at).unwrap();
            gestures.on_change(&change, touches)
        };

        // A quick tap.
        assert!(feed(
            &mut touches,
            touch(1, TouchPhase::Started, 100.0, 100.0),
            Duration::ZERO
        )
        .is_empty());
        let tap = feed(
            &mut touches,
            touch(1, TouchPhase::Ended, 102.0, 101.0),
            Duration::from_millis(80),
        );
        assert_eq!(
            tap,
            vec![Gesture::Tap {
                position: PhysicalPosition::new(100.0, 100.0),
                fingers: 1
            }]
        );

        // A drag past the slop: drags, no tap.
        feed(
            &mut touches,
            touch(2, TouchPhase::Started, 0.0, 0.0),
            Duration::ZERO,
        );
        let drag = feed(
            &mut touches,
            touch(2, TouchPhase::Moved, 40.0, 0.0),
            Duration::from_millis(16),
        );
        assert_eq!(
            drag,
            vec![Gesture::Drag {
                delta: (40.0, 0.0),
                fingers: 1
            }]
        );
        assert!(feed(
            &mut touches,
            touch(2, TouchPhase::Ended, 40.0, 0.0),
            Duration::from_millis(32)
        )
        .is_empty());

        // Two fingers spreading: a pinch that doubles the distance.
        feed(
            &mut touches,
            touch(3, TouchPhase::Started, 100.0, 100.0),
            Duration::ZERO,
        );
        feed(
            &mut touches,
            touch(4, TouchPhase::Started, 200.0, 100.0),
            Duration::ZERO,
        );
        let pinch = feed(
            &mut touches,
            touch(4, TouchPhase::Moved, 300.0, 100.0),
            Duration::from_millis(16),
        );
        assert!(pinch
            .iter()
            .any(|g| matches!(g, Gesture::Pinch { scale, .. } if (scale - 2.0).abs() < 1e-9)));
        feed(
            &mut touches,
            touch(3, TouchPhase::Ended, 100.0, 100.0),
            Duration::from_millis(30),
        );
        assert!(feed(
            &mut touches,
            touch(4, TouchPhase::Ended, 300.0, 100.0),
            Duration::from_millis(40)
        )
        .is_empty());

        // A held finger: one long press.
        feed(
            &mut touches,
            touch(5, TouchPhase::Started, 50.0, 50.0),
            Duration::ZERO,
        );
        assert_eq!(
            gestures.poll(&touches, t0 + Duration::from_millis(100)),
            None
        );
        assert_eq!(
            gestures.poll(&touches, t0 + Duration::from_millis(600)),
            Some(Gesture::LongPress {
                position: PhysicalPosition::new(50.0, 50.0)
            })
        );
        assert_eq!(
            gestures.poll(&touches, t0 + Duration::from_millis(900)),
            None
        );
        touches.clear();
        gestures.clear();
        assert!(touches.is_empty());
    }
}
