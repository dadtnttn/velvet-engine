//! La villa como lugar: dónde está su puerta, cómo se construye a partir de su plano y cómo se sale al bosque.
use super::*;

pub(super) const VILLAGE_GATE_Z: f32 = 20.05;

pub(super) const VILLAGE_ARROW_SHOW_RADIUS: f32 = 8.0;

pub(super) const VILLAGE_INTERACT_RADIUS: f32 = 2.65;

pub(super) const FANTASY_VILLAGE_CENTER_Z: f32 = 32.0;

pub(super) fn village_entry_position() -> Vec3 {
    Vec3::new(nature_trail_center(VILLAGE_GATE_Z), 0.0, VILLAGE_GATE_Z)
}

pub(super) fn village_entry_distance(position: Vec3) -> f32 {
    let entry = village_entry_position();
    Vec3::new(position.x - entry.x, 0.0, position.z - entry.z).length()
}

pub(super) fn fantasy_village_center() -> Vec3 {
    Vec3::new(
        nature_trail_center(VILLAGE_GATE_Z),
        0.0,
        FANTASY_VILLAGE_CENTER_Z,
    )
}

/// The villa ready to draw, with what the time of day changes. It was built in the Three.js
/// prototype (`demos/demon-queen-village-threejs`) and exported by its
/// `tools/export-game.mjs` into `data/assets/3d/villa/` (the plan is `village::plan()`).
pub(super) struct VillageBuild {
    pub(super) draws: Vec<MeshDraw3D>,
    /// World bounds of the draws that can stand between the camera and her (houses, trees,
    /// props; not the ground, the paving, the water or the low plants), for the see-through
    /// window.
    pub(super) bounds: Vec<Option<(Vec3, Vec3)>>,
    /// Window panes: (draw index, the dusk at which those windows light up).
    pub(super) windows: Vec<(usize, f32)>,
    /// Lantern glass, by the doors and on the posts.
    pub(super) lamps: Vec<usize>,
    /// Chimney tops, where smoke rises.
    pub(super) smoke: Vec<Vec3>,
    /// Hot pools, where steam rises: (middle, radius).
    pub(super) steam: Vec<(Vec3, f32)>,
    /// A small sphere for the smoke and steam puffs.
    pub(super) puff: Option<MeshHandle>,
    /// The mill's wheel: its parts in its own frame, the frame in the world, and how fast it turns.
    pub(super) mill: Vec<MeshDraw3D>,
    pub(super) mill_frame: Mat4,
    pub(super) mill_speed: f32,
    /// Its water (simulated instead of the painted surfaces): kind and world triangles per body,
    /// and where the stream from the spout meets the brook.
    pub(super) water: Vec<(water_life::Kind, Vec<[Vec3; 3]>)>,
    pub(super) spout: Option<Vec3>,
}

/// How far into the night a house's windows are lit (0 by day, 1 at night):
/// the same staggered fade as the house's window light.
pub(super) fn village_window_glow(moon_visibility: f32, dusk: f32) -> f32 {
    let night = ((moon_visibility - 0.12) / 0.48).clamp(0.0, 1.0);
    ((night - dusk) / 0.22).clamp(0.0, 1.0)
}

/// How a villa material looks in the game: glass and flames glow (windows and lamps are lit by
/// the time of day each frame), the painted water a little, the rest takes soft contact shading.
pub(super) fn villa_material(name: &str, material: Material3D) -> Material3D {
    match name {
        "flame" => material.with_emission(2.0),
        "waterBrook" => material.with_emission(0.38),
        "waterPool" | "waterWell" | "waterFall" => material.with_emission(0.18),
        "windowGlow" | "windowRose" | "lampGlass" | "heartGlass" | "paperLamp" => material,
        _ => material.with_ambient_occlusion(0.18),
    }
}

