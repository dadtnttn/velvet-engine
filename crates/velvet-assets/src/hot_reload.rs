//! File mtime tracking and change polling for hot-reload.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Watched file entry.
#[derive(Debug, Clone)]
struct WatchEntry {
    path: PathBuf,
    modified: Option<SystemTime>,
    /// Logical asset key (virtual path string).
    key: String,
}

/// Tracks filesystem mtimes and reports changed paths on poll.
#[derive(Debug, Default)]
pub struct HotReloader {
    entries: HashMap<String, WatchEntry>,
    /// Paths that changed since last poll (drained by [`Self::drain_changed`]).
    pending: Vec<String>,
    /// When false, poll is a no-op.
    pub enabled: bool,
}

impl HotReloader {
    /// Create enabled reloader.
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create disabled reloader.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Number of watched files.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Watch a filesystem path under a logical key (e.g. virtual path).
    pub fn watch(&mut self, key: impl Into<String>, path: impl Into<PathBuf>) {
        let key = key.into();
        let path = path.into();
        let modified = read_mtime(&path);
        self.entries.insert(
            key.clone(),
            WatchEntry {
                path,
                modified,
                key,
            },
        );
    }

    /// Stop watching a key.
    pub fn unwatch(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Whether a key is watched.
    pub fn is_watching(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Poll all watched files; returns keys whose mtime changed.
    ///
    /// Changed keys are also queued until [`Self::drain_changed`].
    pub fn poll(&mut self) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }
        let mut changed = Vec::new();
        for entry in self.entries.values_mut() {
            let now = read_mtime(&entry.path);
            if mtime_changed(entry.modified, now) {
                entry.modified = now;
                changed.push(entry.key.clone());
            }
        }
        for key in &changed {
            if !self.pending.iter().any(|p| p == key) {
                self.pending.push(key.clone());
            }
        }
        changed
    }

    /// Drain pending change list (accumulated across polls).
    pub fn drain_changed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending)
    }

    /// Force-mark a key as changed without reading disk.
    pub fn mark_changed(&mut self, key: &str) {
        if self.entries.contains_key(key) && !self.pending.iter().any(|p| p == key) {
            self.pending.push(key.to_string());
        }
    }

    /// Refresh stored mtime from disk without emitting a change.
    pub fn touch_baseline(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.modified = read_mtime(&entry.path);
        }
    }

    /// Filesystem path for a key, if watched.
    pub fn path_of(&self, key: &str) -> Option<&Path> {
        self.entries.get(key).map(|e| e.path.as_path())
    }
}

fn read_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn mtime_changed(prev: Option<SystemTime>, now: Option<SystemTime>) -> bool {
    match (prev, now) {
        (Some(a), Some(b)) => a != b,
        (None, Some(_)) => true, // file appeared
        (Some(_), None) => true, // file disappeared
        (None, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn detects_write() {
        let dir = std::env::temp_dir().join(format!("velvet_hot_reload_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("sample.txt");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "v1").unwrap();
        }
        let mut hr = HotReloader::new();
        hr.watch("sample", &path);
        // baseline set; no change yet
        assert!(hr.poll().is_empty());

        // Ensure mtime can advance on coarse FS clocks.
        std::thread::sleep(std::time::Duration::from_millis(20));
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "v2").unwrap();
        }
        // Some filesystems have 1s mtime resolution — bump via filetime alternative:
        // rewrite again after sleep if first poll empty.
        let mut changed = hr.poll();
        if changed.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(1100));
            {
                let mut f = std::fs::File::create(&path).unwrap();
                writeln!(f, "v3").unwrap();
            }
            changed = hr.poll();
        }
        assert!(changed.iter().any(|k| k == "sample"));
        let drained = hr.drain_changed();
        assert!(drained.iter().any(|k| k == "sample"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disabled_no_poll() {
        let mut hr = HotReloader::disabled();
        hr.watch("x", PathBuf::from("nope.bin"));
        assert!(hr.poll().is_empty());
    }

    #[test]
    fn mark_changed_without_disk() {
        let mut hr = HotReloader::new();
        hr.watch("k", PathBuf::from("missing_on_purpose.dat"));
        hr.mark_changed("k");
        assert_eq!(hr.drain_changed(), vec!["k".to_string()]);
    }
}
