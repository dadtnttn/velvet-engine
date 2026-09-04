//! Asset checking for `Scene3D`: every `.glb` / `.gltf` a scene
//! references must exist under the project directory.
//!
//! Mirrors the VN `check-assets` contract: authors run it before
//! play/export so a missing model is a checklist line, not a black screen.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::doc::{MeshKind3D, Scene3D, SceneNode3D};

/// One missing model reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIssue3D {
    /// Always `model`.
    pub kind: String,
    /// Referenced path as written.
    pub reference: String,
    /// Slash-joined node path (`a/b/c`).
    pub node: String,
}

impl AssetIssue3D {
    /// One-line display.
    pub fn display(&self) -> String {
        format!("{} `{}` (from {})", self.kind, self.reference, self.node)
    }
}

/// Asset check result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetReport3D {
    /// Missing references (empty = ok).
    pub missing: Vec<AssetIssue3D>,
    /// Model references checked on disk.
    pub checked: usize,
}

impl AssetReport3D {
    /// No missing files.
    pub fn is_ok(&self) -> bool {
        self.missing.is_empty()
    }

    /// Human summary for CLI / Studio console.
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "models checked={} missing={}\n",
            self.checked,
            self.missing.len()
        );
        if self.missing.is_empty() {
            out.push_str("models: ok\n");
        } else {
            for m in &self.missing {
                out.push_str(&format!("missing: {}\n", m.display()));
            }
        }
        out
    }
}

/// Check all `.glb` / `.gltf` references in `scene` against `root`.
pub fn check_scene_assets(root: &Path, scene: &Scene3D) -> AssetReport3D {
    let mut report = AssetReport3D::default();
    for node in &scene.nodes {
        check_node(root, node, &node.id, &mut report);
    }
    report
}

fn check_node(root: &Path, node: &SceneNode3D, path: &str, report: &mut AssetReport3D) {
    if let MeshKind3D::Gltf { path: asset } = &node.mesh {
        if !asset.trim().is_empty() {
            report.checked += 1;
            if !root.join(asset).is_file() {
                report.missing.push(AssetIssue3D {
                    kind: "model".into(),
                    reference: asset.clone(),
                    node: path.to_string(),
                });
            }
        }
    }
    for child in &node.children {
        check_node(root, child, &format!("{path}/{}", child.id), report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_scene_from_str;

    #[test]
    fn missing_models_reported() {
        let dir = tempfile::tempdir().unwrap();
        let scene = load_scene_from_str(
            r#"{"nodes": [{"id": "hero", "mesh": {"kind": "gltf", "path": "models/hero.glb"}}]}"#,
        )
        .unwrap();
        let report = check_scene_assets(dir.path(), &scene);
        assert!(!report.is_ok());
        assert_eq!(report.checked, 1);
        assert_eq!(report.missing[0].node, "hero");
        assert!(report.to_text().contains("missing:"));
    }

    #[test]
    fn existing_models_pass() {
        let dir = tempfile::tempdir().unwrap();
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("hero.glb"), b"glb").unwrap();
        let scene = load_scene_from_str(
            r#"{"nodes": [{"id": "hero", "mesh": {"kind": "gltf", "path": "models/hero.glb"}}]}"#,
        )
        .unwrap();
        let report = check_scene_assets(dir.path(), &scene);
        assert!(report.is_ok(), "{}", report.to_text());
    }
}