pub(super) fn build_fantasy_village(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<VillageBuild> {
    prefetch::prefetch(zone_files(root));
    let plan = village::plan();
    let world = Mat4::from_translation(fantasy_village_center());
    let mut cache = HashMap::new();
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    let mut draws = Vec::new();
    let mut bounds = Vec::new();
    let mut windows = Vec::new();
    let mut lamps = Vec::new();
    let mut water = Vec::new();
    let mut spout = None;
    // A box round some local points, in the world.
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
    // The baked parts: every mesh of each file, cut into cells so that frustum culling skips
    // what is off screen (the houses into small ones: the see-through window takes a cell whole).
    for file in [
        "villa_ground.glb",
        "villa_paving.glb",
        "villa_water.glb",
        "villa_town.glb",
        "villa_hedges.glb",
        "villa_life.glb",
    ] {
        let can_hide_her = matches!(file, "villa_town.glb" | "villa_life.glb");
        let cell = if file == "villa_town.glb" { 8.0 } else { 16.0 };
        let path = root.join(file);
        let model = prefetch::load(&path)
            .with_context(|| format!("load villa part: {}", path.display()))?;
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
            let material = villa_material(name, mesh.material);
            let transform = world * model.mesh_transform(index);
            let kind = match name {
                "waterBrook" => Some(water_life::Kind::Brook),
                "waterPool" => Some(water_life::Kind::HotPool),
                "waterWell" => Some(water_life::Kind::Well),
                _ => None,
            };
            if let Some(kind) = kind {
                water.push((kind, mesh.world_triangles(transform)));
                continue;
            }
            if name == "waterFall" {
                // The stream's foot: its lowest points.
                let pts = mesh
                    .world_triangles(transform)
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let low = pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let foot: Vec<&Vec3> = pts.iter().filter(|p| p.y < low + 0.05).collect();
                if !foot.is_empty() {
                    let sum = foot.iter().fold(Vec3::ZERO, |a, p| a + **p);
                    spout = Some(sum * (1.0 / foot.len() as f32));
                }
            }
            for (cell, piece) in mesh.split_into_cells(cell).iter().enumerate() {
                match name {
                    // Each cell of panes lights up a little after the last.
                    "windowGlow" | "windowRose" => {
                        windows.push((draws.len(), 0.12 + (cell % 3) as f32 * 0.06))
                    }
                    "lampGlass" | "heartGlass" | "paperLamp" => lamps.push(draws.len()),
                    _ => {}
                }
                let glass = matches!(
                    name,
                    "windowGlow"
                        | "windowRose"
                        | "lampGlass"
                        | "heartGlass"
                        | "paperLamp"
                        | "flame"
                );
                bounds.push((can_hide_her && !glass).then(|| {
                    world_box(
                        &mut piece
                            .vertices
                            .iter()
                            .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2])),
                        transform,
                    )
                }));
                draws.push(MeshDraw3D {
                    mesh: renderer.upload_mesh(device, piece)?,
                    transform,
                    material,
                    texture,
                });
            }
        }
    }
    // The plants and rocks: each species uploaded once, drawn wherever the plan puts one, its
    // leaves tinted plant by plant.
    let mut species: HashMap<
        &str,
        Vec<(
            MeshHandle,
            Material3D,
            Option<TextureHandle>,
            Mat4,
            bool,
            (Vec3, Vec3),
        )>,
    > = HashMap::new();
    for plant in &plan.plants {
        if !species.contains_key(plant.model.as_str()) {
            let path = root.join(format!("villa_plant_{}.glb", plant.model));
            let model = prefetch::load(&path)
                .with_context(|| format!("load villa plant: {}", path.display()))?;
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
        let placement = world
            * Mat4::from_translation(Vec3::new(plant.x, plant.y, plant.z))
            * Mat4::from_rotation_y(plant.yaw)
            * Mat4::from_scale(Vec3::new(plant.s, plant.s, plant.s));
        for (mesh, material, texture, local, leaf, (lo, hi)) in &species[plant.model.as_str()] {
            let mut material = *material;
            if let (true, Some(tint)) = (*leaf, plant.tint) {
                for (channel, k) in material.base_color[..3].iter_mut().zip(tint) {
                    *channel *= k;
                }
            }
            // Only what is tall enough to hide her takes the see-through window.
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
    // The mill's wheel, in its own frame: it turns as it is drawn.
    let mut mill = Vec::new();
    let path = root.join("villa_mill_wheel.glb");
    let model =
        prefetch::load(&path).with_context(|| format!("load villa mill: {}", path.display()))?;
    for (index, mesh) in model.meshes.iter().enumerate() {
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        let texture =
            upload_texture_pair(renderer, device, queue, albedo, normal, &mut cache, &white)?;
        // The wheel turns through the brook all day: its timber is soaked, darker and a little
        // greener than dry wood.
        let mut material = mesh.material.with_ambient_occlusion(0.18);
        material.base_color = [
            material.base_color[0] * 0.62,
            material.base_color[1] * 0.6,
            material.base_color[2] * 0.56,
            material.base_color[3],
        ];
        mill.push(MeshDraw3D {
            mesh: renderer.upload_mesh(device, mesh)?,
            transform: model.mesh_transform(index),
            material,
            texture,
        });
    }
    let mill_frame = world * Mat4::from_cols_array(plan.mill.matrix);
    let to_world = |p: [f32; 3]| world.transform_point3(Vec3::new(p[0], p[1], p[2]));
    let puff = renderer
        .upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 10, 6, Material3D::rgb(1.0, 1.0, 1.0)),
        )
        .ok();
    Ok(VillageBuild {
        draws,
        bounds,
        windows,
        lamps,
        smoke: plan.smoke.iter().map(|p| to_world(*p)).collect(),
        steam: plan
            .steam
            .iter()
            .map(|s| (to_world([s.x, 0.2, s.z]), s.r))
            .collect(),
        puff,
        mill,
        mill_frame,
        mill_speed: plan.mill.speed,
        water,
        spout,
    })
}

