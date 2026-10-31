//! The app's lifecycle as a game needs it.
//!
//! winit 0.30 reports Android's lifecycle through a few callbacks only: `resumed` when the
//! window's surface exists, `suspended` when it is gone (the app left the screen), focus changes,
//! and `memory_warning`. Android's other stages (pause, stop, destroy, saved instance state) are
//! not forwarded by winit, so a game should treat [`PlatformEvent::Suspended`] as "may be killed
//! without another word": save, pause the music, drop GPU surfaces.
//!
//! [`Lifecycle`] turns those callbacks (plus the back button) into [`PlatformEvent`]s and keeps
//! Android's screen state (immersive mode, safe area) applied.

use winit::event::{ElementState, WindowEvent};

use crate::input::is_back_key;
use crate::screen;

/// Something the platform did that a game may react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformEvent {
    /// The window can be drawn into: create (or re-create) GPU surfaces, resume audio.
    Resumed,
    /// The window's surface is gone (on Android: the app went to the background or the screen
    /// turned off): drop GPU surfaces, pause audio, save. The process may be killed from here on.
    Suspended,
    /// Input focus came or went (a system dialog, the notification shade, multi-window).
    FocusChanged(bool),
    /// The system is short of memory: free caches the game can rebuild.
    LowMemory,
    /// The back button or back gesture (Android). Go back a screen, or quit on the first one.
    Back,
}

/// Where the app is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Started, the window not shown yet.
    #[default]
    Starting,
    /// Shown and focused.
    Running,
    /// Shown but without focus (a dialog or the notification shade over it).
    Unfocused,
    /// Not shown: no surface to draw into.
    Background,
}

/// Follows the app's lifecycle from winit's callbacks.
///
/// ```ignore
/// fn resumed(&mut self, el: &ActiveEventLoop) { self.lifecycle.resumed(); /* surfaces */ }
/// fn suspended(&mut self, _: &ActiveEventLoop) { self.lifecycle.suspended(); /* save */ }
/// fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
///     if let Some(PlatformEvent::Back) = self.lifecycle.window_event(&event) { /* back */ }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Lifecycle {
    state: AppState,
    focused: bool,
    resumes: u32,
    immersive: bool,
    keep_screen_on: bool,
}

impl Lifecycle {
    /// A lifecycle in its [`AppState::Starting`] state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep the Android system bars hidden (re-applied on every resume and focus gain).
    pub fn with_immersive(mut self, immersive: bool) -> Self {
        self.immersive = immersive;
        self
    }

    /// Keep the screen on while the game is shown.
    pub fn with_keep_screen_on(mut self, on: bool) -> Self {
        self.keep_screen_on = on;
        self
    }

    /// Call from `ApplicationHandler::resumed`.
    pub fn resumed(&mut self) -> PlatformEvent {
        self.resumes += 1;
        self.focused = true;
        self.state = AppState::Running;
        self.apply_screen();
        PlatformEvent::Resumed
    }

    /// Call from `ApplicationHandler::suspended`.
    pub fn suspended(&mut self) -> PlatformEvent {
        self.state = AppState::Background;
        PlatformEvent::Suspended
    }

    /// Call from `ApplicationHandler::memory_warning`.
    pub fn memory_warning(&mut self) -> PlatformEvent {
        PlatformEvent::LowMemory
    }

    /// Call from `ApplicationHandler::window_event` with every event: focus changes and the back
    /// button come back as events (the back button only when pressed, not on release or repeat).
    pub fn window_event(&mut self, event: &WindowEvent) -> Option<PlatformEvent> {
        match event {
            WindowEvent::Focused(focused) => {
                self.focused = *focused;
                if self.state != AppState::Background && self.state != AppState::Starting {
                    self.state = if *focused {
                        AppState::Running
                    } else {
                        AppState::Unfocused
                    };
                }
                if *focused {
                    self.apply_screen();
                }
                Some(PlatformEvent::FocusChanged(*focused))
            }
            WindowEvent::Resized(_) => {
                screen::refresh_safe_area();
                None
            }
            WindowEvent::KeyboardInput { event, .. }
                if is_back_key(event) && event.state == ElementState::Pressed && !event.repeat =>
            {
                Some(PlatformEvent::Back)
            }
            _ => None,
        }
    }

    /// The current state.
    pub fn state(&self) -> AppState {
        self.state
    }

    /// Whether the game should run and draw now (shown, focused or not).
    pub fn is_visible(&self) -> bool {
        matches!(self.state, AppState::Running | AppState::Unfocused)
    }

    /// How many times the window was resumed (1 after the first start; more after coming back
    /// from the background).
    pub fn resumes(&self) -> u32 {
        self.resumes
    }

    fn apply_screen(&self) {
        if self.immersive {
            screen::set_immersive(true);
        }
        if self.keep_screen_on {
            screen::set_keep_screen_on(true);
        }
        screen::refresh_safe_area();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_follow_the_callbacks() {
        let mut life = Lifecycle::new().with_immersive(true);
        assert_eq!(life.state(), AppState::Starting);
        assert!(!life.is_visible());
        assert_eq!(life.resumed(), PlatformEvent::Resumed);
        assert_eq!(life.state(), AppState::Running);
        assert_eq!(
            life.window_event(&WindowEvent::Focused(false)),
            Some(PlatformEvent::FocusChanged(false))
        );
        assert_eq!(life.state(), AppState::Unfocused);
        assert!(life.is_visible());
        assert_eq!(life.suspended(), PlatformEvent::Suspended);
        assert_eq!(life.state(), AppState::Background);
        // Focus news while in the background does not bring the app back.
        life.window_event(&WindowEvent::Focused(true));
        assert_eq!(life.state(), AppState::Background);
        life.resumed();
        assert_eq!(life.resumes(), 2);
        assert_eq!(life.state(), AppState::Running);
        assert_eq!(life.memory_warning(), PlatformEvent::LowMemory);
        assert_eq!(life.window_event(&WindowEvent::CloseRequested), None);
    }
}
