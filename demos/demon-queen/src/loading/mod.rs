//! Pixel-art loading screens in the spirit of the classic "NOW LOADING"
//! minigames: each screen is a small scene of a character doing what they do,
//! and mashing a key (or clicking) makes them do it faster while the bar
//! fills. The scenes are ports of the Three.js cave project's `src/loading`
//! (see `tools/export-loading.mjs` there for the baked static layers).
//!
//! The loader is host-agnostic: the game feeds it time, input and whether the
//! real load has finished, and asks it for a 480×270 frame plus the numbers a
//! HUD needs. It is drawn at exactly 4× (1920×1080) with nearest sampling.
pub mod hud;
pub mod nyx;
pub mod palette;
pub mod parts;
pub mod pixel;
pub mod queen;
pub mod scenes;
pub mod sdf;

use std::rc::Rc;

use pixel::Surface;
use scenes::{Banquet, Infirmary, Scene};

/// The bar crawls like a real load, capped until the real load is done.
const CRAWL_CAP: f64 = 0.94;
/// Seconds of "¡LISTO!" shown before leaving.
const DONE_HOLD: f64 = 0.7;
/// A load that finishes before the screen has been up this long leaves at once.
const QUICK_LOAD: f64 = 0.5;

pub struct Loader {
    scenes: Vec<Box<dyn Scene>>,
    index: usize,
    progress: f64,
    done_for: f64,
    tip_index: usize,
    tip_timer: f64,
    last_mash: f64,
    clock: f64,
    counter_seen: i32,
    counter_bumped_at: f64,
    ready: bool,
}

impl Loader {
    /// `start` picks the first scene (modulo the count); `seed` drives the
    /// particles and blinks.
    pub fn new(start: usize, seed: u64) -> Self {
        let palette = Rc::new(palette::Palette::new());
        let scenes: Vec<Box<dyn Scene>> = vec![
            Box::new(Banquet::new(palette.clone(), seed)),
            Box::new(Infirmary::new(palette, seed ^ 0x9e37_79b9)),
        ];
        let index = start % scenes.len();
        let counter_seen = scenes[index].counter();
        Self {
            scenes,
            index,
            progress: 0.0,
            done_for: 0.0,
            tip_index: 0,
            tip_timer: 0.0,
            last_mash: -10.0,
            clock: 0.0,
            counter_seen,
            counter_bumped_at: -10.0,
            ready: false,
        }
    }

    /// Show another scene (`delta` steps, wrapping); the bar keeps its place.
    pub fn switch(&mut self, delta: isize) {
        let n = self.scenes.len() as isize;
        self.index = (((self.index as isize + delta) % n + n) % n) as usize;
        self.tip_index = 0;
        self.tip_timer = 0.0;
        self.counter_seen = self.scenes[self.index].counter();
    }

    /// A key press or click: the scene runs faster for a moment.
    pub fn mash(&mut self) {
        self.last_mash = self.clock;
    }

    pub fn mashing(&self) -> bool {
        self.clock - self.last_mash < 0.3
    }

    /// The real load has finished: the bar runs to the end.
    pub fn set_ready(&mut self) {
        self.ready = true;
    }

    /// True once the screen may give way to what was loading.
    pub fn can_leave(&self) -> bool {
        self.ready
            && (self.clock < QUICK_LOAD || (self.progress >= 1.0 && self.done_for >= DONE_HOLD))
    }

    pub fn tick(&mut self, dt: f64) {
        let dt = dt.clamp(0.0, 0.1);
        self.clock += dt;
        let mashing = self.mashing();
        self.scenes[self.index].update(dt, mashing);
        if self.progress < 1.0 {
            if self.ready {
                self.progress = (self.progress + dt * 1.4).min(1.0);
            } else {
                let burst = if (self.clock * 1.7).sin() > -0.3 {
                    1.0
                } else {
                    0.15
                };
                self.progress = (self.progress
                    + dt * 0.055 * burst * if mashing { 1.8 } else { 1.0 })
                .min(CRAWL_CAP);
            }
        } else {
            self.done_for += dt;
        }
        self.tip_timer += dt;
        let tips = self.scenes[self.index].tips();
        if self.tip_timer > 7.0 {
            self.tip_timer = 0.0;
            self.tip_index = (self.tip_index + 1) % tips.len();
        }
        let counter = self.scenes[self.index].counter();
        if counter != self.counter_seen {
            self.counter_seen = counter;
            self.counter_bumped_at = self.clock;
        }
    }

