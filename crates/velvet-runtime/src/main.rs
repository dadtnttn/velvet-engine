//! Velvet Runtime — packaged game host.
//!
//! Headless smoke without args (CI). With a story path argument, loads and
//! runs the product narrative path (`.vstory` or legacy story files).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use velvet_app::prelude::*;
use velvet_core::RunMode;

fn main() -> ExitCode {
    velvet_core::init_tracing_default("velvet=info,info");
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("velvet-runtime error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8> {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    match first.as_deref() {
        None | Some("--smoke") => smoke_app(),
        Some("--help") | Some("-h") => {
            print_help();
            Ok(0)
        }
        Some(path) => {
            let choice: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let max_steps: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(256);
            play_story(PathBuf::from(path), choice, max_steps)
        }
    }
}

fn print_help() {
    eprintln!(
        "velvet-runtime — Velvet Engine packaged host\n\n\
         Usage:\n\
           velvet-runtime                 Headless 1-frame app smoke\n\
           velvet-runtime --smoke         Same\n\
           velvet-runtime <story> [choice] [max_steps]\n\
             story      path to .vstory or legacy story file\n\
             choice     choice index (default 0)\n\
             max_steps  headless steps (default 256)\n"
    );
}

fn smoke_app() -> Result<u8> {
    let config = EngineConfig {
        name: "Velvet Runtime".into(),
        mode: RunMode::Production,
        ..Default::default()
    };
    let mut app = App::with_config(config);
    app.set_runner(HeadlessRunner {
        max_frames: Some(1),
        delta_secs: 1.0 / 60.0,
    });
    let code = app.run();
    Ok(code.0 as u8)
}

fn play_story(path: PathBuf, choice: usize, max_steps: u32) -> Result<u8> {
    if !path.exists() {
        bail!("story not found: {}", path.display());
    }
    let result = velvet_story_lang::run_story_path_headless(&path, choice, max_steps)
        .with_context(|| format!("run {}", path.display()))?;
    for line in &result.dialogue {
        println!("{line}");
    }
    println!(
        "=> runtime product steps={} ended={} vars={:?}",
        result.steps, result.ended, result.vars
    );
    if result.dialogue.is_empty() && !result.ended {
        bail!("no dialogue produced and story did not end");
    }
    Ok(0)
}
