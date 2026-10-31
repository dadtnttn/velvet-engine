//! A small 2D game on `velvet-host2d` that runs on desktop and on Android with the same code:
//! tap (or click) to ring a chime and leave a ripple, drag to paint a trail, back (or Esc) to
//! quit. The host turns the first finger into the pointer and the back button into Escape;
//! sounds play through rodio (AAudio on Android).
//!
//! ```text
//! cargo run -p hello-mobile                      (desktop)
//! velvet android apk -p hello-mobile             (Android: a debug-signed APK)
//! ```

use std::time::Duration;

use velvet_host2d::velvet_canvas::{Align, Canvas, Color, Font, Point, StrokeStyle, TextStyle};
use velvet_host2d::velvet_platform::PlatformEvent;
use velvet_host2d::{Button, Game2D, HostConfig, Input, InputEvent, KeyCode};
use velvet_sfx::{presets, Player, Sound};

const FONT: &[u8] =
    include_bytes!("../../../crates/velvet-canvas/tests/fixtures/Nunito-ExtraBold.ttf");

/// How long a ripple lasts, in seconds.
const RIPPLE: f32 = 1.2;

/// The game: ripples where the player tapped, and the trail of the current drag.
pub struct Hello {
    ripples: Vec<(Point, f32)>,
    trail: Vec<Point>,
    chime: Sound,
    player: Option<Player>,
    label: TextStyle,
    quit: bool,
    /// Taps since the start (shown on screen).
    taps: u32,
    /// Times the app came back from the background.
    returns: u32,
}

impl Hello {
    /// A game that plays its sounds through `player` (none: silent, as in tests).
    pub fn new(player: Option<Player>) -> Self {
        let font = Font::from_bytes(FONT.to_vec()).expect("the bundled font");
        Self {
            ripples: Vec::new(),
            trail: Vec::new(),
            chime: presets::chime(),
            player,
            label: TextStyle::new(&font, 34.0, Color::hex(0xe8c27a)).align(Align::Center),
            quit: false,
            taps: 0,
            returns: 0,
        }
    }

    /// Ripples on screen.
    pub fn ripples(&self) -> usize {
        self.ripples.len()
    }
}

impl Game2D for Hello {
    fn update(&mut self, input: &Input, dt: f32) {
        for event in input.events() {
            match event {
                InputEvent::PointerDown {
                    button: Button::Left,
                    at,
                } => {
                    self.ripples.push((*at, 0.0));
                    self.trail = vec![*at];
                    self.taps += 1;
                    if let Some(player) = &mut self.player {
                        player.play("chime", &self.chime, 0.8, Duration::from_millis(60));
                    }
                }
                InputEvent::PointerMoved(at) if input.button_down(Button::Left) => {
                    self.trail.push(*at);
                }
                InputEvent::KeyDown(KeyCode::Escape) => self.quit = true,
                _ => {}
            }
        }
        for ripple in &mut self.ripples {
            ripple.1 += dt;
        }
        self.ripples.retain(|r| r.1 < RIPPLE);
        if self.trail.len() > 600 {
            self.trail.drain(..self.trail.len() - 600);
        }
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        canvas.clear(Color::hex(0x1a1326));
        let ink = StrokeStyle::new(6.0, Color::hex(0xb05cd6));
        for pair in self.trail.windows(2) {
            canvas.line(pair[0], pair[1], &ink);
        }
        for (at, age) in &self.ripples {
            let t = age / RIPPLE;
            canvas.stroke_circle(
                *at,
                30.0 + 220.0 * t,
                &StrokeStyle::new(5.0, Color::hex(0xe8c27a).fade(1.0 - t)),
            );
        }
        let w = canvas.width();
        canvas.text(
            "Tap to ring  ·  drag to paint  ·  back to quit",
            w / 2.0,
            90.0,
            &self.label,
        );
        canvas.text(
            &format!(
                "{} taps · back from the background {} times",
                self.taps, self.returns
            ),
            w / 2.0,
            canvas.height() - 80.0,
            &self.label,
        );
    }

    fn platform_event(&mut self, event: PlatformEvent) {
        match event {
            // A drag in flight never ends while the app is away.
            PlatformEvent::Suspended => self.trail.clear(),
            PlatformEvent::Resumed => self.returns += 1,
            _ => {}
        }
    }

    fn quit(&self) -> bool {
        self.quit
    }
}

/// The game in a window (desktop) or full screen (Android).
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    velvet_host2d::run(
        HostConfig::new("Hello Mobile"),
        Hello::new(Some(Player::new())),
    )
}

velvet_platform::android_main!(run);

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_host2d::Headless;

    #[test]
    fn a_tap_rings_and_escape_quits() {
        let mut run = Headless::new(HostConfig::new("t"), 1280, 720);
        let mut game = Hello::new(None);
        run.click(960.0, 540.0);
        run.step(&mut game);
        assert_eq!(game.ripples(), 1);
        run.run_for(&mut game, RIPPLE + 0.1);
        assert_eq!(game.ripples(), 0);
        run.draw(&mut game);
        assert!(!game.quit());
        run.push(InputEvent::KeyDown(KeyCode::Escape));
        run.step(&mut game);
        assert!(game.quit());
    }
}
