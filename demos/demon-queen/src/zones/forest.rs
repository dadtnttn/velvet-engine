//! El Bosque Oscuro como lugar: su sendero, la puerta del castillo, el mago y sus comentarios, y cómo se construye a partir de su exportación.
use super::*;

pub(super) const CASTLE_ENTRY_Z: f32 = -12.0;

pub(super) const CASTLE_ENTRY_SHOW_RADIUS: f32 = 7.0;

pub(super) const CASTLE_ENTRY_INTERACT_RADIUS: f32 = 2.65;

pub(super) const MAGE_SHOW_RADIUS: f32 = 5.5;

pub(super) const MAGE_INTERACT_RADIUS: f32 = 2.65;

pub(super) fn castle_entry_position() -> Vec3 {
    Vec3::new(nature_trail_center(CASTLE_ENTRY_Z), 0.0, CASTLE_ENTRY_Z)
}

pub(super) fn castle_entry_distance(position: Vec3) -> f32 {
    let entry = castle_entry_position();
    Vec3::new(position.x - entry.x, 0.0, position.z - entry.z).length()
}

pub(super) fn mage_position() -> Vec3 {
    Vec3::new(-3.80, 0.0, 3.70)
}

pub(super) fn mage_distance(position: Vec3) -> f32 {
    let mage = mage_position();
    Vec3::new(position.x - mage.x, 0.0, position.z - mage.z).length()
}

pub(super) const NPC_COMMENT_SHOW_RADIUS: f32 = 8.0;

pub(super) const NPC_COMMENT_CYCLE: f32 = 14.0;

#[derive(Clone, Copy)]
pub(super) struct NpcCommentLine {
    pub(super) start: f32,
    pub(super) duration: f32,
    pub(super) text: &'static str,
    pub(super) accent: [f32; 4],
}

pub(super) const MAGE_COMMENT_LINES: &[NpcCommentLine] = &[
    NpcCommentLine {
        start: 0.0,
        duration: 2.6,
        text: "Ps ps...",
        accent: [0.62, 0.32, 0.90, 1.0],
    },
    NpcCommentLine {
        start: 3.4,
        duration: 3.0,
        text: "La reina ya llegó.",
        accent: [0.46, 0.24, 0.82, 1.0],
    },
    NpcCommentLine {
        start: 7.4,
        duration: 2.8,
        text: "No mires ahora...",
        accent: [0.72, 0.40, 0.86, 1.0],
    },
    NpcCommentLine {
        start: 11.2,
        duration: 1.8,
        text: "Ven, te cuento.",
        accent: [0.52, 0.30, 0.88, 1.0],
    },
];

pub(super) fn active_npc_comment(elapsed: f32) -> Option<(NpcCommentLine, f32)> {
    let phase = elapsed.rem_euclid(NPC_COMMENT_CYCLE);
    let fade_window = 0.24_f32;
    MAGE_COMMENT_LINES.iter().copied().find_map(|line| {
        let end = line.start + line.duration;
        if phase < line.start || phase >= end {
            return None;
        }
        let fade_in = ((phase - line.start) / fade_window).clamp(0.0, 1.0);
        let fade_out = ((end - phase) / fade_window).clamp(0.0, 1.0);
        let smooth = |value: f32| value * value * (3.0 - 2.0 * value);
        Some((line, smooth(fade_in.min(fade_out))))
    })
}

/// Shared path centreline.  Keeping the same function for terrain, stones,
/// props and footprints prevents the visual trail from drifting away from the
/// gameplay route when its shape is tuned.
pub(super) fn nature_trail_center(z: f32) -> f32 {
    (z * 0.24).sin() * 1.55 + (z * 0.075).sin() * 0.22
}

pub(super) const NATURE_PLAYABLE_LIMIT: f32 = 25.5;

/// Shared walkable height for the character, marker and dynamic footprints.
/// The terrain relief remains continuous between stones, while this envelope
/// lifts actors to the actual top of the nearest stone instead of letting
/// their feet and decals pass through the taller square modules.
pub(super) fn nature_walk_surface_height(x: f32, z: f32) -> f32 {
    dark_forest::surface_height(x, z)
}

