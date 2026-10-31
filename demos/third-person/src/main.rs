//! Third-person vertical slice (desktop binary).
//!
//! `--headless` runs a scripted walk (CI-friendly, exit 0 on quest
//! completion). Without flags it opens a wgpu window (WASD/arrows move,
//! Space jumps, Esc quits); rendering reuses `velvet-render` through the
//! scene bridge, so the sim stays the source of truth in both modes.
//!
//! The program itself is the `third_person` library, which the Android build
//! runs too.

fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--headless") {
        return third_person::run_headless();
    }
    third_person::run()
}
