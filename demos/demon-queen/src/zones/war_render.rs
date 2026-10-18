//! La guerra dibujada: el campo de batalla (el suelo pintado, el agua, el puente, los bosques, las
//! rocas, las peñas, las murallas, los decorados de Blender), los ejércitos (cada modelo y bando
//! una multitud que la GPU anima), sus sombras y anillos, los estandartes, las flechas, el polvo y
//! las chispas, la «Chispa» de la Reina y Varrak con su propio modelo; y el diorama de la campaña
//! (la tierra, las provincias pintadas, el marco de la mesa, el agua, los caminos, los bosques, los
//! asentamientos con sus banderas, los ejércitos en sus peanas y la niebla). Después, el HUD.
use std::path::PathBuf;

use super::zone_war::{Fx, WarPage, WarSession};
use super::*;
use demon_queen_war as wr;
use velvet_render::CrowdInstance3D;
use wr::data::{Faction, Role};
use wr::models::{self, Piece};

fn war_root() -> PathBuf {
    exploration3d_state::data_dir().join("assets/war")
}

fn look_index(f: Faction) -> u8 {
    match f {
        Faction::Reina => 0,
        Faction::Alianza => 1,
        Faction::Rebeldes => 2,
        Faction::Libres => 3,
    }
}

fn look_of(i: u8) -> Faction {
    match i {
        1 => Faction::Alianza,
        2 => Faction::Rebeldes,
        3 => Faction::Libres,
        _ => Faction::Reina,
    }
}