pub(super) struct ForestBuild {
    pub(super) draws: Vec<MeshDraw3D>,
    /// World bounds of the draws that can stand between the camera and her (trees, the castle's
    /// cells, the mine's frames), for the see-through window.
    pub(super) bounds: Vec<Option<(Vec3, Vec3)>>,
    /// Lantern glass and lit windows: lit with the night.
    pub(super) lamps: Vec<usize>,
    /// Fungi caps, runes, crystals and eyes: a cold glow that grows with the night.
    pub(super) glows: Vec<usize>,
    /// A small sphere for the campfire's smoke.
    pub(super) puff: Option<MeshHandle>,
    /// The pool's surface (world triangles), simulated instead of drawn.
    pub(super) water: Vec<[Vec3; 3]>,
}

/// How a forest material looks in the game: flames and embers burn, the lantern glass and the
/// glowing things are lit each frame by the hour, the rest takes soft contact shading.
pub(super) fn forest_material(name: &str, material: Material3D) -> Material3D {
    match name {
        "fire" | "fireCore" => material.with_emission(2.0),
        "ember" => material.with_emission(1.4),
        "waterPool" => material.with_emission(0.12),
        // Lilies float on the simulated water: drawn with what blends, after it.
        "lily" | "lilyFlower" => {
            let mut m = material.with_ambient_occlusion(0.18);
            m.base_color[3] = 0.985;
            m
        }
        _ if forest_lamp(name) || forest_glow(name) => material,
        _ => material.with_ambient_occlusion(0.18),
    }
}

pub(super) fn forest_lamp(name: &str) -> bool {
    matches!(name, "lampGlass" | "windowGlow")
}

pub(super) fn forest_glow(name: &str) -> bool {
    name.starts_with("caps") || matches!(name, "runes" | "staffOrb" | "oreGem" | "brew" | "eyes")
}