pub(super) fn fantasy_village_position_blocked(position: Vec3) -> bool {
    village::blocked(position)
}

impl RenderState {
    pub(super) fn ensure_fantasy_village_loaded(&mut self) {
        if self.fantasy_village_load_attempted {
            return;
        }
        self.fantasy_village_load_attempted = true;
        let root = fantasy_village_asset_root();
        let started = Instant::now();
        let built = build_fantasy_village(&mut self.renderer, &self.device, &self.queue, &root);
        eprintln!(
            "[3d] villa loaded in {:.0} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        match built {
            Ok(build) => {
                eprintln!(
                    "[3d] Fantasy Village loaded: {} draw parts, {} windows, {} lamps, {} chimneys (pack={})",
                    build.draws.len(),
                    build.windows.len(),
                    build.lamps.len(),
                    build.smoke.len(),
                    root.display()
                );
                self.fantasy_village_draws = build.draws;
                self.fantasy_village_bounds = build.bounds;
                self.fantasy_village_windows = build.windows;
                self.fantasy_village_lamps = build.lamps;
                self.fantasy_village_smoke = build.smoke;
                self.fantasy_village_steam = build.steam;
                self.fantasy_village_puff = build.puff;
                self.fantasy_village_mill = build.mill;
                self.fantasy_village_mill_frame = build.mill_frame;
                self.fantasy_village_mill_speed = build.mill_speed;
                let center = fantasy_village_center();
                let course: Vec<[f32; 2]> = village::plan()
                    .brook
                    .pts
                    .iter()
                    .map(|p| [p[0] + center.x, p[1] + center.z])
                    .collect();
                for (kind, triangles) in &build.water {
                    let flow = (*kind == water_life::Kind::Brook).then_some(course.as_slice());
                    if let Some(body) = water_life::body(*kind, triangles, flow) {
                        self.water_life.add(
                            &mut self.renderer,
                            &self.device,
                            body,
                            water_life::VILLA,
                        );
                    }
                }
                if let Some(spout) = build.spout {
                    self.water_life.set_spout(spout);
                }
                self.water_life
                    .set_mill(build.mill_frame.transform_point3(Vec3::ZERO));
            }
            Err(error) => {
                eprintln!(
                    "[3d] Fantasy Village unavailable ({error:#}); village entry keeps preview"
                );
            }
        }
    }
}

impl App {
    /// Has the villa been loaded (or tried)?
    pub(super) fn fantasy_village_load_attempted(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|s| s.fantasy_village_load_attempted)
    }

    pub(super) fn leave_village_to_nature(&mut self) {
        let entry = village_entry_position();
        self.village_entered = false;
        // Leave on the forest side of the threshold.  Keep a full interaction
        // radius of separation so the next frame cannot immediately re-enter.
        self.player = Vec3::new(entry.x, 0.0, entry.z - 3.0);
        self.camera_focus = self.player;
        self.player_yaw = std::f32::consts::PI;
        self.map_open = false;
        self.footprints.clear();
        self.footstep_distance = 0.0;
        self.footstep_side = 1.0;
        self.clear_input();
        eprintln!("[3d] returned from Fantasy Village to outdoor terrain");
    }
}