/// A crowd of one model: its mesh, how many it holds, this frame's soldiers, how it is drawn.
struct Crowd {
    mesh: MeshHandle,
    cap: usize,
    buf: Vec<CrowdInstance3D>,
    material: Material3D,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Key {
    /// A troop: its model (by name), look, near (0) or far (1).
    Troop(&'static str, u8, u8),
    Standard(u8),
    Blob,
    Ring,
    Arrow,
    Bolt,
    Javelin,
    Dust,
    Glow,
    Cloud,
}

/// A set piece or a settlement, uploaded: its body, its glowing parts, its banners, its sails.
struct Placed {
    body: Option<MeshHandle>,
    glow: Option<MeshHandle>,
    flags: Option<MeshHandle>,
    /// The windmill's sails and the point they turn round.
    sails: Option<(MeshHandle, Vec3)>,
}

#[derive(Clone, Copy)]
struct Particle {
    p: Vec3,
    v: Vec3,
    life: f32,
    ttl: f32,
    s0: f32,
    s1: f32,
    c: [f32; 4],
    grav: f32,
    drag: f32,
}

struct Particles {
    list: Vec<Particle>,
    max: usize,
    seed: u32,
}

impl Particles {
    fn new(max: usize) -> Self {
        Self {
            list: Vec::with_capacity(max),
            max,
            seed: 7,
        }
    }

    fn rand(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / 16_777_216.0
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn(
        &mut self,
        p: Vec3,
        v: Vec3,
        ttl: f32,
        s0: f32,
        s1: f32,
        c: [f32; 4],
        grav: f32,
        drag: f32,
    ) {
        let part = Particle {
            p,
            v,
            life: 0.0,
            ttl,
            s0,
            s1,
            c,
            grav,
            drag,
        };
        if self.list.len() >= self.max {
            let i = (self.rand() * self.max as f32) as usize % self.max;
            self.list[i] = part;
        } else {
            self.list.push(part);
        }
    }

    fn update(&mut self, dt: f32) {
        for q in &mut self.list {
            q.life += dt;
            q.v.y -= q.grav * dt;
            let dr = (1.0 - q.drag * dt).max(0.0);
            q.v = q.v * dr;
            q.p = q.p + q.v * dt;
        }
        self.list.retain(|q| q.life < q.ttl);
    }
}

/// A burst of the Queen's spark: where, how far it reaches, how long ago.
struct Burst {
    at: Vec3,
    radius: f32,
    t: f32,
    /// A power's ring (its colour, no column), else the spark's (violet, with its column).
    color: Option<[f32; 3]>,
}

/// Varrak in the battle: the game's own model with its clips.
struct Hero {
    model: GltfModel,
    meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    scale: f32,
    lift: f32,
    clip: Option<usize>,
    clip_t: f32,
    from: Option<(usize, f32)>,
    fade: f32,
    swap: bool,
    yaw: f32,
}

impl Hero {
    fn clip(&self, name: &str) -> Option<usize> {
        self.model.animation_index(name)
    }

    fn duration(&self, i: usize) -> f32 {
        self.model
            .animation_clips
            .get(i)
            .map_or(1.0, |c| c.duration.max(0.01))
    }

    fn play(&mut self, clip: Option<usize>, fade: f32) {
        if clip == self.clip || clip.is_none() {
            return;
        }
        self.from = self.clip.map(|c| (c, self.clip_t));
        self.fade = if fade <= 0.0 { 1.0 } else { 0.0 };
        self.clip = clip;
        self.clip_t = 0.0;
    }
}

/// The campaign's diorama, uploaded once.
struct CampScene {
    land: MeshHandle,
    overlay: MeshHandle,
    overlay_tex: TextureHandle,
    painted: Option<(u64, u64)>,
    table: MeshHandle,
    water: MeshHandle,
    roads: Option<MeshHandle>,
    woods: Vec<wr::scene::Instance>,
    /// Each province's settlement, placed (the old models: only when the map 2.0's world is missing).
    settlements: Vec<(usize, Mat4)>,
    /// The map 2.0's world (`data/guerra/mundo.glb`): every settlement and landmark of the
    /// prototype, textured as the web map, one draw per material (its texture, whether it
    /// shines); and its anchors (`mundo.json`).
    world: Vec<(MeshHandle, Option<TextureHandle>, Option<f32>)>,
    anchors: Vec<WorldAnchor>,
    /// The land's painted picture (`data/guerra/tierra.jpg`), when there is one: then `land` is
    /// the white mesh it is laid on.
    land_tex: Option<TextureHandle>,
    sea: MeshHandle,
    /// The waves' normal map on the water (the web map's).
    water_tex: Option<TextureHandle>,
    /// The regions' borders (`data/guerra/fronteras.json`) and their ribbons, rebuilt for the
    /// camera (a line keeps its width on the screen) and for who holds what.
    borders: Vec<BorderLine>,
    border_mesh: Option<MeshHandle>,
    border_key: Option<(Vec3, u64)>,
    bases: [(MeshHandle, MeshHandle); 4],
    clouds: Vec<(f32, f32, f32, f32, usize, f32)>,
    cloud_alpha: Vec<f32>,
    /// A campaign to show behind the table's page when none has begun.
    preview: wr::rules::Campaign,
    provinces: wr::campaign_map::Provinces,
}

/// Something the map 2.0's world asks to be shown at a point: a banner in its region's colours, a
/// fire, a glow (the Queen's crystals), steam.
#[derive(Clone, Debug)]
struct WorldAnchor {
    kind: u8,
    at: Vec3,
    size: f32,
    /// The region it belongs to (its index in `PROVINCES`), if any.
    region: Option<usize>,
}

/// A stretch of border between regions `a` and `b` (a on the left of the line), from
/// `fronteras.json`; `prov` when they lie in different provinces.
#[derive(Clone, Debug)]
struct BorderLine {
    a: usize,
    b: usize,
    prov: bool,
    pts: Vec<[f32; 2]>,
}

fn border_lines() -> Vec<BorderLine> {
    let path = exploration3d_state::data_dir()
        .join("guerra")
        .join("fronteras.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let n = wr::data::PROVINCES.len();
    v["lines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let a = l["a"].as_u64()? as usize;
            let b = l["b"].as_u64()? as usize;
            if a >= n || b >= n {
                return None;
            }
            let flat: Vec<f32> = l["pts"]
                .as_array()?
                .iter()
                .filter_map(|x| x.as_f64().map(|x| x as f32))
                .collect();
            let pts: Vec<[f32; 2]> = flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect();
            (pts.len() >= 2).then_some(BorderLine {
                a,
                b,
                prov: l["prov"].as_bool().unwrap_or(false),
                pts,
            })
        })
        .collect()
}

/// What the borders show of the campaign (who holds each region, the fog, the chosen, the
/// hovered and the reachable regions).
struct BorderLook<'a> {
    owner: &'a [Faction],
    fog: &'a [bool],
    reach: &'a [bool],
    selected: Option<usize>,
    hover: Option<usize>,
}

/// The borders as the web map draws them: on each side a band of that side's owner's colour (strong
/// where two owners meet, faint between one owner's regions, wider between provinces), a dark core
/// on the line, a soft glow off it; widths in screen pixels (from each point's distance to the
/// camera), so a line looks the same close up and far out.
fn border_ribbons(lines: &[BorderLine], look: &BorderLook, camera: Camera3D) -> Mesh3D {
    let mut vertices: Vec<MeshVertex3D> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let fov_k = 2.0 * (camera.fov_y_radians * 0.5).tan() / 1080.0;
    let col = |f: Faction| {
        let c = wr::data::rgb(f.info().color);
        [c[0] * 1.3 + 0.04, c[1] * 1.3 + 0.04, c[2] * 1.3 + 0.04]
    };
    let gold = [1.0, 0.86, 0.45];
    // one strip along a line: from offset o0 to o1 (left positive), colours at each edge, raised by `lift`
    let mut strip = |pts: &[(Vec3, Vec3, f32)],
                     o0: &dyn Fn(f32) -> f32,
                     o1: &dyn Fn(f32) -> f32,
                     c0: [f32; 4],
                     c1: [f32; 4],
                     lift: f32| {
        let base = vertices.len() as u32;
        for &(p, nl, px) in pts {
            for (o, c) in [(o0(px), c0), (o1(px), c1)] {
                let q = p + nl * o + Vec3::new(0.0, lift, 0.0);
                let mut v = MeshVertex3D::new([q.x, q.y, q.z], [0.0, 1.0, 0.0], [0.0, 0.0]);
                v.color = c;
                vertices.push(v);
            }
        }
        for i in 0..pts.len() as u32 - 1 {
            let (a, b, c, d) = (
                base + i * 2,
                base + i * 2 + 1,
                base + i * 2 + 2,
                base + i * 2 + 3,
            );
            indices.extend([a, c, b, b, c, d]);
        }
    };
    for l in lines {
        // each point: where (on the land), its left normal, a screen pixel's size there
        let n = l.pts.len();
        let pts: Vec<(Vec3, Vec3, f32)> = (0..n)
            .map(|i| {
                let [x, z] = l.pts[i];
                let [px0, pz0] = l.pts[i.saturating_sub(1)];
                let [px1, pz1] = l.pts[(i + 1).min(n - 1)];
                let t = Vec3::new(px1 - px0, 0.0, pz1 - pz0).normalize_or_zero();
                let nl = Vec3::new(-t.z, 0.0, t.x);
                let y = wr::campaign_map::land_height(x, z).max(wr::campaign_map::WATER) + 0.04;
                let p = Vec3::new(x, y, z);
                let px = (p - camera.position).length() * fov_k;
                (p, nl, px)
            })
            .collect();
        let (oa, ob) = (look.owner[l.a], look.owner[l.b]);
        let frontier = oa != ob;
        let hidden = look.fog.get(l.a).copied().unwrap_or(false)
            && look.fog.get(l.b).copied().unwrap_or(false);
        let tone = |c: [f32; 3], region: usize| -> [f32; 3] {
            let lit = if look.selected == Some(region) {
                [
                    c[0] * 0.3 + gold[0] * 0.7,
                    c[1] * 0.3 + gold[1] * 0.7,
                    c[2] * 0.3 + gold[2] * 0.7,
                ]
            } else if look.hover == Some(region) || look.reach.get(region).copied().unwrap_or(false)
            {
                [c[0] * 1.18 + 0.05, c[1] * 1.18 + 0.05, c[2] * 1.18 + 0.05]
            } else {
                c
            };
            if hidden {
                let g = lit[0] * 0.3 + lit[1] * 0.55 + lit[2] * 0.15;
                [g * 0.45 + 0.02, g * 0.45 + 0.02, g * 0.45 + 0.035]
            } else {
                lit
            }
        };
        let (ca, cb) = (tone(col(oa), l.a), tone(col(ob), l.b));
        let strong = frontier || look.selected == Some(l.a) || look.selected == Some(l.b);
        let k_band = if strong { 1.0 } else { 0.55 };
        let w_reg = move |px: f32| (px * if strong { 3.0 } else { 1.3 }).clamp(0.04, 0.6);
        let w_prov = move |px: f32| (px * if strong { 3.6 } else { 2.6 }).clamp(0.06, 0.8);
        let prov = l.prov;
        let band = move |px: f32| {
            if prov {
                w_prov(px).max(w_reg(px))
            } else {
                w_reg(px)
            }
        };
        let glow_w = |px: f32| (px * 22.0).clamp(0.5, 2.6);
        let g_alpha = if strong { 0.24 } else { 0.08 } * 0.6;
        let a = |c: [f32; 3], al: f32| [c[0], c[1], c[2], al];
        // the glow off each side
        strip(
            &pts,
            &|_| 0.0,
            &|px| glow_w(px),
            a(ca, g_alpha),
            a(ca, 0.0),
            0.02,
        );
        strip(
            &pts,
            &|_| 0.0,
            &|px| -glow_w(px),
            a(cb, g_alpha),
            a(cb, 0.0),
            0.02,
        );
        // the bands, each its own owner's colour
        let al = 0.88 * k_band;
        strip(&pts, &|_| 0.0, &|px| band(px), a(ca, al), a(ca, al), 0.05);
        strip(&pts, &|_| 0.0, &|px| -band(px), a(cb, al), a(cb, al), 0.05);
        // the dark core
        let cw = |px: f32| (px * 0.75).clamp(0.02, 0.1);
        let core = if prov || strong { 0.85 } else { 0.45 };
        let dark = [0.03, 0.02, 0.05, core];
        strip(&pts, &|px| -cw(px), &|px| cw(px), dark, dark, 0.08);
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 1.0,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

/// Reads `data/guerra/mundo.json`: its anchors.
fn world_anchors() -> Vec<WorldAnchor> {
    let path = exploration3d_state::data_dir()
        .join("guerra")
        .join("mundo.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for a in v["anchors"].as_array().into_iter().flatten() {
        let kind = match a["kind"].as_str() {
            Some("banner") => 0,
            Some("fire") => 1,
            Some("glow") => 2,
            Some("steam") => 3,
            _ => continue,
        };
        let p = &a["p"];
        let at = Vec3::new(
            p[0].as_f64().unwrap_or(0.0) as f32,
            p[1].as_f64().unwrap_or(0.0) as f32,
            p[2].as_f64().unwrap_or(0.0) as f32,
        );
        let region = a["region"]
            .as_str()
            .and_then(|id| wr::data::PROVINCES.iter().position(|q| q.id == id));
        out.push(WorldAnchor {
            kind,
            at,
            size: a["size"].as_f64().unwrap_or(0.5) as f32,
            region,
        });
    }
    out
}

pub(crate) struct WarScene {
    troop_pieces: HashMap<&'static str, (Vec<Piece>, Vec<Piece>)>,
    crowds: HashMap<Key, Crowd>,
    scatter: Vec<(MeshHandle, bool)>,
    props: HashMap<String, Placed>,
    sky: MeshHandle,
    spark_ring: MeshHandle,
    spark_column: MeshHandle,
    // the battle on show
    serial: u64,
    /// The ground's grain, tiling over every field (made once).
    ground_detail: Option<TextureHandle>,
    ground_pool: Vec<MeshHandle>,
    ground_used: usize,
    water_pool: Option<MeshHandle>,
    water_on: bool,
    solid_pool: Vec<MeshHandle>,
    solid_used: usize,
    instances: Vec<wr::scene::Instance>,
    placed: Vec<(String, Mat4)>,
    stance: Vec<f32>,
    dust: Particles,
    glow: Particles,
    bursts: Vec<Burst>,
    fx_seen: u64,
    last_time: f32,
    hero: Option<Hero>,
    hero_failed: bool,
    camp: Option<CampScene>,
    sail_turn: f32,
    /// A battle map's own structures (the war map prototype's): what stands still (and its lit
    /// windows and fires apart), and each work that can be broken, by its index.
    structures: Vec<(MeshHandle, bool)>,
    works: Vec<Option<MeshHandle>>,
    /// What armies build (per kind and look: the body, the arm and where it turns), the stone and
    /// the bolt in the air, and the rubble of each work brought down.
    engine_models: HashMap<(wr::siege::Kind, u8), (MeshHandle, Option<(MeshHandle, [f32; 3])>)>,
    shot_meshes: Option<(MeshHandle, MeshHandle)>,
    /// A relic on the field: its plinth and its crystal.
    relic_meshes: Option<(MeshHandle, MeshHandle)>,
    rubble: HashMap<usize, Vec<Mat4>>,
}

/// Where the battle maps live (`data/guerra/batallas/<id>`).
pub(crate) fn battle_maps_root() -> PathBuf {
    exploration3d_state::data_dir()
        .join("guerra")
        .join("batallas")
}

fn upload(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    mesh: &Mesh3D,
) -> Result<MeshHandle> {
    renderer
        .upload_mesh(device, mesh)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}

fn upload_into(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    pool: &mut Vec<MeshHandle>,
    used: usize,
    mesh: &Mesh3D,
) -> Result<MeshHandle> {
    if let Some(&h) = pool.get(used) {
        renderer
            .replace_mesh(device, h, mesh)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(h)
    } else {
        let h = upload(renderer, device, mesh)?;
        pool.push(h);
        Ok(h)
    }
}

/// A set piece or a settlement from its GLB, split into what it needs drawn apart.
fn place_model(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    path: &std::path::Path,
) -> Result<Placed> {
    let pieces = models::load_glb(path)?;
    let sails: Vec<Piece> = pieces
        .iter()
        .filter(|p| p.node.starts_with("SAILS"))
        .cloned()
        .collect();
    let body = models::static_mesh(
        &pieces,
        |p| !models::glows(p) && !models::is_flag(p) && !p.node.starts_with("SAILS"),
        |_| None,
    );
    let glow = models::static_mesh(
        &pieces,
        |p| models::glows(p) && !p.node.starts_with("SAILS"),
        |_| None,
    );
    let flags = models::static_mesh(
        &pieces,
        |p| models::is_flag(p) && !p.node.starts_with("SAILS"),
        |_| Some([1.0; 3]),
    );
    let sail = models::static_mesh(&sails, |_| true, |_| None);
    let pivot = sails.first().map_or(Vec3::ZERO, |p| {
        Vec3::new(p.pivot[0], p.pivot[1], p.pivot[2])
    });
    Ok(Placed {
        body: body.map(|m| upload(renderer, device, &m)).transpose()?,
        glow: glow.map(|m| upload(renderer, device, &m)).transpose()?,
        flags: flags.map(|m| upload(renderer, device, &m)).transpose()?,
        sails: sail
            .map(|m| upload(renderer, device, &m).map(|h| (h, pivot)))
            .transpose()?,
    })
}

impl WarScene {
    fn new(renderer: &mut Wgpu3DRenderer, device: &wgpu::Device) -> Result<Self> {
        let mut scatter = Vec::new();
        for k in wr::scene::kinds() {
            scatter.push((upload(renderer, device, &k.mesh)?, k.shadow));
        }
        let sky = upload(
            renderer,
            device,
            &wr::scene::sky_dome(BATTLE_SUN, 0x2c1d4e, 0x7a4f8e, 0xf0a77a, 0x3a2a40),
        )?;
        let (ring, column) = wr::scene::spark_meshes();
        Ok(Self {
            troop_pieces: HashMap::new(),
            crowds: HashMap::new(),
            scatter,
            props: HashMap::new(),
            sky,
            spark_ring: upload(renderer, device, &ring)?,
            spark_column: upload(renderer, device, &column)?,
            serial: 0,
            ground_detail: None,
            ground_pool: Vec::new(),
            ground_used: 0,
            water_pool: None,
            water_on: false,
            solid_pool: Vec::new(),
            solid_used: 0,
            instances: Vec::new(),
            placed: Vec::new(),
            stance: Vec::new(),
            dust: Particles::new(2600),
            glow: Particles::new(1800),
            bursts: Vec::new(),
            fx_seen: 0,
            last_time: 0.0,
            hero: None,
            hero_failed: false,
            camp: None,
            sail_turn: 0.0,
            structures: Vec::new(),
            works: Vec::new(),
            engine_models: HashMap::new(),
            shot_meshes: None,
            relic_meshes: None,
            rubble: HashMap::new(),
        })
    }

    fn engine_model(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        kind: wr::siege::Kind,
        look: Faction,
    ) -> Option<(MeshHandle, Option<(MeshHandle, [f32; 3])>)> {
        let key = (kind, look_index(look));
        if !self.engine_models.contains_key(&key) {
            let m = wr::engines::model(kind, look);
            let body = upload(renderer, device, &m.body).ok()?;
            let arm = m
                .arm
                .and_then(|(mesh, pivot)| upload(renderer, device, &mesh).ok().map(|h| (h, pivot)));
            self.engine_models.insert(key, (body, arm));
        }
        self.engine_models.get(&key).copied()
    }

    /// The rubble of a work brought down: stones heaped along its footprint (worked out once).
    fn rubble_of(&mut self, field: &wr::field::Field, w: usize) -> &[Mat4] {
        self.rubble.entry(w).or_insert_with(|| {
            let mut out = Vec::new();
            let Some(m) = &field.baked else {
                return out;
            };
            let Some(spec) = m.works.get(w) else {
                return out;
            };
            let mut seed = (w as u32).wrapping_mul(2_654_435_761).wrapping_add(17);
            let mut rnd = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed % 10_000) as f32 / 10_000.0
            };
            let mut pts: Vec<(f32, f32, f32)> = Vec::new();
            match (spec.a, spec.b) {
                (Some(a), Some(b)) => {
                    let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                    let n = (len / 2.2) as usize + 2;
                    let t = spec.thick.unwrap_or(3.0);
                    for k in 0..n {
                        let f = k as f32 / (n - 1) as f32;
                        let (x, z) = (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f);
                        for _ in 0..2 {
                            pts.push((
                                x + (rnd() - 0.5) * t * 2.2,
                                z + (rnd() - 0.5) * t * 2.2,
                                0.9 + rnd() * 1.6,
                            ));
                        }
                    }
                }
                _ => {
                    let [cx, cz] = spec.centre();
                    let r = spec
                        .r
                        .unwrap_or_else(|| spec.w.unwrap_or(8.0).max(spec.d.unwrap_or(8.0)) * 0.5);
                    let n = (r * r * 0.5) as usize + 6;
                    for _ in 0..n.min(90) {
                        let a = rnd() * std::f32::consts::TAU;
                        let d = rnd().sqrt() * r * 1.2;
                        pts.push((cx + a.cos() * d, cz + a.sin() * d, 1.0 + rnd() * 2.0));
                    }
                }
            }
            for (x, z, s) in pts {
                let y = field.height_at(x, z);
                out.push(
                    Mat4::from_translation(Vec3::new(x, y - 0.2, z))
                        * Mat4::from_rotation_y(rnd() * std::f32::consts::TAU)
                        * Mat4::from_scale(Vec3::new(s, s * (0.5 + rnd() * 0.5), s)),
                );
            }
            out
        })
    }

    /// A battle map's structures: `mundo.glb` (one mesh, its lit parts apart) and `obras.glb`
    /// (one mesh per work, named `w<i>:...`).
    fn load_structures(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        map: &wr::baked::BattleMap,
    ) {
        self.structures.clear();
        self.works = vec![None; map.works.len()];
        let dir = battle_maps_root().join(&map.id);
        match models::load_glb(&dir.join("mundo.glb")) {
            Ok(pieces) => {
                let lit = |p: &models::Piece| p.node.ends_with("lampGlass");
                for (glow, mesh) in [
                    (false, models::static_mesh(&pieces, |p| !lit(p), |_| None)),
                    (true, models::static_mesh(&pieces, lit, |_| None)),
                ] {
                    if let Some(m) = mesh {
                        if let Ok(h) = upload(renderer, device, &m) {
                            self.structures.push((h, glow));
                        }
                    }
                }
            }
            Err(e) => eprintln!("[guerra] {}: {e:#}", dir.join("mundo.glb").display()),
        }
        match models::load_glb(&dir.join("obras.glb")) {
            Ok(pieces) => {
                let id_of = |p: &models::Piece| -> Option<usize> {
                    p.node.split(':').next()?.strip_prefix('w')?.parse().ok()
                };
                for w in 0..map.works.len() {
                    if let Some(m) = models::static_mesh(&pieces, |p| id_of(p) == Some(w), |_| None)
                    {
                        self.works[w] = upload(renderer, device, &m).ok();
                    }
                }
            }
            Err(e) => eprintln!("[guerra] {}: {e:#}", dir.join("obras.glb").display()),
        }
    }

    fn prop(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        dir: &str,
        kind: &str,
    ) -> bool {
        let key = format!("{dir}/{kind}");
        if !self.props.contains_key(&key) {
            let path = war_root().join(dir).join(format!("{kind}.glb"));
            match place_model(renderer, device, &path) {
                Ok(p) => {
                    self.props.insert(key.clone(), p);
                }
                Err(e) => {
                    eprintln!("[guerra] {} failed: {e:#}", path.display());
                    self.props.insert(
                        key.clone(),
                        Placed {
                            body: None,
                            glow: None,
                            flags: None,
                            sails: None,
                        },
                    );
                }
            }
        }
        self.props.get(&key).is_some_and(|p| p.body.is_some())
    }

    /// The crowd of `key`, with room for `want` at least (a bigger one replaces it when needed).
    fn crowd(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        key: Key,
        want: usize,
    ) -> Option<&mut Crowd> {
        let need = want.max(1);
        let grow = self.crowds.get(&key).is_none_or(|c| c.cap < need);
        if grow {
            let (model, material) = match key {
                Key::Troop(kind, look, lod) => {
                    if !self.troop_pieces.contains_key(kind) {
                        let near = models::load_glb(
                            &war_root().join("troops").join(format!("{kind}.glb")),
                        );
                        let far = models::load_glb(
                            &war_root().join("troops").join(format!("{kind}_lod.glb")),
                        );
                        match near {
                            Ok(n) => {
                                let f = far.unwrap_or_else(|_| n.clone());
                                self.troop_pieces.insert(kind, (n, f));
                            }
                            Err(e) => {
                                eprintln!("[guerra] troop {kind}: {e:#}");
                                return None;
                            }
                        }
                    }
                    let (near, far) = &self.troop_pieces[kind];
                    let pieces = if lod == 0 { near } else { far };
                    (
                        models::crowd_mesh(kind, pieces, look_of(look)),
                        Material3D::default(),
                    )
                }
                Key::Standard(look) => (wr::scene::standard(look_of(look)), Material3D::default()),
                Key::Blob => (
                    wr::scene::blob_model(),
                    Material3D {
                        base_color: [0.05, 0.02, 0.08, 0.98],
                        ..Material3D::default()
                    },
                ),
                Key::Ring => (
                    wr::scene::ring_model(),
                    Material3D {
                        base_color: [1.0, 1.0, 1.0, 0.98],
                        ..Material3D::default()
                    }
                    .with_emission(0.9),
                ),
                Key::Arrow => (
                    wr::scene::missile_model(0.85, 0.025, 0.035, 0.07),
                    Material3D::default(),
                ),
                Key::Bolt => (
                    wr::scene::missile_model(0.5, 0.04, 0.05, 0.05),
                    Material3D::default(),
                ),
                Key::Javelin => (
                    wr::scene::missile_model(1.4, 0.035, 0.05, 0.0),
                    Material3D::default(),
                ),
                Key::Dust => (
                    wr::scene::speck_model(),
                    Material3D {
                        base_color: [0.74, 0.64, 0.54, 0.98],
                        ..Material3D::default()
                    },
                ),
                Key::Glow => (
                    wr::scene::speck_model(),
                    Material3D::additive([1.1, 1.1, 1.1, 1.0]),
                ),
                Key::Cloud => (
                    wr::scene::speck_model(),
                    Material3D {
                        base_color: [0.85, 0.78, 0.93, 0.98],
                        ..Material3D::default()
                    }
                    .with_emission(0.4),
                ),
            };
            // room to spare, so a bigger battle does not make another at once
            let cap = (need + need / 4).next_multiple_of(64);
            let mesh = match renderer.upload_crowd(device, &model, cap) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[guerra] crowd {key:?}: {e:?}");
                    return None;
                }
            };
            self.crowds.insert(
                key,
                Crowd {
                    mesh,
                    cap,
                    buf: Vec::with_capacity(cap),
                    material,
                },
            );
        }
        self.crowds.get_mut(&key)
    }

