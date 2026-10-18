//! Writes files so that a reader never sees half of one: each file is written under a temporary
//! name in the same folder, then moved into place. A failed write removes its temporary file, and
//! nothing else is ever deleted.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Makes the counter that keeps the temporary names of one process apart.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The temporary name of a file: a dot file in the same folder, so that a leftover from a crash shows
/// up as an orphan in the report instead of looking like art.
fn temporary_path(target: &Path) -> PathBuf {
    let name = target.file_name().map_or_else(
        || "file".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = format!(".{name}.{}-{unique}.tmp", std::process::id());
    target.with_file_name(temporary)
}

/// Writes a file atomically. `write` is given the temporary path and must create the file there. When
/// it succeeds, the file is moved to `target`, replacing any file that is already there.
///
/// The folder of `target` is created when it is missing.
///
/// # Errors
/// Returns the reason when the folder cannot be made, when `write` fails, or when the file cannot be
/// moved. The temporary file is removed in every failing case.
pub fn write_atomic(
    target: &Path,
    write: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(folder) = target.parent() {
        fs::create_dir_all(folder)
            .map_err(|error| format!("cannot create {}: {error}", folder.display()))?;
    }
    let temporary = temporary_path(target);
    let result = write(&temporary).and_then(|()| {
        fs::rename(&temporary, target)
            .map_err(|error| format!("cannot move the file into place: {error}"))
    });
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_appears_only_once_it_is_complete_and_replaces_the_old_one() {
        let folder =
            std::env::temp_dir().join(format!("velvet-artbuild-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let target = folder.join("sub").join("picture.png");
        write_atomic(&target, |tmp| {
            fs::write(tmp, b"first").map_err(|e| e.to_string())
        })
        .unwrap();
        write_atomic(&target, |tmp| {
            fs::write(tmp, b"second").map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"second");
        let names: Vec<String> = fs::read_dir(target.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["picture.png".to_string()]);
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_failed_write_leaves_no_temporary_file_and_keeps_the_old_file() {
        let folder =
            std::env::temp_dir().join(format!("velvet-artbuild-failed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let target = folder.join("picture.png");
        write_atomic(&target, |tmp| {
            fs::write(tmp, b"kept").map_err(|e| e.to_string())
        })
        .unwrap();
        let failure = write_atomic(&target, |tmp| {
            fs::write(tmp, b"partial").map_err(|e| e.to_string())?;
            Err("encoder stopped".to_string())
        });
        assert_eq!(failure.unwrap_err(), "encoder stopped");
        assert_eq!(fs::read(&target).unwrap(), b"kept");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&folder);
    }
}