pub(super) fn build_dark_forest(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<ForestBuild> {
    prefetch::prefetch(zone_files(root));
    let plan = dark_forest::plan();
    let mut cache = HashMap::new();
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    let mut draws = Vec::new();
    let mut bounds = Vec::new();
    let mut lamps = Vec::new();
    let mut glows = Vec::new();
    let mut water = Vec::new();
    let world_box = |points: &mut dyn Iterator<Item = Vec3>, transform: Mat4| -> (Vec3, Vec3) {
        let mut lo = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut hi = -lo;
        for p in points {
            let w = transform.transform_point3(p);
            lo = Vec3::new(lo.x.min(w.x), lo.y.min(w.y), lo.z.min(w.z));
            hi = Vec3::new(hi.x.max(w.x), hi.y.max(w.y), hi.z.max(w.z));
        }
        (lo, hi)
    };
    // The baked parts, cut into cells so that frustum culling skips what is off screen (the
    // places into small ones: the see-through window takes a cell whole).
    for file in [
        "bosque_ground.glb",
        "bosque_trail.glb",
        "bosque_places.glb",
        "bosque_life.glb",
    ] {
        let can_hide_her = matches!(file, "bosque_places.glb" | "bosque_life.glb");
        let cell = if file == "bosque_places.glb" {
            6.0
        } else {
            16.0
        };
        let path = root.join(file);
        let model = prefetch::load(&path)
            .with_context(|| format!("load forest part: {}", path.display()))?;
        for (index, mesh) in model.meshes.iter().enumerate() {
            let name = model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
            let normal = mesh
                .normal_texture_index
                .and_then(|i| model.textures.get(i));
            let texture =
                upload_texture_pair(renderer, device, queue, albedo, normal, &mut cache, &white)?;
            let material = forest_material(name, mesh.material);
            let transform = model.mesh_transform(index);
            let lit = forest_lamp(name)
                || forest_glow(name)
                || matches!(name, "fire" | "fireCore" | "ember");
            if name == "waterPool" {
                water.extend(mesh.world_triangles(transform));
                continue;
            }
            for piece in mesh.split_into_cells(cell) {
                if forest_lamp(name) {
                    lamps.push(draws.len());
                } else if forest_glow(name) {
                    glows.push(draws.len());
                }
                bounds.push((can_hide_her && !lit).then(|| {
                    world_box(
                        &mut piece
                            .vertices
                            .iter()
                            .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2])),
                        transform,
                    )
                }));
                draws.push(MeshDraw3D {
                    mesh: renderer.upload_mesh(device, &piece)?,
                    transform,
                    material,
                    texture,
                });
            }
        }
    }
    // The trees, plants and rocks: each species uploaded once, drawn wherever the plan puts one,
    // its leaves tinted plant by plant.
    type Part = (
        MeshHandle,
        Material3D,
        Option<TextureHandle>,
        Mat4,
        bool,
        (Vec3, Vec3),
    );
    let mut species: HashMap<&str, Vec<Part>> = HashMap::new();
    for plant in &plan.plants {
        if !species.contains_key(plant.model.as_str()) {
            let path = root.join(format!("bosque_plant_{}.glb", plant.model));
            let model = prefetch::load(&path)
                .with_context(|| format!("load forest plant: {}", path.display()))?;
            let mut parts = Vec::with_capacity(model.meshes.len());
            for (index, mesh) in model.meshes.iter().enumerate() {
                let name = model
                    .mesh_names
                    .get(index)
                    .map(String::as_str)
                    .unwrap_or("");
                let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
                let normal = mesh
                    .normal_texture_index
                    .and_then(|i| model.textures.get(i));
                let texture = upload_texture_pair(
                    renderer, device, queue, albedo, normal, &mut cache, &white,
                )?;
                let local = model.mesh_transform(index);
                let size = world_box(
                    &mut mesh
                        .vertices
                        .iter()
                        .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2])),
                    local,
                );
                parts.push((
                    renderer.upload_mesh(device, mesh)?,
                    mesh.material.with_ambient_occlusion(0.18),
                    texture,
                    local,
                    name.starts_with("leaf:"),
                    size,
                ));
            }
            species.insert(plant.model.as_str(), parts);
        }
        let placement = Mat4::from_translation(Vec3::new(plant.x, plant.y, plant.z))
            * Mat4::from_rotation_y(plant.yaw)
            * Mat4::from_scale(Vec3::new(plant.s, plant.s, plant.s));
        for (mesh, material, texture, local, leaf, (lo, hi)) in &species[plant.model.as_str()] {
            let mut material = *material;
            if let (true, Some(tint)) = (*leaf, plant.tint) {
                for (channel, k) in material.base_color[..3].iter_mut().zip(tint) {
                    *channel *= k;
                }
            }
            let corners = [
                Vec3::new(lo.x, lo.y, lo.z),
                Vec3::new(hi.x, hi.y, hi.z),
                Vec3::new(lo.x, hi.y, hi.z),
                Vec3::new(hi.x, lo.y, lo.z),
            ];
            let placed = world_box(&mut corners.into_iter(), placement);
            bounds.push((placed.1.y - placed.0.y > 1.2).then_some(placed));
            draws.push(MeshDraw3D {
                mesh: *mesh,
                transform: placement * *local,
                material,
                texture: *texture,
            });
        }
    }
    let puff = renderer
        .upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 10, 6, Material3D::rgb(1.0, 1.0, 1.0)),
        )
        .ok();
    Ok(ForestBuild {
        draws,
        bounds,
        lamps,
        glows,
        puff,
        water,
    })
}

pub(super) fn nature_position_blocked(position: Vec3) -> bool {
    dark_forest::blocked(position)
}

/// Whether she stands at the way up to el Peñascal (the wood's west edge).
pub(super) fn penascal_way_near(position: Vec3) -> bool {
    dark_forest::exit("penascal")
        .is_some_and(|(at, r)| (at.x - position.x).hypot(at.z - position.z) <= r.max(1.6))
}

