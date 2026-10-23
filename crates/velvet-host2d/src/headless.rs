//! The same game without a window: fixed steps, scripted input and drawn frames, for tests, bots and captures.

use velvet_canvas::{Canvas, Point};

use crate::input::{Button, Input, InputEvent};
use crate::{Game2D, HostConfig};

/// A windowless run of a game, at a chosen canvas size.
pub struct Headless {
    pub config: HostConfig,
    pub canvas: Canvas,
    pub input: Input,
    steps: u64,
}

impl Headless {
    /// A run whose canvas is drawn at `pw`×`ph` pixels.
    pub fn new(config: HostConfig, pw: u32, ph: u32) -> Self {
        let canvas = Canvas::with_pixels(config.logical_width, config.logical_height, pw, ph);
        Self {
            config,
            canvas,
            input: Input::new(),
            steps: 0,
        }
    }

    /// Records what the player does (it reaches the game at the next step).
    pub fn push(&mut self, event: InputEvent) -> &mut Self {
        self.input.push(event);
        self
    }

    /// Moves the pointer to a logical point.
    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.push(InputEvent::PointerMoved(Point::new(x, y)))
    }

    /// A press and a release of the left button at a logical point, in the same step.
    pub fn click(&mut self, x: f32, y: f32) -> &mut Self {
        let at = Point::new(x, y);
        self.push(InputEvent::PointerDown {
            button: Button::Left,
            at,
        });
        self.push(InputEvent::PointerUp {
            button: Button::Left,
            at,
        })
    }

    /// One fixed step of the game.
    pub fn step<G: Game2D>(&mut self, game: &mut G) {
        game.update(&self.input, self.config.step);
        self.input.end_step(self.config.step);
        self.steps += 1;
    }

    /// Steps for a span of game time.
    pub fn run_for<G: Game2D>(&mut self, game: &mut G, seconds: f32) {
        let n = (seconds / self.config.step).round().max(1.0) as u64;
        for _ in 0..n {
            self.step(game);
        }
    }

    /// Draws a frame and gives the canvas.
    pub fn draw<G: Game2D>(&mut self, game: &mut G) -> &Canvas {
        game.draw(&mut self.canvas);
        &self.canvas
    }

    /// The steps run so far.
    pub fn steps(&self) -> u64 {
        self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_canvas::{Color, Rect};

    /// Counts clicks, and draws a square where the last one was.
    #[derive(Default)]
    struct Clicks {
        clicks: u32,
        last: Option<Point>,
    }

    impl Game2D for Clicks {
        fn update(&mut self, input: &Input, _dt: f32) {
            for e in input.events() {
                if let InputEvent::PointerUp { at, .. } = e {
                    self.clicks += 1;
                    self.last = Some(*at);
                }
            }
        }

        fn draw(&mut self, canvas: &mut Canvas) {
            canvas.clear(Color::BLACK);
            if let Some(p) = self.last {
                canvas.fill_rect(
                    Rect::new(p.x - 10.0, p.y - 10.0, 20.0, 20.0),
                    &Color::WHITE.into(),
                );
            }
        }
    }

    #[test]
    fn scripted_clicks_reach_the_game_and_frames_are_drawn_at_the_asked_pixels() {
        let mut run = Headless::new(HostConfig::new("t"), 960, 540);
        let mut game = Clicks::default();
        run.click(100.0, 200.0);
        run.step(&mut game);
        run.step(&mut game);
        assert_eq!(game.clicks, 1, "an event reaches one step only");
        let canvas = run.draw(&mut game);
        assert_eq!((canvas.pixel_width(), canvas.pixel_height()), (960, 540));
        assert_eq!(
            canvas.pixel(50, 100),
            Some([255, 255, 255, 255]),
            "logical (100, 200) at half size"
        );
        run.run_for(&mut game, 1.0);
        assert_eq!(run.steps(), 62);
        assert!((run.input.time - 62.0 / 60.0).abs() < 1e-4);
    }
}