    fn begin_frame(&mut self) {
        for c in self.crowds.values_mut() {
            c.buf.clear();
        }
    }

    fn push(&mut self, key: Key, i: CrowdInstance3D) {
        if let Some(c) = self.crowds.get_mut(&key) {
            if c.buf.len() < c.cap {
                c.buf.push(i);
            }
        }
    }

    /// Hands every crowd its soldiers and adds the draws of those that have any.
    fn end_frame(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        time: f32,
        draws: &mut Vec<MeshDraw3D>,
        blended: &mut Vec<MeshDraw3D>,
    ) {
        let mut keys: Vec<Key> = self.crowds.keys().copied().collect();
        keys.sort_by_key(|k| format!("{k:?}"));
        for k in keys {
            let c = self.crowds.get_mut(&k).expect("a crowd");
            let n = renderer.set_crowd(queue, c.mesh, &c.buf, time);
            if n == 0 {
                continue;
            }
            let draw = MeshDraw3D {
                mesh: c.mesh,
                transform: Mat4::IDENTITY,
                material: c.material,
                texture: None,
            };
            if c.material.is_additive() || c.material.base_color[3] < 0.99 {
                blended.push(draw);
            } else {
                draws.push(draw);
            }
        }
    }

    /// A new battle on show: its field uploaded (in the pools of the last one).
    fn load_battle(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        war: &WarSession,
    ) -> Option<Vec<u8>> {
        let run = war.battle.as_ref()?;
        let field = &run.b.sim.field;
        let t0 = std::time::Instant::now();
        let s = wr::scene::build(field);
        if self.ground_detail.is_none() {
            let size = 512;
            self.ground_detail = renderer
                .upload_texture(
                    device,
                    queue,
                    &TextureData3D {
                        width: size as u32,
                        height: size as u32,
                        rgba: wr::scene::ground_detail(size),
                    },
                )
                .ok();
        }
        self.ground_used = 0;
        for m in &s.ground {
            if upload_into(renderer, device, &mut self.ground_pool, self.ground_used, m).is_ok() {
                self.ground_used += 1;
            }
        }
        self.water_on = false;
        if let Some(w) = &s.water {
            let ok = match self.water_pool {
                Some(h) => renderer.replace_mesh(device, h, w).is_ok(),
                None => upload(renderer, device, w)
                    .map(|h| self.water_pool = Some(h))
                    .is_ok(),
            };
            self.water_on = ok;
        }
        self.solid_used = 0;
        for m in &s.solids {
            if upload_into(renderer, device, &mut self.solid_pool, self.solid_used, m).is_ok() {
                self.solid_used += 1;
            }
        }
        let mut inst = s.instances;
        inst.sort_by_key(|i| i.kind);
        self.instances = inst;
        match &field.baked {
            Some(m) => {
                let m = std::sync::Arc::clone(m);
                self.load_structures(renderer, device, &m);
            }
            None => {
                self.structures.clear();
                self.works.clear();
            }
        }
        self.rubble.clear();
        if self.relic_meshes.is_none() {
            let (plinth, crystal) = wr::engines::relic();
            if let (Ok(a), Ok(b)) = (
                upload(renderer, device, &plinth),
                upload(renderer, device, &crystal),
            ) {
                self.relic_meshes = Some((a, b));
            }
        }
        if self.shot_meshes.is_none() {
            if let (Ok(s), Ok(b)) = (
                upload(renderer, device, &wr::engines::stone()),
                upload(renderer, device, &wr::engines::bolt()),
            ) {
                self.shot_meshes = Some((s, b));
            }
        }
        self.placed.clear();
        for p in &s.props {
            if self.prop(renderer, device, "props", p.kind) {
                self.placed.push((format!("props/{}", p.kind), p.transform));
            }
        }
        self.stance = vec![0.0; run.b.sim.n];
        self.dust.list.clear();
        self.glow.list.clear();
        self.bursts.clear();
        self.fx_seen = war.fx_first;
        self.last_time = war.time;
        // every crowd the armies need, big enough
        let mut want: HashMap<Key, usize> = HashMap::new();
        for r in &run.b.sim.regiments {
            if r.ty.model == "varrak" {
                continue;
            }
            let kind = r.ty.model;
            let look = look_index(r.look);
            let n = r.soldiers.len();
            *want.entry(Key::Troop(kind, look, 0)).or_default() += n.min(400);
            *want.entry(Key::Troop(kind, look, 1)).or_default() += n;
            *want.entry(Key::Standard(look)).or_default() += 1;
        }
        let n = run.b.sim.n;
        for (k, v) in [
            (Key::Blob, n + 200),
            (Key::Ring, n + 1400),
            (Key::Arrow, 3000),
            (Key::Bolt, 1500),
            (Key::Javelin, 800),
            (Key::Dust, 2600),
            (Key::Glow, 1800),
        ] {
            want.insert(k, v);
        }
        for (k, n) in want {
            let _ = self.crowd(renderer, device, k, n);
        }
        if run.b.sim.regiments.iter().any(|r| r.ty.model == "varrak")
            && self.hero.is_none()
            && !self.hero_failed
        {
            match load_hero(renderer, device, queue) {
                Ok(h) => self.hero = Some(h),
                Err(e) => {
                    eprintln!("[guerra] Varrak's model failed: {e:#}");
                    self.hero_failed = true;
                }
            }
        }
        let (w, h, img, _) = wr::scene::minimap(field, 720);
        eprintln!(
            "[guerra] field «{}» ready in {} ms ({} scatter, {} soldiers)",
            field.spec.kind,
            t0.elapsed().as_millis(),
            self.instances.len(),
            run.b.sim.n
        );
        let mut out = Vec::with_capacity(8 + img.len());
        out.extend((w as u32).to_le_bytes());
        out.extend((h as u32).to_le_bytes());
        out.extend(img);
        Some(out)
    }
}

fn load_hero(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Hero> {
    let file = runtime_3d_root().join("characters").join("varrak.glb");
    let model = GltfModel::load(&file).with_context(|| format!("Varrak: {}", file.display()))?;
    let mut meshes = Vec::new();
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };
    let mut cache = HashMap::new();
    for (i, mesh) in model.meshes.iter().enumerate() {
        let handle = renderer
            .upload_skinned_mesh(device, mesh)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        let albedo = mesh.texture_index.and_then(|k| model.textures.get(k));
        let normal = mesh
            .normal_texture_index
            .and_then(|k| model.textures.get(k));
        let texture =
            upload_texture_pair(renderer, device, queue, albedo, normal, &mut cache, &white)?;
        meshes.push((handle, mesh.material, texture));
        let m = model.mesh_transform(i);
        for v in &mesh.vertices {
            let y = m
                .transform_point3(Vec3::new(v.position[0], v.position[1], v.position[2]))
                .y;
            lo = lo.min(y);
            hi = hi.max(y);
        }
    }
    let scale = 3.5 / (hi - lo).max(0.01);
    Ok(Hero {
        model,
        meshes,
        scale,
        lift: -lo * scale,
        clip: None,
        clip_t: 0.0,
        from: None,
        fade: 1.0,
        swap: false,
        yaw: 0.0,
    })
}

/// The war's dusk: a warm low sun, a violet sky fill, haze that thins as the eye rises.
/// The battles' sun: a late golden afternoon. (At dusk, 21° up and facing the eye, the fields lay
/// dark and muddy and every pool shone back its glare.)
const BATTLE_SUN: [f32; 3] = [-0.55, 0.6, -0.42];

fn war_mood(
    mut env: EnvironmentFrame3D,
    sun: Vec3,
    dist: f32,
    campaign: bool,
) -> EnvironmentFrame3D {
    let lin = |hex: u32| {
        let c = wr::data::rgb(hex);
        [c[0], c[1], c[2]]
    };
    let scale = |c: [f32; 3], k: f32| c.map(|v| v * k);
    env.lighting.direction_to_light = sun.normalize_or_zero();
    if campaign {
        // the web map's day: a warm high sun from the north-west, a sky-blue fill, the earth's
        // brown bounce, a pale blue haze far off and the sea's horizon
        let sun = Vec3::new(-90.0, 140.0, 70.0).normalize_or_zero();
        env.lighting.direction_to_light = sun;
        env.lighting.key_color = scale(lin(0xffe6c2), 1.45);
        env.lighting.ambient_color = scale(lin(0xd6e6ff), 0.62);
        env.lighting.bounce_color = scale(lin(0x5a4c34), 0.55);
        env.lighting.shadow_extent = (dist * 1.1).clamp(40.0, 175.0);
        env.atmosphere.fog_color = lin(0xa9b8c9);
        env.atmosphere.fog_start = 150.0 + dist * 0.5;
        env.atmosphere.fog_end = 420.0 + dist * 1.2;
        env.atmosphere.fog_density = 0.0;
        env.atmosphere.height_falloff = 0.0;
        let sky = lin(0x7fa6cf);
        env.clear_color = [sky[0], sky[1], sky[2], 1.0];
        env.celestial.sun_direction = sun;
        env.celestial.sun_visibility = 1.0;
        env.celestial.moon_visibility = 0.0;
        env.celestial.moon_intensity = 0.0;
        env.celestial.reflection_strength = 0.0;
        env.celestial.horizon_tint = [1.0, 0.95, 0.88];
        env.celestial.horizon_strength = 0.1;
        return env;
    }
    env.lighting.key_color = if campaign {
        scale(lin(0xffcf9a), 0.386 * 2.25)
    } else {
        scale(lin(0xffdcae), 0.386 * 2.2)
    };
    env.lighting.ambient_color = if campaign {
        scale(lin(0xd9c6f2), 0.29 * 1.3)
    } else {
        scale(lin(0xd2cdee), 0.29 * 1.3)
    };
    env.lighting.bounce_color = scale(lin(0x4a3a4a), 0.47 * 1.25);
    env.lighting.shadow_extent = if campaign {
        (dist * 0.9).clamp(60.0, 170.0)
    } else {
        (dist * 0.95).clamp(45.0, 260.0)
    };
    let fog = if campaign {
        lin(0x6e5070)
    } else {
        lin(0xc88f86)
    };
    env.atmosphere.fog_color = fog;
    if campaign {
        env.atmosphere.fog_start = 300.0;
        env.atmosphere.fog_end = 760.0;
    } else {
        env.atmosphere.fog_start = 160.0 + dist * 1.2;
        env.atmosphere.fog_end = 700.0 + dist * 2.4;
    }
    env.atmosphere.fog_density = 0.0;
    env.atmosphere.height_falloff = 0.0;
    env.clear_color = [0.0, 0.0, 0.0, 1.0];
    // Its own dusk, whatever the hour at the castle: no moon, no moonlit water.
    env.celestial.sun_direction = sun.normalize_or_zero();
    env.celestial.sun_visibility = 1.0;
    env.celestial.moon_visibility = 0.0;
    env.celestial.moon_intensity = 0.0;
    env.celestial.reflection_strength = 0.0;
    env.celestial.horizon_tint = [1.0, 0.72, 0.5];
    env.celestial.horizon_strength = 0.25;
    env
}

