//! Loading + validation for `Scene3D` documents.

use std::collections::HashSet;
use std::path::Path;

use thiserror::Error;

use crate::doc::{Scene3D, SceneNode3D};

/// Scene load / validation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SceneError {
    /// JSON did not parse.
    #[error("scene json: {0}")]
    Json(String),
    /// Structural problem with a node path (`a/b/c`).
    #[error("scene node `{path}`: {message}")]
    Node {
        /// Slash-joined node path.
        path: String,
        /// What is wrong.
        message: String,
    },
    /// File could not be read.
    #[error("io {0}")]
    Io(String),
}

/// Parse + validate a scene document from a JSON string.
pub fn load_scene_from_str(text: &str) -> Result<Scene3D, SceneError> {
    let scene: Scene3D = serde_json::from_str(text).map_err(|e| SceneError::Json(e.to_string()))?;
    validate_scene(&scene)?;
    Ok(scene)
}

/// Read + parse + validate a `.scene3d.json` file.
pub fn load_scene_from_path(path: &Path) -> Result<Scene3D, SceneError> {
    let text = std::fs::read_to_string(path).map_err(|e| SceneError::Io(e.to_string()))?;
    load_scene_from_str(&text)
}

/// Structural validation: finite numbers, unique ids, sane camera/lights.
pub fn validate_scene(scene: &Scene3D) -> Result<(), SceneError> {
    if scene.camera.near <= 0.0 || !scene.camera.near.is_finite() {
        return Err(SceneError::Node {
            path: "<camera>".into(),
            message: "near plane must be a finite number > 0".into(),
        });
    }
    if scene.camera.far <= scene.camera.near || !scene.camera.far.is_finite() {
        return Err(SceneError::Node {
            path: "<camera>".into(),
            message: "far plane must be finite and beyond near".into(),
        });
    }
    if !(1.0..=179.0).contains(&scene.camera.fov_deg) {
        return Err(SceneError::Node {
            path: "<camera>".into(),
            message: "fov_deg must be within 1..=179".into(),
        });
    }
    let mut ids = HashSet::new();
    for node in &scene.nodes {
        validate_node(node, &mut Vec::new(), &mut ids)?;
    }
    Ok(())
}

fn validate_node(
    node: &SceneNode3D,
    trail: &mut Vec<String>,
    ids: &mut HashSet<String>,
) -> Result<(), SceneError> {
    if node.id.trim().is_empty() {
        return Err(SceneError::Node {
            path: trail.join("/"),
            message: "node id must not be empty".into(),
        });
    }
    if !ids.insert(node.id.clone()) {
        return Err(SceneError::Node {
            path: node.id.clone(),
            message: format!("duplicate node id `{}`", node.id),
        });
    }
    trail.push(node.id.clone());
    let path = trail.join("/");
    for v in node
        .transform
        .position
        .iter()
        .chain(node.transform.rotation_deg.iter())
        .chain(node.transform.scale.iter())
    {
        if !v.is_finite() {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "transform numbers must be finite".into(),
            });
        }
    }
    if let crate::doc::MeshKind3D::Gltf { path: asset } = &node.mesh {
        if asset.trim().is_empty() {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "gltf path must not be empty".into(),
            });
        }
    }
    if let crate::doc::MeshKind3D::Heightfield {
        resolution,
        size,
        heights,
    } = &node.mesh
    {
        if *resolution < 2 {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "heightfield resolution must be >= 2".into(),
            });
        }
        if !size.is_finite() || *size <= 0.0 {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "heightfield size must be finite and > 0".into(),
            });
        }
        if heights.len() != (*resolution as usize) * (*resolution as usize) {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "heightfield heights must hold resolution² samples".into(),
            });
        }
        if heights.iter().any(|h| !h.is_finite()) {
            return Err(SceneError::Node {
                path: path.clone(),
                message: "heightfield heights must be finite".into(),
            });
        }
    }
    for child in &node.children {
        validate_node(child, trail, ids)?;
    }
    trail.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample_json() -> &'static str {
        r#"{
            "name": "plaza",
            "nodes": [
                {"id": "ground", "mesh": {"kind": "plane"},
                 "transform": {"position": [0, 0, 0], "scale": [20, 1, 20]}},
                {"id": "hero", "mesh": {"kind": "gltf", "path": "models/hero.glb"},
                 "transform": {"position": [1, 0, 2], "rotation_deg": [0, 45, 0]},
                 "children": [
                    {"id": "sword", "mesh": {"kind": "cube"},
                     "transform": {"position": [0.4, 1.0, 0]}}
                 ]}
            ],
            "camera": {"position": [4.5, 3.2, 5.0], "target": [0, 1, 0]},
            "lights": [{"kind": "directional"}]
        }"#
    }

    #[test]
    fn loads_sample_scene() {
        let scene = load_scene_from_str(sample_json()).unwrap();
        assert_eq!(scene.name, "plaza");
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.nodes[1].children.len(), 1);
    }

    #[test]
    fn rejects_duplicate_ids() {
        let err = load_scene_from_str(
            r#"{"nodes": [{"id": "a"}, {"id": "a", "children": [{"id": "a"}]}]}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate node id `a`"));
    }

    #[test]
    fn rejects_bad_camera() {
        let err = load_scene_from_str(r#"{"camera": {"near": -1}} "#).unwrap_err();
        assert!(err.to_string().contains("near plane"));
    }

    #[test]
    fn rejects_non_finite_transform() {
        // JSON has no NaN literal; bypass the parser to hit the check.
        let mut scene = load_scene_from_str(r#"{"nodes": [{"id": "a"}]}"#).unwrap();
        scene.nodes[0].transform.position[0] = f32::NAN;
        let err = validate_scene(&scene).unwrap_err();
        assert!(err.to_string().contains("finite"));
    }
}