impl App {
    /// What F does in the wood: the castle's gate, the mage, the mine, the villa's gate.
    pub(super) fn forest_prompt(&self) -> Option<Prompt> {
        if castle_entry_distance(self.player) <= CASTLE_ENTRY_INTERACT_RADIUS {
            return Some(
                Prompt::new(Icon::Castle, "Entrar al castillo").detail("Puerta principal"),
            );
        }
        if mage_distance(self.player) <= MAGE_INTERACT_RADIUS {
            return Some(Prompt::new(Icon::Talk, "Hablar").detail("El mago hipnotista"));
        }
        if mine::entry_distance(self.player) <= mine::INTERACT_RADIUS {
            let detail = if self.has_weapon() {
                "Las Cavernas del Sello"
            } else {
                "Sin arma: no es seguro"
            };
            return Some(Prompt::new(Icon::Mine, "Bajar a la mina").detail(detail));
        }
        if village_entry_distance(self.player) <= VILLAGE_INTERACT_RADIUS {
            return Some(
                Prompt::new(Icon::House, "Entrar al pueblito").detail("La villa del valle"),
            );
        }
        if penascal_way_near(self.player) {
            return Some(Prompt::new(Icon::Forest, "Subir").detail("El camino del Peñascal"));
        }
        if shrine_near(self.player) {
            return Some(Prompt::new(Icon::Forest, "Mirar").detail("Santuario en ruinas"));
        }
        return None;
    }
}

impl App {
    /// F in the wood: the mage, the castle's gate, the mine, the villa's gate. Whether that ends
    /// the key's handling.
    pub(super) fn interact_forest(&mut self) -> bool {
        let close_to_mage = self.nature_mode
            && !self.village_entered
            && mage_distance(self.player) <= MAGE_INTERACT_RADIUS;
        if close_to_mage {
            self.map_open = false;
            self.clear_input();
            // Not to the Queen in her royal dress: he only stammers.
            if !self.mage_will_talk() {
                return true;
            }
            self.persist_3d_state(true);
            // His stall: talk, buy, or go (`explore/mage.rs`).
            self.open_mage();
            self.achievement_event("talk:mago");
            return true;
        }
        let close_to_castle_entry =
            self.nature_mode && castle_entry_distance(self.player) <= CASTLE_ENTRY_INTERACT_RADIUS;
        if close_to_castle_entry {
            self.enter_castle_from_nature();
            self.persist_3d_state(true);
            return true;
        }
        let close_to_mine = self.nature_mode
            && !self.village_entered
            && mine::entry_distance(self.player) <= mine::INTERACT_RADIUS;
        if close_to_mine {
            self.persist_3d_state(true);
            self.enter_dungeon();
            return true;
        }
        if self.nature_mode && !self.village_entered && shrine_near(self.player) {
            if !self.start_talk(None, Some("bosque_santuario")) {
                if let Some((_, text)) = dark_forest::spot("santuario") {
                    self.queen_thought = Some((text, self.elapsed + 5.0));
                }
            }
            return true;
        }
        if self.nature_mode && !self.village_entered && penascal_way_near(self.player) {
            // Up the way between the crags: el Peñascal.
            self.enter_penascal_from_forest();
            return true;
        }
        let close_to_village = self.nature_mode
            && !self.village_entered
            && village_entry_distance(self.player) <= VILLAGE_INTERACT_RADIUS;
        if close_to_village {
            // Through the gate: she stands just inside it,
            // the bridge and the plaza ahead of her.
            let (spawn, _) = village::spawn();
            self.player = spawn;
            self.camera_focus = self.player;
            self.village_entered = true;
            self.footprints.clear();
            self.footstep_distance = 0.0;
            eprintln!("[3d] entered Fantasy Village");
            self.achievement_event("visit:villa");
        }
        false
    }
}

/// Whether she is by the shrine in the wood (its spot in `data/bosque/bosque.json`).
fn shrine_near(player: Vec3) -> bool {
    dark_forest::spot("santuario")
        .is_some_and(|(at, _)| (at.x - player.x).hypot(at.z - player.z) <= 2.4)
}