    /// The current 480×270 frame (opaque RGBA).
    pub fn frame(&mut self) -> &Surface {
        let t = self.clock;
        self.scenes[self.index].draw(t)
    }

    pub fn kicker(&self) -> String {
        format!("Pantalla {} / {}", self.index + 1, self.scenes.len())
    }
    pub fn title(&self) -> &'static str {
        self.scenes[self.index].title()
    }
    pub fn subtitle(&self) -> &'static str {
        self.scenes[self.index].subtitle()
    }
    pub fn counter_label(&self) -> &'static str {
        self.scenes[self.index].counter_label()
    }
    pub fn counter(&self) -> i32 {
        self.scenes[self.index].counter()
    }
    /// 0..1: how recently the counter went up (for its little pop).
    pub fn counter_bump(&self) -> f64 {
        (1.0 - (self.clock - self.counter_bumped_at) / 0.25).clamp(0.0, 1.0)
    }
    pub fn tip(&self) -> &'static str {
        let tips = self.scenes[self.index].tips();
        tips[self.tip_index % tips.len()]
    }
    pub fn progress(&self) -> f64 {
        self.progress
    }
    pub fn done(&self) -> bool {
        self.progress >= 1.0
    }
    pub fn clock(&self) -> f64 {
        self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::palette::Palette;
    use super::pixel::Surface;
    use super::*;

    fn reference(bytes: &[u8]) -> Surface {
        let image = image::load_from_memory(bytes)
            .expect("reference png")
            .into_rgba8();
        Surface::from_rgba(
            image.width() as usize,
            image.height() as usize,
            image.into_raw(),
        )
    }

    /// (differing pixels, worst channel error) between two same-size surfaces.
    fn diff(a: &Surface, b: &Surface) -> (usize, u8) {
        assert_eq!((a.w, a.h), (b.w, b.h));
        let mut count = 0;
        let mut worst = 0u8;
        for (p, q) in a.data.chunks_exact(4).zip(b.data.chunks_exact(4)) {
            // Transparent pixels carry no colour.
            if p[3] == 0 && q[3] == 0 {
                continue;
            }
            let d = (0..4).map(|k| p[k].abs_diff(q[k])).max().unwrap();
            if d > 0 {
                count += 1;
                worst = worst.max(d);
            }
        }
        (count, worst)
    }

    fn assert_matches(name: &str, ours: &Surface, theirs: &Surface, allowed: usize) {
        let (count, worst) = diff(ours, theirs);
        assert!(
            count <= allowed,
            "{name}: {count} pixels differ (worst channel error {worst})"
        );
    }

    #[test]
    fn queen_sprites_match_the_javascript_renders() {
        let palette = Palette::new();
        use super::queen::{draw_queen, QueenPose};
        let base = QueenPose {
            has_meat: true,
            meat: Some(1.0),
            ..Default::default()
        };
        let cases: [(&str, QueenPose, &[u8]); 4] = [
            (
                "rest",
                QueenPose {
                    hand: Some([85.0, 143.0]),
                    forearm: Some(21.0),
                    ..base.clone()
                },
                include_bytes!("../../data/loading/ref/queen_rest.png"),
            ),
            (
                "bite",
                QueenPose {
                    meat: Some(0.7),
                    eyes: "happy",
                    mouth: "chomp",
                    hand: Some([92.0, 88.0]),
                    forearm: Some(30.0),
                    ..base.clone()
                },
                include_bytes!("../../data/loading/ref/queen_bite.png"),
            ),
            (
                "chew",
                QueenPose {
                    meat: Some(0.35),
                    eyes: "happy",
                    mouth: "chewA",
                    puff: 1.0,
                    hy: 1.0,
                    hand: Some([88.0, 120.0]),
                    forearm: Some(26.0),
                    ..base.clone()
                },
                include_bytes!("../../data/loading/ref/queen_chew.png"),
            ),
            (
                "grab",
                QueenPose {
                    has_meat: false,
                    hand: Some([80.0, 170.0]),
                    forearm: Some(29.0),
                    goblet_hand: Some([117.0, 145.0]),
                    ..base
                },
                include_bytes!("../../data/loading/ref/queen_grab.png"),
            ),
        ];
        for (name, pose, png) in cases {
            assert_matches(name, &draw_queen(&palette, &pose), &reference(png), 0);
        }
    }

    #[test]
    fn the_comparison_notices_a_different_pose() {
        // Guards the parity tests above against passing vacuously.
        let palette = Palette::new();
        use super::queen::{draw_queen, QueenPose};
        let rest = QueenPose {
            hand: Some([85.0, 143.0]),
            forearm: Some(21.0),
            has_meat: true,
            meat: Some(1.0),
            ..Default::default()
        };
        let bite_reference = reference(include_bytes!("../../data/loading/ref/queen_bite.png"));
        assert!(diff(&draw_queen(&palette, &rest), &bite_reference).0 > 200);
    }

    #[test]
    fn nyx_sprites_match_the_javascript_renders() {
        let palette = Palette::new();
        use super::nyx::{draw_nyx, NyxPose};
        let base = NyxPose {
            level: 0.3,
            ..Default::default()
        };
        let cases: [(&str, NyxPose, &[u8]); 3] = [
            (
                "calm",
                base.clone(),
                include_bytes!("../../data/loading/ref/nyx_calm.png"),
            ),
            (
                "squeeze",
                NyxPose {
                    squeeze: 1.0,
                    level: 0.6,
                    liquid: [140.0, 120.0, 255.0],
                    glint: 0.5,
                    eyes: "blink",
                    hy: 1.0,
                    ..base.clone()
                },
                include_bytes!("../../data/loading/ref/nyx_squeeze.png"),
            ),
            (
                "happy",
                NyxPose {
                    eyes: "shut",
                    mouth: "smile",
                    level: 0.85,
                    liquid: [255.0, 150.0, 220.0],
                    hy: -1.0,
                    ..base
                },
                include_bytes!("../../data/loading/ref/nyx_happy.png"),
            ),
        ];
        for (name, pose, png) in cases {
            assert_matches(name, &draw_nyx(&palette, &pose), &reference(png), 0);
        }
    }

    #[test]
    fn fresh_scenes_match_the_javascript_frames() {
        let palette = Rc::new(Palette::new());
        let mut banquet = Banquet::new(palette.clone(), 1);
        assert_matches(
            "banquet",
            banquet.draw(1.0),
            &reference(include_bytes!("../../data/loading/ref/banquet_frame.png")),
            0,
        );
        let mut infirmary = Infirmary::new(palette, 1);
        assert_matches(
            "infirmary",
            infirmary.draw(1.0),
            &reference(include_bytes!("../../data/loading/ref/infirmary_frame.png")),
            0,
        );
    }

    #[test]
    fn mashing_speeds_the_banquet_up_and_the_counter_counts_plates() {
        let run = |mash: bool| {
            let mut loader = Loader::new(0, 7);
            for _ in 0..(30 * 12) {
                if mash {
                    loader.mash();
                }
                loader.tick(1.0 / 30.0);
                let _ = loader.frame();
            }
            loader.counter()
        };
        let (idle, mashed) = (run(false), run(true));
        assert!(idle > 3, "plates never counted: {idle}");
        assert!(
            mashed > idle,
            "mashing must eat faster ({mashed} vs {idle})"
        );
    }

    #[test]
    fn the_bar_waits_for_the_real_load_then_finishes_and_leaves() {
        let mut loader = Loader::new(1, 3);
        for _ in 0..(30 * 60) {
            loader.tick(1.0 / 30.0);
        }
        assert!(
            loader.progress() <= CRAWL_CAP + 1e-9,
            "the bar must not finish before the load"
        );
        assert!(!loader.can_leave());
        loader.set_ready();
        let mut frames = 0;
        while !loader.can_leave() {
            loader.tick(1.0 / 30.0);
            frames += 1;
            assert!(frames < 30 * 10, "never left");
        }
        assert!(loader.done());
        // A load that beats the screen leaves at once.
        let mut quick = Loader::new(0, 3);
        quick.set_ready();
        assert!(quick.can_leave());
    }
}
