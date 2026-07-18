//! Optional Steamworks surface for host export config.
//!
//! Init / achievement / presence hooks call without panicking when the Steam
//! client is absent (loopback no-op backend). Feature `steam` can later wire a
//! real SDK; default builds use this safe stub.

use serde::{Deserialize, Serialize};

/// Steam integration status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteamStatus {
    /// Whether a Steam client appears available.
    pub client_available: bool,
    /// App id from config (0 = unset).
    pub app_id: u32,
    /// Backend: `none` | `stub` | `steamworks` (feature).
    pub backend: String,
    /// Last error or note.
    pub note: String,
}

/// Host export / runtime Steam config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteamConfig {
    /// Steam App ID.
    pub app_id: u32,
    /// Enable Steam integration when client present.
    pub enabled: bool,
}

impl Default for SteamConfig {
    fn default() -> Self {
        Self {
            app_id: 0,
            enabled: true,
        }
    }
}

/// In-memory Steam hook (no panic without client).
#[derive(Debug, Clone)]
pub struct SteamHook {
    /// Status after init.
    pub status: SteamStatus,
    /// Unlocked achievements (local mirror).
    unlocked: Vec<String>,
    /// Rich presence key/values.
    presence: Vec<(String, String)>,
}

impl SteamHook {
    /// Initialize Steam for the given config. Never panics if Steam is missing.
    pub fn init(config: &SteamConfig) -> Self {
        if !config.enabled {
            return Self {
                status: SteamStatus {
                    client_available: false,
                    app_id: config.app_id,
                    backend: "none".into(),
                    note: "steam disabled in config".into(),
                },
                unlocked: Vec::new(),
                presence: Vec::new(),
            };
        }
        let client_available = detect_steam_client();
        let (backend, note) = if client_available {
            (
                "stub".to_string(),
                "steam client detected; using stub hooks (no steamworks feature)".into(),
            )
        } else {
            (
                "stub".to_string(),
                "steam client absent; achievement/presence are local no-ops".into(),
            )
        };
        Self {
            status: SteamStatus {
                client_available,
                app_id: config.app_id,
                backend,
                note,
            },
            unlocked: Vec::new(),
            presence: Vec::new(),
        }
    }

    /// Unlock an achievement id. Returns whether recorded (always true on stub).
    pub fn set_achievement(&mut self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        if !self.unlocked.iter().any(|a| a == id) {
            self.unlocked.push(id.to_string());
        }
        true
    }

    /// Whether achievement was unlocked this session.
    pub fn is_achievement_unlocked(&self, id: &str) -> bool {
        self.unlocked.iter().any(|a| a == id)
    }

    /// Set rich presence key.
    pub fn set_presence(&mut self, key: &str, value: &str) -> bool {
        if key.is_empty() {
            return false;
        }
        if let Some(slot) = self.presence.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value.into();
        } else {
            self.presence.push((key.into(), value.into()));
        }
        true
    }

    /// Read presence.
    pub fn presence(&self) -> &[(String, String)] {
        &self.presence
    }

    /// Unlocked list.
    pub fn unlocked(&self) -> &[String] {
        &self.unlocked
    }
}

fn detect_steam_client() -> bool {
    // Non-invasive: check common Steam install env / path without loading DLL.
    if std::env::var_os("SteamAppId").is_some() || std::env::var_os("SteamOverlayGameId").is_some()
    {
        return true;
    }
    let candidates = [
        r"C:\Program Files (x86)\Steam\steam.exe",
        r"C:\Program Files\Steam\steam.exe",
        "/usr/bin/steam",
        "/usr/games/steam",
    ];
    candidates.iter().any(|p| std::path::Path::new(p).is_file())
}

/// Write a steam_appid.txt next to an export binary (Steam requirement).
pub fn write_steam_appid_file(out_dir: &std::path::Path, app_id: u32) -> std::io::Result<()> {
    std::fs::write(out_dir.join("steam_appid.txt"), format!("{app_id}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_without_client_does_not_panic_and_achievements_work() {
        let mut hook = SteamHook::init(&SteamConfig {
            app_id: 480,
            enabled: true,
        });
        assert_eq!(hook.status.app_id, 480);
        assert!(hook.status.backend == "stub" || hook.status.backend == "none");
        assert!(hook.set_achievement("FIRST_ENDING"));
        assert!(hook.is_achievement_unlocked("FIRST_ENDING"));
        assert!(hook.set_presence("story", "chapter_1"));
        assert!(hook.presence().iter().any(|(k, v)| k == "story" && v == "chapter_1"));
        assert!(!hook.set_achievement(""));
    }

    #[test]
    fn disabled_config() {
        let hook = SteamHook::init(&SteamConfig {
            app_id: 0,
            enabled: false,
        });
        assert_eq!(hook.status.backend, "none");
    }
}