impl App {
    /// The preview's title in the wood: what is near on the trail.
    pub(super) fn forest_title_hint(&self) -> &'static str {
        let castle_distance = castle_entry_distance(self.player);
        let village_distance = village_entry_distance(self.player);
        if castle_distance <= CASTLE_ENTRY_INTERACT_RADIUS {
            "  |  [F] Entrar al castillo"
        } else if castle_distance <= CASTLE_ENTRY_SHOW_RADIUS {
            "  |  entrada al castillo cercana"
        } else if mage_distance(self.player) <= MAGE_INTERACT_RADIUS {
            "  |  [F] Hablar con el mago"
        } else if mage_distance(self.player) <= MAGE_SHOW_RADIUS {
            "  |  mago cercano"
        } else if village_distance <= VILLAGE_INTERACT_RADIUS {
            "  |  [F] Entrar al pueblito"
        } else if village_distance <= VILLAGE_ARROW_SHOW_RADIUS {
            "  |  sigue la flecha al pueblito"
        } else {
            ""
        }
    }
}

/// The Dark Forest as a place.
pub(super) struct ForestZone;

impl Zone for ForestZone {
    fn id(&self) -> ZoneId {
        ZoneId::Forest
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.walk_outdoors(dt);
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        app.forest_prompt()
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_forest()
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Forest
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, green_outfit) = state.prepare_above_ground(frame);
        state.render_outdoors(frame, bob, green_outfit);
    }
    fn title(&self, app: &App) -> (&'static str, &'static str) {
        app.outdoors_title()
    }
    fn prefetch(&self, app: &mut App) {
        app.prefetch_villa();
    }
}

impl App {
    /// In the wood: the villa, read ahead from within 14 m of its gate (the game's own measure) and
    /// let go beyond 40 m, until it has loaded.
    pub(super) fn prefetch_villa(&mut self) {
        if !Self::prefetch_allowed() || self.fantasy_village_load_attempted() {
            return;
        }
        // The forest's way to the villa (its distance measured by the game's own rule).
        let d = village_entry_distance(self.player);
        if d < 14.0 && self.prefetched.insert("villa") {
            prefetch::prefetch(zone_files(&fantasy_village_asset_root()));
        } else if d > 40.0 && self.prefetched.remove("villa") {
            prefetch::forget(&fantasy_village_asset_root());
        }
    }
}

/// The mage of the wood: his model, his two idle clips and his meshes.
pub(super) struct MageAssets {
    pub(super) mage_model: GltfModel,
    pub(super) mage_dismissing_animation: Option<usize>,
    pub(super) mage_petting_animation: Option<usize>,
    pub(super) mage_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
}

/// Load it: the mage of the wood: his model, his two idle clips and his meshes.
pub(super) fn load_mage(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model_texture_cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white_texture: &TextureData3D,
) -> Result<MageAssets> {
    let mage_file = mage_model_path();
    let mage_model = GltfModel::load(&mage_file)
        .with_context(|| format!("load authored mage GLB: {}", mage_file.display()))?;
    if mage_model.meshes.is_empty() {
        anyhow::bail!("Mage GLB contains no mesh primitives");
    }
    let mage_dismissing_animation = mage_model.animation_index("Mage_Dismissing");
    let mage_petting_animation = mage_model.animation_index("Mage_Petting");
    eprintln!(
        "[3d] mage meshes={} vertices={} animations={:?} dismiss={mage_dismissing_animation:?} petting={mage_petting_animation:?}",
        mage_model.meshes.len(),
        mage_model
            .meshes
            .iter()
            .map(|mesh| mesh.vertices.len())
            .sum::<usize>(),
        mage_model.animations
    );
    let mut mage_meshes = Vec::with_capacity(mage_model.meshes.len());
    for mesh in &mage_model.meshes {
        let handle = renderer.upload_skinned_mesh(&device, mesh)?;
        let albedo_data = mesh
            .texture_index
            .and_then(|index| mage_model.textures.get(index));
        let normal_data = mesh
            .normal_texture_index
            .and_then(|index| mage_model.textures.get(index));
        let texture = upload_texture_pair(
            renderer,
            &device,
            &queue,
            albedo_data,
            normal_data,
            model_texture_cache,
            &white_texture,
        )?;
        mage_meshes.push((handle, mesh.material, texture));
    }
    Ok(MageAssets {
        mage_model,
        mage_dismissing_animation,
        mage_petting_animation,
        mage_meshes,
    })
}

