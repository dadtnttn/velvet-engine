//! Dónde están y cómo se suben los assets del mundo: las carpetas de datos, los modelos de la Reina y los personajes, y las piezas estáticas con sus texturas.
use super::*;

pub(super) fn model_path() -> PathBuf {
    let from_root = runtime_3d_root().join("characters");
    let hair_micro = from_root.join("queen_chibi_v2_hair_micro_run.glb");
    if hair_micro.exists() {
        return hair_micro;
    }
    let hair_lowpoly = from_root.join("queen_chibi_v2_hair_lowpoly_run.glb");
    if hair_lowpoly.exists() {
        return hair_lowpoly;
    }
    // Prefer the game-ready hair-card variant.  It keeps the same body/rig and
    // clips as the detailed asset, but avoids the expensive curve groom at
    // runtime.  The detailed model remains as a local fallback.
    let hair_lite_run = from_root.join("queen_chibi_v2_hair_lite_run.glb");
    if hair_lite_run.exists() {
        return hair_lite_run;
    }
    let hair_run = from_root.join("queen_chibi_v2_hair_run.glb");
    if hair_run.exists() {
        return hair_run;
    }
    let run_ready = from_root.join("queen_chibi_v2_mixamo_run.glb");
    if run_ready.exists() {
        return run_ready;
    }
    let native_mixamo = from_root.join("queen_chibi_v2_mixamo_native.glb");
    if native_mixamo.exists() {
        return native_mixamo;
    }
    let retargeted = from_root.join("queen_chibi_v2_mixamo_retargeted.glb");
    if retargeted.exists() {
        return retargeted;
    }
    let preferred = from_root.join("queen_chibi_v2_mixamo_walk.glb");
    if preferred.exists() {
        return preferred;
    }
    let authored = from_root.join("queen_chibi_v2_animated.glb");
    if authored.exists() {
        return authored;
    }
    from_root.join("queen_chibi_v2_animated.glb")
}

pub(super) fn green_dress_model_path() -> PathBuf {
    runtime_3d_root()
        .join("characters")
        .join("queen_green_dress.glb")
}

pub(super) fn mage_model_path() -> PathBuf {
    let from_root = runtime_3d_root().join("characters");
    let animated = from_root.join("mage_chibi_mixamo_loops.glb");
    if animated.exists() {
        return animated;
    }
    let rigged = from_root.join("mage_chibi_rigged.glb");
    if rigged.exists() {
        return rigged;
    }
    from_root.join("mage_chibi_mini_textured_2048.glb")
}

pub(super) fn maid_model_path() -> PathBuf {
    runtime_3d_root()
        .join("characters")
        .join("maid_animated.glb")
}

pub(super) fn maid_alt_model_path() -> PathBuf {
    runtime_3d_root()
        .join("characters")
        .join("maid_walking4.glb")
}

pub(super) fn runtime_3d_root() -> PathBuf {
    exploration3d_state::data_dir().join("assets/3d")
}

/// A plan a prototype exports (`data/<zone>/<zone>.json`), read the first time its zone needs it
/// rather than built into the executable: a re-export takes effect without rebuilding the game.
pub(super) fn plan_text(relative: &str) -> String {
    let data = runtime_3d_root()
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let path = data.join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

pub(super) fn fantasy_village_asset_root() -> PathBuf {
    runtime_3d_root().join("villa")
}

#[derive(Debug, Clone, Copy)]
pub(super) struct StaticMeshPart {
    pub(super) mesh: MeshHandle,
    pub(super) local_transform: Mat4,
    pub(super) material: Material3D,
    pub(super) texture: Option<TextureHandle>,
}

pub(super) type TextureCacheKey = (u32, u32, u64, u32, u32, u64);

pub(super) fn upload_texture_pair(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    albedo_data: Option<&TextureData3D>,
    normal_data: Option<&TextureData3D>,
    texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white_texture: &TextureData3D,
) -> Result<Option<TextureHandle>> {
    if albedo_data.is_none() && normal_data.is_none() {
        return Ok(None);
    }
    // Normal maps are linear data and must travel with the albedo handle;
    // uploading them as independent sRGB textures erases the authored detail.
    let albedo = albedo_data.unwrap_or(white_texture);
    let cache_key = texture_cache_key(albedo, normal_data);
    let handle = if let Some(handle) = texture_cache.get(&cache_key) {
        *handle
    } else {
        // Prepared on a loader thread when the model was read ahead; else made here.
        let handle = match prefetch::take_prepared(&cache_key) {
            Some(prepared) => renderer.upload_prepared_texture(device, queue, &prepared),
            None => renderer.upload_texture_with_normal(device, queue, albedo, normal_data)?,
        };
        texture_cache.insert(cache_key, handle);
        handle
    };
    super::sign_textures::note_upload(cache_key.2, normal_data.is_some(), handle);
    Ok(Some(handle))
}

/// What identifies an (albedo, normal) pair for upload caching.
pub(crate) fn texture_cache_key(
    albedo: &TextureData3D,
    normal_data: Option<&TextureData3D>,
) -> TextureCacheKey {
    let mut albedo_hasher = DefaultHasher::new();
    albedo.rgba.hash(&mut albedo_hasher);
    let (normal_width, normal_height, normal_hash) = if let Some(normal) = normal_data {
        let mut normal_hasher = DefaultHasher::new();
        normal.rgba.hash(&mut normal_hasher);
        (normal.width, normal.height, normal_hasher.finish())
    } else {
        (0, 0, 0)
    };
    (
        albedo.width,
        albedo.height,
        albedo_hasher.finish(),
        normal_width,
        normal_height,
        normal_hash,
    )
}

#[derive(Debug, Default)]
pub(super) struct StaticAsset {
    pub(super) parts: Vec<StaticMeshPart>,
    /// glTF mesh name of each part, so authored pieces can be left out.
    pub(super) part_names: Vec<String>,
    /// Each part's mesh's own box (its vertices' least and greatest corner): to stand a loose
    /// piece on the ground whichever way it lies.
    pub(super) part_bounds: Vec<([f32; 3], [f32; 3])>,
}

/// Every model file of a zone's folder.
pub(crate) fn zone_files(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "glb"))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn upload_static_asset(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &Path,
    texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
) -> Result<StaticAsset> {
    let model =
        prefetch::load(path).with_context(|| format!("load room asset: {}", path.display()))?;
    let mut parts = Vec::with_capacity(model.meshes.len());
    let mut part_bounds = Vec::with_capacity(model.meshes.len());
    let white_texture = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };
    for (index, mesh) in model.meshes.iter().enumerate() {
        let local_transform = model.mesh_transform(index);
        let albedo_data = mesh
            .texture_index
            .and_then(|texture_index| model.textures.get(texture_index));
        let normal_data = mesh
            .normal_texture_index
            .and_then(|texture_index| model.textures.get(texture_index));
        let texture = upload_texture_pair(
            renderer,
            device,
            queue,
            albedo_data,
            normal_data,
            texture_cache,
            &white_texture,
        )?;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &mesh.vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        if mesh.vertices.is_empty() {
            (lo, hi) = ([0.0; 3], [0.0; 3]);
        }
        part_bounds.push((lo, hi));
        parts.push(StaticMeshPart {
            mesh: renderer.upload_mesh(device, mesh)?,
            local_transform,
            material: mesh.material,
            texture,
        });
    }
    Ok(StaticAsset {
        parts,
        part_names: model.mesh_names.clone(),
        part_bounds,
    })
}

pub(super) fn dark_forest_asset_root() -> PathBuf {
    runtime_3d_root().join("bosque")
}
