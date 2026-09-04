//! Velvet Scene3D CLI: check, flatten.

use std::path::PathBuf;

use anyhow::{Context, Result};

/// `velvet scene3d check <scene.scene3d.json> [--project root]`
///
/// Validates the document and verifies every referenced model exists.
pub fn cmd_scene3d_check(path: PathBuf, project: Option<PathBuf>) -> Result<()> {
    let scene = velvet_scene3d::load_scene_from_path(&path)
        .map_err(|e| anyhow::anyhow!("load {}: {e}", path.display()))?;
    let root = project.unwrap_or_else(|| {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let report = velvet_scene3d::check_scene_assets(&root, &scene);
    print!("{}", report.to_text());
    if !report.is_ok() {
        anyhow::bail!(
            "{} missing model(s) under {}",
            report.missing.len(),
            root.display()
        );
    }
    println!(
        "ok: scene `{}` nodes={} lights={} ({})",
        scene.name,
        count_nodes(&scene),
        scene.lights.len(),
        path.display()
    );
    Ok(())
}

/// `velvet scene3d flatten <scene.scene3d.json>` — dump world-space nodes.
pub fn cmd_scene3d_flatten(path: PathBuf) -> Result<()> {
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let scene = velvet_scene3d::load_scene_from_str(&text)
        .map_err(|e| anyhow::anyhow!("load {}: {e}", path.display()))?;
    for node in velvet_scene3d::flatten_scene(&scene) {
        let p = node.world_position;
        println!(
            "{} depth={} pos=({:.2}, {:.2}, {:.2}) shadow={}",
            node.id, node.depth, p.x, p.y, p.z, node.cast_shadow
        );
    }
    Ok(())
}

fn count_nodes(scene: &velvet_scene3d::Scene3D) -> usize {
    fn count(node: &velvet_scene3d::SceneNode3D) -> usize {
        1 + node.children.iter().map(count).sum::<usize>()
    }
    scene.nodes.iter().map(count).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_and_flatten_sample_scene() {
        let dir = tempfile::tempdir().unwrap();
        let scene_path = dir.path().join("plaza.scene3d.json");
        std::fs::write(
            &scene_path,
            r#"{"name": "plaza", "nodes": [
                {"id": "ground", "mesh": {"kind": "plane"}},
                {"id": "hero", "mesh": {"kind": "gltf", "path": "models/hero.glb"}}
            ]}"#,
        )
        .unwrap();
        // Missing model fails the check.
        assert!(cmd_scene3d_check(scene_path.clone(), Some(dir.path().to_path_buf())).is_err());
        std::fs::create_dir_all(dir.path().join("models")).unwrap();
        std::fs::write(dir.path().join("models/hero.glb"), b"glb").unwrap();
        cmd_scene3d_check(scene_path.clone(), Some(dir.path().to_path_buf())).unwrap();
        cmd_scene3d_flatten(scene_path).unwrap();
    }
}
