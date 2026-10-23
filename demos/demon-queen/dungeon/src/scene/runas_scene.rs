//! The hand-made mine's seal stones, memory stone, sanctuary font and ways out
//! (tools/runas_blender.py): a menhir on two courses of blocks with its numeral and a bronze dial
//! whose pointer turns to the mark of the phase it is set to (that mark lit; the numeral lit once it
//! is the right one); the tablet of the three suns in order; a basin of lit water with its crystals;
//! the miner's waymark at the mouth with its swaying lantern; the stele beyond the Heart, chained and
//! red while it beats, gold once it is still.
use super::pozo_scene::load_texture;
use super::*;

struct Part {
    name: String,
    mesh: MeshHandle,
    base: Mat4,
    color: [f32; 3],
    texture: Option<TextureHandle>,
}

pub(super) struct RuneArt {
    parts: Vec<Part>,
}

impl RuneArt {
    pub(super) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<Self> {
        let path = super::super::assets::root().join("mina").join("runas.glb");
        let model = GltfModel::load(&path).ok()?;
        let rock = load_texture(renderer, device, queue, "rock_face_04");
        let blocks = load_texture(renderer, device, queue, "medieval_blocks_06");
        let mut parts = Vec::new();
        for (index, mesh) in model.meshes.iter().enumerate() {
            let name = model.mesh_names.get(index).cloned().unwrap_or_default();
            // the scanned textures are dark: lifted, as the mine's
            let (texture, lift) = if name.contains("roca") || name == "fin_corazon" {
                (rock, 1.45)
            } else if name.contains("sillar") {
                (blocks, 1.3)
            } else {
                (None, 1.0)
            };
            let c = mesh.material.base_color;
            parts.push(Part {
                name,
                mesh: renderer.upload_mesh(device, mesh).ok()?,
                base: model.mesh_transform(index),
                color: [c[0] * lift, c[1] * lift, c[2] * lift],
                texture,
            });
        }
        (!parts.is_empty()).then_some(Self { parts })
    }

    fn push(draws: &mut Vec<MeshDraw3D>, part: &Part, transform: Mat4, color: [f32; 3], glow: f32) {
        let mut material =
            Material3D::rgb(color[0], color[1], color[2]).with_ambient_occlusion(0.2);
        if glow > 0.0 {
            material = material.with_emission(glow);
        }
        draws.push(MeshDraw3D {
            mesh: part.mesh,
            transform,
            material,
            texture: part.texture,
        });
    }

