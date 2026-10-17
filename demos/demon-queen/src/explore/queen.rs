//! La Reina: su escala, sus atuendos y tintes, el pelo del vestido verde ajustado a su cabeza, el posado de sus mallas animadas y las huellas que deja.
use super::ui_sfx::{self, UiSfx};
use super::*;

pub(super) const QUEEN_WORLD_SCALE: f32 = 2.0;

pub(super) const GREEN_HAIR_FORWARD_OFFSET: f32 = 0.075;

pub(super) const GREEN_HAIR_CONTACT_DROP: f32 = 0.035;

pub(super) const GREEN_HAIR_HEAD_FIT_XZ: f32 = 0.90;

pub(super) const GREEN_HAIR_HEAD_FIT_Y: f32 = 0.98;

pub(super) const QUEEN_FOOTPRINT_WIDTH_SCALE: f32 = 0.68;

pub(super) const QUEEN_FOOTPRINT_LENGTH_SCALE: f32 = 0.44;

pub(super) const QUEEN_FOOTPRINT_SIDE_OFFSET: f32 = 0.10;

/// Outfits past the two built in: (slot, name, model file). From the achievements catalog
/// (outfits with a `model` whose file is there) and, for testing, `VELVET_TEST_OUTFIT=<file>`.
pub(super) fn extra_outfit_specs() -> &'static [(u8, String, PathBuf)] {
    static SPECS: OnceLock<Vec<(u8, String, PathBuf)>> = OnceLock::new();
    SPECS.get_or_init(|| {
        let dir = runtime_3d_root().join("characters");
        let mut out: Vec<(u8, String, PathBuf)> = achievements::catalog()
            .outfits
            .iter()
            .filter(|o| o.slot >= 2)
            .filter_map(|o| {
                o.model
                    .as_ref()
                    .map(|m| (o.slot, o.name.clone(), dir.join(m)))
            })
            .filter(|(_, _, p)| p.is_file())
            .collect();
        if let Ok(file) = std::env::var("VELVET_TEST_OUTFIT") {
            let slot = out.iter().map(|(s, ..)| s + 1).max().unwrap_or(2).max(2);
            let path = dir.join(&file);
            if path.is_file() {
                out.push((slot, "Atuendo de prueba".to_owned(), path));
            }
        }
        out.sort_by_key(|(s, ..)| *s);
        out
    })
}

/// How many outfits the wardrobe holds (the two built in and the extra ones).
pub(super) fn wardrobe_outfit_count() -> u8 {
    extra_outfit_specs()
        .iter()
        .map(|(s, ..)| s + 1)
        .max()
        .unwrap_or(2)
        .max(2)
}

pub(super) fn outfit_tint(outfit: u8) -> [f32; 3] {
    match outfit % wardrobe_outfit_count() {
        // Original dark royal palette.
        0 => [1.0, 1.0, 1.0],
        // The forest outfit is an authored green-dress GLB. Keep its
        // embedded albedo untouched; the model swap below supplies the look.
        1 => [1.0, 1.0, 1.0],
        _ => [1.0, 1.0, 1.0],
    }
}

pub(super) fn material_for_outfit(mut material: Material3D, outfit: u8) -> Material3D {
    let tint = outfit_tint(outfit);
    for (channel, factor) in material.base_color[..3].iter_mut().zip(tint) {
        *channel *= factor;
    }
    material
}

pub(super) fn head_vertex_indices(mesh: &Mesh3D) -> Vec<usize> {
    // Sampled/skinned runtime frame keeps the head-height separation in Y.
    let min_height = mesh
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::INFINITY, f32::min);
    let max_height = mesh
        .vertices
        .iter()
        .map(|vertex| vertex.position[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let threshold = min_height + (max_height - min_height).max(0.001) * 0.70;
    let mut indices = mesh
        .vertices
        .iter()
        .enumerate()
        .filter_map(|(index, vertex)| (vertex.position[1] >= threshold).then_some(index))
        .collect::<Vec<_>>();
    if indices.len() < 32 {
        indices = mesh
            .vertices
            .iter()
            .enumerate()
            .filter_map(|(index, vertex)| {
                (vertex.position[1] >= min_height + (max_height - min_height).max(0.001) * 0.60)
                    .then_some(index)
            })
            .collect();
    }
    indices
}

pub(super) fn vertex_bounds_center(vertices: &[MeshVertex3D], indices: &[usize]) -> Vec3 {
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut found = false;
    for &index in indices {
        let Some(vertex) = vertices.get(index) else {
            continue;
        };
        let position = Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]);
        min.x = min.x.min(position.x);
        min.y = min.y.min(position.y);
        min.z = min.z.min(position.z);
        max.x = max.x.max(position.x);
        max.y = max.y.max(position.y);
        max.z = max.z.max(position.z);
        found = true;
    }
    if found {
        (min + max) * 0.5
    } else {
        Vec3::ZERO
    }
}

