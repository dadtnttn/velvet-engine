//! The Tendero de la Bocamina in 3D: the dwarf miner of the user's model (rigged with AccuRig), with
//! clips of his own from tools/enano_blender.py — Idle (he breathes and looks about), Wave (as she
//! comes up to his stall) and Talk (while it is open). He turns to her when she is near; the lamp
//! on his helmet and the lantern at his belt are lit.
use super::*;

pub(super) struct Tendero {
    model: GltfModel,
    meshes: Vec<MeshHandle>,
    textures: Vec<TextureHandle>,
    /// Idle, Wave and Talk (clip indices).
    clips: [usize; 3],
    /// The clip on show, the pose of the one before it and when it changed (a short cross-fade).
    clip: usize,
    from: Option<Pose3D>,
    changed_at: f32,
    sampled: Option<(usize, i64)>,
    transforms: Vec<Mat4>,
    /// His head in his own frame (the lamp on his helmet follows it).
    head: Vec3,
    /// His facing, eased toward her, and the clock it was eased at.
    yaw: Option<f32>,
    last_t: f32,
}

/// He is drawn this tall (the model is a metre; a dwarf stands to her chest).
const SCALE: f32 = 1.3;
/// His clips are sampled this often.
const FPS: f32 = 30.0;
/// From his head to the lamp on his helmet, and where the lantern hangs at his belt (glTF frame:
/// y up, he faces +z).
const LAMP: Vec3 = Vec3::new(0.0, 0.146, 0.123);
const LANTERN: Vec3 = Vec3::new(0.116, 0.336, 0.148);

impl Tendero {
    pub(super) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<Self> {
        let path = super::super::assets::root()
            .join("mina")
            .join("tendero_enano.glb");
        let model = GltfModel::load(&path).ok()?;
        let clip = |name: &str| model.animations.iter().position(|a| a == name);
        let clips = [clip("Idle")?, clip("Wave")?, clip("Talk")?];
        let mut textures = Vec::with_capacity(model.textures.len());
        for texture in &model.textures {
            textures.push(
                renderer
                    .upload_texture_with_normal(device, queue, texture, None)
                    .ok()?,
            );
        }
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for mesh in &model.meshes {
            meshes.push(renderer.upload_skinned_mesh(device, mesh).ok()?);
        }
        let transforms = (0..model.meshes.len())
            .map(|index| model.mesh_transform(index))
            .collect();
        Some(Self {
            model,
            meshes,
            textures,
            clips,
            clip: clips[0],
            from: None,
            changed_at: 0.0,
            sampled: None,
            transforms,
            head: Vec3::new(0.0, 0.75, 0.035),
            yaw: None,
            last_t: 0.0,
        })
    }

    /// Him at `feet`, facing `face`; `queen` where she stands; `open` whether his stall is open.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        draws: &mut Vec<MeshDraw3D>,
        lights: &mut Vec<PointLight3D>,
        feet: Vec3,
        face: Vec3,
        queen: Vec3,
        open: bool,
        t: f32,
    ) {
        let wrap = |mut a: f32| {
            while a > std::f32::consts::PI {
                a -= std::f32::consts::TAU;
            }
            while a < -std::f32::consts::PI {
                a += std::f32::consts::TAU;
            }
            a
        };
        let to_her = queen - feet;
        let near = Vec3::new(to_her.x, 0.0, to_her.z).length();
        // he turns to her when she is near (never his back to his stall)
        let rest = face.x.atan2(face.z);
        let want = if near < 7.0 {
            rest + wrap(to_her.x.atan2(to_her.z) - rest).clamp(-1.1, 1.1)
        } else {
            rest
        };
        let dt = (t - self.last_t).clamp(0.0, 0.1);
        self.last_t = t;
        let yaw = match self.yaw {
            Some(yaw) => yaw + wrap(want - yaw) * (1.0 - (-4.0 * dt).exp()),
            None => want,
        };
        self.yaw = Some(yaw);
        // talking while the stall is open, a wave as she comes up to it, else his idle
        let clip = if open {
            self.clips[2]
        } else if (2.5..6.0).contains(&near) {
            self.clips[1]
        } else {
            self.clips[0]
        };
        if clip != self.clip {
            self.from = self.model.pose(self.clip, t);
            self.clip = clip;
            self.changed_at = t;
        }
        let frame = (t * FPS).floor() as i64;
        if self.sampled != Some((clip, frame)) || self.from.is_some() {
            if let Some(mut pose) = self.model.pose(clip, frame as f32 / FPS) {
                let k = ((t - self.changed_at) / 0.3).clamp(0.0, 1.0);
                match self.from.as_ref() {
                    Some(from) if k < 1.0 => pose = from.blend(&pose, k),
                    _ => self.from = None,
                }
                for (index, handle) in self.meshes.iter().enumerate() {
                    if let Some(joints) = self.model.joint_matrices(index, &pose) {
                        let _ = renderer.set_skin_joints(queue, *handle, &joints);
                    }
                }
                self.transforms = self.model.pose_mesh_transforms(&pose);
                if let Some(head) = self.model.pose_node_transform(&pose, "CC_Base_Head") {
                    self.head = head.transform_point3(Vec3::ZERO);
                }
                self.sampled = Some((clip, frame));
            }
        }
        let root = Mat4::from_translation(feet)
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(SCALE, SCALE, SCALE));
        for (index, mesh) in self.model.meshes.iter().enumerate() {
            draws.push(MeshDraw3D {
                mesh: self.meshes[index],
                transform: root * self.transforms[index],
                material: mesh.material.with_ambient_occlusion(0.15),
                texture: mesh
                    .texture_index
                    .and_then(|texture| self.textures.get(texture).copied()),
            });
        }
        let lamp = root.transform_point3(self.head + LAMP);
        lights.push(PointLight3D::new(lamp, [1.0, 0.86, 0.55], 1.6, 6.0));
        lights.push(PointLight3D::new(
            root.transform_point3(LANTERN),
            [1.0, 0.7, 0.35],
            1.0 + (t * 6.0).sin() * 0.08,
            3.5,
        ));
    }
}