fn war_finish(dist: f32) -> Finish3D {
    Finish3D {
        ink: 0.85,
        ink_color: [0.0070, 0.0030, 0.0115],
        ink_thickness: 1.25,
        ink_fade: [dist * 1.6 + 60.0, dist * 3.5 + 220.0],
        saturation: 1.08,
        contrast: 1.04,
        bloom: 0.28,
        bloom_threshold: 0.9,
        exposure: 1.1,
        occlusion: 0.0,
        occlusion_radius: 0.5,
        gi: 0.0,
        gi_radius: 3.0,
        gi_ambient: 1.0,
    }
}

/// The campaign map as the web map shades it: soft light (no hard toon bands), the textures'
/// relief, no lines.
fn campaign_toon() -> velvet_render::ToonStyle3D {
    velvet_render::ToonStyle3D {
        softness: 0.32,
        specular: 0.06,
        normal_detail: 0.9,
        shadow_tint: [0.86, 0.9, 1.0],
        rim: 0.02,
    }
}

fn campaign_finish() -> Finish3D {
    Finish3D {
        ink: 0.0,
        saturation: 1.04,
        contrast: 1.02,
        bloom: 0.28,
        bloom_threshold: 0.92,
        exposure: 1.12,
        ..war_finish(100.0)
    }
}

fn war_toon() -> velvet_render::ToonStyle3D {
    velvet_render::ToonStyle3D {
        normal_detail: 0.55,
        rim: 0.09,
        shadow_tint: [0.8, 0.72, 0.86],
        ..Default::default()
    }
}

/// A speck turned to face the camera from `at`.
fn facing(camera: Camera3D, at: Vec3, size: f32, color: [f32; 3], alpha: f32) -> CrowdInstance3D {
    let d = camera.position - at;
    let yaw = d.x.atan2(d.z);
    let pitch = d.y.atan2(d.x.hypot(d.z));
    let mut i = CrowdInstance3D::at([at.x, at.y, at.z], yaw, size);
    i.anim_b[0] = -pitch / 1.42;
    i.color = [color[0], color[1], color[2], 1.0];
    i.fade = 1.0 - alpha.clamp(0.0, 1.0);
    i
}

fn decal(at: Vec3, size: f32, color: [f32; 3], alpha: f32) -> CrowdInstance3D {
    let mut i = CrowdInstance3D::at([at.x, at.y, at.z], 0.0, size);
    i.color = [color[0], color[1], color[2], 1.0];
    i.fade = 1.0 - alpha.clamp(0.0, 1.0);
    i
}

fn hex3(hex: u32) -> [f32; 3] {
    wr::data::rgb(hex)
}