    /// Seal stone `index` at `foot` facing `yaw`, set to `phase`, its dial at `angle`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_seal(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        foot: Vec3,
        yaw: f32,
        index: usize,
        phase: u8,
        angle: f32,
        t: f32,
    ) {
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        let right = phase as usize == index + 1;
        let tint = runes::phase_tint(phase);
        let pulse = 0.85 + 0.15 * (t * 2.6).sin();
        for part in self.parts.iter().filter(|p| p.name.starts_with("runa_")) {
            let name = part.name.as_str();
            if let Some(n) = name.strip_prefix("runa_numero_") {
                if n.parse::<usize>().ok() != Some(index + 1) {
                    continue;
                }
                let (color, glow) = if right {
                    ([0.25, 1.0, 0.4], 1.6 * pulse)
                } else {
                    ([0.75, 0.62, 0.4], 0.25)
                };
                Self::push(draws, part, root * part.base, color, glow);
            } else if let Some(k) = name.strip_prefix("runa_glifo_") {
                let lit = k.parse::<u8>().ok() == Some(phase) && phase > 0;
                let (color, glow) = if lit {
                    ([tint[0] * 0.75, tint[1] * 0.7, tint[2] * 0.6], 2.8 * pulse)
                } else {
                    ([0.55, 0.45, 0.3], 0.12)
                };
                Self::push(draws, part, root * part.base, color, glow);
            } else if name.starts_with("runa_disco") {
                // the dial and its needle turn together; the needle catches the light
                let glow = if name.ends_with("aguja") { 0.35 } else { 0.0 };
                Self::push(
                    draws,
                    part,
                    root * part.base * Mat4::from_rotation_z(angle),
                    part.color,
                    glow,
                );
            } else {
                Self::push(draws, part, root * part.base, part.color, 0.0);
            }
        }
    }

    /// The memory stone at `foot` facing `yaw`: its suns and numerals softly lit.
    pub(super) fn draw_memory(&self, draws: &mut Vec<MeshDraw3D>, foot: Vec3, yaw: f32, t: f32) {
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        let pulse = 0.8 + 0.2 * (t * 1.7).sin();
        for part in self.parts.iter().filter(|p| p.name.starts_with("memoria_")) {
            let glow = if part.name.contains("roca") || part.name.contains("sillar") {
                0.0
            } else {
                1.3 * pulse
            };
            Self::push(draws, part, root * part.base, part.color, glow);
        }
    }

    /// The way out at the mouth at `foot` facing `yaw`: its picks and arrow softly lit, its lantern
    /// swaying a little on the hook.
    pub(super) fn draw_way_out(&self, draws: &mut Vec<MeshDraw3D>, foot: Vec3, yaw: f32, t: f32) {
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        let sway = Mat4::from_rotation_z(0.07 * (t * 1.1).sin())
            * Mat4::from_rotation_x(0.04 * (t * 0.8).cos());
        for part in self.parts.iter().filter(|p| p.name.starts_with("salida_")) {
            let (local, glow) = match part.name.as_str() {
                "salida_glifo" => (part.base, 0.4 + 0.08 * (t * 1.4).sin()),
                "salida_farol" => (part.base * sway, 0.0),
                "salida_luz" => (
                    part.base * sway,
                    2.2 + 0.25 * (t * 7.3).sin() * (t * 3.1).cos(),
                ),
                _ => (part.base, 0.0),
            };
            Self::push(draws, part, root * local, part.color, glow);
        }
    }

    /// The way up beyond the Heart at `foot` facing `yaw`: chained, its heart red and beating while
    /// the Heart lives (`open` false); free and gold once it is still.
    pub(super) fn draw_way_up(
        &self,
        draws: &mut Vec<MeshDraw3D>,
        foot: Vec3,
        yaw: f32,
        open: bool,
        t: f32,
    ) {
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        for part in self.parts.iter().filter(|p| p.name.starts_with("fin_")) {
            let (color, glow) = match (part.name.as_str(), open) {
                ("fin_cadena", true) => continue,
                ("fin_corazon", false) => {
                    let beat = (t * 3.4).sin().max(0.0).powi(6);
                    ([0.75, 0.12, 0.1], 0.5 + 1.1 * beat)
                }
                ("fin_corazon", true) => ([1.0, 0.75, 0.32], 1.8 + 0.2 * (t * 1.5).sin()),
                ("fin_grieta", true) => ([1.0, 0.92, 0.7], 2.4),
                _ => (part.color, 0.0),
            };
            Self::push(draws, part, root * part.base, color, glow);
        }
    }

    /// The sanctuary's font at `foot`: its water lit, its crystals slowly turning.
    pub(super) fn draw_font(&self, draws: &mut Vec<MeshDraw3D>, foot: Vec3, yaw: f32, t: f32) {
        let root = Mat4::from_translation(foot) * Mat4::from_rotation_y(yaw);
        for part in self.parts.iter().filter(|p| p.name.starts_with("pila_")) {
            let (local, glow) = match part.name.as_str() {
                "pila_agua" => (part.base, 1.2 + 0.2 * (t * 1.3).sin()),
                "pila_cristal" => (part.base * Mat4::from_rotation_y(t * 0.3), 0.9),
                _ => (part.base, 0.0),
            };
            Self::push(draws, part, root * local, part.color, glow);
        }
    }
}
