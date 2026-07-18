//! Shared test utilities for Velvet Engine crates.

#![deny(missing_docs)]

use velvet_app::prelude::*;

/// Build a minimal app with a headless runner limited to `frames`.
pub fn headless_app(frames: u64) -> App {
    let mut app = App::new();
    app.set_runner(HeadlessRunner {
        max_frames: Some(frames),
        delta_secs: 1.0 / 60.0,
    });
    app
}

/// Run `frames` empty updates and return the final frame count from [`Time`].
pub fn run_empty_frames(frames: u64) -> u64 {
    let mut app = headless_app(frames);
    let _ = app.run();
    app.resource::<Time>().map(|t| t.frame_count()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_frames() {
        assert_eq!(run_empty_frames(3), 3);
    }
}