impl App {
    /// What F does in the villa: the way out, or the nearest building's door.
    pub(super) fn village_prompt(&self) -> Option<Prompt> {
        if village::entrance_distance(self.player) <= VILLAGE_INTERACT_RADIUS {
            return Some(
                Prompt::new(Icon::Forest, "Volver al bosque").detail("Sendero del castillo"),
            );
        }
        if let Some(spot) = village::talk_spot(self.player) {
            return Some(Prompt::new(Icon::Talk, "Acercarse").detail(spot.title.clone()));
        }
        let (building, distance) = village::nearest_building(self.player)?;
        if distance > village::BUILDING_INTERACT_RADIUS {
            return None;
        }
        return Some(building_prompt(building));
    }
}

impl App {
    /// F in the villa: back to the wood, or a building's door (the shop, the springs, the
    /// brothel, a guion, or its scene). Whether that ends the key's handling.
    pub(super) fn interact_village(&mut self) -> bool {
        let close_to_village_exit = self.nature_mode
            && self.village_entered
            && village::entrance_distance(self.player) <= VILLAGE_INTERACT_RADIUS;
        if close_to_village_exit {
            self.leave_village_to_nature();
            self.persist_3d_state(true);
            return true;
        }
        if self.nature_mode && self.village_entered {
            // A place that is not a house (Grizna's corral): its guion, else what it looks like.
            if let Some(spot) = village::talk_spot(self.player) {
                if !self.start_talk(None, Some(spot.id.as_str())) {
                    self.queen_thought = Some((spot.text.as_str(), self.elapsed + 5.0));
                }
                return true;
            }
            if let Some((building, distance)) = village::nearest_building(self.player) {
                if distance <= village::BUILDING_INTERACT_RADIUS {
                    if building.id == "villa_tienda" {
                        self.open_shop();
                        self.persist_3d_state(true);
                        return true;
                    }
                    if building.id == "villa_banos_termales" {
                        // Through the door: the springs behind it.
                        self.enter_baths();
                        return true;
                    }
                    if building.id == "villa_burdel" {
                        // Through the door (at night, in the cloak, until Rosalía's token):
                        // the house itself, with Rosalía and her book at the desk inside.
                        self.try_enter_brothel();
                        return true;
                    }
                    // A guion written for this door plays in the box.
                    if self.start_talk(None, Some(building.id)) {
                        return true;
                    }
                    // Nothing behind it yet: the notice says it is being made.
                    if let Some(place) = village::in_development(building.id) {
                        self.notice_in_development(place.to_owned());
                        return true;
                    }
                    self.map_open = false;
                    self.pending_villa_interaction = Some(building.id.to_owned());
                    self.clear_input();
                    self.persist_3d_state(true);
                    eprintln!(
                        "[3d] villa VN hook ready: {} ({})",
                        building.id, building.label
                    );
                    return true;
                }
            }
        }
        false
    }
}

impl App {
    /// The preview's title in the villa: the way out, or the building in reach.
    pub(super) fn village_title_hint(&self) -> &'static str {
        if village::entrance_distance(self.player) <= VILLAGE_INTERACT_RADIUS {
            "  |  [F] Volver al terreno"
        } else if village::entrance_distance(self.player) <= VILLAGE_ARROW_SHOW_RADIUS {
            "  |  salida de la villa cercana"
        } else if village::nearest_building(self.player)
            .is_some_and(|(_, distance)| distance <= village::BUILDING_INTERACT_RADIUS)
        {
            if village::nearest_building(self.player)
                .is_some_and(|(building, _)| building.id == "villa_tienda")
            {
                "  |  [F] Abrir tienda"
            } else {
                "  |  [F] Abrir VN del edificio"
            }
        } else if village::nearest_building(self.player)
            .is_some_and(|(_, distance)| distance <= village::BUILDING_SHOW_RADIUS)
        {
            "  |  edificio de la villa cercano"
        } else {
            "  |  Villa Fantasy"
        }
    }
}

/// The villa as a place.
pub(super) struct VillageZone;

