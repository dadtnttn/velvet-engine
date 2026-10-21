//! The Tendero de la Bocamina in 3D (tools/tendero_blender.py): a chibi dwarf miner moved in parts —
//! he breathes, turns his head to the Queen when she is near, waves when she comes up to the stall,
//! talks with his free hand while it is open, now and then scratches his beard; his helmet lamp and
//! the lantern at his belt are lit.
use super::*;

struct Part {
    group: Group,
    mesh: MeshHandle,
    base: Mat4,
    material: Material3D,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Body,
    Head,
    RightArm,
    LeftArm,
}

pub(super) struct Tendero {
    parts: Vec<Part>,
}

/// Where his parts turn (glTF frame: y up, he faces +z).
const NECK: Vec3 = Vec3::new(0.0, 0.93, 0.0);
const SHOULDER_R: Vec3 = Vec3::new(-0.29, 0.82, 0.0);
const SHOULDER_L: Vec3 = Vec3::new(0.29, 0.82, 0.0);

impl Tendero {
    pub(super) fn load(renderer: &mut Wgpu3DRenderer, device: &wgpu::Device) -> Option<Self> {
        let path = super::super::assets::root()
            .join("mina")
            .join("tendero.glb");
        let model = GltfModel::load(&path).ok()?;
        let mut parts = Vec::new();
        for (index, mesh) in model.meshes.iter().enumerate() {
            let name = model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            let group = match name.split('_').next().unwrap_or("") {
                "cabeza" => Group::Head,
                "brazod" => Group::RightArm,
                "brazoi" => Group::LeftArm,
                _ => Group::Body,
            };
            let lit = name.contains("lampara") || name.contains("farol");
            let mut material = mesh.material.with_ambient_occlusion(0.15);
            if lit {
                material = material.with_emission(2.6);
            }
            let handle = renderer.upload_mesh(device, mesh).ok()?;
            parts.push(Part {
                group,
                mesh: handle,
                base: model.mesh_transform(index),
                material,
            });
        }
        (!parts.is_empty()).then_some(Self { parts })
    }

    /// Him at `feet`, facing `face`; `queen` where she stands; `open` whether his stall is open.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        feet: Vec3,
        face: Vec3,
        queen: Vec3,
        open: bool,
        t: f32,
    ) {
        let yaw = face.x.atan2(face.z);
        let root = Mat4::from_translation(feet) * Mat4::from_rotation_y(yaw);
        let to_her = queen - feet;
        let near = Vec3::new(to_her.x, 0.0, to_her.z).length();
        // his head turns to her when she is near (within what a neck allows)
        let look = if near < 7.0 {
            let mut d = to_her.x.atan2(to_her.z) - yaw;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            d.clamp(-1.0, 1.0)
        } else {
            (t * 0.21).sin() * 0.35
        };
        let nod = if open {
            (t * 7.5).sin() * 0.07
        } else {
            (t * 0.9).sin() * 0.03
        };
        let breath = 1.0 + (t * 1.7).sin() * 0.012;
        // the left hand: a wave as she comes up, talking while the stall is open, a scratch of the
        // beard now and then
        let scratch = ((t * 0.13).sin() > 0.93) as u8 as f32;
        let (left_up, left_fwd) = if open {
            (0.25, -0.7 + (t * 4.0).sin() * 0.25)
        } else if (2.5..6.0).contains(&near) {
            (1.9 + (t * 8.0).sin() * 0.3, -0.2)
        } else if scratch > 0.0 {
            (0.35, -1.9 + (t * 11.0).sin() * 0.08)
        } else {
            (0.05, (t * 1.3).sin() * 0.04)
        };
        let right_fwd = (t * 1.1).sin() * 0.04;
        let about = |pivot: Vec3, turn: Mat4| {
            Mat4::from_translation(pivot) * turn * Mat4::from_translation(-pivot)
        };
        for part in &self.parts {
            let local = match part.group {
                Group::Body => Mat4::from_scale(Vec3::new(1.0, breath, 1.0)),
                Group::Head => {
                    Mat4::from_translation(Vec3::Y * (breath - 1.0) * 0.9)
                        * about(
                            NECK,
                            Mat4::from_rotation_y(look) * Mat4::from_rotation_x(nod),
                        )
                }
                Group::RightArm => about(SHOULDER_R, Mat4::from_rotation_x(right_fwd)),
                Group::LeftArm => about(
                    SHOULDER_L,
                    Mat4::from_rotation_z(left_up) * Mat4::from_rotation_x(left_fwd),
                ),
            };
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: root * local * part.base,
                material: part.material,
                texture: None,
            });
        }
        let lamp = root.transform_point3(Vec3::new(0.0, 1.29, 0.3));
        lights.push(PointLight3D::new(lamp, [1.0, 0.86, 0.55], 1.6, 6.0));
        let belt = root.transform_point3(Vec3::new(0.25, 0.38, 0.12));
        lights.push(PointLight3D::new(
            belt,
            [1.0, 0.7, 0.35],
            1.0 + (t * 6.0).sin() * 0.08,
            3.5,
        ));
    }
}