pub(super) fn ping_pong_animation_time(phase: f32, duration: f32) -> f32 {
    let duration = duration.max(0.1);
    let folded = phase.rem_euclid(duration * 2.0);
    if folded <= duration {
        folded
    } else {
        duration * 2.0 - folded
    }
}

/// An outfit with its own model.
pub(super) struct ExtraOutfit {
    pub(super) slot: u8,
    pub(super) model: GltfModel,
    pub(super) meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    pub(super) walk: Option<usize>,
    pub(super) run: Option<usize>,
    pub(super) idle: Option<usize>,
    pub(super) relax: retarget::RelaxClips,
}

/// Poses a character's GPU-skinned meshes (from `upload_skinned_mesh`): each skinned primitive
/// gets the joint matrices of `pose`; rigid ones are left as they are.
pub(super) fn pose_skinned_meshes(
    renderer: &mut Wgpu3DRenderer,
    queue: &wgpu::Queue,
    model: &GltfModel,
    meshes: &[(MeshHandle, Material3D, Option<TextureHandle>)],
    pose: &velvet_render::Pose3D,
) {
    for (index, (handle, _, _)) in meshes.iter().enumerate() {
        if let Some(joints) = model.joint_matrices(index, pose) {
            let _ = renderer.set_skin_joints(queue, *handle, &joints);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Footprint {
    pub(super) position: Vec3,
    pub(super) rotation_y: f32,
    pub(super) age: f32,
    pub(super) scale: f32,
}

/// A two-pad sole decal used for dynamic footprints.  Separating heel and toe
/// leaves a small tread gap that reads clearly at the isometric camera scale;
/// the wider toe and tapered heel make each print read as a step instead of a
/// generic dark blob. One mesh is reused for every step, so the trail stays
/// cheap to draw.
pub(super) fn build_nature_footprint_mesh() -> Mesh3D {
    const PADS: &[&[(f32, f32)]] = &[
        // Heel pad.
        &[
            (-0.100, -0.255),
            (0.100, -0.255),
            (0.112, -0.120),
            (0.066, 0.008),
            (-0.066, 0.008),
            (-0.112, -0.120),
        ],
        // Toe pad, wider and rounder than the heel, with a soft taper at the
        // front so the silhouette follows a real boot sole.
        &[
            (-0.125, 0.035),
            (0.125, 0.035),
            (0.140, 0.120),
            (0.108, 0.188),
            (0.066, 0.248),
            (0.020, 0.292),
            (-0.020, 0.292),
            (-0.066, 0.248),
            (-0.108, 0.188),
            (-0.140, 0.120),
        ],
    ];
    let mut vertices = Vec::with_capacity(20);
    let mut indices = Vec::with_capacity(30);
    for outline in PADS {
        let center = vertices.len() as u32;
        vertices.push(MeshVertex3D::new(
            [
                0.0,
                0.0,
                (outline[0].1 + outline[outline.len() - 1].1) * 0.5,
            ],
            [0.0, 1.0, 0.0],
            [0.5, 0.5],
        ));
        let first = vertices.len() as u32;
        for &(x, z) in *outline {
            vertices.push(MeshVertex3D::new(
                [x, 0.0, z],
                [0.0, 1.0, 0.0],
                [x * 3.0 + 0.5, z * 2.0 + 0.5],
            ));
        }
        for index in 0..outline.len() {
            let current = first + index as u32;
            let next = first + ((index + 1) % outline.len()) as u32;
            indices.extend_from_slice(&[center, next, current]);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::rgb(0.055, 0.018, 0.008),
        metallic_factor: 0.0,
        roughness_factor: 0.7,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

impl RenderState {
    /// The Queen's meshes for an outfit (its own model, the green dress, or her own), with each
    /// part's transform in the model.
    pub(super) fn queen_parts(
        &self,
        outfit: u8,
    ) -> Vec<(MeshHandle, Material3D, Option<TextureHandle>, Mat4)> {
        if let Some(e) = self.extra_outfits.iter().find(|e| e.slot == outfit) {
            return e
                .meshes
                .iter()
                .enumerate()
                .map(|(i, (h, m, t))| (*h, *m, *t, e.model.mesh_transform(i)))
                .collect();
        }
        if outfit % wardrobe_outfit_count() == 1 {
            if let Some(g) = self.green_model.as_ref() {
                return self
                    .green_queen_meshes
                    .iter()
                    .enumerate()
                    .map(|(i, (h, m, t))| (*h, *m, *t, g.mesh_transform(i)))
                    .collect();
            }
        }
        self.queen_meshes
            .iter()
            .enumerate()
            .map(|(i, (h, m, t))| (*h, *m, *t, self.model.mesh_transform(i)))
            .collect()
    }
}

impl App {
    /// The wardrobe's card under the pointer (the 1920×1080 screen), as
    /// `persona_screens::draw_wardrobe` lays them out: side by side from y 236, 720 tall.
    pub(super) fn wardrobe_card_at(&self, x: f32, y: f32) -> Option<u8> {
        let count = self.wardrobe_view().outfits.len().max(2);
        let width = WINDOW_WIDTH as f32;
        let gap = 46.0;
        let card_w = ((width - 260.0 - gap * (count as f32 - 1.0)) / count as f32).min(430.0);
        let x0 = (width - (card_w * count as f32 + gap * (count as f32 - 1.0))) * 0.5;
        if !(236.0..956.0).contains(&y) || x < x0 {
            return None;
        }
        let i = ((x - x0) / (card_w + gap)) as usize;
        (i < count && x - x0 - i as f32 * (card_w + gap) <= card_w).then_some(i as u8)
    }

    /// Puts on the outfit chosen in the wardrobe (Enter, or a second click on its card) and
    /// closes it; one the place does not allow is refused (and its note shown).
    pub(super) fn wardrobe_confirm(&mut self) {
        let chosen = self.wardrobe_selection % wardrobe_outfit_count();
        if !self.wardrobe_allows(chosen) {
            self.wardrobe_selection = self.outfit;
            self.wardrobe_open = false;
            self.clear_input();
            return;
        }
        self.outfit = chosen;
        ui_sfx::play(UiSfx::Equip);
        if let Some(o) = achievements::catalog().outfit_in_slot(self.outfit) {
            let event = format!("outfit:{}", o.id);
            self.achievement_event(&event);
        }
        self.wardrobe_open = false;
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// A click in the wardrobe: a card is chosen, and a second click on it puts it on.
    pub(super) fn wardrobe_click(&mut self) {
        let Some(slot) = self
            .shop_pointer_position()
            .and_then(|(x, y)| self.wardrobe_card_at(x, y))
        else {
            return;
        };
        if slot == self.wardrobe_selection {
            self.wardrobe_confirm();
        } else if self
            .achievements
            .outfit_available(achievements::catalog(), slot)
        {
            self.wardrobe_selection = slot;
            ui_sfx::play(UiSfx::Move);
        }
    }

    /// The wardrobe slot after `from`, `step` (±1) at a time, skipping outfits not yet earned.
    pub(super) fn wardrobe_step(&self, from: u8, step: i32) -> u8 {
        let n = wardrobe_outfit_count() as i32;
        let mut slot = from as i32;
        for _ in 0..n {
            slot = (slot + step).rem_euclid(n);
            if self
                .achievements
                .outfit_available(achievements::catalog(), slot as u8)
            {
                return slot as u8;
            }
        }
        from
    }
}

impl RenderState {
    /// Pose the Queen (her outfit's clip for walking, running, standing or relaxing) and the
    /// people around her for this frame; the walk's bob and whether the green dress is on.
    pub(super) fn pose_characters(&mut self, frame: &Frame) -> (f32, bool) {
        let Frame {
            moving,
            running,
            elapsed,
            outfit,
            baths_state,
            brothel_state,
            drink,
            lying,
            ..
        } = *frame;
        // Walking up behind a girl of the brothel, slower than her own pace.
        let escort_rate = if brothel_state.is_some_and(|b| b.escort.is_some()) {
            brothel::ESCORT_SPEED / 2.2
        } else {
            1.0
        };
        let bob = if moving {
            // Keep the walk bob subtle enough that the feet do not dip below
            // the taller square stepping stones on the downbeat.
            (elapsed * 9.0).sin() * 0.014
        } else {
            0.0
        };
        let green_outfit = outfit % wardrobe_outfit_count() == 1 && self.green_model.is_some();
        let extra = self.extra_outfits.iter().find(|e| e.slot == outfit);
        let (walk_animation, run_animation, idle_animation) = if let Some(e) = extra {
            (e.walk, e.run.or(e.walk), e.idle)
        } else if green_outfit {
            (
                self.green_walk_animation,
                self.green_run_animation,
                self.green_idle_animation,
            )
        } else {
            (self.walk_animation, self.run_animation, self.idle_animation)
        };
        let relax_clips = match extra {
            Some(e) => e.relax,
            None if green_outfit => self.green_relax,
            None => self.relax,
        };
        let relaxing = baths_state
            .map(|b| (b.relax, b.relax_blend))
            .filter(|(r, _)| *r != baths::Relax::None);
        // In her own bed she lies as on the baths' loungers.
        let in_bed = lying.then_some(relax_clips.lie).flatten();
        let relax_clip = in_bed.or_else(|| {
            relaxing.and_then(|(r, _)| match r {
                baths::Relax::Soaking => relax_clips.sit,
                baths::Relax::Lying { .. } => relax_clips.lie,
                baths::Relax::None => None,
            })
        });
        let drink_clip = drink.and_then(|t| relax_clips.drink.map(|clip| (clip, t)));
        let sampled_clip = if let Some(drinking) = drink_clip {
            Some(drinking)
        } else if let Some(clip) = relax_clip {
            Some((clip, elapsed))
        } else if moving {
            let clip = if running {
                run_animation.or(walk_animation)
            } else {
                walk_animation
            };
            let playback_rate = if running && run_animation.is_some() {
                // Both green-dress clips are compact Mixamo loops. Slightly
                // increase playback so cadence matches world movement.
                1.08
            } else {
                1.0
            };
            clip.map(|clip| (clip, elapsed * playback_rate * escort_rate))
        } else if let Some(clip) = idle_animation {
            Some((clip, elapsed))
        } else {
            // A Mixamo download often contains only the requested walk clip.
            // Hold its first frame while stationary instead of freezing at an
            // arbitrary point from the previous step.
            walk_animation.map(|clip| (clip, 0.0))
        };
        let sample_key = sampled_clip.map(|(clip, sample_time)| (clip, sample_time.to_bits()));
        let extra_index = self.extra_outfits.iter().position(|e| e.slot == outfit);
        if let Some(k) = extra_index {
            if let Some((clip, sample_time)) = sampled_clip {
                let e = &self.extra_outfits[k];
                if let Some(pose) = e.model.pose(clip, sample_time) {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        &e.model,
                        &e.meshes,
                        &pose,
                    );
                }
            }
        } else if green_outfit {
            if self.last_green_skin_sample != sample_key {
                if let (Some((clip, sample_time)), Some(green_model)) =
                    (sampled_clip, self.green_model.as_ref())
                {
                    // The green hair is deliberately unskinned. Track the
                    // animated head from its own body vertices instead of a
                    // bone, then apply only that geometric displacement to
                    // the authored hair cards.
                    let body_vertices = green_model.sample_animation(0, clip, sample_time);
                    for (index, (handle, _, _)) in self.green_queen_meshes.iter().enumerate() {
                        if Some(index) == self.green_hair_mesh_index {
                            continue;
                        }
                        let vertices = if index == 0 {
                            body_vertices.clone()
                        } else {
                            green_model.sample_animation(index, clip, sample_time)
                        };
                        if let Some(vertices) = vertices {
                            let _ =
                                self.renderer
                                    .update_mesh_vertices(&self.queue, *handle, &vertices);
                        }
                    }
                    if let (Some(body_vertices), Some(hair_index)) =
                        (body_vertices.as_ref(), self.green_hair_mesh_index)
                    {
                        let head_center =
                            vertex_bounds_center(body_vertices, &self.green_head_indices);
                        let body_transform = green_model.mesh_transform(0);
                        let hair_transform = green_model.mesh_transform(hair_index);
                        let head_world = body_transform.transform_point3(head_center);
                        let mut hair_anchor_world = Vec3::ZERO;
                        let mut anchor_count = 0.0;
                        for &index in &self.green_hair_anchor_indices {
                            if let Some(vertex) = self.green_hair_base_vertices.get(index) {
                                hair_anchor_world += hair_transform.transform_point3(Vec3::new(
                                    vertex.position[0],
                                    vertex.position[1],
                                    vertex.position[2],
                                ));
                                anchor_count += 1.0;
                            }
                        }
                        if anchor_count > 0.0 {
                            hair_anchor_world = hair_anchor_world * (1.0 / anchor_count);
                        } else {
                            hair_anchor_world = hair_transform.transform_point3(Vec3::ZERO);
                        }
                        // Hair is an unskinned static child. Align its
                        // scalp anchor to the animated head in world space,
                        // then convert the rigid displacement back to
                        // hair-local coordinates. No bone weights or physics.
                        let forward_world = body_transform.transform_vector3(Vec3::new(
                            0.0,
                            GREEN_HAIR_FORWARD_OFFSET,
                            0.0,
                        ));
                        let contact_drop_world = Vec3::new(0.0, -GREEN_HAIR_CONTACT_DROP, 0.0);
                        let hair_inverse = hair_transform.inverse().unwrap_or(Mat4::IDENTITY);
                        if let Some((handle, _, _)) = self.green_queen_meshes.get(hair_index) {
                            let mut hair_vertices = self.green_hair_base_vertices.clone();
                            for vertex in &mut hair_vertices {
                                let base_local = Vec3::new(
                                    vertex.position[0],
                                    vertex.position[1],
                                    vertex.position[2],
                                );
                                let base_world = hair_transform.transform_point3(base_local);
                                let relative = base_world - hair_anchor_world;
                                let fitted_world = head_world
                                    + Vec3::new(
                                        relative.x * GREEN_HAIR_HEAD_FIT_XZ,
                                        relative.y * GREEN_HAIR_HEAD_FIT_Y,
                                        relative.z * GREEN_HAIR_HEAD_FIT_XZ,
                                    )
                                    + forward_world
                                    + contact_drop_world;
                                vertex.position =
                                    hair_inverse.transform_point3(fitted_world).to_array();
                            }
                            let _ = self.renderer.update_mesh_vertices(
                                &self.queue,
                                *handle,
                                &hair_vertices,
                            );
                        }
                    }
                }
                self.last_green_skin_sample = sample_key;
            }
        } else if self.last_skin_sample != sample_key {
            if let Some((clip, sample_time)) = sampled_clip {
                if let Some(pose) = self.model.pose(clip, sample_time) {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        &self.model,
                        &self.queen_meshes,
                        &pose,
                    );
                }
            }
            self.last_skin_sample = sample_key;
        }
        // The outdoor mage alternates the two supplied Mixamo clips as one
        // continuous idle loop. Each clip plays forward and then reverses to
        // its initial pose before the next clip starts, avoiding the visible
        // end-to-start snap of a normal modulo loop.
        let mage_animation_mix = match (self.mage_dismissing_animation, self.mage_petting_animation)
        {
            (Some(dismiss), Some(petting)) => {
                let dismiss_duration = self
                    .mage_model
                    .animation_clips
                    .get(dismiss)
                    .map(|clip| clip.duration.max(0.1))
                    .unwrap_or(2.3);
                let petting_duration = self
                    .mage_model
                    .animation_clips
                    .get(petting)
                    .map(|clip| clip.duration.max(0.1))
                    .unwrap_or(8.3);
                let dismiss_cycle = dismiss_duration * 2.0;
                let petting_cycle = petting_duration * 2.0;
                let blend_duration = 0.38_f32
                    .min(dismiss_duration * 0.22)
                    .min(petting_duration * 0.22)
                    .max(0.08);
                let phase = elapsed.rem_euclid(dismiss_cycle + petting_cycle);
                if phase < dismiss_cycle {
                    let primary = (dismiss, ping_pong_animation_time(phase, dismiss_duration));
                    let blend = if phase >= dismiss_cycle - blend_duration {
                        let progress = (phase - (dismiss_cycle - blend_duration)) / blend_duration;
                        // Hold the next clip at its first pose while the
                        // current clip returns there. This removes the hard
                        // keyframe jump without changing either clip's timing.
                        Some((petting, 0.0_f32, progress.clamp(0.0, 1.0)))
                    } else {
                        None
                    };
                    (primary, blend)
                } else {
                    let petting_phase = phase - dismiss_cycle;
                    let primary = (
                        petting,
                        ping_pong_animation_time(petting_phase, petting_duration),
                    );
                    let blend = if petting_phase >= petting_cycle - blend_duration {
                        let progress =
                            (petting_phase - (petting_cycle - blend_duration)) / blend_duration;
                        // The dismissing clip starts from its neutral pose;
                        // cross-fade into that pose before the cycle wraps.
                        Some((dismiss, 0.0_f32, progress.clamp(0.0, 1.0)))
                    } else {
                        None
                    };
                    (primary, blend)
                }
            }
            (Some(clip), None) | (None, Some(clip)) => {
                let duration = self
                    .mage_model
                    .animation_clips
                    .get(clip)
                    .map(|animation| animation.duration.max(0.1))
                    .unwrap_or(1.0);
                ((clip, ping_pong_animation_time(elapsed, duration)), None)
            }
            (None, None) => ((usize::MAX, 0.0_f32), None),
        };
        let (mage_sampled_clip, mage_blend) = mage_animation_mix;
        let mage_sample_key = (
            mage_sampled_clip.0,
            mage_sampled_clip.1.to_bits(),
            mage_blend.map(|(clip, _, _)| clip).unwrap_or(usize::MAX),
            mage_blend
                .map(|(_, sample_time, _)| sample_time.to_bits())
                .unwrap_or(0),
            mage_blend
                .map(|(_, _, weight)| weight.to_bits())
                .unwrap_or(0),
        );
        if self.last_mage_skin_sample != Some(mage_sample_key) {
            if mage_sampled_clip.0 != usize::MAX {
                // The cross-fade blends the two poses (joint by joint), then the GPU deforms.
                let pose = self
                    .mage_model
                    .pose(mage_sampled_clip.0, mage_sampled_clip.1)
                    .map(|primary| {
                        match mage_blend.and_then(|(clip, time, weight)| {
                            Some((self.mage_model.pose(clip, time)?, weight))
                        }) {
                            Some((other, weight)) => primary.blend(&other, weight),
                            None => primary,
                        }
                    });
                if let Some(pose) = pose {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        &self.mage_model,
                        &self.mage_meshes,
                        &pose,
                    );
                }
            }
            self.last_mage_skin_sample = Some(mage_sample_key);
        }
        // The primary maid uses the authored walk loop continuously. Apply
        // Character Lab arm spacing after animation channels so it remains
        // stable instead of being overwritten by each keyframe.
        let maid_sample_key = self
            .maid_walk_animation
            .map(|clip| (clip, (elapsed * 1.0).to_bits()));
        if self.last_maid_skin_sample != maid_sample_key {
            if let Some(pose) = self
                .maid_walk_animation
                .and_then(|clip| self.maid_model.pose(clip, elapsed))
            {
                pose_skinned_meshes(
                    &mut self.renderer,
                    &self.queue,
                    &self.maid_model,
                    &self.maid_meshes,
                    &pose,
                );
            }
            self.last_maid_skin_sample = maid_sample_key;
        }
        let maid_alt_sample_key = self
            .maid_alt_walk_animation
            .map(|clip| (clip, (elapsed * 1.0).to_bits()));
        if self.last_maid_alt_skin_sample != maid_alt_sample_key {
            if let (Some(model), Some(clip)) =
                (self.maid_alt_model.as_ref(), self.maid_alt_walk_animation)
            {
                if let Some(pose) = model.pose(clip, elapsed) {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        model,
                        &self.maid_alt_meshes,
                        &pose,
                    );
                }
            }
            self.last_maid_alt_skin_sample = maid_alt_sample_key;
        }
        (bob, green_outfit)
    }
}

/// The pose she stands in, for a clip that starts and ends on it: her idle (it moves), or the
/// walk's first frame (held).
fn stand_clip(idle: Option<usize>, walk: Option<usize>) -> Option<(usize, bool)> {
    idle.map(|c| (c, true)).or(walk.map(|c| (c, false)))
}

impl RenderState {
    /// Her model for an outfit and its drink clip.
    fn drink_model(&self, outfit: u8) -> Option<(&GltfModel, usize)> {
        if let Some(e) = self.extra_outfits.iter().find(|e| e.slot == outfit) {
            return Some((&e.model, e.relax.drink?));
        }
        if outfit % wardrobe_outfit_count() == 1 {
            if let Some(g) = self.green_model.as_ref() {
                return Some((g, self.green_relax.drink?));
            }
        }
        Some((&self.model, self.relax.drink?))
    }

    /// How she lies (the baths' lying clip) for an outfit, in her model: the way her body runs
    /// (hips to head), where her head is, and how low her back is (scene units).
    pub(super) fn queen_lying(&self, outfit: u8) -> Option<(Vec3, Vec3, f32)> {
        let (model, relax) = if let Some(e) = self.extra_outfits.iter().find(|e| e.slot == outfit) {
            (&e.model, e.relax)
        } else if outfit % wardrobe_outfit_count() == 1 && self.green_model.is_some() {
            (self.green_model.as_ref()?, self.green_relax)
        } else {
            (&self.model, self.relax)
        };
        let pose = model.pose(relax.lie?, 0.0)?;
        let at = |n: &str| {
            model
                .pose_node_transform(&pose, n)
                .map(|m| Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2]))
        };
        let head = at("mixamorig:Head").or_else(|| at("head"))?;
        let hips = at("mixamorig:Hips").or_else(|| at("pelvis"))?;
        Some((head - hips, head, relax.lie_low))
    }

    /// Her right hand in her model, `t` seconds into the drink.
    pub(super) fn queen_hand(&self, outfit: u8, t: f32) -> Option<Mat4> {
        let (model, clip) = self.drink_model(outfit)?;
        model.sample_animation_node_transform(clip, t, elixir::HAND)
    }

    /// Her lips, her belly and the flask in her model, `t` seconds into the drink (for the words
    /// the overlay puts beside them).
    pub(super) fn queen_drink_points(&self, outfit: u8, t: f32) -> Option<(Vec3, Vec3, Vec3)> {
        let (model, clip) = self.drink_model(outfit)?;
        let (lips, belly) = elixir::lips_and_belly(model, clip, t)?;
        let hand = model.sample_animation_node_transform(clip, t, elixir::HAND)?;
        let flask = elixir::in_hand(hand, t).transform_point3(Vec3::new(0.0, 0.3, 0.0));
        Some((lips, belly, flask))
    }
}

/// The Queen as the world draws her: her model with its walk, run, idle and relax clips, the
/// green dress (its hair fitted to her head), and the outfits with models of their own.
pub(super) struct QueenAssets {
    pub(super) model: GltfModel,
    pub(super) relax: retarget::RelaxClips,
    pub(super) walk_animation: Option<usize>,
    pub(super) run_animation: Option<usize>,
    pub(super) idle_animation: Option<usize>,
    pub(super) queen_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    pub(super) green_relax: retarget::RelaxClips,
    pub(super) green_model: Option<GltfModel>,
    pub(super) green_queen_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    pub(super) green_walk_animation: Option<usize>,
    pub(super) green_run_animation: Option<usize>,
    pub(super) green_idle_animation: Option<usize>,
    pub(super) extra_outfits: Vec<ExtraOutfit>,
    pub(super) green_hair_mesh_index: Option<usize>,
    pub(super) green_hair_base_vertices: Vec<MeshVertex3D>,
    pub(super) green_hair_anchor_indices: Vec<usize>,
    pub(super) green_head_indices: Vec<usize>,
}

/// Load it: the Queen as the world draws her: her model with its walk, run, idle and relax clips, the
pub(super) fn load_queen(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_file: &Path,
    model_texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white_texture: &TextureData3D,
) -> Result<QueenAssets> {
    let mut model = GltfModel::load(model_file)
        .with_context(|| format!("load authored Queen GLB: {}", model_file.display()))?;
    // Sitting back in a pool and lying on a lounger, borrowed from the spa girl's clips.
    let mut relax = retarget::add_relax_clips(&mut model);
    if model.meshes.is_empty() {
        anyhow::bail!("Queen GLB contains no mesh primitives");
    }
    eprintln!(
        "[3d] queen meshes={} vertices={} animations={:?}",
        model.meshes.len(),
        model
            .meshes
            .iter()
            .map(|mesh| mesh.vertices.len())
            .sum::<usize>(),
        model.animations
    );
    let walk_animation = model.animation_index("Queen_Walk");
    let run_animation = model.animation_index("Queen_Run");
    let idle_animation = model.animation_index("Queen_Idle");
    relax.drink = elixir::add_clip(&mut model, stand_clip(idle_animation, walk_animation));
    eprintln!(
        "[3d] animation clips: walk={walk_animation:?} run={run_animation:?} idle={idle_animation:?} drink={:?}",
        relax.drink
    );
    let mut queen_meshes = Vec::with_capacity(model.meshes.len());
    for mesh in &model.meshes {
        let handle = renderer.upload_skinned_mesh(&device, mesh)?;
        let albedo_data = mesh
            .texture_index
            .and_then(|index| model.textures.get(index));
        let normal_data = mesh
            .normal_texture_index
            .and_then(|index| model.textures.get(index));
        let texture = upload_texture_pair(
            renderer,
            &device,
            &queue,
            albedo_data,
            normal_data,
            model_texture_cache,
            &white_texture,
        )?;
        queen_meshes.push((handle, mesh.material, texture));
    }
    // Outfit 1 is a real authored model, not a material tint. Load it
    // once beside the default Queen so opening the wardrobe never stalls
    // or drops the current animation state.
    let green_file = green_dress_model_path();
    let mut green_relax = retarget::RelaxClips::default();
    let (
        green_model,
        green_queen_meshes,
        green_walk_animation,
        green_run_animation,
        green_idle_animation,
    ) = if green_file.is_file() {
        let mut green_model = GltfModel::load(&green_file)
            .with_context(|| format!("load green-dress Queen GLB: {}", green_file.display()))?;
        green_relax = retarget::add_relax_clips(&mut green_model);
        if green_model.meshes.is_empty() {
            anyhow::bail!("green-dress Queen GLB contains no mesh primitives");
        }
        let green_walk_animation = green_model.animation_index("Queen_Walk");
        let green_run_animation = green_model.animation_index("Queen_Run");
        let green_idle_animation = green_model.animation_index("Queen_Idle");
        green_relax.drink = elixir::add_clip(
            &mut green_model,
            stand_clip(green_idle_animation, green_walk_animation),
        );
        eprintln!(
            "[3d] green outfit meshes={} vertices={} animations={:?} walk={green_walk_animation:?} run={green_run_animation:?}",
            green_model.meshes.len(),
            green_model
                .meshes
                .iter()
                .map(|mesh| mesh.vertices.len())
                .sum::<usize>(),
            green_model.animations
        );
        let mut green_queen_meshes = Vec::with_capacity(green_model.meshes.len());
        for mesh in &green_model.meshes {
            let handle = renderer.upload_mesh(&device, mesh)?;
            let albedo_data = mesh
                .texture_index
                .and_then(|index| green_model.textures.get(index));
            let normal_data = mesh
                .normal_texture_index
                .and_then(|index| green_model.textures.get(index));
            let texture = upload_texture_pair(
                renderer,
                &device,
                &queue,
                albedo_data,
                normal_data,
                model_texture_cache,
                &white_texture,
            )?;
            green_queen_meshes.push((handle, mesh.material, texture));
        }
        (
            Some(green_model),
            green_queen_meshes,
            green_walk_animation,
            green_run_animation,
            green_idle_animation,
        )
    } else {
        eprintln!(
            "[3d] green-dress outfit unavailable: {} (wardrobe falls back to tint)",
            green_file.display()
        );
        (None, Vec::new(), None, None, None)
    };
    // Outfits with their own model: uploaded like the Queen's own (skinned on the GPU).
    let mut extra_outfits = Vec::new();
    for (slot, name, path) in extra_outfit_specs() {
        let mut model = match GltfModel::load(path) {
            Ok(m) if !m.meshes.is_empty() => m,
            Ok(_) => continue,
            Err(error) => {
                eprintln!("[3d] outfit {name} unavailable: {error}");
                continue;
            }
        };
        let mut relax = retarget::add_relax_clips(&mut model);
        let mut meshes = Vec::with_capacity(model.meshes.len());
        for mesh in &model.meshes {
            let handle = renderer.upload_skinned_mesh(&device, mesh)?;
            let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
            let normal = mesh
                .normal_texture_index
                .and_then(|i| model.textures.get(i));
            let texture = upload_texture_pair(
                renderer,
                &device,
                &queue,
                albedo,
                normal,
                model_texture_cache,
                &white_texture,
            )?;
            meshes.push((handle, mesh.material, texture));
        }
        let (walk, run, idle) = (
            model.animation_index("Queen_Walk"),
            model.animation_index("Queen_Run"),
            model.animation_index("Queen_Idle"),
        );
        relax.drink = elixir::add_clip(&mut model, stand_clip(idle, walk));
        eprintln!(
            "[3d] outfit {name} (slot {slot}): {} meshes, walk={walk:?} run={run:?} idle={idle:?}",
            meshes.len()
        );
        extra_outfits.push(ExtraOutfit {
            slot: *slot,
            model,
            meshes,
            walk,
            run,
            idle,
            relax,
        });
    }
    let (
        green_hair_mesh_index,
        green_hair_base_vertices,
        green_hair_anchor_indices,
        green_head_indices,
    ) = green_model
        .as_ref()
        .map(|model| {
            let hair_index = model
                .meshes
                .iter()
                .enumerate()
                .find(|(_, mesh)| mesh.skinning.is_none())
                .map(|(index, _)| index);
            let base_vertices = hair_index
                .and_then(|index| model.meshes.get(index))
                .map(|mesh| mesh.vertices.clone())
                .unwrap_or_default();
            let head_indices = model
                .meshes
                .first()
                .map(head_vertex_indices)
                .unwrap_or_default();
            let head_center = model
                .meshes
                .first()
                .map(|mesh| vertex_bounds_center(&mesh.vertices, &head_indices))
                .unwrap_or(Vec3::ZERO);
            let head_world = model.mesh_transform(0).transform_point3(head_center);
            let hair_transform = hair_index
                .map(|index| model.mesh_transform(index))
                .unwrap_or(Mat4::IDENTITY);
            let mut ranked = base_vertices
                .iter()
                .enumerate()
                .map(|(index, vertex)| {
                    let position = hair_transform.transform_point3(Vec3::new(
                        vertex.position[0],
                        vertex.position[1],
                        vertex.position[2],
                    ));
                    ((position - head_world).length_squared(), index)
                })
                .collect::<Vec<_>>();
            ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let anchor_count = (ranked.len() / 5).max(64).min(ranked.len());
            let anchor_indices = ranked
                .into_iter()
                .take(anchor_count)
                .map(|(_, index)| index)
                .collect::<Vec<_>>();
            (hair_index, base_vertices, anchor_indices, head_indices)
        })
        .unwrap_or((None, Vec::new(), Vec::new(), Vec::new()));
    Ok(QueenAssets {
        model,
        relax,
        walk_animation,
        run_animation,
        idle_animation,
        queen_meshes,
        green_relax,
        green_model,
        green_queen_meshes,
        green_walk_animation,
        green_run_animation,
        green_idle_animation,
        extra_outfits,
        green_hair_mesh_index,
        green_hair_base_vertices,
        green_hair_anchor_indices,
        green_head_indices,
    })
}
