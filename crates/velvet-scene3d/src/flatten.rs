//! Flatten a `Scene3D` tree into world-space draw nodes.
//!
//! Pure math over `velvet-math`: parent world matrices compose with local
//! TRS (rotations convert from author degrees to radians). Disabled nodes
//! (and their subtrees) are skipped.

use velvet_math::{Mat4, Vec3};

use crate::doc::{MaterialDoc, MeshKind3D, Scene3D, SceneNode3D, TransformDoc};

/// One flattened node, ready for a renderer or physics bake.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatNode3D {
    /// Stable id (`a/b/c` for nested nodes).
    pub id: String,
    /// World matrix (column-major, Y-up).
    pub world: Mat4,
    /// World position (translation column of `world`).
    pub world_position: Vec3,
    /// What to draw.
    pub mesh: MeshKind3D,
    /// Material override.
    pub material: MaterialDoc,
    /// Casts directional shadows.
    pub cast_shadow: bool,
    /// Contributes a static collider when baked.
    pub collider: bool,
    /// Depth in the tree (roots = 0).
    pub depth: usize,
}

/// Local TRS matrix (XYZ Euler, degrees in → same order as the renderer).
pub fn local_matrix(t: &TransformDoc) -> Mat4 {
    let r = t.rotation_deg.map(|d| d.to_radians());
    Mat4::from_translation(Vec3::new(t.position[0], t.position[1], t.position[2]))
        * Mat4::from_rotation_z(r[2])
        * Mat4::from_rotation_y(r[1])
        * Mat4::from_rotation_x(r[0])
        * Mat4::from_scale(Vec3::new(t.scale[0], t.scale[1], t.scale[2]))
}

/// Flatten `scene` depth-first (parents before children).
pub fn flatten_scene(scene: &Scene3D) -> Vec<FlatNode3D> {
    let mut out = Vec::new();
    for node in &scene.nodes {
        flatten_node(
            node,
            &node.id,
            Mat4::from_translation(Vec3::ZERO),
            0,
            &mut out,
        );
    }
    out
}

fn flatten_node(
    node: &SceneNode3D,
    path: &str,
    parent: Mat4,
    depth: usize,
    out: &mut Vec<FlatNode3D>,
) {
    if node.disabled {
        return;
    }
    let world = parent * local_matrix(&node.transform);
    out.push(FlatNode3D {
        id: path.to_string(),
        world,
        world_position: world_translation(world),
        mesh: sanitize_mesh(&node.mesh),
        material: sanitize_material(&node.material),
        cast_shadow: node.cast_shadow,
        collider: node.collider,
        depth,
    });
    for child in &node.children {
        flatten_node(
            child,
            &format!("{path}/{}", child.id),
            world,
            depth + 1,
            out,
        );
    }
}

/// Translation column of a column-major TRS matrix.
fn world_translation(world: Mat4) -> Vec3 {
    Vec3::new(world.w_axis[0], world.w_axis[1], world.w_axis[2])
}

fn sanitize_mesh(mesh: &MeshKind3D) -> MeshKind3D {
    match mesh {
        MeshKind3D::Sphere { segments } => MeshKind3D::Sphere {
            segments: (*segments).clamp(3, 64),
        },
        other => other.clone(),
    }
}

fn sanitize_material(mat: &MaterialDoc) -> MaterialDoc {
    let mut out = mat.clone();
    out.ambient_occlusion = out.ambient_occlusion.clamp(0.0, 1.0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::load_scene_from_str;

    fn plaza() -> Scene3D {
        load_scene_from_str(
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
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn flatten_resolves_world_positions() {
        let flat = flatten_scene(&plaza());
        // ground, hero, hero/sword
        assert_eq!(flat.len(), 3);
        let hero = flat.iter().find(|n| n.id == "hero").unwrap();
        assert_eq!(hero.world_position, Vec3::new(1.0, 0.0, 2.0));
        // sword inherits the hero translation (plus its local offset,
        // rotated 45° around Y: x' = 0.4·cos45 ≈ 0.283, z' = -0.4·sin45).
        let sword = flat.iter().find(|n| n.id == "hero/sword").unwrap();
        assert!((sword.world_position.x - 1.283).abs() < 0.01);
        assert!((sword.world_position.y - 1.0).abs() < 1e-5);
        assert!((sword.world_position.z - 1.717).abs() < 0.01);
        assert_eq!(sword.depth, 1);
    }

    #[test]
    fn flatten_skips_disabled_subtrees() {
        let mut scene = plaza();
        scene.nodes[1].disabled = true;
        let flat = flatten_scene(&scene);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].id, "ground");
    }

    #[test]
    fn flatten_clamps_segments_and_ao() {
        let scene = load_scene_from_str(
            r#"{"nodes": [{"id": "s", "mesh": {"kind": "sphere", "segments": 500},
                "material": {"ambient_occlusion": 5.0}}]}"#,
        )
        .unwrap();
        let flat = flatten_scene(&scene);
        assert!(matches!(flat[0].mesh, MeshKind3D::Sphere { segments: 64 }));
        assert_eq!(flat[0].material.ambient_occlusion, 1.0);
    }
}
