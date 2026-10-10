//! `velvet terrain generate` — bake islands to scene + preview.

use std::path::PathBuf;

use anyhow::{Context, Result};
use velvet_terrain::{erode_hydraulic, erode_thermal, ErosionParams, TerrainGraph, TerrainParams};

/// Bake a terrain: `<out>.terrain.json` (params), `<out>.scene3d.json`,
/// `<out>_preview.png` (hillshade).
#[allow(clippy::too_many_arguments)]
pub fn cmd_terrain_generate(
    preset: &str,
    seed: u64,
    resolution: u32,
    size: f32,
    erode: bool,
    droplets: u32,
    out: PathBuf,
) -> Result<()> {
    let mut params = if preset == "cordillera" {
        TerrainParams::cordillera(seed)
    } else {
        TerrainParams {
            seed,
            size: size.max(1.0),
            ..Default::default()
        }
    };
    params.seed = seed;
    params.resolution = resolution.clamp(9, 513);
    if preset == "island" {
        params.size = size.max(1.0);
    }
    let graph = TerrainGraph::new(params.clone());
    let mut field = graph.bake();
    if erode {
        erode_thermal(&mut field, &ErosionParams::default());
        erode_hydraulic(
            &mut field,
            &ErosionParams {
                droplets,
                ..Default::default()
            },
            seed ^ 0xE0DE,
        );
    }
    let params_path = out.with_extension("terrain.json");
    std::fs::write(
        &params_path,
        serde_json::to_string_pretty(&params).unwrap_or_default(),
    )
    .with_context(|| format!("write {}", params_path.display()))?;
    let scene = velvet_scene3d::terrain_scene(
        out.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("terrain"),
        &field,
        [0.32, 0.48, 0.26, 1.0],
    );
    velvet_scene3d::validate_scene(&scene).map_err(|e| anyhow::anyhow!("{e}"))?;
    let scene_path = out.with_extension("scene3d.json");
    std::fs::write(
        &scene_path,
        serde_json::to_string_pretty(&scene).unwrap_or_default(),
    )
    .with_context(|| format!("write {}", scene_path.display()))?;
    let preview = hillshade(&field);
    let png = velvet_image::encode(&preview, velvet_image::ImageEncode::default())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let preview_path = out.with_extension("preview.png");
    std::fs::write(&preview_path, png)
        .with_context(|| format!("write {}", preview_path.display()))?;
    let (lo, hi) = field.range();
    println!(
        "terrain seed={seed} res={} size={} range=[{lo:.1}, {hi:.1}]",
        field.resolution, field.size,
    );
    println!(
        "wrote {} / {} / {}",
        params_path.display(),
        scene_path.display(),
        preview_path.display()
    );
    Ok(())
}

/// Hillshade preview via the shared terrain rasterizer.
fn hillshade(field: &velvet_terrain::Heightfield) -> velvet_image::RgbaImage {
    let res = field.resolution;
    velvet_image::RgbaImage {
        width: res,
        height: res,
        pixels: velvet_terrain::preview_rgba(field),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_writes_three_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("isle");
        cmd_terrain_generate("island", 42, 33, 64.0, true, 200, out.clone()).unwrap();
        assert!(out.with_extension("terrain.json").exists());
        let scene_path = out.with_extension("scene3d.json");
        assert!(scene_path.exists());
        // Preview is a decodable PNG.
        let preview = std::fs::read(dir.path().join("isle.preview.png")).unwrap();
        let info = velvet_image::probe(&preview).unwrap();
        assert_eq!((info.width, info.height), (33, 33));
        // Scene round-trips through the validator.
        let scene: velvet_scene3d::Scene3D =
            serde_json::from_str(&std::fs::read_to_string(&scene_path).unwrap()).unwrap();
        velvet_scene3d::validate_scene(&scene).unwrap();
        assert_eq!(scene.nodes.len(), 1);
    }
}
