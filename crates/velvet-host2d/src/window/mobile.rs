//! The window on Android (and on touch screens): fingers become the pointer, the back button
//! becomes Escape, and the GPU surface goes away and comes back with the app.

use velvet_platform::touch::TouchMouse;
use velvet_platform::{Lifecycle, PlatformEvent};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowId;

use super::{App, Presenter};
use crate::input::InputEvent;
use crate::Game2D;

/// What the host keeps for the platform: the touch-to-pointer translation and the lifecycle.
pub(super) struct Mobile {
    touch: TouchMouse,
    lifecycle: Lifecycle,
}

impl Mobile {
    pub(super) fn new() -> Self {
        Self {
            touch: TouchMouse::new(),
            // A game shows full screen, with the screen kept on (both no-ops on desktop).
            lifecycle: Lifecycle::new()
                .with_immersive(true)
                .with_keep_screen_on(true),
        }
    }
}

impl<G: Game2D> App<G> {
    /// Touch and the back button. Returns true when the event was used up here.
    pub(super) fn mobile_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        match event {
            // The first finger is the pointer with the left button held (the host's pointer
            // code then applies the letterbox as for the mouse).
            WindowEvent::Touch(_) => {
                for pointer in self.mobile.touch.translate(event) {
                    self.window_event(event_loop, id, pointer);
                }
                true
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
        self.last = Some(std::time::Instant::now());
        self.behind = 0.0;
        self.game.platform_event(event);
    }

    /// The window's surface is going away (Android: the app left the screen).
    pub(super) fn suspend(&mut self) {
        let event = self.mobile.lifecycle.suspended();
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
    /// Makes the window's surface again and configures it at the window's size.
    fn resume(&mut self) -> Result<(), wgpu::CreateSurfaceError> {
        self.surface.resume(&self.device)?;
        let size = self.surface.window().inner_size();
        if size.width > 0 && size.height > 0 {
            self.config.width = size.width;
            self.config.height = size.height;
        }
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }
}
