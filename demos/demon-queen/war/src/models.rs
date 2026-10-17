//! The war's models from Blender (`data/assets/war/{troops,props,campaign}/<kind>.glb`): every
//! object of a file is a piece (`ARM_W__orc`: its part, and its origin is the joint it turns
//! round), every material a colour role (`cloth`, `trim`, `skin_orc`… repainted per faction).
//! Troops become crowd meshes; set pieces and settlements, plain vertex-coloured meshes.
use std::path::Path;

use anyhow::{Context, Result};
use velvet_math::{Mat4, Vec3};
use velvet_render::{crowd3d::part, CrowdMesh3D, CrowdVertex3D, Material3D, Mesh3D, MeshVertex3D};

use crate::data::{rgb, Faction};

/// One object of a model, in the model's space.
#[derive(Clone, Debug)]
pub struct Piece {
    /// The object's name (or the nearest ancestor's that names a part).
    pub node: String,
    pub part: f32,
    /// Where its joint is.
    pub pivot: [f32; 3],
    /// Its material's name (the colour role), without Blender's `.001` suffix.
    pub material: String,
    /// The material's own colour (linear).
    pub color: [f32; 4],
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

fn strip_suffix(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => {
            head.to_owned()
        }
        _ => name.to_owned(),
    }
}

/// Every piece of a GLB, placed in the model's space.
pub fn load_glb(path: &Path) -> Result<Vec<Piece>> {
    // The release packs its models (VGLZ): unpacked as the renderer does.
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let bytes = velvet_render::unpack_model_bytes(&bytes).map_err(|e| anyhow::anyhow!("{e}"))?;
    let (doc, buffers, _) =
        gltf::import_slice(&bytes).with_context(|| format!("reading {}", path.display()))?;
    let mut out = Vec::new();
    let scene = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .context("no scene")?;
    fn walk(
        node: gltf::Node<'_>,
        parent: Mat4,
        named: Option<(String, [f32; 3])>,
        buffers: &[gltf::buffer::Data],
        out: &mut Vec<Piece>,
    ) {
        let world = parent * {
            let m = node.transform().matrix();
            Mat4::from_cols(m[0], m[1], m[2], m[3])
        };
        let name = node.name().unwrap_or("").to_owned();
        let here = world.transform_point3(Vec3::ZERO);
        let named = if name.contains("__") {
            Some((name.clone(), [here.x, here.y, here.z]))
        } else {
            named
        };
        if let Some(mesh) = node.mesh() {
            let (node_name, pivot) = named
                .clone()
                .unwrap_or((name.clone(), [here.x, here.y, here.z]));
            let part_name = strip_suffix(node_name.split("__").next().unwrap_or("BODY"));
            for prim in mesh.primitives() {
                let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d.0[..]));
                let Some(positions) = reader.read_positions() else {
                    continue;
                };
                let normal_m = world.inverse().unwrap_or(Mat4::IDENTITY).transpose();
                let positions: Vec<[f32; 3]> = positions
                    .map(|p| {
                        let q = world.transform_point3(Vec3::new(p[0], p[1], p[2]));
                        [q.x, q.y, q.z]
                    })
                    .collect();
                let normals: Vec<[f32; 3]> = match reader.read_normals() {
                    Some(n) => n
                        .map(|n| {
                            let q = normal_m
                                .transform_vector3(Vec3::new(n[0], n[1], n[2]))
                                .normalize_or_zero();
                            [q.x, q.y, q.z]
                        })
                        .collect(),
                    None => Vec::new(),
                };
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(i) => i.into_u32().collect(),
                    None => (0..positions.len() as u32).collect(),
                };
                let material = prim.material();
                let pbr = material.pbr_metallic_roughness();
                out.push(Piece {
                    node: node_name.clone(),
                    part: part::by_name(&part_name),
                    pivot,
                    material: strip_suffix(material.name().unwrap_or("")),
                    color: pbr.base_color_factor(),
                    normals: if normals.len() == positions.len() {
                        normals
                    } else {
                        flat_normals(&positions, &indices)
                    },
                    positions,
                    indices,
                });
            }
        }
        for child in node.children() {
            walk(child, world, named.clone(), buffers, out);
        }
    }
    for node in scene.nodes() {
        walk(node, Mat4::IDENTITY, None, &buffers, &mut out);
    }
    Ok(out)
}

/// Normals from the triangles (each vertex gets the sum of its faces').
fn flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![Vec3::ZERO; positions.len()];
    for t in indices.chunks_exact(3) {
        let v = |i: u32| {
            let p = positions[i as usize];
            Vec3::new(p[0], p[1], p[2])
        };
        let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
        let f = (b - a).cross(c - a);
        for &i in t {
            n[i as usize] += f;
        }
    }
    n.into_iter()
        .map(|v| {
            let v = v.normalize_or_zero();
            [v.x, v.y, v.z]
        })
        .collect()
}