impl RenderState {
    fn war_scene(&mut self) -> Option<WarScene> {
        if let Some(s) = self.war_scene.take() {
            return Some(s);
        }
        match WarScene::new(&mut self.renderer, &self.device) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[guerra] the war's scene failed: {e:#}");
                None
            }
        }
    }

    /// The war: the table's page (the diorama behind it), a battle or the campaign, and the HUD.
    pub(super) fn render_war(&mut self, frame: &Frame, war: &WarSession) {
        let Some(mut scene) = self.war_scene() else {
            return;
        };
        let battle = matches!(war.page, WarPage::Battle) && war.battle.is_some();
        let saved_toon =
            self.renderer
                .replace_toon_style(if battle { war_toon() } else { campaign_toon() });
        // QA: VELVET_WAR_NOSHADOW draws the war without the sun's shadows.
        let saved_shadows = std::env::var_os("VELVET_WAR_NOSHADOW")
            .map(|_| self.renderer.replace_directional_shadows(false));
        let dist = war.cam.dist;
        let saved_finish = self.renderer.replace_finish(if battle {
            war_finish(dist)
        } else {
            campaign_finish()
        });
        match war.page {
            WarPage::Battle if war.battle.is_some() => {
                self.render_war_battle(&mut scene, frame, war)
            }
            _ => self.render_war_map(&mut scene, frame, war),
        }
        self.renderer.replace_finish(saved_finish);
        self.renderer.replace_toon_style(saved_toon);
        if let Some(s) = saved_shadows {
            self.renderer.replace_directional_shadows(s);
        }
        self.war_scene = Some(scene);
        // the HUD
        let mut view = war.view.clone();
        view.minimap_ready = self.war_minimap_serial == war.battle_serial && war.battle.is_some();
        self.map_overlay.set_war(Some(view));
        self.draw_world_overlay(frame);
        self.map_overlay.set_war(None);
    }

    /// The overlay as the fight draws it: only the place's own panel (no map, no prompts).
    fn draw_world_overlay(&mut self, frame: &Frame) {
        let Frame {
            player,
            yaw,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            shop_preview_rotation,
            venue,
            elapsed,
            ..
        } = *frame;
        self.map_overlay.draw(
            &self.device,
            &self.queue,
            &self.target_view,
            (WINDOW_WIDTH, WINDOW_HEIGHT),
            player,
            yaw,
            false,
            false,
            map_open,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab,
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory,
            shop_stock,
            shop_notice,
            shop_pointer,
            elapsed,
            shop_preview_rotation,
            &[],
            Some(venue),
            None,
            None,
            false,
        );
    }

    fn render_war_battle(&mut self, scene: &mut WarScene, frame: &Frame, war: &WarSession) {
        let Some(run) = war.battle.as_ref() else {
            return;
        };
        if scene.serial != war.battle_serial {
            scene.serial = war.battle_serial;
            if let Some(img) = scene.load_battle(&mut self.renderer, &self.device, &self.queue, war)
            {
                let w = u32::from_le_bytes(img[0..4].try_into().expect("4"));
                let h = u32::from_le_bytes(img[4..8].try_into().expect("4"));
                self.map_overlay.set_picture_rgba(
                    &self.device,
                    &self.queue,
                    super::map_overlay::WAR_MINIMAP,
                    w,
                    h,
                    &img[8..],
                );
                self.war_minimap_serial = war.battle_serial;
            }
        }
        let camera = war.cam.camera();
        let sim = &run.b.sim;
        let field = &sim.field;
        let dt = (war.time - scene.last_time).clamp(0.0, 0.1);
        scene.last_time = war.time;
        let play_dt = if run.b.paused { 0.0 } else { dt * run.b.speed };
        let mut draws: Vec<MeshDraw3D> = Vec::with_capacity(12_000);
        let mut blended: Vec<MeshDraw3D> = Vec::new();
        let mut lights = Vec::new();
        // the sky, far out round the eye
        draws.push(MeshDraw3D {
            mesh: scene.sky,
            transform: Mat4::from_translation(camera.position)
                * Mat4::from_scale(Vec3::new(2200.0, 2200.0, 2200.0)),
            material: Material3D::sky([1.0, 1.0, 1.0, 1.0]),
            texture: None,
        });
        for &g in &scene.ground_pool[..scene.ground_used] {
            let material = Material3D {
                texture: scene.ground_detail,
                ..Material3D::default()
            };
            draws.push(MeshDraw3D {
                mesh: g,
                transform: Mat4::IDENTITY,
                material,
                texture: scene.ground_detail,
            });
        }
        for &s in &scene.solid_pool[..scene.solid_used] {
            draws.push(MeshDraw3D {
                mesh: s,
                transform: Mat4::IDENTITY,
                material: Material3D::default().with_ambient_occlusion(0.3),
                texture: None,
            });
        }
        // a battle map's structures, and its works still standing
        for &(mesh, glow) in &scene.structures {
            draws.push(MeshDraw3D {
                mesh,
                transform: Mat4::IDENTITY,
                material: if glow {
                    Material3D::default().with_emission(1.6)
                } else {
                    Material3D::default().with_ambient_occlusion(0.3)
                },
                texture: None,
            });
        }
        for (w, mesh) in scene.works.iter().enumerate() {
            if let Some(mesh) = *mesh {
                if !field.open.get(w).copied().unwrap_or(false) {
                    draws.push(MeshDraw3D {
                        mesh,
                        transform: Mat4::IDENTITY,
                        material: Material3D::default().with_ambient_occlusion(0.3),
                        texture: None,
                    });
                }
            }
        }
        // what was built: engines (their arms swinging), stakes, palisades; the stones in the air;
        // the rubble where a work came down
        // the relic lying on the field: its plinth, its crystal turning, a column of light
        if let (Some(rel), Some((plinth, crystal))) =
            (run.relic.as_ref().filter(|r| !r.taken), scene.relic_meshes)
        {
            let y = field.walk_height(rel.x, rel.z);
            let base = Mat4::from_translation(Vec3::new(rel.x, y, rel.z));
            draws.push(MeshDraw3D {
                mesh: plinth,
                transform: base,
                material: Material3D::default().with_ambient_occlusion(0.3),
                texture: None,
            });
            let bob = (war.time * 1.6).sin() * 0.15;
            draws.push(MeshDraw3D {
                mesh: crystal,
                transform: base
                    * Mat4::from_translation(Vec3::new(0.0, bob, 0.0))
                    * Mat4::from_rotation_y(war.time * 0.9),
                material: Material3D::default().with_emission(2.2),
                texture: None,
            });
            blended.push(MeshDraw3D {
                mesh: scene.spark_column,
                transform: base * Mat4::from_scale(Vec3::new(1.4, 26.0, 1.4)),
                material: Material3D::additive([
                    1.0,
                    0.72,
                    0.3,
                    0.16 + 0.05 * (war.time * 2.0).sin(),
                ]),
                texture: None,
            });
            lights.push(PointLight3D::new(
                Vec3::new(rel.x, y + 2.0, rel.z),
                [1.0, 0.8, 0.45],
                5.0,
                12.0,
            ));
            if scene.glow.rand() < 0.4 {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let up = 2.0 + scene.glow.rand() * 2.0;
                scene.glow.spawn(
                    Vec3::new(rel.x + a.cos() * 0.8, y + 0.8, rel.z + a.sin() * 0.8),
                    Vec3::new(0.0, up, 0.0),
                    1.2,
                    0.35,
                    0.1,
                    [1.0, 0.85, 0.5, 1.0],
                    -0.3,
                    0.4,
                );
            }
        }
        // her powers at work, and the ring of the one she aims
        if let Some(pw) = sim.powers.as_ref() {
            for a in &pw.active {
                let c = wr::data::rgb(a.power.spec().color);
                let y = field.walk_height(a.x, a.z);
                let left = 1.0 - a.t / a.dur.max(0.01);
                blended.push(MeshDraw3D {
                    mesh: scene.spark_ring,
                    transform: Mat4::from_translation(Vec3::new(a.x, y + 0.3, a.z))
                        * Mat4::from_scale(Vec3::new(a.r, 1.0, a.r)),
                    material: Material3D::additive([c[0], c[1], c[2], 0.25 + 0.2 * left]),
                    texture: None,
                });
                lights.push(PointLight3D::new(
                    Vec3::new(a.x, y + 4.0, a.z),
                    c,
                    4.0 * left + 1.0,
                    a.r,
                ));
                let n = match a.power {
                    wr::powers::Power::Niebla => 6,
                    wr::powers::Power::Raices | wr::powers::Power::Grito => 3,
                    _ => 0,
                };
                for _ in 0..n {
                    let ang = scene.dust.rand() * std::f32::consts::TAU;
                    let d = scene.dust.rand().sqrt() * a.r;
                    let (px, pz) = (a.x + ang.cos() * d, a.z + ang.sin() * d);
                    let py = field.walk_height(px, pz);
                    match a.power {
                        wr::powers::Power::Niebla => {
                            let drift = scene.dust.rand() - 0.5;
                            scene.dust.spawn(
                                Vec3::new(px, py + 0.6, pz),
                                Vec3::new(drift, 0.25, -drift),
                                3.2,
                                3.5,
                                7.0,
                                [0.62, 0.5, 0.95, 0.28],
                                0.0,
                                0.2,
                            );
                        }
                        _ => {
                            let up = 1.5 + scene.glow.rand() * 2.0;
                            scene.glow.spawn(
                                Vec3::new(px, py + 0.2, pz),
                                Vec3::new(0.0, up, 0.0),
                                0.9,
                                0.45,
                                0.1,
                                [c[0], c[1], c[2], 1.0],
                                -0.2,
                                0.4,
                            );
                        }
                    }
                }
            }
        }
        if let (Some(p), Some((gx, gz))) = (run.power_aim, run.power_at) {
            let sp = p.spec();
            let c = wr::data::rgb(sp.color);
            let y = field.walk_height(gx, gz);
            let pulse = 0.55 + 0.2 * (war.time * 5.0).sin();
            blended.push(MeshDraw3D {
                mesh: scene.spark_ring,
                transform: Mat4::from_translation(Vec3::new(gx, y + 0.35, gz))
                    * Mat4::from_scale(Vec3::new(sp.radius, 1.0, sp.radius)),
                material: Material3D::additive([c[0] * 1.3, c[1] * 1.3, c[2] * 1.3, pulse]),
                texture: None,
            });
        }
        if let Some(siege) = &sim.siege {
            let looks = [run.b.setup.player.faction, run.b.setup.enemy.faction];
            for e in &siege.engines {
                if !e.alive && e.burn <= 0.0 && e.built < 1.0 {
                    continue;
                }
                let Some((body, arm)) = scene.engine_model(
                    &mut self.renderer,
                    &self.device,
                    e.kind,
                    looks[e.side as usize],
                ) else {
                    continue;
                };
                let y = field.walk_height(e.x, e.z);
                let mut t =
                    Mat4::from_translation(Vec3::new(e.x, y, e.z)) * Mat4::from_rotation_y(e.yaw);
                if !e.alive {
                    t = t
                        * Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0))
                        * Mat4::from_rotation_z(0.22);
                }
                let rise = if e.built < 1.0 {
                    0.12 + 0.88 * e.built
                } else {
                    1.0
                };
                let tb = t * Mat4::from_scale(Vec3::new(1.0, rise, 1.0));
                let material = if !e.alive {
                    Material3D::rgb(0.22, 0.19, 0.17)
                } else if e.built < 1.0 {
                    Material3D::rgb(0.85, 0.8, 0.72).with_ambient_occlusion(0.3)
                } else if e.selected {
                    Material3D::rgb(1.25, 1.18, 0.9).with_ambient_occlusion(0.3)
                } else {
                    Material3D::default().with_ambient_occlusion(0.3)
                };
                draws.push(MeshDraw3D {
                    mesh: body,
                    transform: tb,
                    material,
                    texture: None,
                });
                if let Some((arm_mesh, pivot)) = arm {
                    let ang = if e.kind == wr::siege::Kind::TorreAsedio {
                        if e.docked.is_some() {
                            1.42
                        } else {
                            0.0
                        }
                    } else if e.alive {
                        wr::engines::arm_angle(e.kind, e.fired, e.spec().reload)
                    } else {
                        0.6
                    };
                    let p = Vec3::new(pivot[0], pivot[1], pivot[2]);
                    draws.push(MeshDraw3D {
                        mesh: arm_mesh,
                        // (an arm is modelled round its own pivot)
                        transform: tb * Mat4::from_translation(p) * Mat4::from_rotation_x(ang),
                        material,
                        texture: None,
                    });
                }
                if !e.alive && e.burn > 0.0 {
                    let c = Vec3::new(e.x, y + 1.5, e.z);
                    lights.push(PointLight3D::new(
                        c,
                        [1.0, 0.55, 0.22],
                        3.0 * (e.burn / 9.0),
                        10.0,
                    ));
                    if scene.glow.rand() < 0.5 {
                        let a = scene.glow.rand() * std::f32::consts::TAU;
                        let up = 2.5 + scene.glow.rand() * 2.0;
                        scene.glow.spawn(
                            c + Vec3::new(a.cos() * 1.2, 0.0, a.sin() * 1.2),
                            Vec3::new(0.0, up, 0.0),
                            0.6,
                            0.6,
                            0.1,
                            [1.0, 0.6, 0.2, 1.0],
                            8.0,
                            1.0,
                        );
                    }
                }
            }
            if let Some((stone, bolt)) = scene.shot_meshes {
                for s in &siege.shots {
                    let t = if s.from.bolts() {
                        let yaw = s.vx.atan2(s.vz);
                        let pitch = -(s.vy).atan2(s.vx.hypot(s.vz));
                        Mat4::from_translation(Vec3::new(s.x, s.y, s.z))
                            * Mat4::from_rotation_y(yaw)
                            * Mat4::from_rotation_x(pitch)
                    } else {
                        let k = if s.from == wr::siege::Kind::Trabuquete {
                            1.9
                        } else {
                            1.0
                        };
                        Mat4::from_translation(Vec3::new(s.x, s.y, s.z))
                            * Mat4::from_rotation_x(war.time * 6.0)
                            * Mat4::from_scale(Vec3::new(k, k, k))
                    };
                    draws.push(MeshDraw3D {
                        mesh: if s.from.bolts() { bolt } else { stone },
                        transform: t,
                        material: Material3D::default(),
                        texture: None,
                    });
                }
            }
            // what she is placing, under the pointer: green where it can go, red where not
            if let (Some((gx, gz, yaw, why)), Some(kind)) = (run.ghost, run.build) {
                if let Some((body, arm)) =
                    scene.engine_model(&mut self.renderer, &self.device, kind, looks[0])
                {
                    let t =
                        Mat4::from_translation(Vec3::new(gx, field.walk_height(gx, gz) + 0.05, gz))
                            * Mat4::from_rotation_y(yaw);
                    let c = if why.is_none() {
                        [0.45, 1.25, 0.55]
                    } else {
                        [1.4, 0.35, 0.3]
                    };
                    let m = Material3D::rgb(c[0], c[1], c[2]).with_emission(0.35);
                    draws.push(MeshDraw3D {
                        mesh: body,
                        transform: t,
                        material: m,
                        texture: None,
                    });
                    if let Some((arm_mesh, pivot)) = arm {
                        draws.push(MeshDraw3D {
                            mesh: arm_mesh,
                            transform: t * Mat4::from_translation(Vec3::new(
                                pivot[0], pivot[1], pivot[2],
                            )),
                            material: m,
                            texture: None,
                        });
                    }
                }
            }
            let rock = scene.scatter[5].0;
            let broken: Vec<usize> = (0..field.open.len()).filter(|&w| field.open[w]).collect();
            for w in broken {
                for t in scene.rubble_of(field, w).to_vec() {
                    draws.push(MeshDraw3D {
                        mesh: rock,
                        transform: t,
                        material: Material3D::rgb(0.62, 0.58, 0.66),
                        texture: None,
                    });
                }
            }
        }
        for i in &scene.instances {
            let (mesh, _) = scene.scatter[i.kind];
            draws.push(MeshDraw3D {
                mesh,
                transform: i.transform,
                material: Material3D::rgb(i.tint[0], i.tint[1], i.tint[2]),
                texture: None,
            });
        }
        scene.sail_turn += dt * 0.6;
        for (key, t) in &scene.placed {
            if let Some(p) = scene.props.get(key) {
                if let Some(b) = p.body {
                    draws.push(MeshDraw3D {
                        mesh: b,
                        transform: *t,
                        material: Material3D::default().with_ambient_occlusion(0.3),
                        texture: None,
                    });
                }
                if let Some(g) = p.glow {
                    draws.push(MeshDraw3D {
                        mesh: g,
                        transform: *t,
                        material: Material3D::default().with_emission(1.6),
                        texture: None,
                    });
                }
                if let Some(f) = p.flags {
                    let c = hex3(0x6a32c0);
                    draws.push(MeshDraw3D {
                        mesh: f,
                        transform: *t,
                        material: Material3D::rgb(c[0], c[1], c[2]),
                        texture: None,
                    });
                }
                if let Some((s, pivot)) = p.sails {
                    let spin = *t
                        * Mat4::from_translation(pivot)
                        * Mat4::from_rotation_z(scene.sail_turn)
                        * Mat4::from_translation(-pivot);
                    draws.push(MeshDraw3D {
                        mesh: s,
                        transform: spin,
                        material: Material3D::default(),
                        texture: None,
                    });
                }
            }
        }
        // the soldiers
        scene.begin_frame();
        if scene.stance.len() != sim.n {
            scene.stance = vec![0.0; sim.n];
        }
        let eye = camera.position;
        let far2 = 88.0f32 * 88.0;
        let mut near_count: HashMap<Key, usize> = HashMap::new();
        for i in 0..sim.n {
            let r = &sim.regiments[sim.reg[i]];
            let t = r.ty;
            if t.model == "varrak" {
                continue;
            }
            let st = sim.state[i];
            if st == 2 {
                continue;
            }
            let sp = sim.vx[i].hypot(sim.vz[i]);
            let dead = st == 1;
            let mv = if dead {
                0.0
            } else {
                (sp / (t.run * 0.85)).min(1.15)
            };
            let want = if !dead
                && (sim.target[i] >= 0
                    || matches!(r.state, wr::sim::RState::Melee | wr::sim::RState::Firing)
                    || r.charging)
            {
                1.0
            } else {
                0.0
            };
            scene.stance[i] += (want - scene.stance[i]) * (dt * 4.0).min(1.0);
            let fly = if t.flyer {
                ((sim.alt[i] - 0.6) / 2.5).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut tint = if r.selected {
                1.22
            } else if run.hover == Some(r.index) {
                1.1
            } else {
                1.0
            };
            if r.dimmed {
                tint *= 0.72;
            }
            let p = Vec3::new(sim.px[i], sim.py[i], sim.pz[i]);
            let d2 = (p - eye).length_squared();
            let look = look_index(r.look);
            let mut key = Key::Troop(t.model, look, if d2 > far2 { 1 } else { 0 });
            if let Key::Troop(_, _, 0) = key {
                let n = near_count.entry(key).or_default();
                *n += 1;
                if scene.crowds.get(&key).is_none_or(|c| *n > c.cap) {
                    key = Key::Troop(t.model, look, 1);
                }
            }
            let mut inst = CrowdInstance3D::at(
                [p.x, p.y, p.z],
                sim.yaw[i],
                t.scale * (0.95 + sim.jx[i] * 0.25),
            );
            inst.anim_a = [sim.walk[i], mv, sim.act[i], sim.death[i]];
            inst.anim_b = [scene.stance[i], sim.flash[i], sim.knock[i], fly];
            inst.color = [tint, tint, tint, 1.0];
            scene.push(key, inst);
            if !dead {
                let gy = field.walk_height(p.x, p.z) + 0.06;
                let size = t.radius * if t.large { 3.2 } else { 2.7 } * (1.0 + sim.alt[i] * 0.08);
                let a = 0.34 * (1.0 - (sim.alt[i] / 12.0).min(0.7));
                if r.selected {
                    scene.push(
                        Key::Ring,
                        decal(
                            Vec3::new(p.x, gy, p.z),
                            size * 1.25,
                            hex3(t.faction.info().bright),
                            0.95,
                        ),
                    );
                } else {
                    scene.push(Key::Blob, decal(Vec3::new(p.x, gy, p.z), size, [1.0; 3], a));
                }
            }
        }
        // Varrak, in his own skin
        let mut hero_decals: Vec<(Key, CrowdInstance3D)> = Vec::new();
        if let Some(hero) = scene.hero.as_mut() {
            for r in sim.regiments.iter().filter(|r| r.ty.model == "varrak") {
                let Some(&i) = r.soldiers.first() else {
                    continue;
                };
                if sim.state[i] == 2 || (sim.state[i] == 1 && !r.awake) {
                    continue;
                }
                let sp = sim.vx[i].hypot(sim.vz[i]);
                let pick = if r.awake {
                    hero.clip("Varrak_Kneel")
                } else if sim.stagger[i] > 0.3 {
                    hero.clip("Varrak_Stun")
                } else if sim.act[i] > 0.0 {
                    let cur = hero.clip;
                    let (smash, sweep) = (hero.clip("Varrak_Smash"), hero.clip("Varrak_Sweep"));
                    if cur != smash && cur != sweep {
                        hero.swap = !hero.swap;
                        if hero.swap {
                            sweep.or(smash)
                        } else {
                            smash
                        }
                    } else {
                        cur
                    }
                } else if sp > 0.5 {
                    hero.clip("Varrak_Walk")
                } else {
                    hero.clip("Varrak_Idle")
                };
                hero.play(pick, 0.2);
                let rate = if sp > 0.5 && sim.act[i] <= 0.0 {
                    (sp / 1.4).max(0.6)
                } else {
                    1.0
                };
                hero.clip_t += play_dt * rate;
                hero.fade = (hero.fade + play_dt / 0.2).min(1.0);
                if let Some((_, ft)) = hero.from.as_mut() {
                    *ft += play_dt;
                }
                let once = r.awake;
                let sample = |h: &Hero, c: usize, t: f32| {
                    let d = h.duration(c);
                    let t = if once {
                        t.min(d - 0.001)
                    } else {
                        t.rem_euclid(d)
                    };
                    h.model.pose(c, t)
                };
                let pose =
                    hero.clip
                        .and_then(|c| sample(hero, c, hero.clip_t))
                        .map(|now| match hero.from {
                            Some((c, t)) if hero.fade < 1.0 => match sample(hero, c, t) {
                                Some(before) => before.blend(&now, hero.fade),
                                None => now,
                            },
                            _ => now,
                        });
                if let Some(pose) = &pose {
                    pose_skinned_meshes(
                        &mut self.renderer,
                        &self.queue,
                        &hero.model,
                        &hero.meshes,
                        pose,
                    );
                }
                hero.yaw = sim.yaw[i];
                let root =
                    Mat4::from_translation(Vec3::new(sim.px[i], sim.py[i] + hero.lift, sim.pz[i]))
                        * Mat4::from_rotation_y(hero.yaw)
                        * Mat4::from_scale(Vec3::new(hero.scale, hero.scale, hero.scale));
                let flash = sim.flash[i];
                for (index, (mesh, material, texture)) in hero.meshes.iter().enumerate() {
                    let mut m = (*material).with_ambient_occlusion(0.28);
                    if flash > 0.05 {
                        m = m.with_emission(flash * 1.5);
                    }
                    draws.push(MeshDraw3D {
                        mesh: *mesh,
                        transform: root * hero.model.mesh_transform(index),
                        material: m,
                        texture: *texture,
                    });
                }
                let gy = field.walk_height(sim.px[i], sim.pz[i]) + 0.06;
                let at = Vec3::new(sim.px[i], gy, sim.pz[i]);
                if r.selected {
                    hero_decals.push((Key::Ring, decal(at, 3.6, [1.0, 0.8, 0.4], 0.95)));
                } else {
                    hero_decals.push((Key::Blob, decal(at, 3.2, [1.0; 3], 0.4)));
                }
            }
        }
        for (k, i) in hero_decals {
            scene.push(k, i);
        }
        // the Queen's light and her aura when chosen
        if let Some(q) = run.b.queen {
            let r = &sim.regiments[q];
            if let Some(&i) = r.soldiers.iter().find(|&&i| sim.state[i] == 0) {
                lights.push(PointLight3D::new(
                    Vec3::new(sim.px[i], sim.py[i] + 2.8, sim.pz[i]),
                    hex3(0xc79aff),
                    7.0,
                    18.0,
                ));
                if r.selected {
                    if let Some(aura) = r.ty.aura {
                        let big = aura.radius;
                        for k in 0..140 {
                            if k % 4 == 3 {
                                continue;
                            }
                            let ang = k as f32 / 140.0 * std::f32::consts::TAU + war.time * 0.05;
                            let (x, z) = (sim.px[i] + ang.cos() * big, sim.pz[i] + ang.sin() * big);
                            scene.push(
                                Key::Ring,
                                decal(
                                    Vec3::new(x, field.walk_height(x, z) + 0.1, z),
                                    1.1,
                                    [0.62, 0.36, 1.0],
                                    0.5,
                                ),
                            );
                        }
                    }
                }
            }
        }
        // the ghosts of orders given, the line being dragged, the deployment zone
        for g in &run.ghosts {
            let a = 0.85 * (1.0 - g.t / 1.6);
            if a <= 0.0 {
                continue;
            }
            for &(x, z) in &g.pts {
                scene.push(
                    Key::Ring,
                    decal(
                        Vec3::new(x, field.walk_height(x, z) + 0.08, z),
                        g.size,
                        [1.0, 0.78, 0.35],
                        a,
                    ),
                );
            }
        }
        for s in &run.preview {
            let r = &sim.regiments[s.reg];
            let n = r.alive as usize;
            for k in (0..n).step_by(if n > 60 { 2 } else { 1 }) {
                let (x, z) = sim.slot_pos(s.reg, k, s.x, s.z, s.heading, s.files);
                scene.push(
                    Key::Ring,
                    decal(
                        Vec3::new(x, field.walk_height(x, z) + 0.08, z),
                        r.ty.radius * 2.4,
                        [1.0, 0.85, 0.45],
                        0.85,
                    ),
                );
            }
        }
        if run.b.deploying {
            let zn = run.b.zone;
            let mut edge = |x: f32, z: f32| {
                scene.push(
                    Key::Ring,
                    decal(
                        Vec3::new(x, field.walk_height(x, z) + 0.1, z),
                        1.4,
                        [1.0, 0.75, 0.3],
                        0.7,
                    ),
                )
            };
            let mut x = zn.x0;
            while x <= zn.x1 {
                edge(x, zn.z0);
                edge(x, zn.z1);
                x += 5.0;
            }
            let mut z = zn.z0;
            while z <= zn.z1 {
                edge(zn.x0, z);
                edge(zn.x1, z);
                z += 5.0;
            }
        }
        // the standards
        for r in &sim.regiments {
            if r.alive == 0
                || r.state == wr::sim::RState::Fled
                || r.ty.role == Role::Hero
                || r.ty.flyer
            {
                continue;
            }
            let files = r.order_files().min(r.alive).max(1) as usize;
            let k = (files / 2 + files).min(r.soldiers.len().saturating_sub(1));
            let Some(&b) = r.soldiers.get(k).or(r.soldiers.first()) else {
                continue;
            };
            let mut inst = CrowdInstance3D::at(
                [sim.px[b], sim.py[b], sim.pz[b]],
                sim.yaw[b] + std::f32::consts::FRAC_PI_2,
                1.0,
            );
            let bright = if r.selected { 1.3 } else { 1.0 };
            inst.color = [bright, bright, bright, 1.0];
            inst.anim_a = [
                r.index as f32 * 1.7,
                if r.state.routed() { 1.0 } else { 0.3 },
                0.0,
                0.0,
            ];
            scene.push(Key::Standard(look_index(r.look)), inst);
        }
        // missiles in flight and standing in the ground
        for m in &sim.missiles {
            if m.kind == wr::data::Projectile::Light {
                if !m.done {
                    scene.glow.spawn(
                        Vec3::new(m.x, m.y, m.z),
                        Vec3::ZERO,
                        0.16,
                        0.7,
                        0.25,
                        [1.0, 0.85, 0.55, 0.55],
                        0.0,
                        0.0,
                    );
                }
                continue;
            }
            let key = match m.kind {
                wr::data::Projectile::Bolt => Key::Bolt,
                wr::data::Projectile::Javelin => Key::Javelin,
                _ => Key::Arrow,
            };
            let lift = if m.done {
                if m.stuck <= 0.2 {
                    continue;
                }
                0.25
            } else {
                0.0
            };
            let yaw = m.vx.atan2(m.vz);
            let pitch = m.vy.atan2(m.vx.hypot(m.vz));
            let mut inst = CrowdInstance3D::at([m.x, m.y + lift, m.z], yaw, 1.0);
            inst.anim_b[0] = -pitch / 1.42;
            scene.push(key, inst);
        }
        // effects the battle asked for
        let new_from = scene.fx_seen.max(war.fx_first);
        let skip = (new_from - war.fx_first) as usize;
        for fx in war.fx.iter().skip(skip) {
            spawn_fx(scene, field, *fx);
        }
        scene.fx_seen = war.fx_first + war.fx.len() as u64;
        // dust from running feet
        for k in 0..24u32 {
            if sim.n == 0 {
                break;
            }
            let i = ((war.time * 977.0) as u32)
                .wrapping_mul(2_654_435_761)
                .wrapping_add(k * 40_503) as usize
                % sim.n;
            if sim.state[i] != 0 || sim.alt[i] > 0.5 || play_dt == 0.0 {
                continue;
            }
            let t = sim.regiments[sim.reg[i]].ty;
            let sp = sim.vx[i].hypot(sim.vz[i]);
            if sp < t.walk * 1.5 || field.depth_at(sim.px[i], sim.pz[i]) > -0.1 {
                continue;
            }
            if scene.dust.rand() < if t.large { 0.5 } else { 0.15 } {
                puff(
                    &mut scene.dust,
                    Vec3::new(sim.px[i], sim.py[i], sim.pz[i]),
                    1,
                    if t.large { 2.0 } else { 1.3 },
                    0.6,
                );
            }
        }
        scene.dust.update(play_dt);
        scene.glow.update(play_dt);
        draw_particles(scene, camera);
        // the bursts of the spark
        for b in &mut scene.bursts {
            b.t += play_dt;
        }
        scene.bursts.retain(|b| b.t < 1.9);
        for b in &scene.bursts {
            let t = (b.t / 1.3).min(1.0);
            let s = b.radius * (0.15 + 0.95 * (1.0 - (1.0 - t) * (1.0 - t)));
            let a = (1.0 - t * t).max(0.0);
            if let Some(c) = b.color {
                // a power's ring: its colour, no column
                blended.push(MeshDraw3D {
                    mesh: scene.spark_ring,
                    transform: Mat4::from_translation(b.at + Vec3::new(0.0, 0.4, 0.0))
                        * Mat4::from_scale(Vec3::new(s, 1.0, s)),
                    material: Material3D::additive([c[0] * 1.4, c[1] * 1.4, c[2] * 1.4, a]),
                    texture: None,
                });
                lights.push(PointLight3D::new(
                    b.at + Vec3::new(0.0, 3.0, 0.0),
                    c,
                    7.0 * (1.0 - t),
                    b.radius * 1.2,
                ));
                continue;
            }
            blended.push(MeshDraw3D {
                mesh: scene.spark_ring,
                transform: Mat4::from_translation(b.at + Vec3::new(0.0, 0.4, 0.0))
                    * Mat4::from_scale(Vec3::new(s, 1.0, s)),
                // Under the bloom more than this burns the violet and amber out to white.
                material: Material3D::additive([0.72, 0.72, 0.72, a]),
                texture: None,
            });
            let ct = (b.t / 1.8).min(1.0);
            blended.push(MeshDraw3D {
                mesh: scene.spark_column,
                transform: Mat4::from_translation(b.at)
                    * Mat4::from_scale(Vec3::new(
                        1.6 + t * 2.5,
                        30.0 * (1.0 - t * 0.3),
                        1.6 + t * 2.5,
                    )),
                material: Material3D::additive([0.8, 0.8, 0.8, (1.0 - ct) * 0.6]),
                texture: None,
            });
            lights.push(PointLight3D::new(
                b.at + Vec3::new(0.0, 3.0, 0.0),
                hex3(0xb48aff),
                9.0 * (1.0 - t),
                b.radius * 1.2,
            ));
        }
        if self.water_visible_for_war(scene) {
            if let Some(w) = scene.water_pool {
                blended.push(MeshDraw3D {
                    mesh: w,
                    transform: Mat4::IDENTITY,
                    material: Material3D {
                        base_color: [1.0, 1.0, 1.0, 0.97],
                        ..Material3D::default()
                    }
                    .with_emission(0.3),
                    texture: None,
                });
            }
        }
        scene.end_frame(
            &mut self.renderer,
            &self.queue,
            war.time,
            &mut draws,
            &mut blended,
        );
        draws.extend(blended);
        // QA: VELVET_WAR_ONLY=ground keeps only the ground tiles (to look at the field's paint).
        if std::env::var("VELVET_WAR_ONLY").is_ok_and(|v| v == "ground") {
            let tiles: std::collections::HashSet<MeshHandle> = scene.ground_pool
                [..scene.ground_used]
                .iter()
                .copied()
                .collect();
            draws.retain(|d| tiles.contains(&d.mesh));
        }
        if std::env::var_os("VELVET_WAR_NOLIGHTS").is_some() {
            lights.clear();
        }
        let env = war_mood(
            frame.environment,
            Vec3::new(BATTLE_SUN[0], BATTLE_SUN[1], BATTLE_SUN[2]),
            war.cam.dist,
            false,
        );
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            env,
            &lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
    }

    fn water_visible_for_war(&self, scene: &WarScene) -> bool {
        scene.water_on
    }

    /// The campaign's diorama (and the table's page over it).
    fn render_war_map(&mut self, scene: &mut WarScene, frame: &Frame, war: &WarSession) {
        if scene.camp.is_none() {
            match load_camp(
                &mut self.renderer,
                &self.device,
                &self.queue,
                scene,
                frame.varrak_awake,
            ) {
                Ok(c) => scene.camp = Some(c),
                Err(e) => {
                    eprintln!("[guerra] the campaign's map failed: {e:#}");
                    return;
                }
            }
        }
        let state = frame.war_campaign.filter(|_| war.page == WarPage::Campaign);
        let mut camp_owned = scene.camp.take().expect("the map");
        let camp = &mut camp_owned;
        let s = state.unwrap_or(&camp.preview);
        let run = war.camp.as_ref().filter(|_| war.page == WarPage::Campaign);
        // the camera: the player's, or a slow turn round the table behind its page
        let camera = if run.is_some() {
            war.cam.camera()
        } else {
            let t = war.time * 0.035;
            let (d, h) = (190.0, 150.0);
            Camera3D {
                position: Vec3::new(-10.0 + t.sin() * d, h, t.cos() * d),
                target: Vec3::new(-10.0, 0.0, 0.0),
                up: Vec3::Y,
                fov_y_radians: 32.0f32.to_radians(),
                orthographic_height: None,
                near: 0.8,
                far: 2600.0,
            }
        };
        let dt = (war.time - scene.last_time).clamp(0.0, 0.1);
        scene.last_time = war.time;
        // repaint the provinces when something on them changed
        let fog: Vec<bool> = match run {
            Some(_) => s.visible().into_iter().map(|v| !v).collect(),
            None => vec![false; wr::data::PROVINCES.len()],
        };
        let (reach, selected, hover) = match run {
            Some(c) => {
                let army = match &c.sel {
                    Some(super::zone_war::Sel::Army(id)) => s.army(id),
                    _ => None,
                };
                let reach: Vec<bool> = match army {
                    Some(a)
                        if a.faction == Faction::Reina
                            && s.phase == wr::rules::Phase::Player
                            && !c.busy =>
                    {
                        let r = s.reachable(a);
                        wr::data::PROVINCES
                            .iter()
                            .map(|p| r.contains(&p.id))
                            .collect()
                    }
                    _ => vec![false; wr::data::PROVINCES.len()],
                };
                let sel = match &c.sel {
                    Some(super::zone_war::Sel::Province(p)) => Some(*p),
                    Some(super::zone_war::Sel::Army(_)) => {
                        army.map(|a| wr::data::province_index(&a.at))
                    }
                    None => None,
                };
                (reach, sel, c.hover)
            }
            None => (vec![false; wr::data::PROVINCES.len()], None, None),
        };
        let owners: u64 = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for p in &s.provinces {
                (p.owner as u8, p.lit).hash(&mut h);
            }
            fog.hash(&mut h);
            reach.hash(&mut h);
            selected.hash(&mut h);
            hover.hash(&mut h);
            h.finish()
        };
        let version = run.map_or(0, |c| c.paint_version);
        // the borders' ribbons, again when the eye has moved or the map has changed
        if !camp.borders.is_empty() {
            let look_hash = owners ^ version.wrapping_mul(0x9e37_79b9);
            let eye = (camera.position - camera.target).length().max(1.0);
            let stale = match camp.border_key {
                None => true,
                Some((p, h)) => h != look_hash || (p - camera.position).length() > eye * 0.02,
            };
            if stale {
                let owner_list: Vec<Faction> = s.provinces.iter().map(|p| p.owner).collect();
                let mesh = border_ribbons(
                    &camp.borders,
                    &BorderLook {
                        owner: &owner_list,
                        fog: &fog,
                        reach: &reach,
                        selected,
                        hover,
                    },
                    camera,
                );
                match camp.border_mesh {
                    Some(h) => {
                        let _ = self.renderer.replace_mesh(&self.device, h, &mesh);
                    }
                    None => {
                        camp.border_mesh = self.renderer.upload_mesh(&self.device, &mesh).ok();
                    }
                }
                camp.border_key = Some((camera.position, look_hash));
            }
        }
        if camp.painted != Some((version, owners)) {
            camp.painted = Some((version, owners));
            let t_paint = std::time::Instant::now();
            let img = camp.provinces.paint(
                s,
                &wr::campaign_map::Paint {
                    fog: fog.clone(),
                    reach,
                    selected,
                    hover,
                },
            );
            let ms = t_paint.elapsed().as_secs_f32() * 1000.0;
            if ms > 25.0 {
                eprintln!("[guerra] the provinces' picture took {ms:.0} ms");
            }
            let tex = TextureData3D {
                width: camp.provinces.pw as u32,
                height: camp.provinces.ph as u32,
                rgba: img,
            };
            let _ =
                self.renderer
                    .replace_texture(&self.device, &self.queue, camp.overlay_tex, &tex);
        }
        let mut draws: Vec<MeshDraw3D> = Vec::with_capacity(8000);
        let mut blended: Vec<MeshDraw3D> = Vec::new();
        draws.push(MeshDraw3D {
            mesh: scene.sky,
            transform: Mat4::from_translation(camera.position)
                * Mat4::from_scale(Vec3::new(2200.0, 2200.0, 2200.0)),
            material: Material3D::sky([1.0, 1.0, 1.0, 1.0]),
            texture: None,
        });
        draws.push(MeshDraw3D {
            mesh: camp.land,
            transform: Mat4::IDENTITY,
            material: Material3D {
                texture: camp.land_tex,
                ..Material3D::default()
            },
            texture: camp.land_tex,
        });
        // the sea to the horizon (the web map's; the old table and its frame are gone): in the
        // water sheet when the land is painted, else its own plain sheet
        if camp.land_tex.is_none() {
            draws.push(MeshDraw3D {
                mesh: camp.sea,
                transform: Mat4::IDENTITY,
                material: Material3D::default(),
                texture: None,
            });
        }
        if let Some(r) = camp.roads.filter(|_| camp.land_tex.is_none()) {
            draws.push(MeshDraw3D {
                mesh: r,
                transform: Mat4::from_translation(Vec3::new(0.0, 0.02, 0.0)),
                material: Material3D::default(),
                texture: None,
            });
        }
        for i in &camp.woods {
            let (mesh, _) = scene.scatter[i.kind];
            draws.push(MeshDraw3D {
                mesh,
                transform: i.transform,
                material: Material3D::default(),
                texture: None,
            });
        }
        for &(mesh, tex, glow) in &camp.world {
            let material = match glow {
                Some(e) => Material3D::default().with_emission(e),
                None => Material3D {
                    texture: tex,
                    ..Material3D::default()
                }
                .with_ambient_occlusion(0.3),
            };
            draws.push(MeshDraw3D {
                mesh,
                transform: Mat4::IDENTITY,
                material,
                texture: tex,
            });
        }
        scene.sail_turn += dt * 0.8;
        for &(pi, t) in &camp.settlements {
            let p = &wr::data::PROVINCES[pi];
            let key = format!("campaign/{}", p.settlement);
            let Some(placed) = scene.props.get(&key) else {
                continue;
            };
            if let Some(b) = placed.body {
                draws.push(MeshDraw3D {
                    mesh: b,
                    transform: t,
                    material: Material3D::default().with_ambient_occlusion(0.3),
                    texture: None,
                });
            }
            if let Some(g) = placed.glow {
                draws.push(MeshDraw3D {
                    mesh: g,
                    transform: t,
                    material: Material3D::default().with_emission(1.6),
                    texture: None,
                });
            }
            if let Some(f) = placed.flags {
                let c = wr::campaign_map::flag_color(s.prov(p.id).owner);
                draws.push(MeshDraw3D {
                    mesh: f,
                    transform: t,
                    material: Material3D::rgb(c[0], c[1], c[2]),
                    texture: None,
                });
            }
            if let Some((sm, pivot)) = placed.sails {
                let spin = t
                    * Mat4::from_translation(pivot)
                    * Mat4::from_rotation_z(scene.sail_turn)
                    * Mat4::from_translation(-pivot);
                draws.push(MeshDraw3D {
                    mesh: sm,
                    transform: spin,
                    material: Material3D::default(),
                    texture: None,
                });
            }
        }
        // the provinces painted over the land, and the water
        blended.push(MeshDraw3D {
            mesh: camp.water,
            transform: Mat4::IDENTITY,
            material: Material3D {
                base_color: [1.0, 1.0, 1.0, 1.0],
                texture: camp.water_tex,
                ..Material3D::default()
            },
            texture: camp.water_tex,
        });
        blended.push(MeshDraw3D {
            mesh: camp.overlay,
            transform: Mat4::IDENTITY,
            material: Material3D {
                base_color: [1.0, 1.0, 1.0, 0.98],
                texture: Some(camp.overlay_tex),
                occlusion_window: [0.0; 4],
            }
            .with_emission(0.22),
            texture: Some(camp.overlay_tex),
        });
        if let Some(m) = camp.border_mesh {
            blended.push(MeshDraw3D {
                mesh: m,
                transform: Mat4::IDENTITY,
                material: Material3D::default().with_emission(0.35),
                texture: None,
            });
        }
        // the armies on their bases
        scene.begin_frame();
        let mut figures: Vec<(Key, CrowdInstance3D)> = Vec::new();
        for a in s.armies.iter().filter(|a| !a.units.is_empty()) {
            if a.faction != Faction::Reina
                && fog
                    .get(wr::data::province_index(&a.at))
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }
            let (at, yaw) = match run.and_then(|c| c.tokens.get(&a.id)) {
                Some(t) => (t.at, t.yaw),
                None => {
                    let p = wr::data::province(&a.at);
                    let (x, z) = p.site();
                    (Vec3::new(x, camp_top(camp, x, z + 10.0), z + 10.0), 0.0)
                }
            };
            let fi = look_index(a.faction) as usize;
            let (base, rim) = camp.bases[fi];
            // the base grows to hold the army's regiments; the whole token at the web map's scale
            // (a general about as tall as a house, not as a tower)
            const K: f32 = 0.6;
            let (spots, radius) = super::zone_war::formation(a.units.len());
            let radius = radius * K;
            let grow = Vec3::new(radius / 2.75, K, radius / 2.75);
            draws.push(MeshDraw3D {
                mesh: base,
                transform: Mat4::from_translation(at + Vec3::new(0.0, 0.35 * K, 0.0))
                    * Mat4::from_scale(grow),
                material: Material3D::default(),
                texture: None,
            });
            draws.push(MeshDraw3D {
                mesh: rim,
                transform: Mat4::from_translation(at + Vec3::new(0.0, 0.62 * K, 0.0))
                    * Mat4::from_scale(grow),
                material: Material3D::default(),
                texture: None,
            });
            // a figure for each regiment: the lead in front, the rest in rows behind it
            let (fwd, right) = (
                Vec3::new(yaw.sin(), 0.0, yaw.cos()),
                Vec3::new(yaw.cos(), 0.0, -yaw.sin()),
            );
            let lead_index = a
                .units
                .iter()
                .position(|u| std::ptr::eq(u, a.lead()))
                .unwrap_or(0);
            let order =
                std::iter::once(lead_index).chain((0..a.units.len()).filter(|&i| i != lead_index));
            for (k, (ui, spot)) in order.zip(spots.iter()).enumerate() {
                let t = wr::data::unit(&a.units[ui].ty);
                let kind = if t.model == "varrak" { "orc" } else { t.model };
                let full = match kind {
                    "knight" => 2.1,
                    "queen" => 2.6,
                    _ if t.model == "varrak" => 3.3,
                    _ => 2.7,
                };
                // the lead stands full size, the regiments a little smaller
                let scale = if k == 0 { full } else { full * 0.72 } * K;
                let p = at + (right * spot[0] + fwd * spot[1]) * K;
                let mut inst = CrowdInstance3D::at([p.x, at.y + 0.7 * K, p.z], yaw, scale);
                inst.anim_a[0] = war.time * 0.4 + a.id.len() as f32 + k as f32 * 0.37;
                figures.push((Key::Troop(kind, look_index(a.faction), 0), inst));
            }
            let back = at - fwd * (radius - 0.9) - right * (radius * 0.55);
            let mut flag = CrowdInstance3D::at([back.x, at.y + 0.6 * K, back.z], 0.0, 1.45 * K);
            flag.anim_a = [a.id.len() as f32 * 1.7, 0.3, 0.0, 0.0];
            figures.push((Key::Standard(look_index(a.faction)), flag));
            let chosen = run.is_some_and(
                |c| matches!(&c.sel, Some(super::zone_war::Sel::Army(id)) if *id == a.id),
            );
            if chosen {
                let pulse = 0.6 + (war.time * 5.0).sin() * 0.3;
                figures.push((
                    Key::Ring,
                    decal(
                        at + Vec3::new(0.0, 0.12, 0.0),
                        8.9 * radius / 2.75,
                        [1.0, 0.85, 0.54],
                        pulse,
                    ),
                ));
            }
        }
        // the world's banners in their region's colours, its fires and the Queen's crystals glowing
        let mut world_lights: Vec<PointLight3D> = Vec::new();
        for (k, an) in camp.anchors.iter().enumerate() {
            match an.kind {
                0 => {
                    let owner = an
                        .region
                        .map_or(Faction::Libres, |r| s.prov(wr::data::PROVINCES[r].id).owner);
                    let mut flag = CrowdInstance3D::at(
                        [an.at.x, an.at.y - 0.05, an.at.z],
                        (k as f32 * 2.3).sin() * 0.6,
                        an.size * 0.42,
                    );
                    flag.anim_a = [k as f32 * 1.7, 0.3, 0.0, 0.0];
                    figures.push((Key::Standard(look_index(owner)), flag));
                }
                1 => {
                    let flicker = 0.85 + 0.15 * (war.time * 9.0 + k as f32).sin();
                    figures.push((
                        Key::Cloud,
                        facing(
                            camera,
                            an.at + Vec3::new(0.0, 0.25, 0.0),
                            0.9 * flicker,
                            [1.0, 0.6, 0.25],
                            0.85,
                        ),
                    ));
                    if world_lights.len() < 24 {
                        world_lights.push(PointLight3D::new(
                            an.at + Vec3::new(0.0, 0.6, 0.0),
                            [1.0, 0.55, 0.22],
                            2.2 * flicker,
                            6.0,
                        ));
                    }
                }
                2 => {
                    let pulse = 0.75 + 0.25 * (war.time * 1.6 + k as f32).sin();
                    figures.push((
                        Key::Cloud,
                        facing(camera, an.at, 1.6 * pulse, [0.7, 0.42, 1.0], 0.55),
                    ));
                    if world_lights.len() < 24 {
                        world_lights.push(PointLight3D::new(
                            an.at,
                            [0.62, 0.32, 1.0],
                            2.6 * pulse,
                            7.0,
                        ));
                    }
                }
                _ => {
                    let rise = (war.time * 0.4 + k as f32 * 0.3).fract();
                    figures.push((
                        Key::Cloud,
                        facing(
                            camera,
                            an.at + Vec3::new(0.0, rise * 2.5, 0.0),
                            1.0 + rise,
                            [0.95, 0.95, 1.0],
                            0.35 * (1.0 - rise),
                        ),
                    ));
                }
            }
        }
        // the cloud over what the Queen cannot see
        for (k, &(x, y, z, size, prov, phase)) in camp.clouds.iter().enumerate() {
            let want = if fog.get(prov).copied().unwrap_or(false) {
                0.78
            } else {
                0.0
            };
            let a = &mut camp.cloud_alpha[k];
            *a += (want - *a) * (dt * 1.6).min(1.0);
            if *a < 0.01 {
                continue;
            }
            let at = Vec3::new(
                x + (war.time * 0.05 + phase).sin() * 3.0,
                y,
                z + (war.time * 0.04 + phase).cos() * 2.0,
            );
            // thinning as the eye comes down into them
            let near = ((at - camera.position).length() - size * 0.6).max(0.0);
            let k = ((near - 20.0) / 60.0).clamp(0.0, 1.0);
            if k <= 0.01 {
                continue;
            }
            figures.push((Key::Cloud, facing(camera, at, size, [1.0; 3], *a * 0.8 * k)));
        }
        let mut want: HashMap<Key, usize> = HashMap::new();
        for (k, _) in &figures {
            *want.entry(*k).or_default() += 1;
        }
        for (k, n) in want {
            let have = scene.crowds.get(&k).map_or(0, |c| c.cap);
            if have < n {
                let _ = scene.crowd(&mut self.renderer, &self.device, k, n);
            }
        }
        for (k, i) in figures {
            scene.push(k, i);
        }
        scene.end_frame(
            &mut self.renderer,
            &self.queue,
            war.time,
            &mut draws,
            &mut blended,
        );
        draws.extend(blended);
        scene.camp = Some(camp_owned);
        let dist = if run.is_some() { war.cam.dist } else { 200.0 };
        let env = war_mood(frame.environment, Vec3::new(-0.5, 0.6, -0.36), dist, true);
        let (scene_view, scene_size) = self.scene_target();
        self.renderer.render_with_environment_and_lights(
            &self.device,
            &self.queue,
            &scene_view,
            scene_size,
            camera,
            env,
            &world_lights,
            &draws,
        );
        self.finish_scene(&scene_view, scene_size);
    }
}