/// The Dark Forest exported from its prototype: its draws, its lamps and glows, the pieces that
/// can hide her, the campfire's puff, its air and its pool's water.
pub(super) struct ForestAssets {
    pub(super) water_life: WaterLife3D,
    pub(super) forest_air: Option<dark_forest::Air>,
    pub(super) nature_draws: Vec<MeshDraw3D>,
    pub(super) forest_lamps: Vec<usize>,
    pub(super) forest_glows: Vec<usize>,
    pub(super) forest_bounds: Vec<Option<(Vec3, Vec3)>>,
    pub(super) forest_puff: Option<MeshHandle>,
}

/// Load it: the Dark Forest exported from its prototype: its draws, its lamps and glows, the pieces that
pub(super) fn load_forest(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<ForestAssets> {
    // The Dark Forest exported from its prototype.
    let mut water_life = WaterLife3D::new();
    let forest_started = Instant::now();
    let forest = build_dark_forest(renderer, &device, &queue, &dark_forest_asset_root())
        .with_context(|| {
            format!(
                "load the Dark Forest ({})",
                dark_forest_asset_root().display()
            )
        })?;
    eprintln!(
        "[3d] Dark Forest loaded in {:.0} ms: {} draw parts, {} lamps, {} glows (pack={})",
        forest_started.elapsed().as_secs_f64() * 1000.0,
        forest.draws.len(),
        forest.lamps.len(),
        forest.glows.len(),
        dark_forest_asset_root().display()
    );
    if let Some(body) = water_life::body(water_life::Kind::ForestPool, &forest.water, None) {
        water_life.add(renderer, &device, body, water_life::FOREST);
    }
    let forest_air = dark_forest::build_air(renderer, &device, &queue)
        .map_err(|error| eprintln!("[3d] forest air unavailable ({error:#})"))
        .ok();
    let ForestBuild {
        draws: nature_draws,
        lamps: forest_lamps,
        glows: forest_glows,
        bounds: forest_bounds,
        puff: forest_puff,
        ..
    } = forest;
    Ok(ForestAssets {
        water_life,
        forest_air,
        nature_draws,
        forest_lamps,
        forest_glows,
        forest_bounds,
        forest_puff,
    })
}

impl RenderState {
    /// What the wood adds to the frame: the mage, the trees and stones (cut round her where
    /// they hide her), the lanterns and glows by the hour, the air, the campfire's smoke and
    /// the medallions of the mine, the mage and the two gates.
    pub(super) fn forest_draws(
        &mut self,
        frame: &Frame,
        draws: &mut Vec<MeshDraw3D>,
        player_ground_y: f32,
    ) {
        let Frame {
            camera,
            player,
            elapsed,
            village_entered,
            environment,
            ..
        } = *frame;
        {
            let mage = mage_position();
            let mage_ground_y = nature_walk_surface_height(mage.x, mage.z);
            // Face the same trail direction as the Queen's default outdoor
            // pose. The fixed anchor keeps the NPC readable while the player
            // and camera move around it.
            let mage_matrix =
                Mat4::from_translation(Vec3::new(mage.x, mage_ground_y + 0.018, mage.z))
                    * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4);
            for (index, (mesh, material, texture)) in self.mage_meshes.iter().enumerate() {
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: mage_matrix * self.mage_model.mesh_transform(index),
                    material: (*material).with_ambient_occlusion(0.28),
                    texture: *texture,
                });
            }
        }
        if !self.nature_draws.is_empty() {
            let base = draws.len();
            draws.extend_from_slice(&self.nature_draws);
            ISO.cut_see_through(
                draws,
                base,
                &self.forest_bounds,
                &camera,
                player,
                player_ground_y,
            );
            // Lanterns glow faintly by day, brightly at night; fungi, runes and crystals keep
            // a cold glow that grows with the dark.
            let night = dark_forest::night(environment.celestial.moon_visibility);
            let lamp_glow = village_window_glow(environment.celestial.moon_visibility, 0.1);
            for &index in &self.forest_lamps {
                if let Some(draw) = draws.get_mut(base + index) {
                    let day = [1.0, 0.86, 0.62];
                    let lit = [1.0, 0.5, 0.1];
                    let mix = |i: usize| day[i] + (lit[i] - day[i]) * lamp_glow;
                    draw.material.base_color = [mix(0), mix(1), mix(2), 1.0];
                    draw.material = draw.material.with_emission(0.3 + lamp_glow * 1.5);
                }
            }
            for &index in &self.forest_glows {
                if let Some(draw) = draws.get_mut(base + index) {
                    draw.material = draw.material.with_emission(0.45 + night * 0.75);
                }
            }
            if let Some(air) = &self.forest_air {
                dark_forest::air(air, elapsed, &environment, player, draws);
            }
            // The campfire's smoke: thin grey puffs that rise and drift.
            if let Some(puff) = self.forest_puff {
                for (fire, top) in dark_forest::smoke().iter().enumerate() {
                    for k in 0..8 {
                        let age = (elapsed * 0.22 + k as f32 / 8.0 + fire as f32 * 0.37).fract();
                        let drift =
                            Vec3::new((age * 3.0).sin() * 0.3 + age * 0.6, age * 3.2, age * 0.25);
                        let size = 0.14 + age * 0.45;
                        let mut material = Material3D::rgb(0.55, 0.54, 0.6);
                        material.base_color[3] = 0.32 * (1.0 - age) * age.min(0.15) / 0.15;
                        draws.push(MeshDraw3D {
                            mesh: puff,
                            transform: Mat4::from_translation(*top + drift)
                                * Mat4::from_scale(Vec3::new(size, size * 0.85, size)),
                            material,
                            texture: None,
                        });
                    }
                }
            }
            // The mine's medallion over the pit.
            let mine_distance = mine::entry_distance(player);
            if mine_distance <= mine::SHOW_RADIUS {
                push_prompt_medallion(
                    Icon::Mine,
                    mine::entry_position() + Vec3::new(1.3, 0.0, 0.0),
                    2.05,
                    mine_distance,
                    mine::SHOW_RADIUS,
                    mine::INTERACT_RADIUS,
                );
            }
            // The mage's interaction key floats above the NPC in the same
            // chunky cap style used by the wardrobe and gates.  It lives
            // after the foliage draws so nearby leaves cannot swallow the
            // affordance, while the smoothstep fade avoids a hard pop at the
            // edge of the approach radius.
            let mage_distance = mage_distance(player);
            if mage_distance <= MAGE_SHOW_RADIUS {
                let mage = mage_position();
                let ground_y = nature_walk_surface_height(mage.x, mage.z);
                push_prompt_medallion(
                    Icon::Talk,
                    Vec3::new(mage.x, ground_y, mage.z),
                    2.72,
                    mage_distance,
                    MAGE_SHOW_RADIUS,
                    MAGE_INTERACT_RADIUS,
                );
            }
            // The castle gate is the return route to the interior.  Its
            // prompt floats above the doorway so the keep does not hide it.
            let castle_distance = castle_entry_distance(player);
            if castle_distance <= CASTLE_ENTRY_SHOW_RADIUS {
                let entry = castle_entry_position();
                let ground_y = nature_walk_surface_height(entry.x, entry.z);
                push_prompt_medallion(
                    Icon::Castle,
                    Vec3::new(entry.x, ground_y, entry.z),
                    4.05,
                    castle_distance,
                    CASTLE_ENTRY_SHOW_RADIUS,
                    CASTLE_ENTRY_INTERACT_RADIUS,
                );
            }
            // The final stone marks the village entrance.  Keep the marker in
            // world space so it remains anchored while the camera orbits; it
            // fades in as the player approaches instead of cluttering the
            // entire meadow.  The directional arrow is deliberately oversized
            // enough to read against foliage and roofs from every camera turn.
            if !village_entered {
                let village_distance = village_entry_distance(player);
                if village_distance <= VILLAGE_ARROW_SHOW_RADIUS {
                    let entry = village_entry_position();
                    let ground_y = nature_walk_surface_height(entry.x, entry.z);
                    push_prompt_medallion(
                        Icon::House,
                        Vec3::new(entry.x, ground_y, entry.z),
                        2.6,
                        village_distance,
                        VILLAGE_ARROW_SHOW_RADIUS,
                        VILLAGE_INTERACT_RADIUS,
                    );
                    let proximity =
                        (1.0 - village_distance / VILLAGE_ARROW_SHOW_RADIUS).clamp(0.0, 1.0);
                    // Fade over the whole approach band. The old marker kept
                    // a strong alpha until the radius edge, then vanished in
                    // one frame. Smoothstep keeps the arrow readable nearby
                    // while dissolving it naturally as the Queen walks away.
                    let fade = proximity * proximity * (3.0 - 2.0 * proximity);
                    let pulse = (elapsed * 3.8).sin() * 0.06;
                    let ring_scale = 0.92 + proximity * 0.12 + (elapsed * 2.4).sin().abs() * 0.025;
                    draws.push(MeshDraw3D {
                        mesh: self.village_entry_marker,
                        transform: Transform3D {
                            position: Vec3::new(entry.x, ground_y + 0.028, entry.z),
                            scale: Vec3::new(ring_scale, ring_scale, ring_scale),
                            ..Default::default()
                        }
                        .matrix(),
                        material: Material3D {
                            base_color: [1.05, 0.13 + proximity * 0.18, 0.018, fade * 0.76],
                            texture: None,
                            occlusion_window: [0.0; 4],
                        },
                        texture: None,
                    });
                    // Its arrow is the marker's own, on the screen (`push_prompt_medallion`).
                    let _ = pulse;
                }
            }
            // Fireflies drift slowly above the ground cover.  The motion is
            // deterministic and subtle, so screenshots stay composed while
            // the clearing still feels inhabited.
            for (x, y, z, phase) in [
                (-6.4_f32, 1.10, -8.4, 0.2_f32),
                (-2.1, 0.95, -7.2, 1.8),
                (4.5, 1.30, -8.6, 3.4),
                (7.2, 0.90, -5.2, 4.7),
                (-7.8, 1.25, -1.8, 2.6),
                (6.8, 1.15, 0.8, 5.2),
                (-5.7, 1.00, 3.7, 3.9),
                (4.9, 1.35, 4.2, 0.9),
                (-8.4, 1.20, 8.6, 4.1),
                (7.9, 1.05, 9.1, 2.3),
                (-3.6, 1.45, 10.3, 5.7),
                (3.1, 1.25, 11.0, 1.4),
            ] {
                let drift = elapsed * 0.72 + phase;
                let offset = Vec3::new(
                    drift.sin() * 0.18,
                    (drift * 1.23).sin() * 0.16,
                    (drift * 0.83).cos() * 0.14,
                );
                draws.push(MeshDraw3D {
                    mesh: self.nature_mote,
                    transform: Transform3D {
                        position: Vec3::new(x, y, z) + offset,
                        rotation: Vec3::new(0.0, drift, drift * 0.65),
                        scale: Vec3::new(1.0, 1.0, 1.0),
                    }
                    .matrix(),
                    material: Material3D {
                        base_color: [1.25, 0.80, 0.20, 0.88],
                        texture: None,
                        occlusion_window: [0.0; 4],
                    },
                    texture: None,
                });
            }
        }
    }
}