/// What each faction wears: these colour roles are repainted; any other keeps Blender's colour.
pub fn look_color(look: Faction, role: &str) -> Option<[f32; 3]> {
    let hex = match (look, role) {
        (Faction::Reina, "cloth") => 0x6a32c0,
        (Faction::Reina, "trim") => 0xf0b44a,
        (Faction::Reina, "cloth_dark") => 0x3a1e66,
        (Faction::Alianza, "cloth") => 0xece6d4,
        (Faction::Alianza, "trim") => 0x3d6fd0,
        (Faction::Alianza, "cloth_dark") => 0x24489a,
        (Faction::Rebeldes, "cloth") => 0xa8442a,
        (Faction::Rebeldes, "trim") => 0xd8c290,
        (Faction::Rebeldes, "cloth_dark") => 0x5a2416,
        (Faction::Rebeldes, "metal") => 0x6a5a4c,
        (Faction::Rebeldes, "skin_orc") => 0x8f9150,
        (Faction::Rebeldes, "skin_orc_dark") => 0x666a36,
        (Faction::Rebeldes, "skin_goblin") => 0x9a9a48,
        (Faction::Rebeldes, "feather") => 0x7a5e4a,
        (Faction::Rebeldes, "feather_dark") => 0x5a4436,
        _ => return None,
    };
    Some(rgb(hex))
}

/// The troops that walk on four legs (their bodies pitch, they fall on their side).
pub fn quadruped(kind: &str) -> bool {
    matches!(kind, "knight" | "wolf")
}

/// A troop's crowd mesh for a faction's look, from its pieces.
pub fn crowd_mesh(kind: &str, pieces: &[Piece], look: Faction) -> CrowdMesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for p in pieces {
        let c = look_color(look, &p.material).unwrap_or([p.color[0], p.color[1], p.color[2]]);
        let base = vertices.len() as u32;
        for (k, pos) in p.positions.iter().enumerate() {
            vertices.push(CrowdVertex3D {
                position: *pos,
                part: p.part,
                normal: p.normals[k],
                _pad: 0.0,
                color: [c[0], c[1], c[2], 1.0],
                pivot: [p.pivot[0], p.pivot[1], p.pivot[2], 0.0],
            });
        }
        indices.extend(p.indices.iter().map(|i| i + base));
    }
    CrowdMesh3D {
        vertices,
        indices,
        quadruped: quadruped(kind),
    }
}

/// A set piece or a settlement as one vertex-coloured mesh; `keep` picks which pieces go in (the
/// flags apart, to paint them in their owner's colours), `paint` repaints a material.
pub fn static_mesh(
    pieces: &[Piece],
    keep: impl Fn(&Piece) -> bool,
    paint: impl Fn(&Piece) -> Option<[f32; 3]>,
) -> Option<Mesh3D> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for p in pieces.iter().filter(|p| keep(p)) {
        let c = paint(p).unwrap_or([p.color[0], p.color[1], p.color[2]]);
        let base = vertices.len() as u32;
        for (k, pos) in p.positions.iter().enumerate() {
            let mut v = MeshVertex3D::new(*pos, p.normals[k], [0.0, 0.0]);
            v.color = [c[0], c[1], c[2], 1.0];
            vertices.push(v);
        }
        indices.extend(p.indices.iter().map(|i| i + base));
    }
    if vertices.is_empty() {
        return None;
    }
    Some(Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 0.9,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    })
}

/// The pieces that glow (fires, windows, the seal): drawn apart with emission.
pub fn glows(p: &Piece) -> bool {
    matches!(
        p.material.as_str(),
        "fire" | "glow" | "flame" | "ember" | "lamp" | "window_lit"
    )
}

/// The banners of a settlement (painted in their owner's colours).
pub fn is_flag(p: &Piece) -> bool {
    p.material == "flag" || p.node.starts_with("FLAG_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/assets/war")
    }

    #[test]
    #[ignore = "reads the private art pack (data/assets/war); run with --ignored on a dev machine"]
    fn every_troop_loads_into_a_crowd_mesh_with_its_parts() {
        for kind in [
            "orc", "dimmed", "goblin", "harpy", "lamia", "skeleton", "spearman", "knight",
            "crossbow", "priest", "wolf", "queen",
        ] {
            for lod in ["", "_lod"] {
                let path = assets().join(format!("troops/{kind}{lod}.glb"));
                let pieces = load_glb(&path).unwrap();
                let mesh = crowd_mesh(kind, &pieces, Faction::Reina);
                assert!(
                    !mesh.vertices.is_empty() && mesh.indices.len() % 3 == 0,
                    "{kind}{lod}"
                );
                let parts: std::collections::BTreeSet<i32> =
                    mesh.vertices.iter().map(|v| v.part as i32).collect();
                assert!(parts.len() > 1, "{kind}{lod}: only one part {parts:?}");
                let top = mesh
                    .vertices
                    .iter()
                    .map(|v| v.position[1])
                    .fold(0.0f32, f32::max);
                assert!((0.6..6.0).contains(&top), "{kind}{lod}: {top} m tall");
            }
        }
    }

    #[test]
    #[ignore = "reads the private art pack (data/assets/war); run with --ignored on a dev machine"]
    fn set_pieces_and_settlements_load() {
        for dir in ["props", "campaign"] {
            for entry in std::fs::read_dir(assets().join(dir)).unwrap() {
                let path = entry.unwrap().path();
                let pieces = load_glb(&path).unwrap();
                assert!(
                    static_mesh(&pieces, |_| true, |_| None).is_some(),
                    "{}",
                    path.display()
                );
            }
        }
    }
}
