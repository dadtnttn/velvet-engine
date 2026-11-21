//! The window on Android (and on touch screens): fingers become the pointer (or gestures when
//! `HostConfig::touch_gestures` is on), the back button becomes Escape, and the GPU surface goes
//! away and comes back with the app.

use std::time::{Duration, Instant};

use velvet_canvas::Point;
use velvet_platform::touch::{TouchMouse, Touches};
use velvet_platform::{Lifecycle, PlatformEvent};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Touch, TouchPhase, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowId;

use super::{App, Presenter};
use crate::input::{Button, InputEvent};
use crate::Game2D;

/// What a gesture-aware touch session emits (in window physical pixels, translated to logical
/// coordinates by `App`).
#[derive(Debug, Clone, Copy, PartialEq)]
enum TouchAction {
    Tap(PhysicalPosition<f64>),
    RightClick(PhysicalPosition<f64>),
    Wheel(f32),
    Swipe { dx: f32, dy: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragAxis {
    None,
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy)]
struct GestureSession {
    primary_id: u64,
    start: PhysicalPosition<f64>,
    pressed_at: Instant,
    max_fingers: usize,
    max_travel: f64,
    axis: DragAxis,
    /// Accumulated vertical drag not yet emitted as `Wheel` ticks (pixels).
    v_accum: f64,
    /// Total horizontal displacement since start (pixels).
    h_total: f64,
    /// Whether vertical or horizontal events were already emitted in this session.
    emitted_drag: bool,
    long_pressed: bool,
    cancelled: bool,
}

/// Gesture recogniser for touch-first games (tap on release, long press = right click, vertical
/// drag = wheel, horizontal swipe = `Swipe`, two-finger tap = right click).
struct TouchGestures {
    touches: Touches,
    session: Option<GestureSession>,
    scale_factor: f64,
}

impl TouchGestures {
    fn new() -> Self {
        Self {
            touches: Touches::new(),
            session: None,
            scale_factor: 1.0,
        }
    }

    fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor.max(0.5);
    }

    fn slop(&self) -> f64 {
        14.0 * self.scale_factor
    }

    fn v_step(&self) -> f64 {
        72.0 * self.scale_factor
    }

    fn v_flick(&self) -> f64 {
        28.0 * self.scale_factor
    }

    fn h_flick(&self) -> f64 {
        36.0 * self.scale_factor
    }

    fn long_press_time() -> Duration {
        Duration::from_millis(450)
    }

    fn clear(&mut self) {
        self.touches.clear();
        self.session = None;
    }

