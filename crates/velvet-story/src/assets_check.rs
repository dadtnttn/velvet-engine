//! Project asset check: every file a story references must exist.
//!
//! Scans backgrounds, music, sfx, character portraits, `gallery.json` and
//! `music.json` entries against the project directory. Authors run it before
//! play/export so a missing png is a checklist line, not a black screen.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ir::{StoryOp, StoryProgram};

/// One missing reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIssue {
    /// Kind: `background` | `music` | `sound` | `portrait` | `gallery` | `music-catalog` | `catalog` | `screen-image`.
    pub kind: String,
    /// Referenced path as written.
    pub reference: String,
    /// Scene or catalog id (`<characters>` / `<gallery>` / `<music>` for non-scene).
    pub scene: String,
}

impl AssetIssue {
    /// One-line display.
    pub fn display(&self) -> String {
        format!("{} `{}` (from {})", self.kind, self.reference, self.scene)
    }
}

/// Asset check result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReport {
    /// Missing references (empty = ok).
    pub missing: Vec<AssetIssue>,
    /// Files checked on disk.
    pub checked: usize,
    /// Catalog files found (`gallery.json`, `music.json`, `voice_cast.json`).
    pub catalogs: Vec<String>,
}

impl AssetReport {
    /// No missing files.
    pub fn is_ok(&self) -> bool {
        self.missing.is_empty()
    }

    /// Human summary for CLI / Studio console.
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "assets checked={} missing={} catalogs=[{}]\n",
            self.checked,
            self.missing.len(),
            self.catalogs.join(", ")
        );
        if self.missing.is_empty() {
            out.push_str("assets: ok\n");
        } else {
            for m in &self.missing {
                out.push_str(&format!("missing: {}\n", m.display()));
            }
        }
        out
    }
}

/// Check all asset references in `program` against `root` directory.
pub fn check_project_assets(root: &Path, program: &StoryProgram) -> AssetReport {
    let mut report = AssetReport::default();
    let mut need = |kind: &str, reference: &str, scene: &str| {
        if reference.trim().is_empty() {
            return;
        }
        report.checked += 1;
        // Cinema clips are frame directories (a lone file also passes).
        let full = root.join(reference);
        let ok = if kind == "movie" {
            full.is_dir() || full.is_file()
        } else {
            full.is_file()
        };
        if !ok {
            report.missing.push(AssetIssue {
                kind: kind.into(),
                reference: reference.into(),
                scene: scene.into(),
            });
        }
    };

    for (scene_name, scene) in &program.scenes {
        check_ops(&scene.ops, scene_name, &mut need);
    }
    for (screen_id, def) in &program.screens {
        for image in &def.images {
            need("screen-image", image, &format!("<screen {screen_id}>"));
        }
    }
    for (id, ch) in &program.characters {
        if let Some(p) = ch.portrait.as_deref() {
            need("portrait", p, &format!("<character {id}>"));
        }
        for expr_path in ch.expressions.values() {
            // Expression values may be asset paths (`happy = "luna/happy.png"`)
            // or tags; only check values that look like files.
            if looks_like_file(expr_path) {
                need("portrait", expr_path, &format!("<character {id}>"));
            }
        }
    }

    check_gallery_catalog(root, &mut report);
    check_music_catalog(root, &mut report);
    check_voice_catalog(root, &mut report);
    report
}

fn check_ops(ops: &[StoryOp], scene: &str, need: &mut impl FnMut(&str, &str, &str)) {
    for op in ops {
        match op {
            StoryOp::Background { path } => need("background", path, scene),
            StoryOp::Music { path, .. } => need("music", path, scene),
            StoryOp::Sound { path } => need("sound", path, scene),
            StoryOp::Movie { path, .. } => need("movie", path, scene),
            StoryOp::Choice { options } => {
                for arm in options {
                    check_ops(&arm.body, scene, need);
                }
            }
            StoryOp::If {
                then_ops, else_ops, ..
            } => {
                check_ops(then_ops, scene, need);
                check_ops(else_ops, scene, need);
            }
            _ => {}
        }
    }
}

fn check_gallery_catalog(root: &Path, report: &mut AssetReport) {
    let path = root.join("gallery.json");
    if !path.is_file() {
        return;
    }
    report.catalogs.push("gallery.json".into());
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            report.missing.push(AssetIssue {
                kind: "catalog".into(),
                reference: "gallery.json".into(),
                scene: "<gallery>".into(),
            });
            return;
        }
    };
    match crate::gallery::Gallery::from_json(&text) {
        Ok(g) => {
            for (id, entry) in &g.entries {
                report.checked += 1;
                if !root.join(&entry.path).is_file() {
                    report.missing.push(AssetIssue {
                        kind: "gallery".into(),
                        reference: entry.path.clone(),
                        scene: format!("<gallery {id}>"),
                    });
                }
            }
        }
        Err(e) => report.missing.push(AssetIssue {
            kind: "catalog".into(),
            reference: format!("gallery.json: {e}"),
            scene: "<gallery>".into(),
        }),
    }
}