fn camp_top(c: &CampScene, x: f32, z: f32) -> f32 {
    let _ = c;
    wr::campaign_map::land_height(x, z).max(wr::campaign_map::WATER)
}

fn load_camp(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &mut WarScene,
    varrak: bool,
) -> Result<CampScene> {
    let t0 = std::time::Instant::now();
    super::zone_war::install_war_map();
    let land = wr::campaign_map::Land::new();
    let provinces = wr::campaign_map::Provinces::new();
    let preview = wr::rules::Campaign::new(1, varrak);
    let img = provinces.paint(
        &preview,
        &wr::campaign_map::Paint {
            fog: vec![false; wr::data::PROVINCES.len()],
            ..Default::default()
        },
    );
    let overlay_tex = renderer
        .upload_texture(
            device,
            queue,
            &TextureData3D {
                width: provinces.pw as u32,
                height: provinces.ph as u32,
                rgba: img,
            },
        )
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    // the map 2.0's world: every settlement and landmark of the prototype
    let world_path = exploration3d_state::data_dir()
        .join("guerra")
        .join("mundo.glb");
    let mut world = Vec::new();
    match models::load_textured(&world_path) {
        Ok(parts) => {
            for p in parts {
                if p.mesh.vertices.is_empty() {
                    continue;
                }
                let mesh = upload(renderer, device, &p.mesh)?;
                let tex = match &p.albedo {
                    Some(a) => Some(
                        renderer
                            .upload_texture_with_normal(device, queue, a, p.normal.as_ref())
                            .map_err(|e| anyhow::anyhow!("{e:?}"))?,
                    ),
                    None => None,
                };
                let glow = p.emissive.map(|e| {
                    let m = e[0].max(e[1]).max(e[2]);
                    if p.material == "fire" {
                        3.0 * m
                    } else {
                        1.8 * m
                    }
                });
                world.push((mesh, tex, glow));
            }
        }
        Err(e) => eprintln!("[guerra] {}: {e:#}", world_path.display()),
    }
    let anchors = if world.is_empty() {
        Vec::new()
    } else {
        world_anchors()
    };
    // the land's painted picture
    let land_tex = (|| -> Option<TextureHandle> {
        let path = exploration3d_state::data_dir()
            .join("guerra")
            .join("tierra.jpg");
        let img = velvet_avif::open_image(path.as_ref()).ok()?.to_rgba8();
        renderer
            .upload_texture(
                device,
                queue,
                &TextureData3D {
                    width: img.width(),
                    height: img.height(),
                    rgba: img.into_raw(),
                },
            )
            .ok()
    })();
    let land_tex_present = land_tex.is_some();
    let mut settlements = Vec::new();
    for (i, p) in wr::data::PROVINCES.iter().enumerate() {
        if !world.is_empty() {
            break;
        }
        if scene.prop(renderer, device, "campaign", p.settlement) {
            let (sx, sz) = p.site();
            let y = wr::campaign_map::land_height(sx, sz).max(wr::campaign_map::WATER) - 0.1;
            settlements.push((i, Mat4::from_translation(Vec3::new(sx, y, sz))));
        }
    }
    let mut base = |f: Faction| -> Result<(MeshHandle, MeshHandle)> {
        let fi = f.info();
        let mut b = wr::prims::Builder::default();
        b.add(
            &wr::prims::cylinder(2.4, 2.75, 0.7, 20),
            wr::data::rgb(fi.dark),
        );
        let mut r = wr::prims::Builder::default();
        r.add(
            &wr::prims::cylinder(2.78, 2.78, 0.2, 20),
            wr::data::rgb(fi.color),
        );
        Ok((
            renderer
                .upload_mesh(device, &b.build().expect("a base"))
                .map_err(|e| anyhow::anyhow!("{e:?}"))?,
            renderer
                .upload_mesh(device, &r.build().expect("a rim"))
                .map_err(|e| anyhow::anyhow!("{e:?}"))?,
        ))
    };
    let bases = [
        base(Faction::Reina)?,
        base(Faction::Alianza)?,
        base(Faction::Rebeldes)?,
        base(Faction::Libres)?,
    ];
    let roads = wr::campaign_map::roads(&land);
    let clouds = wr::campaign_map::clouds();
    let c = CampScene {
        land: upload(
            renderer,
            device,
            &if land_tex.is_some() {
                land.mesh_textured()
            } else {
                land.mesh(false)
            },
        )?,
        land_tex,
        sea: upload(renderer, device, &wr::campaign_map::sea())?,
        water_tex: {
            let size = 256usize;
            let normal = TextureData3D {
                width: size as u32,
                height: size as u32,
                rgba: wr::campaign_map::wave_normals(size, 7),
            };
            let white = TextureData3D {
                width: size as u32,
                height: size as u32,
                rgba: vec![255u8; size * size * 4],
            };
            renderer
                .upload_texture_with_normal(device, queue, &white, Some(&normal))
                .ok()
        },
        borders: border_lines(),
        border_mesh: None,
        border_key: None,
        overlay: upload(renderer, device, &land.mesh(true))?,
        overlay_tex,
        painted: None,
        table: upload(renderer, device, &wr::campaign_map::table(&land))?,
        water: upload(
            renderer,
            device,
            &if land_tex_present {
                wr::campaign_map::water_web(&land)
            } else {
                wr::campaign_map::water(&land)
            },
        )?,
        roads: roads.map(|r| upload(renderer, device, &r)).transpose()?,
        woods: wr::campaign_map::woods(&land),
        settlements,
        world,
        anchors,
        bases,
        cloud_alpha: vec![0.0; clouds.len()],
        clouds,
        preview,
        provinces,
    };
    eprintln!(
        "[guerra] the campaign's map ready in {} ms",
        t0.elapsed().as_millis()
    );
    Ok(c)
}