    fn handle_at(&mut self, touch: &Touch, now: Instant) -> Vec<TouchAction> {
        let mut out = Vec::new();
        let Some(change) = self.touches.handle_at(touch, now) else {
            return out;
        };
        let slop = self.slop();
        let v_step = self.v_step();
        let v_flick = self.v_flick();
        let h_flick = self.h_flick();

        match touch.phase {
            TouchPhase::Started => {
                let f = change.finger();
                let session = self.session.get_or_insert(GestureSession {
                    primary_id: f.id,
                    start: f.start,
                    pressed_at: now,
                    max_fingers: 0,
                    max_travel: 0.0,
                    axis: DragAxis::None,
                    v_accum: 0.0,
                    h_total: 0.0,
                    emitted_drag: false,
                    long_pressed: false,
                    cancelled: false,
                });
                session.max_fingers = session.max_fingers.max(self.touches.len());
            }
            TouchPhase::Moved => {
                let f = change.finger();
                let Some(session) = self.session.as_mut() else {
                    return out;
                };
                session.max_travel = session.max_travel.max(f.travel);
                if session.long_pressed || session.cancelled || f.id != session.primary_id {
                    return out;
                }
                let dx_total = f.position.x - session.start.x;
                let dy_total = f.position.y - session.start.y;
                if session.axis == DragAxis::None && session.max_travel > slop {
                    session.axis = if dy_total.abs() >= dx_total.abs() {
                        DragAxis::Vertical
                    } else {
                        DragAxis::Horizontal
                    };
                    session.v_accum = dy_total;
                    session.h_total = dx_total;
                } else {
                    match session.axis {
                        DragAxis::Vertical => {
                            session.v_accum += f.position.y - f.previous.y;
                        }
                        DragAxis::Horizontal => {
                            session.h_total = dx_total;
                        }
                        DragAxis::None => {}
                    }
                }
                if session.axis == DragAxis::Vertical {
                    while session.v_accum >= v_step {
                        session.v_accum -= v_step;
                        session.emitted_drag = true;
                        out.push(TouchAction::Wheel(1.0));
                    }
                    while session.v_accum <= -v_step {
                        session.v_accum += v_step;
                        session.emitted_drag = true;
                        out.push(TouchAction::Wheel(-1.0));
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let f = change.finger();
                if let Some(session) = self.session.as_mut() {
                    session.max_travel = session.max_travel.max(f.travel);
                    if touch.phase == TouchPhase::Cancelled {
                        session.cancelled = true;
                    }
                    if f.id == session.primary_id && session.axis == DragAxis::Horizontal {
                        session.h_total = f.position.x - session.start.x;
                    }
                }
                if self.touches.is_empty() {
                    if let Some(session) = self.session.take() {
                        if session.cancelled || session.long_pressed {
                            return out;
                        }
                        match session.axis {
                            DragAxis::None if session.max_travel <= slop => {
                                if session.max_fingers >= 2 {
                                    out.push(TouchAction::RightClick(session.start));
                                } else {
                                    out.push(TouchAction::Tap(session.start));
                                }
                            }
                            DragAxis::Vertical if !session.emitted_drag => {
                                if session.v_accum >= v_flick {
                                    out.push(TouchAction::Wheel(1.0));
                                } else if session.v_accum <= -v_flick {
                                    out.push(TouchAction::Wheel(-1.0));
                                }
                            }
                            DragAxis::Horizontal => {
                                if session.h_total >= h_flick {
                                    out.push(TouchAction::Swipe { dx: 1.0, dy: 0.0 });
                                } else if session.h_total <= -h_flick {
                                    out.push(TouchAction::Swipe { dx: -1.0, dy: 0.0 });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        out
    }

    fn poll_at(&mut self, now: Instant) -> Option<TouchAction> {
        let slop = self.slop();
        let session = self.session.as_mut()?;
        let finger = self.touches.primary()?;
        if self.touches.len() == 1
            && !session.long_pressed
            && !session.cancelled
            && session.axis == DragAxis::None
            && session.max_fingers == 1
            && session.max_travel <= slop
            && finger.travel <= slop
            && now.saturating_duration_since(session.pressed_at) >= Self::long_press_time()
        {
            session.long_pressed = true;
            return Some(TouchAction::RightClick(finger.position));
        }
        None
    }
}

/// What the host keeps for the platform: the touch-to-pointer translation and the lifecycle.
pub(super) struct Mobile {
    touch: TouchMouse,
    gestures: TouchGestures,
    lifecycle: Lifecycle,
}

impl Mobile {
    pub(super) fn new() -> Self {
        Self {
            touch: TouchMouse::new(),
            gestures: TouchGestures::new(),
            // A game shows full screen, with the screen kept on (both no-ops on desktop).
            lifecycle: Lifecycle::new()
                .with_immersive(true)
                .with_keep_screen_on(true),
        }
    }

    pub(super) fn set_scale_factor(&mut self, scale_factor: f64) {
        self.gestures.set_scale_factor(scale_factor);
    }

    pub(super) fn is_visible(&self) -> bool {
        self.lifecycle.is_visible()
    }
}

impl<G: Game2D> App<G> {
    fn emit_touch_action(&mut self, action: TouchAction) {
        let viewport = self.running.as_ref().and_then(|r| r.viewport);
        match action {
            TouchAction::Tap(pos) => {
                if let Some((lx, ly)) = viewport.and_then(|v| v.to_logical(pos.x, pos.y)) {
                    let at = Point::new(lx, ly);
                    self.input.push(InputEvent::PointerMoved(at));
                    self.input.push(InputEvent::PointerDown {
                        button: Button::Left,
                        at,
                    });
                    self.input.push(InputEvent::PointerUp {
                        button: Button::Left,
                        at,
                    });
                    self.input.push(InputEvent::PointerLeft);
                }
            }
            TouchAction::RightClick(pos) => {
                if let Some(v) = viewport {
                    let (lx, ly) = v.to_logical_clamped(pos.x, pos.y);
                    let at = Point::new(lx, ly);
                    self.input.push(InputEvent::PointerDown {
                        button: Button::Right,
                        at,
                    });
                    self.input.push(InputEvent::PointerUp {
                        button: Button::Right,
                        at,
                    });
                }
            }
            TouchAction::Wheel(lines) => {
                self.input.push(InputEvent::Wheel(lines));
            }
            TouchAction::Swipe { dx, dy } => {
                self.input.push(InputEvent::Swipe { dx, dy });
            }
        }
    }

    /// Polls time-based touch gestures (long press).
    pub(super) fn mobile_poll(&mut self) {
        if !self.config.touch_gestures {
            return;
        }
        if let Some(action) = self.mobile.gestures.poll_at(Instant::now()) {
            self.emit_touch_action(action);
        }
    }

    /// Touch and the back button. Returns true when the event was used up here.
    pub(super) fn mobile_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        match event {
            WindowEvent::Touch(touch) => {
                if self.config.touch_gestures {
                    let actions = self.mobile.gestures.handle_at(touch, Instant::now());
                    for action in actions {
                        self.emit_touch_action(action);
                    }
                } else {
                    // The first finger is the pointer with the left button held (the host's pointer
                    // code then applies the letterbox as for the mouse).
                    for pointer in self.mobile.touch.translate(event) {
                        self.window_event(event_loop, id, pointer);
                    }
                }
                true
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.mobile.set_scale_factor(*scale_factor);
                false
            }
            // Android's back button arrives as an unnamed key: the game hears Escape.
            WindowEvent::KeyboardInput { event: key, .. }
                if velvet_platform::input::is_back_key(key) =>
            {
                match key.state {
                    ElementState::Pressed => {
                        if !key.repeat {
                            self.game.platform_event(PlatformEvent::Back);
                        }
                        self.input.push(InputEvent::KeyDown(KeyCode::Escape));
                    }
                    ElementState::Released => self.input.push(InputEvent::KeyUp(KeyCode::Escape)),
                }
                true
            }
            // Focus changes reach the game; a resize reads the safe area again. The host goes on
            // handling both.
            WindowEvent::Focused(_) | WindowEvent::Resized(_) => {
                if let Some(platform) = self.mobile.lifecycle.window_event(event) {
                    self.game.platform_event(platform);
                }
                false
            }
            _ => false,
        }
    }

    /// The window was just opened: full screen and screen-on on Android, the safe area read.
    pub(super) fn started(&mut self) {
        let event = self.mobile.lifecycle.resumed();
        self.game.platform_event(event);
    }

    /// The window came back (Android): a new surface, and the sizes again.
    pub(super) fn resume(&mut self) {
        let event = self.mobile.lifecycle.resumed();
        let Some(running) = self.running.as_mut() else {
            return;
        };
        if let Err(e) = running.presenter.resume() {
            eprintln!("[velvet-host2d] the surface could not be made again: {e}");
            return;
        }
        let size = running.window.inner_size();
        self.resized(size.width, size.height);
        // No catching up for the time spent in the background.
        self.last = Some(Instant::now());
        self.behind = 0.0;
        self.game.platform_event(event);
    }

    /// The window's surface is going away (Android: the app left the screen).
    pub(super) fn suspend(&mut self) {
        let event = self.mobile.lifecycle.suspended();
        self.mobile.gestures.clear();
        for pointer in self.mobile.touch.reset() {
            if let WindowEvent::MouseInput { state, button, .. } = pointer {
                self.button(state, button);
            }
        }
        if let Some(running) = self.running.as_mut() {
            running.presenter.surface.suspend();
        }
        self.game.platform_event(event);
    }

    /// The system is short of memory.
    pub(super) fn low_memory(&mut self) {
        let event = self.mobile.lifecycle.memory_warning();
        self.game.platform_event(event);
    }
}

impl Presenter {
    /// Makes the window's surface again. The host then applies the current window size.
    fn resume(&mut self) -> Result<(), wgpu::CreateSurfaceError> {
        self.surface.resume(&self.device)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::event::DeviceId;

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
    fn gestures_recognise_tap_long_press_and_swipes() {
        let t0 = Instant::now();
        let mut g = TouchGestures::new();
        g.set_scale_factor(2.0);

        // Quick tap emits on release.
        assert!(g
            .handle_at(&touch(1, TouchPhase::Started, 500.0, 400.0), t0)
            .is_empty());
        assert_eq!(
            g.handle_at(
                &touch(1, TouchPhase::Ended, 504.0, 402.0),
                t0 + Duration::from_millis(90)
            ),
            vec![TouchAction::Tap(PhysicalPosition::new(500.0, 400.0))]
        );

        // Long press fires via poll and suppresses tap on release.
        g.handle_at(&touch(2, TouchPhase::Started, 300.0, 300.0), t0);
        assert_eq!(g.poll_at(t0 + Duration::from_millis(200)), None);
        assert_eq!(
            g.poll_at(t0 + Duration::from_millis(500)),
            Some(TouchAction::RightClick(PhysicalPosition::new(300.0, 300.0)))
        );
        assert!(g
            .handle_at(
                &touch(2, TouchPhase::Ended, 300.0, 300.0),
                t0 + Duration::from_millis(600)
            )
            .is_empty());

        // Vertical swipe down emits Wheel(+1.0) without tapping.
        g.handle_at(&touch(3, TouchPhase::Started, 400.0, 200.0), t0);
        let drag = g.handle_at(
            &touch(3, TouchPhase::Moved, 400.0, 380.0),
            t0 + Duration::from_millis(80),
        );
        assert_eq!(drag, vec![TouchAction::Wheel(1.0)]);
        assert!(g
            .handle_at(
                &touch(3, TouchPhase::Ended, 400.0, 380.0),
                t0 + Duration::from_millis(100)
            )
            .is_empty());

        // Horizontal swipe left emits Swipe { dx: -1.0, dy: 0.0 }.
        g.handle_at(&touch(4, TouchPhase::Started, 600.0, 400.0), t0);
        g.handle_at(
            &touch(4, TouchPhase::Moved, 420.0, 405.0),
            t0 + Duration::from_millis(60),
        );
        assert_eq!(
            g.handle_at(
                &touch(4, TouchPhase::Ended, 420.0, 405.0),
                t0 + Duration::from_millis(90)
            ),
            vec![TouchAction::Swipe { dx: -1.0, dy: 0.0 }]
        );

        // Cancelled touch emits nothing.
        g.handle_at(&touch(5, TouchPhase::Started, 100.0, 100.0), t0);
        assert!(g
            .handle_at(
                &touch(5, TouchPhase::Cancelled, 100.0, 100.0),
                t0 + Duration::from_millis(50)
            )
            .is_empty());
    }
}
