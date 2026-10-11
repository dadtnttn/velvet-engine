use std::path::PathBuf;

use anyhow::Result;
use velvet_vs3_app::{
    capture_package_with_action, profile_package, run_package, validate_package, AppOptions,
    ProfileEntry,
};

pub fn cmd_app_check(path: PathBuf) -> Result<()> {
    let manifest = validate_package(&path)?;
    println!("ok: VS3 app `{}`", manifest.app.title);
    println!(
        "canvas={}x{} window={}x{} assets={} fonts={} capabilities={}",
        manifest.app.canvas_width,
        manifest.app.canvas_height,
        manifest.app.window_width,
        manifest.app.window_height,
        manifest.images.len(),
        manifest.fonts.len(),
        manifest
            .capabilities
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(())
}

pub fn cmd_app_run(
    path: PathBuf,
    headless: bool,
    frames: u64,
    save_dir: Option<PathBuf>,
) -> Result<()> {
    run_package(
        path,
        AppOptions {
            headless,
            frames: if frames == 0 { 60 } else { frames },
            save_dir,
        },
    )
}

pub fn cmd_app_profile(path: PathBuf, actions: Vec<String>, frames: u64, top: usize) -> Result<()> {
    let report = profile_package(&path, &actions, frames)?;
    print_profile(&report, top);
    Ok(())
}

/// Print a profile table: self instructions, share of the total, calls.
pub fn print_profile(report: &[ProfileEntry], top: usize) {
    let total: u64 = report.iter().map(|row| row.instructions).sum();
    println!(
        "{:>14} {:>6} {:>10}  function",
        "instructions", "%", "calls"
    );
    for row in report.iter().take(top) {
        let share = if total == 0 {
            0.0
        } else {
            row.instructions as f64 * 100.0 / total as f64
        };
        println!(
            "{:>14} {:>5.1}% {:>10}  {}",
            row.instructions, share, row.calls, row.name
        );
    }
    println!("{total:>14} total ({} functions)", report.len());
}

pub fn cmd_app_capture(
    path: PathBuf,
    output: PathBuf,
    frames: u64,
    action: Option<String>,
) -> Result<()> {
    capture_package_with_action(path, &output, frames, action.as_deref())?;
    println!("capture={}", output.display());
    Ok(())
}
