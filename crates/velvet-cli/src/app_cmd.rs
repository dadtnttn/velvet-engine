use std::path::PathBuf;

use anyhow::Result;
use velvet_vs3_app::{capture_package_with_action, run_package, validate_package, AppOptions};

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

pub fn cmd_app_run(path: PathBuf, headless: bool, frames: u64) -> Result<()> {
    run_package(
        path,
        AppOptions {
            headless,
            frames: if frames == 0 { 60 } else { frames },
        },
    )
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
