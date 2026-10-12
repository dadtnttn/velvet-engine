use crate::app::Vs3App;
use crate::graphics::window_to_canvas;
use softbuffer::{Context as SoftContext, Surface};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};
impl ApplicationHandler for Vs3App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let spec = &self.manifest.app;
        // Logical sizes: the window keeps its size on screens with scaling
        // (150% on a 4K monitor opens it at 1920x1080 real pixels, which the
        // renderer then fills at that resolution).
        let attrs = Window::default_attributes()
            .with_title(spec.title.clone())
            .with_inner_size(LogicalSize::new(spec.window_width, spec.window_height))
            .with_min_inner_size(LogicalSize::new(spec.minimum_width, spec.minimum_height))
            .with_resizable(spec.resizable)
            .with_position(PhysicalPosition::new(80, 50));
        match event_loop.create_window(attrs) {
            Ok(window) => {
                // Text input for names and chat: also lets the platform IME
                // (Chinese, Japanese, Korean...) commit composed text.
                window.set_ime_allowed(true);
                let window = Arc::new(window);
                match SoftContext::new(window.clone()).and_then(|context| {
                    Surface::new(&context, window.clone()).map(|surface| (context, surface))
                }) {
                    Ok((context, surface)) => {
                        self.window = Some(window);
                        self.context = Some(context);
                        self.surface = Some(surface);
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    Err(e) => {
                        eprintln!("could not create surface: {e}");
                        event_loop.exit();
                    }
                }
            }
            Err(e) => {
                eprintln!("could not create window: {e}");
                event_loop.exit();
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => self.input.press(key, event.repeat),
                        ElementState::Released => self.input.release(key),
                    }
                }
                if event.state == ElementState::Pressed {
                    if let Some(text) = &event.text {
                        self.input.text(text);
                    }
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => self.input.text(&text),
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(window) = &self.window {
                    // Logical units: the canvas itself may be drawn bigger.
                    self.cursor = window_to_canvas(
                        position,
                        window.inner_size(),
                        self.manifest.app.canvas_width,
                        self.manifest.app.canvas_height,
                    );
                    self.input
                        .pointer(self.cursor.map(|p| [p[0] as f64, p[1] as f64]));
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.mouse_down = true;
                    self.input.mouse(true)
                }
                ElementState::Released => {
                    let was = self.mouse_down;
                    self.mouse_down = false;
                    self.input.mouse(false);
                    if was {
                        self.click();
                    }
                }
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                self.input.wheel(amount);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.update(dt);
                if self.exit_requested {
                    event_loop.exit();
                    return;
                }
                self.render();
                self.present();
                self.frame_count += 1;
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
