//! Where the program's messages go.
//!
//! Every line goes to the terminal, and once [`open`] has run, also to `velvet-desktop.log` in the
//! game's data folder (started afresh on each run). A release build on Windows has no console, so
//! the file is the only place its messages and the game's `velvet.log` calls can be read.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The log file's name inside the data folder.
pub const LOG_FILE: &str = "velvet-desktop.log";

static FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Starts the log file in `data_dir` (creating the folder). Returns its path, or `None` when it
/// cannot be written: the terminal still gets every line.
pub fn open(data_dir: &Path) -> Option<PathBuf> {
    let path = data_dir.join(LOG_FILE);
    std::fs::create_dir_all(data_dir).ok()?;
    let file = File::create(&path).ok()?;
    FILE.set(Mutex::new(file)).ok()?;
    Some(path)
}

/// Writes one line to the terminal and to the log file.
pub fn line(text: &str) {
    eprintln!("{text}");
    if let Some(file) = FILE.get() {
        if let Ok(mut file) = file.lock() {
            let _ = writeln!(file, "{text}");
        }
    }
}
