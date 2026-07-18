//! Optional winit-based window runner.

use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use tracing::{debug, info};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::app::{App, AppExitCode, Runner};
use crate::schedule::ScheduleLabel;
use velvet_core::config::WindowConfig;
use velvet_events::{AppLifecycleEvent, WindowResized};

/// Runs the app with an OS window.
#[derive(Default)]
pub struct WindowRunner {
    /// Optional max frames for automated tests (`None` = until close).
    pub max_frames: Option<u64>,
}

/// Frame presenter callback type.
pub type FramePresenter = Box<dyn FnMut(&mut App, &Window) + Send>;

/// Callback invoked once when the OS window is created (GPU init, etc.).
pub type WindowOnCreate = Box<dyn FnMut(&mut App, &Arc<Window>) + Send>;

/// Resource wrapping the frame presenter callback (mutex for `Sync`).
pub struct WindowFrameHook {
    /// Present callback.
    pub present: Mutex<FramePresenter>,
}

/// Called once when the window is created (GPU init, etc.).
pub struct WindowInitHook {
    /// Callback.
    pub on_window: Mutex<WindowOnCreate>,
}

/// Called on resize.
pub struct WindowResizeHook {
    /// Callback.
    pub on_resize: Mutex<Box<dyn FnMut(u32, u32) + Send>>,
}

impl Runner for WindowRunner {
    fn run(&mut self, app: &mut App) -> AppExitCode {
        if let Err(e) = app.finish_plugins() {
            tracing::error!("plugin finish failed: {e}");
            return AppExitCode(1);
        }
        app.run_startup();

        // When max_frames is set (automated / short host ticks), allow any-thread
        // construction on Windows so CLI workers can exercise the real WindowRunner
        // without hanging the main thread forever.
        let event_loop = {
            #[cfg(windows)]
            {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                if self.max_frames.is_some() {
                    match EventLoop::builder().with_any_thread(true).build() {
                        Ok(el) => el,
                        Err(e) => {
                            tracing::error!("failed to create event loop (any_thread): {e}");
                            return AppExitCode(1);
                        }
                    }
                } else {
                    match EventLoop::new() {
                        Ok(el) => el,
                        Err(e) => {
                            tracing::error!("failed to create event loop: {e}");
                            return AppExitCode(1);
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            {
                match EventLoop::new() {
                    Ok(el) => el,
                    Err(e) => {
                        tracing::error!("failed to create event loop: {e}");
                        return AppExitCode(1);
                    }
                }
            }
        };
        event_loop.set_control_flow(ControlFlow::Poll);

        app.events_mut().add_event::<AppLifecycleEvent>();
        app.events_mut().add_event::<WindowResized>();

        let mut handler = VelvetWindowApp {
            app,
            window: None,
            max_frames: self.max_frames,
            frames: 0,
            last_instant: Instant::now(),
            exit_code: 0,
            pending_init: false,
        };

        if let Err(e) = event_loop.run_app(&mut handler) {
            tracing::error!("event loop error: {e}");
            return AppExitCode(1);
        }
        let exit_code = handler.exit_code;
        drop(handler);
        app.run_schedule(ScheduleLabel::Shutdown);
        AppExitCode(exit_code)
    }
}

struct VelvetWindowApp<'a> {
    app: &'a mut App,
    window: Option<Arc<Window>>,
    max_frames: Option<u64>,
    frames: u64,
    last_instant: Instant,
    exit_code: i32,
    pending_init: bool,
}

impl ApplicationHandler for VelvetWindowApp<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let cfg = self.app.config().window.clone();
        let attrs = window_attributes(&cfg);
        match event_loop.create_window(attrs) {
            Ok(window) => {
                info!(
                    title = %cfg.title,
                    width = cfg.width,
                    height = cfg.height,
                    "window created"
                );
                self.window = Some(Arc::new(window));
                self.pending_init = true;
                self.app
                    .events_mut()
                    .writer::<AppLifecycleEvent>()
                    .send(AppLifecycleEvent::Resumed);
            }
            Err(e) => {
                tracing::error!("failed to create window: {e}");
                self.exit_code = 1;
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Run deferred window init (avoids borrow of app during create).
        if self.pending_init {
            if let Some(window) = self.window.clone() {
                if let Some(hook) = self.app.resources_mut().remove::<WindowInitHook>() {
                    {
                        let mut cb = hook.on_window.lock();
                        (cb)(self.app, &window);
                    }
                    self.app.insert_resource(hook);
                }
            }
            self.pending_init = false;
        }

        match event {
            WindowEvent::CloseRequested => {
                debug!("close requested");
                self.app.request_exit();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.app
                    .events_mut()
                    .writer::<WindowResized>()
                    .send(WindowResized {
                        width: size.width,
                        height: size.height,
                    });
                if let Some(hook) = self.app.resources_mut().remove::<WindowResizeHook>() {
                    {
                        let mut cb = hook.on_resize.lock();
                        (cb)(size.width, size.height);
                    }
                    self.app.insert_resource(hook);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_instant).as_secs_f32();
                self.last_instant = now;
                self.app.tick_frame(dt);
                self.frames += 1;

                if let Some(window) = self.window.clone() {
                    if let Some(hook) = self.app.resources_mut().remove::<WindowFrameHook>() {
                        {
                            let mut present = hook.present.lock();
                            (present)(self.app, &window);
                        }
                        self.app.insert_resource(hook);
                    }
                    window.request_redraw();
                }

                if let Some(max) = self.max_frames {
                    if self.frames >= max {
                        self.app.request_exit();
                        event_loop.exit();
                    }
                }
                if self.app.should_exit() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: DeviceEvent,
    ) {
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn window_attributes(cfg: &WindowConfig) -> WindowAttributes {
    WindowAttributes::default()
        .with_title(cfg.title.clone())
        .with_inner_size(winit::dpi::LogicalSize::new(cfg.width, cfg.height))
        .with_resizable(cfg.resizable)
}