impl Zone for VillageZone {
    fn id(&self) -> ZoneId {
        ZoneId::Village
    }
    fn tick(&self, app: &mut App, dt: f32) {
        app.walk_outdoors(dt);
    }
    fn prompt(&self, app: &App) -> Option<Prompt> {
        app.village_prompt()
    }
    fn interact(&self, app: &mut App) -> bool {
        app.interact_village()
    }
    fn soundtrack(&self, _app: &App) -> soundtrack::Zone {
        soundtrack::Zone::Village
    }
    fn render(&self, state: &mut RenderState, frame: &Frame) {
        let (bob, green_outfit) = state.prepare_above_ground(frame);
        state.render_outdoors(frame, bob, green_outfit);
    }
    fn title(&self, app: &App) -> (&'static str, &'static str) {
        app.outdoors_title()
    }
    fn prefetch(&self, app: &mut App) {
        app.prefetch_villa_doors();
    }
}

impl App {
    /// A place read ahead stays decoded (~700 MB for the brothel) until she goes in or walks
    /// off; a zone lets go of what it read only while she is in it. Leaving it any other way (a
    /// door to the castle, a jump in the story) let go of nothing, and the memory stayed taken
    /// for the rest of the game: what no longer belongs to where she is goes here.
    pub(super) fn release_stale_prefetches(&mut self, zone: ZoneId) {
        let stale: Vec<&'static str> = self
            .prefetched
            .iter()
            .copied()
            .filter(|key| match *key {
                "baths" | "brothel" => {
                    !matches!(zone, ZoneId::Village | ZoneId::Baths | ZoneId::Brothel)
                }
                "villa" => !matches!(zone, ZoneId::Forest | ZoneId::Village),
                _ => false,
            })
            .collect();
        for key in stale {
            self.prefetched.remove(key);
            let root = match key {
                "villa" => fantasy_village_asset_root(),
                other => runtime_3d_root().join(other),
            };
            prefetch::forget(&root);
        }
    }

