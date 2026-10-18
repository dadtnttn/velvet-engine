//! The `velvet-desktop` program: runs a JavaScript game in a desktop window.
//!
//! Usage: `velvet-desktop <game-folder> [--dev <url>] [--devtools] [--size WxH]
//! [--capture <file.png> [--after <ms>]]`, or with no arguments the game in `game/` next to the
//! program (a release folder). See the crate README.
//!
//! A release build on Windows is a window program, without a console: its messages go to
//! `velvet-desktop.log` in the game's data folder (see `velvet_desktop::log`).

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::Path;
use std::process::ExitCode;

use anyhow::anyhow;

use velvet_desktop::app::{self, Launch};
use velvet_desktop::cli::{self, Command};
use velvet_desktop::Manifest;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Every error message already includes its cause, so the top message is enough.
            velvet_desktop::log::line(&format!("velvet-desktop: {error}"));
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let args = std::env::args_os()
        .skip(1)
        .map(|arg| {
            arg.into_string()
                .map_err(|_| anyhow!("command-line arguments must be valid Unicode"))
        })
        .collect::<anyhow::Result<Vec<String>>>()?;

    // A release keeps its game in `game/` next to the program, which is what a bare start runs.
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));

    match cli::parse(args, exe_dir.as_deref())? {
        Command::Help => {
            println!("{}", cli::USAGE);
            Ok(())
        }
        Command::Run(options) => {
            // The manifest error already names the file that failed.
            let manifest = Manifest::load(&options.game_dir)?;
            app::run(Launch {
                game_root: options.game_dir,
                manifest,
                dev_url: options.dev_url,
                devtools: options.devtools,
                size: options.size,
                capture: options.capture,
            })?;
            Ok(())
        }
    }
}
