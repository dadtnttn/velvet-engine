//! Terrain bridge: [`Heightfield`] → [`Scene3D`].
//!
//! One static `terrain` node carries the grid as
//! [`MeshKind3D::Heightfield`](crate::doc::MeshKind3D); colliders bake from
//! its bounds and walkers query [`sample_heightfield`](crate::physics::sample_heightfield).

use crate::doc::{MaterialDoc, MeshKind3D, Scene3D, SceneNode3D};
use velvet_terrain::Heightfield;

/// Build a single-node scene from a baked heightfield.
///
/// The node keeps identity transform (grid already spans `-size/2..=size/2`
/// in world units); `base_color` tints the terrain material.
pub fn terrain_scene(name: &str, field: &Heightfield, base_color: [f32; 4]) -> Scene3D {
    Scene3D {
        name: name.to_string(),
        nodes: vec![SceneNode3D {
            id: "terrain".to_string(),
            mesh: MeshKind3D::Heightfield {
                resolution: field.resolution,
                size: field.size,
                heights: field.heights.clone(),
            },
            transform: Default::default(),
            material: MaterialDoc {
                base_color,
                ..Default::default()
            },
            cast_shadow: true,
            collider: true,
            disabled: false,
            children: Vec::new(),
        }],
        camera: Default::default(),
        lights: Vec::new(),
        environment: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{flatten_scene, sample_heightfield, validate_scene};
    use velvet_terrain::{TerrainGraph, TerrainParams};

    fn island() -> Heightfield {
        TerrainGraph::new(TerrainParams {
            seed: 21,
            resolution: 33,
            ..Default::default()
        })
        .bake()
    }

    #[test]
    fn terrain_scene_validates_and_flattens() {
        let field = island();
        let scene = terrain_scene("isle", &field, [0.3, 0.5, 0.25, 1.0]);
        validate_scene(&scene).unwrap();
        let flat = flatten_scene(&scene);
        assert_eq!(flat.len(), 1);
        assert!(matches!(flat[0].mesh, MeshKind3D::Heightfield { .. }));
        // Walker query agrees with the baked grid.
        let h = sample_heightfield(field.resolution, field.size, &field.heights, 0.0, 0.0).unwrap();
        assert!((h - field.sample_world(0.0, 0.0)).abs() < 1e-4);
        // Outside the rim: no ground.
        assert_eq!(
            sample_heightfield(field.resolution, field.size, &field.heights, 10_000.0, 0.0),
            None
        );
    }

    #[test]
    fn malformed_heightfields_rejected() {
        let mut scene = terrain_scene("bad", &island(), [0.0; 4]);
        if let MeshKind3D::Heightfield { heights, .. } = &mut scene.nodes[0].mesh {
            heights.pop();
        }
        assert!(validate_scene(&scene).is_err());
    }
}