fn check_music_catalog(root: &Path, report: &mut AssetReport) {
    let path = root.join("music.json");
    if !path.is_file() {
        return;
    }
    report.catalogs.push("music.json".into());
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            report.missing.push(AssetIssue {
                kind: "catalog".into(),
                reference: "music.json".into(),
                scene: "<music>".into(),
            });
            return;
        }
    };
    match crate::music_room::MusicRoom::from_json(&text) {
        Ok(m) => {
            for (id, track) in &m.tracks {
                report.checked += 1;
                if !root.join(&track.path).is_file() {
                    report.missing.push(AssetIssue {
                        kind: "music-catalog".into(),
                        reference: track.path.clone(),
                        scene: format!("<music {id}>"),
                    });
                }
            }
        }
        Err(e) => report.missing.push(AssetIssue {
            kind: "catalog".into(),
            reference: format!("music.json: {e}"),
            scene: "<music>".into(),
        }),
    }
}

fn check_voice_catalog(root: &Path, report: &mut AssetReport) {
    let path = root.join("voice_cast.json");
    if !path.is_file() {
        return;
    }
    report.catalogs.push("voice_cast.json".into());
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            report.missing.push(AssetIssue {
                kind: "catalog".into(),
                reference: "voice_cast.json".into(),
                scene: "<voice>".into(),
            });
            return;
        }
    };
    if crate::voice_cast::VoiceCast::from_json(&text).is_err() {
        report.missing.push(AssetIssue {
            kind: "catalog".into(),
            reference: "voice_cast.json: invalid json".into(),
            scene: "<voice>".into(),
        });
    }
}

/// Heuristic: expression values with an image/audio extension are file paths.
fn looks_like_file(value: &str) -> bool {
    let v = value.to_ascii_lowercase();
    v.ends_with(".png")
        || v.ends_with(".jpg")
        || v.ends_with(".jpeg")
        || v.ends_with(".webp")
        || v.ends_with(".ogg")
        || v.ends_with(".mp3")
        || v.ends_with(".wav")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_program_from_source;

    fn prog() -> StoryProgram {
        let src = r#"
character hero { name: "H" portrait: "characters/hero.png" }
scene main {
    background "assets/bg/room.png"
    music "assets/music/soft.ogg" fade_in 1
    show hero at right
    hero "Hi."
    choice {
        "A" { jump good }
        "B" { jump bad }
    }
}
scene good { "Ending: G" }
scene bad { sound "assets/sfx/no.ogg" "Ending: B" }
"#;
        load_program_from_source(src, Some("a.vel"), "A").unwrap()
    }

    #[test]
    fn missing_files_reported() {
        let dir = tempfile::tempdir().unwrap();
        let report = check_project_assets(dir.path(), &prog());
        assert!(!report.is_ok());
        assert!(report.missing.iter().any(|m| m.kind == "background"));
        assert!(report.missing.iter().any(|m| m.kind == "music"));
        assert!(report.missing.iter().any(|m| m.kind == "sound"));
        assert!(report.missing.iter().any(|m| m.kind == "portrait"));
        assert!(report.to_text().contains("missing:"));
    }

    #[test]
    fn existing_files_pass() {
        let dir = tempfile::tempdir().unwrap();
        for f in [
            "assets/bg/room.png",
            "assets/music/soft.ogg",
            "assets/sfx/no.ogg",
            "characters/hero.png",
        ] {
            let p = dir.path().join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        let report = check_project_assets(dir.path(), &prog());
        assert!(report.is_ok(), "{}", report.to_text());
        assert_eq!(report.checked, 4);
    }

    #[test]
    fn screen_images_checked() {
        let dir = tempfile::tempdir().unwrap();
        let src = "screen hud {\n image { path: \"ui/hud.png\" }\n}\nscene main {\n screen show hud\n \"x\"\n}\n";
        let program = load_program_from_source(src, Some("h.vel"), "H").unwrap();
        let report = check_project_assets(dir.path(), &program);
        assert!(report.missing.iter().any(|m| m.kind == "screen-image"));
        let p = dir.path().join("ui/hud.png");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
        let report = check_project_assets(dir.path(), &program);
        assert!(report.is_ok(), "{}", report.to_text());
    }

    #[test]
    fn gallery_catalog_missing_entry_reported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gallery.json"),
            r#"{"entries":[{"id":"cg1","title":"T","path":"cg/a.png"}]}"#,
        )
        .unwrap();
        let mut program = prog();
        program.scenes.clear();
        program.entry = "main".into();
        program.scenes.insert(
            "main".into(),
            crate::ir::StoryScene {
                name: "main".into(),
                ops: vec![],
                labels: Default::default(),
            },
        );
        let report = check_project_assets(dir.path(), &program);
        assert!(report.missing.iter().any(|m| m.kind == "gallery"));
        assert!(report.catalogs.contains(&"gallery.json".to_string()));
    }
}