    /// In the villa: the springs and the brothel behind their doors, read ahead from within 9 m of
    /// the nearest one and let go beyond 25 m (12 m when another is nearer).
    pub(super) fn prefetch_villa_doors(&mut self) {
        if !Self::prefetch_allowed() {
            return;
        }
        let mut doors: Vec<(&'static str, Vec3, PathBuf)> = Vec::new();
        for (zone, id, root) in [
            (
                "baths",
                "villa_banos_termales",
                runtime_3d_root().join("baths"),
            ),
            ("brothel", "villa_burdel", runtime_3d_root().join("brothel")),
        ] {
            if let Some((door, _)) = village::building_anchors()
                .iter()
                .find(|a| a.id == id)
                .and_then(|a| village::door_anchor(a.footprint_index))
            {
                doors.push((zone, door, root));
            }
        }
        // Only the door she is nearest, once she is close (a zone read ahead holds its decoded
        // art in memory until she goes in or walks off).
        let nearest = doors
            .iter()
            .map(|(zone, at, _)| {
                (
                    *zone,
                    Vec3::new(self.player.x - at.x, 0.0, self.player.z - at.z).length(),
                )
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (zone, at, root) in doors {
            let d = Vec3::new(self.player.x - at.x, 0.0, self.player.z - at.z).length();
            let is_nearest = nearest.is_some_and(|(z, _)| z == zone);
            if d < 9.0 && is_nearest && self.prefetched.insert(zone) {
                prefetch::prefetch(zone_files(&root));
            } else if (d > 25.0 || !is_nearest && d > 12.0) && self.prefetched.remove(zone) {
                prefetch::forget(&root);
            }
        }
    }
}

impl RenderState {
    /// What the villa adds to the frame: its houses (cut round her where they hide her), the
    /// windows and lamps by the hour, the chimneys' smoke and the springs' steam, the mill and
    /// the medallions of its doors.
    pub(super) fn village_draws(
        &mut self,
        frame: &Frame,
        draws: &mut Vec<MeshDraw3D>,
        player_ground_y: f32,
    ) {
        let Frame {
            camera,
            player,
            elapsed,
            environment,
            ..
        } = *frame;

        let base = draws.len();
        draws.extend_from_slice(&self.fantasy_village_draws);
        // The see-through window: whatever stands between the camera and her (a house's
        // cell, a tree) is cut round her on screen, as the castle's walls are.
        ISO.cut_see_through(
            draws,
            base,
            &self.fantasy_village_bounds,
            &camera,
            player,
            player_ground_y,
        );
        // Windows: plain glass by day, warm light as each house wakes up
        // for the night. Lanterns glow faintly by day, brightly at night.
        let moon = environment.celestial.moon_visibility;
        for &(index, dusk) in &self.fantasy_village_windows {
            let glow = village_window_glow(moon, dusk);
            if let Some(draw) = draws.get_mut(base + index) {
                let day = [0.34, 0.44, 0.54];
                // Saturated amber: the shader clamps colour and emission
                // washes toward white, so keep hue strong and glow modest.
                let night = [1.0, 0.42, 0.06];
                let mix = |i: usize| day[i] + (night[i] - day[i]) * glow;
                draw.material.base_color = [mix(0), mix(1), mix(2), 1.0];
                // Bright enough to bloom softly once lit.
                draw.material = draw.material.with_emission(glow * 0.95);
            }
        }
        let lamp_glow = village_window_glow(moon, 0.1);
        for &index in &self.fantasy_village_lamps {
            if let Some(draw) = draws.get_mut(base + index) {
                let day = [1.0, 0.86, 0.62];
                let night = [1.0, 0.5, 0.1];
                let mix = |i: usize| day[i] + (night[i] - day[i]) * lamp_glow;
                draw.material.base_color = [mix(0), mix(1), mix(2), 1.0];
                draw.material = draw.material.with_emission(0.25 + lamp_glow * 1.4);
            }
        }
        // The mill's wheel, turned by the brook.
        let spin = Mat4::from_rotation_z(-elapsed * self.fantasy_village_mill_speed);
        for part in &self.fantasy_village_mill {
            draws.push(MeshDraw3D {
                transform: self.fantasy_village_mill_frame * spin * part.transform,
                ..*part
            });
        }
        // Steam off the hot pools: low, wide and slow.
        if let Some(puff) = self.fantasy_village_puff {
            for (pool, &(middle, radius)) in self.fantasy_village_steam.iter().enumerate() {
                for k in 0..9 {
                    let age = (elapsed * 0.18 + k as f32 / 9.0 + pool as f32 * 0.41).fract();
                    let a = k as f32 * 2.39 + pool as f32;
                    let spread = radius * 0.55;
                    let drift =
                        Vec3::new(a.cos() * spread + age * 0.35, age * 1.6, a.sin() * spread);
                    let size = 0.35 + age * 0.75;
                    let mut material = Material3D::rgb(0.95, 0.96, 0.97);
                    material.base_color[3] = 0.32 * (1.0 - age) * age.min(0.2) / 0.2;
                    draws.push(MeshDraw3D {
                        mesh: puff,
                        transform: Mat4::from_translation(middle + drift)
                            * Mat4::from_scale(Vec3::new(size, size * 0.7, size)),
                        material,
                        texture: None,
                    });
                }
            }
        }
        // Chimney smoke: soft puffs that rise, drift and thin out.
        if let Some(puff) = self.fantasy_village_puff {
            for (chimney, top) in self.fantasy_village_smoke.iter().enumerate() {
                for k in 0..7 {
                    let age = (elapsed * 0.32 + k as f32 / 7.0 + chimney as f32 * 0.37).fract();
                    let drift = Vec3::new(
                        (age * 3.0 + chimney as f32).sin() * 0.25 + age * 0.5,
                        age * 1.9,
                        age * 0.2,
                    );
                    let size = 0.12 + age * 0.34;
                    let mut material = Material3D::rgb(0.86, 0.86, 0.88);
                    material.base_color[3] = 0.55 * (1.0 - age) * age.min(0.15) / 0.15;
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
        // Every building owns a stable VN ID.  Reveal only nearby caps so
        // the village stays readable, but keep one marker per building as
        // the player approaches the council, brothel, shop, tavern,
        // houses or hot springs.
        for footprint_index in 0..village::layout().footprints.len() {
            let Some(building) = village::building_anchor(footprint_index) else {
                continue;
            };
            let building_distance = village::building_distance(player, footprint_index);
            push_villa_building_prompt(building, building_distance);
        }
        // The village uses the same gate marker for a reversible return
        // to the forest.  Keep the prompt close to the actual threshold,
        // not floating over the settlement centre.
        let entry = village::entrance();
        let distance = village::entrance_distance(player);
        if distance <= VILLAGE_ARROW_SHOW_RADIUS {
            let ground_y = village::surface_height(entry.x, entry.z);
            push_prompt_medallion(
                Icon::Forest,
                Vec3::new(entry.x, ground_y, entry.z),
                2.85,
                distance,
                VILLAGE_ARROW_SHOW_RADIUS,
                VILLAGE_INTERACT_RADIUS,
            );
        }
    }
}