fn puff(dust: &mut Particles, p: Vec3, n: u32, size: f32, strength: f32) {
    for _ in 0..n {
        let a = dust.rand() * std::f32::consts::TAU;
        let s = (0.4 + dust.rand() * 1.2) * strength;
        let up = 0.5 + dust.rand() * 0.8;
        let ttl = 1.2 + dust.rand();
        dust.spawn(
            p + Vec3::new(a.cos() * 0.5, 0.3, a.sin() * 0.5),
            Vec3::new(a.cos() * s, up, a.sin() * s),
            ttl,
            size * 0.6,
            size * 2.0,
            [0.74, 0.64, 0.54, 0.2],
            -0.2,
            1.2,
        );
    }
}

fn spawn_fx(scene: &mut WarScene, field: &wr::field::Field, fx: Fx) {
    match fx {
        Fx::Puff {
            x,
            y,
            z,
            n,
            size,
            strength,
        } => puff(&mut scene.dust, Vec3::new(x, y, z), n, size, strength),
        Fx::Sparks { x, y, z, n, color } => {
            for _ in 0..n {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let s = 2.0 + scene.glow.rand() * 4.0;
                let up = 2.0 + scene.glow.rand() * 3.0;
                let ttl = 0.25 + scene.glow.rand() * 0.25;
                scene.glow.spawn(
                    Vec3::new(x, y, z),
                    Vec3::new(a.cos() * s, up, a.sin() * s),
                    ttl,
                    0.35,
                    0.08,
                    [color[0], color[1], color[2], 1.0],
                    12.0,
                    2.0,
                );
            }
        }
        Fx::Spark { x, z, r } => {
            let y = field.walk_height(x, z);
            scene.bursts.push(Burst {
                at: Vec3::new(x, y, z),
                radius: r,
                t: 0.0,
                color: None,
            });
            scene.glow.spawn(
                Vec3::new(x, y + 2.0, z),
                Vec3::ZERO,
                0.5,
                18.0,
                40.0,
                [0.9, 0.7, 1.0, 0.3],
                0.0,
                0.0,
            );
            scene.glow.spawn(
                Vec3::new(x, y + 1.0, z),
                Vec3::ZERO,
                0.9,
                8.0,
                22.0,
                [1.0, 0.8, 0.45, 0.28],
                0.0,
                0.0,
            );
            for _ in 0..240 {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let d = scene.glow.rand() * r;
                let violet = scene.glow.rand() < 0.55;
                let up = 3.0 + scene.glow.rand() * 6.0;
                let ttl = 1.2 + scene.glow.rand() * 1.2;
                let c = if violet {
                    [0.7, 0.45, 1.0, 1.0]
                } else {
                    [1.0, 0.72, 0.3, 1.0]
                };
                scene.glow.spawn(
                    Vec3::new(x + a.cos() * d, y + 0.5, z + a.sin() * d),
                    Vec3::new(a.cos() * 2.0, up, a.sin() * 2.0),
                    ttl,
                    0.9,
                    0.2,
                    c,
                    -0.5,
                    0.6,
                );
            }
        }
        Fx::Ring { x, z, r, color } => {
            let y = field.walk_height(x, z);
            scene.bursts.push(Burst {
                at: Vec3::new(x, y, z),
                radius: r,
                t: 0.0,
                color: Some(color),
            });
            for _ in 0..80 {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let d = r * (0.85 + scene.glow.rand() * 0.15);
                let up = 1.0 + scene.glow.rand() * 3.0;
                let ttl = 0.9 + scene.glow.rand() * 0.6;
                scene.glow.spawn(
                    Vec3::new(x + a.cos() * d, y + 0.4, z + a.sin() * d),
                    Vec3::new(0.0, up, 0.0),
                    ttl,
                    0.8,
                    0.2,
                    [color[0], color[1], color[2], 1.0],
                    -0.4,
                    0.6,
                );
            }
        }
        Fx::Ember { x, z } => {
            let y = field.walk_height(x, z);
            // its streak down from the sky, the blow, the fire and the dust
            for k in 0..8 {
                let h = 6.0 + k as f32 * 2.2;
                scene.glow.spawn(
                    Vec3::new(x + h * 0.25, y + h, z - h * 0.12),
                    Vec3::new(-10.0, -40.0, 5.0),
                    0.3,
                    1.4 - k as f32 * 0.1,
                    0.4,
                    [1.0, 0.55 + k as f32 * 0.03, 0.2, 1.0],
                    0.0,
                    0.0,
                );
            }
            for _ in 0..24 {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let s = 3.0 + scene.glow.rand() * 6.0;
                let up = 3.0 + scene.glow.rand() * 5.0;
                let ttl = 0.5 + scene.glow.rand() * 0.5;
                scene.glow.spawn(
                    Vec3::new(x, y + 0.5, z),
                    Vec3::new(a.cos() * s, up, a.sin() * s),
                    ttl,
                    0.6,
                    0.1,
                    [1.0, 0.6, 0.2, 1.0],
                    12.0,
                    2.0,
                );
            }
            puff(&mut scene.dust, Vec3::new(x, y, z), 4, 2.6, 2.4);
            scene.bursts.push(Burst {
                at: Vec3::new(x, y, z),
                radius: 4.0,
                t: 0.4,
                color: Some([1.0, 0.45, 0.15]),
            });
        }
        Fx::Pillar { x, z, color } => {
            let y = field.walk_height(x, z);
            for _ in 0..90 {
                let a = scene.glow.rand() * std::f32::consts::TAU;
                let d = scene.glow.rand() * 2.5;
                let h = scene.glow.rand() * 2.0;
                let up = 4.0 + scene.glow.rand() * 8.0;
                let ttl = 1.4 + scene.glow.rand();
                scene.glow.spawn(
                    Vec3::new(x + a.cos() * d, y + h, z + a.sin() * d),
                    Vec3::new(0.0, up, 0.0),
                    ttl,
                    1.0,
                    0.3,
                    [color[0], color[1], color[2], 1.0],
                    -1.0,
                    0.3,
                );
            }
        }
    }
}

fn draw_particles(scene: &mut WarScene, camera: Camera3D) {
    let mut out: Vec<(Key, CrowdInstance3D)> = Vec::new();
    for (key, list) in [(Key::Dust, &scene.dust.list), (Key::Glow, &scene.glow.list)] {
        for q in list {
            let t = q.life / q.ttl.max(0.001);
            let size = q.s0 + (q.s1 - q.s0) * t;
            let a = q.c[3]
                * if t < 0.15 {
                    t / 0.15
                } else {
                    1.0 - (t - 0.15) / 0.85
                };
            let c = [q.c[0], q.c[1], q.c[2]];
            let mut i = facing(camera, q.p, size, c, a);
            if key == Key::Glow {
                // added light: dimmer is fainter
                i.color = [c[0] * a, c[1] * a, c[2] * a, 1.0];
                i.fade = 0.0;
            }
            out.push((key, i));
        }
    }
    for (k, i) in out {
        scene.push(k, i);
    }
}
