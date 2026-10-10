//! Bridge from declarative [`velvet_scene3d`] documents to GPU draws.
//!
//! [`upload_scene_meshes`] uploads one mesh per flattened node (primitives
//! built in code, `.glb` loaded from disk), and [`scene_draws`] rebuilds
//! the per-frame [`MeshDraw3D`] list with optional world-matrix overrides
//! for dynamic bodies and animated nodes. Games own the cache; the scene
//! document stays the source of truth for static geometry.

use std::collections::HashMap;
use std::path::Path;

use super::mesh3d::{
    GltfModel, Material3D, Mesh3D, MeshDraw3D, MeshHandle, Render3DError, Wgpu3DRenderer,
};
use velvet_math::Mat4;
use velvet_scene3d::{FlatNode3D, MaterialDoc, MeshKind3D};

/// Uploaded meshes for one flattened scene, keyed by flat node id.
#[derive(Debug, Default)]
pub struct SceneMeshCache {
    entries: HashMap<String, (MeshHandle, Material3D)>,
}

impl SceneMeshCache {
    /// Uploaded node count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty check.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Handle + material for a flat node id.
    pub fn get(&self, id: &str) -> Option<(MeshHandle, Material3D)> {
        self.entries.get(id).copied()
    }
}

/// Material override from a scene document.
pub fn material_from_doc(doc: &MaterialDoc) -> Material3D {
    Material3D {
        base_color: doc.base_color,
        texture: None,
        occlusion_window: [0.0, doc.ambient_occlusion.clamp(0.0, 1.0), 0.0, 0.0],
    }
}

/// Upload one mesh per flattened node.
///
/// `root` resolves `.glb` paths. Missing files surface as
/// [`Render3DError::Gltf`] — run `velvet scene3d check` first so a typo
/// is a checklist line instead of a boot failure.
pub fn upload_scene_meshes(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    root: &Path,
    flat: &[FlatNode3D],
) -> Result<SceneMeshCache, Render3DError> {
    let mut cache = SceneMeshCache::default();
    for node in flat {
        let material = material_from_doc(&node.material);
        let handle = match &node.mesh {
            MeshKind3D::Cube => renderer.upload_mesh(device, &Mesh3D::cube(1.0, material))?,
            MeshKind3D::Plane => renderer.upload_mesh(device, &Mesh3D::plane(1.0, material))?,
            MeshKind3D::Sphere { segments } => renderer.upload_mesh(
                device,
                &Mesh3D::uv_sphere(1.0, *segments as usize, 12, material),
            )?,
            MeshKind3D::Heightfield {
                resolution,
                size,
                heights,
            } => renderer.upload_mesh(
                device,
                &Mesh3D::heightfield(*resolution as usize, *size, heights, material),
            )?,
            MeshKind3D::Empty => continue,
            MeshKind3D::Gltf { path } => {
                let model = GltfModel::load(root.join(path))?;
                let mesh = model.meshes.first().ok_or(Render3DError::EmptyMesh)?;
                renderer.upload_mesh(device, mesh)?
            }
        };
        cache.entries.insert(node.id.clone(), (handle, material));
    }
    Ok(cache)
}

/// Rebuild per-frame draws: static nodes use their baked world matrix,
/// `overrides` (body transforms, animated nodes) win by flat node id.
pub fn scene_draws(
    cache: &SceneMeshCache,
    flat: &[FlatNode3D],
    overrides: &HashMap<String, Mat4>,
) -> Vec<MeshDraw3D> {
    let mut draws = Vec::with_capacity(flat.len());
    for node in flat {
        let Some((mesh, material)) = cache.get(&node.id) else {
            continue;
        };
        draws.push(MeshDraw3D {
            mesh,
            transform: overrides.get(&node.id).copied().unwrap_or(node.world),
            material,
            texture: None,
        });
    }
    draws
}
