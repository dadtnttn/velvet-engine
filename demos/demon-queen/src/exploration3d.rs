//! Authored 3D scene, embedded in the VN's window and GPU device.
//! The small standalone preview host reuses this scene for world editing.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Deserialize;
use velvet_math::{Mat4, Vec3};
use velvet_render::{
    Camera3D, DayNightCycle3D, EnvironmentFrame3D, EnvironmentPreset3D, Finish3D, GltfModel,
    IsoCamera3D, IsoSettings3D, Material3D, Mesh3D, MeshDraw3D, MeshHandle, MeshVertex3D,
    PointLight3D, TextureData3D, TextureHandle, Transform3D, WaterLife3D, Wgpu3DRenderer,
};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::exploration3d_state;
use crate::window_display::{GpuContext, WindowDisplay};
pub(crate) use demon_queen_dungeon as dungeon;
#[path = "overlay/map_overlay.rs"]
pub(crate) mod map_overlay;
use dungeon::prompt_icons::{Icon, Prompt};
#[path = "achievements.rs"]
pub(crate) mod achievements;
#[path = "baths.rs"]
pub(crate) mod baths;
#[path = "brothel.rs"]
pub(crate) mod brothel;
#[path = "castle3d.rs"]
pub(crate) mod castle3d;
#[path = "guion.rs"]
pub(crate) mod guion;
#[path = "missions.rs"]
pub(crate) mod missions;
#[path = "penascal3d.rs"]
pub(crate) mod penascal3d;
#[path = "shop.rs"]
mod shop;
#[path = "toon_capture.rs"]
mod toon_capture;
#[path = "varrak_fight.rs"]
pub(crate) mod varrak_fight;
#[path = "venue.rs"]
pub(crate) mod venue;
use velvet_nav as nav;
#[path = "backpack.rs"]
pub(crate) mod backpack;
#[path = "castle.rs"]
mod castle;
#[path = "clip_video.rs"]
pub(crate) mod clip_video;
#[path = "dark_forest.rs"]
mod dark_forest;
#[path = "elixir.rs"]
pub(crate) mod elixir;
#[path = "ero.rs"]
pub(crate) mod ero;
#[path = "gait.rs"]
pub(crate) mod gait;
#[path = "gallery_seen.rs"]
pub(crate) mod gallery_seen;
#[path = "game_audio.rs"]
mod game_audio;
#[path = "mine.rs"]
mod mine;
#[path = "mugo.rs"]
pub(crate) mod mugo;
#[path = "prefetch.rs"]
pub(crate) mod prefetch;
#[path = "prefs.rs"]
pub(crate) mod prefs;
#[path = "qa_bot.rs"]
pub(crate) mod qa_bot;
#[path = "retarget.rs"]
pub(crate) mod retarget;
#[path = "sign_textures.rs"]
pub(crate) mod sign_textures;
#[path = "soundtrack.rs"]
pub(crate) mod soundtrack;
#[path = "spa_life.rs"]
pub(crate) mod spa_life;
#[path = "ui_sfx.rs"]
pub(crate) mod ui_sfx;
#[path = "village.rs"]
mod village;
#[path = "explore/walk.rs"]
mod walk;
#[path = "water_life.rs"]
mod water_life;
use walk::*;
#[path = "zones/forest.rs"]
mod zone_forest;
use zone_forest::*;
#[path = "zones/village.rs"]
mod zone_village;
use zone_village::*;
#[path = "zones/dungeon.rs"]
mod zone_dungeon;
use zone_dungeon::*;
#[path = "explore/input.rs"]
mod input;
#[path = "zones/zone.rs"]
mod zone;
use zone::*;
#[path = "zones/castle_life.rs"]
mod castle_life;
#[path = "zones/castle.rs"]
mod zone_castle;
#[path = "zones/outdoors.rs"]
mod zone_outdoors;
use zone_castle::*;
#[path = "explore/camera.rs"]
mod camera;
#[path = "zones/baths.rs"]
mod zone_baths;
#[path = "zones/brothel.rs"]
mod zone_brothel;
#[path = "zones/penascal.rs"]
mod zone_penascal;
use zone_penascal::penascal_return;
#[path = "explore/act_one.rs"]
mod act_one;
#[path = "explore/guide.rs"]
mod guide;
#[path = "zones/war_render.rs"]
mod war_render;
#[path = "zones/war_story.rs"]
mod war_story;
#[path = "zones/boss.rs"]
mod zone_boss;
#[path = "zones/war.rs"]
mod zone_war;
use camera::*;
use zone_boss::{boss_camera, BossInput};
#[path = "explore/queen.rs"]
mod queen;
use queen::*;
#[path = "explore/hud.rs"]
mod hud;
use hud::*;
#[path = "explore/assets.rs"]
mod assets;
use assets::*;
#[path = "explore/save.rs"]
mod save;
use save::*;
#[path = "explore/alcove.rs"]
mod alcove;
#[path = "explore/audio.rs"]
mod audio;
#[path = "explore/bag.rs"]
mod bag;
#[path = "explore/bed_scene.rs"]
mod bed_scene;
#[path = "explore/caricias.rs"]
mod caricias;
#[path = "explore/court.rs"]
mod court;
#[path = "explore/court_days.rs"]
mod court_days;
#[path = "explore/dates.rs"]
pub(crate) mod dates;
#[path = "explore/deuda.rs"]
mod deuda;
#[path = "explore/divan.rs"]
mod divan;
#[path = "explore/drink.rs"]
mod drink;
#[path = "explore/floats.rs"]
mod floats;
#[path = "explore/mage.rs"]
mod mage;
#[path = "explore/mascot.rs"]
pub(crate) mod mascot;
#[path = "explore/play_scene.rs"]
mod play_scene;
#[path = "explore/rules.rs"]
mod rules;
#[path = "explore/shop_games.rs"]
mod shop_games;
#[path = "explore/shop_ui.rs"]
mod shop_ui;
#[path = "explore/stage.rs"]
mod stage;
#[path = "explore/talk.rs"]
mod talk;
#[path = "explore/world_tags.rs"]
pub(crate) mod world_tags;

/// Compact item data exposed to the pause-menu renderer. The shop remains
/// data-driven; this view gives the menu catalog labels and quantities
/// without duplicating item definitions in another UI module.
#[derive(Clone, Debug)]
pub(crate) struct InventoryMenuItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) icon: String,
    pub(crate) icon_svg: Option<String>,
    pub(crate) quantity: u32,
    /// Lower-case shop category ("consumables", "materials", ...).
    pub(crate) category: String,
    pub(crate) effect: String,
}

/// Inventory key of the relic sword found in the treasuries.
const RELIC_SWORD_ID: &str = "espada_reliquia";

const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;
// The replacement Mixamo/Reyna asset is authored at half the world scale of
// the original Queen and mage assets. Keep its feet on the same ground while
// matching the existing character silhouette in both outdoor and room scenes.
// Static hair cards sit a little behind the animated head after the separate
// hair asset is anchored. The exported GLB keeps Mixamo's forward in local
// +Y, so add a small model-space nudge toward the face while keeping
// vertex-based tracking.
// Small world-down contact correction: the authored cards sit just above the
// scalp after their forward alignment. Keep it tiny so ears and horns remain
// clear while the hair base visibly touches the head.
// Fit the static card shell to the actual animated head footprint. Keep
// vertical length nearly unchanged so the side/back locks remain full.
// Footprints are authored in world units, so they need their own calibration
// after the Queen was doubled.  The old uniform scale produced prints about
// 0.33 x 0.63 while the current boot sole is about 0.18 x 0.22.  Keep the
// tread shape, but fit its width/length independently to the actual sole.
// Fine camera increments keep wall joints inspectable without forcing a full
// quarter-turn every time Z/X is pressed.
// 35.26439° in radians (to_radians is not const on MSRV 1.80).
// Keep a slight tilt: Camera3D clamps orbit elevation below the exact pole,
// and the remaining tilt prevents an unstable look-at matrix.
// Imported castle tiles peak at Y=0.05. Keep the selection ring just above
// that surface so depth testing does not bury it inside the floor.

// The village gate sits on the final stone before the preview plaza.  Keep
// these values in one place so the visual marker, interaction radius and
// collision envelope cannot drift apart while the layout is iterated.
// Where the villa's plaza stands in the world (the villa is its own scene,
// reached through the trail's gate; its plan is in metres, at the Queen's scale).
// Outdoor side of the same gate.  The Queen arrives here after leaving the
// castle and can use the same F action to return inside.
// The mage is a world interaction, not a map waypoint.  Keep the prompt
// readable on the clearing approach and use the same usable radius as the
// other F interactions so the keycap never lies about reachability.

// Outdoor clearing staging point shown in the reference screenshot. Keep the
// mage beside the small pebble landmark at (-2.5, 3.3), outside the trail so
// the interaction anchor and the visual placement stay together.

// Maids stay in authored castle rooms. Movement uses a small local navigation
// grid rebuilt when target/obstacles change. A* handles furniture, walls,
// closed doors, the player, and the other maid; steering only follows its
// resulting path. Doors are deliberate zone boundaries for NPC patrols.

// The schedule is deliberately data-only: adding another NPC later means
// adding its actor/position and lines, not changing the bubble renderer.

/// Gameplay bounds for the nature clearing.  They are deliberately smaller
/// than the visual meadow: the player hits an invisible wall while the camera
/// still sees a continuous world of trees and undergrowth beyond it.
// The clearing now opens into a small preview hamlet at the far end of the
// trail.  Keep the invisible boundary beyond the houses so the player can
// reach the plaza while the surrounding forest still reads as endless.

/// What one frame of the world is drawn from: the camera, the Queen, the hour, what is open over
/// the world and the state of the place she is in.
#[derive(Clone, Copy)]
struct Frame<'a> {
    /// The place she is in.
    zone: ZoneId,
    camera: Camera3D,
    player: Vec3,
    yaw: f32,
    moving: bool,
    running: bool,
    elapsed: f32,
    footprints: &'a [Footprint],
    queen_thought: Option<(&'static str, f32)>,
    nature_mode: bool,
    village_entered: bool,
    dungeon_run: Option<&'a dungeon::Dungeon>,
    map_open: bool,
    wardrobe_open: bool,
    wardrobe_selection: u8,
    outfit: u8,
    shop_open: bool,
    shop_tab: shop::Tab,
    shop_category: shop::Category,
    shop_selection: usize,
    shop_quantity: u32,
    gold: u32,
    inventory: &'a BTreeMap<String, u32>,
    shop_stock: &'a BTreeMap<String, u32>,
    shop_notice: Option<&'a str>,
    shop_pointer: Option<(f32, f32)>,
    shop_preview_rotation: Option<(f32, f32)>,
    venue: &'a venue::Menu,
    environment: EnvironmentFrame3D,
    action_prompt: Option<&'a Prompt>,
    brothel_state: Option<&'a brothel::State>,
    baths_state: Option<&'a baths::State>,
    castle_state: Option<&'a castle3d::State>,
    penascal_state: Option<&'a penascal3d::State>,
    /// Varrak has woken (he kneels by the market stone).
    varrak_awake: bool,
    /// The followed mission's guide (its arrows and light in the world).
    mission: Option<&'a guide::Guide>,
    /// The fight with Varrak, in first person.
    boss: Option<&'a varrak_fight::Fight>,
    /// The war table, a battle or the campaign.
    war: Option<&'a zone_war::WarSession>,
    war_campaign: Option<&'a demon_queen_war::rules::Campaign>,
    /// She is drinking Rosalía's elixir: seconds into her clip.
    drink: Option<f32>,
    /// She lies in her bed (the alcove's menu).
    lying: bool,
}

pub(crate) struct RenderState {
    target: wgpu::Texture,
    pub(crate) target_view: wgpu::TextureView,
    target_format: wgpu::TextureFormat,
    /// The world drawn smaller than it is shown (the render scale), or with finishing passes
    /// (FXAA, sharpening): its own picture, brought to `target` by `upscaler` before the overlays.
    scene: Option<(wgpu::Texture, wgpu::TextureView)>,
    upscaler: velvet_render::Upscaler3D,
    /// The graphics settings' finishing passes and render scale (1: as shown).
    finishing: velvet_render::Finishing3D,
    render_scale: f32,
    /// Dynamic resolution: the frame time it keeps under (ms), its smoothed measure, and the
    /// scale it has come to (within `render_scale`).
    dynamic_budget: Option<f32>,
    dynamic_ms: f32,
    dynamic_scale: f32,
    last_render: Option<std::time::Instant>,
    /// The size the world is drawn at (the screen's own, or 1920×1080); its overlays are laid
    /// out at 1920×1080 all the same.
    pub(crate) render_size: (u32, u32),
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Wgpu3DRenderer,
    model: GltfModel,
    green_model: Option<GltfModel>,
    mage_model: GltfModel,
    maid_model: GltfModel,
    maid_alt_model: Option<GltfModel>,
    last_skin_sample: Option<(usize, u32)>,
    last_green_skin_sample: Option<(usize, u32)>,
    last_mage_skin_sample: Option<(usize, u32, usize, u32, u32)>,
    last_maid_skin_sample: Option<(usize, u32)>,
    last_maid_alt_skin_sample: Option<(usize, u32)>,
    queen_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    green_queen_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    green_hair_mesh_index: Option<usize>,
    green_hair_base_vertices: Vec<MeshVertex3D>,
    green_hair_anchor_indices: Vec<usize>,
    green_head_indices: Vec<usize>,
    mage_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    maid_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    maid_alt_meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    walk_animation: Option<usize>,
    /// Sitting back and lying down (borrowed clips), for each outfit.
    relax: retarget::RelaxClips,
    green_relax: retarget::RelaxClips,
    /// Outfits with their own model (past the two built in).
    extra_outfits: Vec<ExtraOutfit>,
    run_animation: Option<usize>,
    idle_animation: Option<usize>,
    green_walk_animation: Option<usize>,
    green_run_animation: Option<usize>,
    green_idle_animation: Option<usize>,
    mage_dismissing_animation: Option<usize>,
    mage_petting_animation: Option<usize>,
    maid_walk_animation: Option<usize>,
    maid_alt_walk_animation: Option<usize>,
    /// Nyxara in the infirmary.
    doctor: Option<DoctorAssets>,
    dungeon_scene: dungeon::Scene,
    nature_footprint: MeshHandle,
    player_marker: MeshHandle,
    nature_mote: MeshHandle,
    celestial_orb: MeshHandle,
    village_entry_marker: MeshHandle,
    /// The villa's interaction medallion.
    nature_draws: Vec<MeshDraw3D>,
    /// The Dark Forest's lantern glass and glowing things among `nature_draws`, the pieces that
    /// can hide her, and the campfire's puff (empty unless the exported wood is in use).
    forest_lamps: Vec<usize>,
    forest_glows: Vec<usize>,
    forest_bounds: Vec<Option<(Vec3, Vec3)>>,
    forest_puff: Option<MeshHandle>,
    /// The wood's air: pools of light, flecks, mist, fireflies.
    forest_air: Option<dark_forest::Air>,
    /// The simulated water and what disturbs it.
    water_life: WaterLife3D,
    fantasy_village_draws: Vec<MeshDraw3D>,
    fantasy_village_bounds: Vec<Option<(Vec3, Vec3)>>,
    /// Window panes and lanterns among those draws, lit with the night.
    fantasy_village_windows: Vec<(usize, f32)>,
    fantasy_village_lamps: Vec<usize>,
    fantasy_village_smoke: Vec<Vec3>,
    fantasy_village_steam: Vec<(Vec3, f32)>,
    fantasy_village_puff: Option<MeshHandle>,
    fantasy_village_mill: Vec<MeshDraw3D>,
    fantasy_village_mill_frame: Mat4,
    fantasy_village_mill_speed: f32,
    fantasy_village_load_attempted: bool,
    /// The brothel's interior, loaded the first time she walks in.
    brothel_scene: Option<brothel::Scene>,
    /// The flask that goes in her hand when she drinks the elixir.
    elixir_bottle: Vec<elixir::BottlePart>,
    brothel_load_attempted: bool,
    /// The hot springs, loaded the first time she walks in.
    baths_scene: Option<baths::Scene>,
    baths_load_attempted: bool,
    /// The castle from its prototype, loaded the first time she walks in.
    castle_scene: Option<castle3d::Scene>,
    /// The court's life in the castle's rooms (built the first time the castle is drawn).
    castle_life: Option<castle_life::Scene>,
    castle_life_tried: bool,
    castle_load_attempted: bool,
    /// El Peñascal, loaded the first time she climbs to it.
    penascal_scene: Option<penascal3d::Scene>,
    penascal_load_attempted: bool,
    /// The mission guide's arrows and column of light (made the first time they show).
    guide_meshes: Option<guide::GuideMeshes>,
    /// The fight with Varrak, loaded the first time she challenges him.
    boss_scene: Option<zone_boss::BossScene>,
    boss_load_attempted: bool,
    war_scene: Option<war_render::WarScene>,
    /// The battle whose minimap the overlay holds.
    war_minimap_serial: u64,
    map_overlay: map_overlay::MapOverlay,
}

impl Drop for RenderState {
    /// Lets the GPU finish everything it was given before the device goes: otherwise, with the GPU
    /// busy (another program rendering), dropping the queue waits with a time limit and panics
    /// "timed out while waiting on the last successful submission" on the way out.
    fn drop(&mut self) {
        let _ = self.device.poll(wgpu::Maintain::Wait);
    }
}

/// The world's frame at `size`.
fn make_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("exploration-frame"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

impl RenderState {
    /// The graphics settings: the renderer's quality, the finishing passes, the render scale and
    /// dynamic resolution (`budget_ms`: the frame time it keeps under; `None` off).
    pub(crate) fn set_graphics(
        &mut self,
        quality: velvet_render::Quality3D,
        finishing: velvet_render::Finishing3D,
        render_scale: f32,
        budget_ms: Option<f32>,
    ) {
        if self.renderer.quality() != quality {
            self.renderer.set_quality(quality);
        }
        self.finishing = finishing;
        self.render_scale = render_scale.clamp(0.33, 1.0);
        if budget_ms != self.dynamic_budget {
            self.dynamic_budget = budget_ms;
            self.dynamic_scale = 1.0;
            self.dynamic_ms = 0.0;
        }
    }

    /// Where this frame's world is drawn and at what size: straight into the shown target when
    /// nothing is to be done to it, else into the scene picture (smaller with a render scale).
    pub(crate) fn scene_target(&mut self) -> (wgpu::TextureView, (u32, u32)) {
        // Dynamic resolution: the time since the last frame, smoothed, steers the scale.
        let now = std::time::Instant::now();
        if let (Some(budget), Some(last)) = (self.dynamic_budget, self.last_render) {
            let ms = now.duration_since(last).as_secs_f32() * 1000.0;
            if ms < 250.0 {
                self.dynamic_ms = if self.dynamic_ms == 0.0 {
                    ms
                } else {
                    self.dynamic_ms * 0.9 + ms * 0.1
                };
                if self.dynamic_ms > budget * 1.06 {
                    self.dynamic_scale = (self.dynamic_scale - 0.02).max(0.5);
                } else if self.dynamic_ms < budget * 0.8 {
                    self.dynamic_scale = (self.dynamic_scale + 0.01).min(1.0);
                }
            }
        }
        self.last_render = Some(now);
        let scale = self.render_scale
            * if self.dynamic_budget.is_some() {
                self.dynamic_scale
            } else {
                1.0
            };
        let shown = self.render_size;
        let drawn = (
            ((shown.0 as f32 * scale).round() as u32).max(320),
            ((shown.1 as f32 * scale).round() as u32).max(180),
        );
        let drawn = (drawn.0.min(shown.0), drawn.1.min(shown.1));
        if !self.finishing.needed(drawn, shown) {
            self.scene = None;
            return (self.target_view.clone(), shown);
        }
        let fits = self
            .scene
            .as_ref()
            .is_some_and(|(t, _)| (t.width(), t.height()) == drawn);
        if !fits {
            let texture = make_target(&self.device, self.target_format, drawn);
            let view = texture.create_view(&Default::default());
            self.scene = Some((texture, view));
        }
        let view = self.scene.as_ref().expect("made above").1.clone();
        (view, drawn)
    }

    /// The world, drawn into `scene` (at `drawn`), brought to the shown target.
    pub(crate) fn finish_scene(&mut self, scene: &wgpu::TextureView, drawn: (u32, u32)) {
        if self.scene.is_none() {
            return;
        }
        let shown = self.render_size;
        let target = self.target_view.clone();
        self.upscaler.run(
            &self.device,
            &self.queue,
            scene,
            drawn,
            &target,
            shown,
            self.finishing,
        );
    }

    /// The world drawn at `size` from now on (the frame made again when it changes).
    pub(crate) fn set_render_size(&mut self, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if size == self.render_size {
            return;
        }
        self.target = make_target(&self.device, self.target_format, size);
        self.target_view = self.target.create_view(&Default::default());
        self.render_size = size;
        eprintln!("[3d] render size {}x{}", size.0, size.1);
    }

    pub(crate) fn load(gpu: GpuContext, model_file: &Path, dungeon_seed: u64) -> Result<Self> {
        // A new renderer: the signs it uploads are noted afresh.
        sign_textures::forget_uploads();
        let GpuContext {
            device,
            queue,
            format,
            msaa_samples,
        } = gpu;
        let (width, height) = (WINDOW_WIDTH, WINDOW_HEIGHT);
        let target = make_target(&device, format, (width, height));
        let target_view = target.create_view(&Default::default());
        eprintln!("[3d] MSAA samples={msaa_samples}");
        let mut renderer =
            Wgpu3DRenderer::new_with_msaa(&device, &queue, format, (width, height), msaa_samples);
        renderer.set_shading_style(velvet_render::ShadingStyle3D::Toon);
        let mut model_texture_cache = HashMap::new();
        let white_texture = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        };
        let QueenAssets {
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
        } = load_queen(
            &mut renderer,
            &device,
            &queue,
            model_file,
            &mut model_texture_cache,
            &white_texture,
        )?;
        let elixir_bottle = elixir::load_bottle(
            &mut renderer,
            &device,
            &queue,
            &mut model_texture_cache,
            &white_texture,
        );
        let MageAssets {
            mage_model,
            mage_dismissing_animation,
            mage_petting_animation,
            mage_meshes,
        } = load_mage(
            &mut renderer,
            &device,
            &queue,
            &mut model_texture_cache,
            &white_texture,
        )?;
        let doctor = load_doctor(
            &mut renderer,
            &device,
            &queue,
            &mut model_texture_cache,
            &white_texture,
        );
        let MaidAssets {
            maid_model,
            maid_walk_animation,
            maid_meshes,
            maid_alt_model,
            maid_alt_meshes,
            maid_alt_walk_animation,
        } = load_maids(
            &mut renderer,
            &device,
            &queue,
            &mut model_texture_cache,
            &white_texture,
        )?;
        // The characters' pictures are on the GPU now: their decoded copies are not kept in RAM
        // (only their animation and skin data is read from here on).
        let (mut model, mut green_model, mut mage_model, mut maid_model, mut maid_alt_model) =
            (model, green_model, mage_model, maid_model, maid_alt_model);
        let mut freed = 0usize;
        for m in [
            Some(&mut model),
            green_model.as_mut(),
            Some(&mut mage_model),
            Some(&mut maid_model),
            maid_alt_model.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            freed += m.textures.iter().map(|t| t.rgba.len()).sum::<usize>();
            m.textures = Vec::new();
        }
        eprintln!("[3d] character textures freed from RAM: {} MB", freed >> 20);
        let player_marker = renderer.upload_mesh(
            &device,
            &Mesh3D::ring(
                0.62,
                0.48,
                48,
                Material3D {
                    base_color: [0.42, 0.12, 0.82, 0.9],
                    texture: None,
                    occlusion_window: [0.0; 4],
                },
            ),
        )?;
        // Tiny warm motes add a little life to the clearing. They share the
        // normal lit path so they remain inexpensive and still pick up the
        // green ambient bounce.
        let nature_mote = renderer.upload_mesh(
            &device,
            &Mesh3D::cube(0.075, Material3D::rgb(1.20, 0.78, 0.18)),
        )?;
        // A single reusable low-poly sphere is instanced as the sun or moon.
        // Its position and tint come from the environment frame, so changing
        // the preset or clock never rebuilds the world meshes.
        let celestial_orb = renderer.upload_mesh(
            &device,
            &Mesh3D::uv_sphere(0.90, 20, 10, Material3D::rgb(1.0, 1.0, 1.0)),
        )?;
        let village_entry_marker = renderer.upload_mesh(
            &device,
            &Mesh3D::ring(
                0.98,
                0.82,
                40,
                Material3D {
                    base_color: [1.0, 0.22, 0.025, 0.86],
                    texture: None,
                    occlusion_window: [0.0; 4],
                },
            ),
        )?;
        // Floating medallions show each interaction's own icon in the scene;
        // the on-screen plate names the key and the action.
        let dungeon_scene = dungeon::Scene::new(&mut renderer, &device, &queue, format)?;
        eprintln!("[3d] dungeon v2 ready, seed {dungeon_seed}; geometry streams on entry");
        let nature_footprint = renderer.upload_mesh(&device, &build_nature_footprint_mesh())?;
        let ForestAssets {
            water_life,
            forest_air,
            nature_draws,
            forest_lamps,
            forest_glows,
            forest_bounds,
            forest_puff,
        } = load_forest(&mut renderer, &device, &queue)?;
        // Keep the heavy settlement out of the initial GPU upload. The player
        // normally starts in the castle/forest; the GLB is uploaded only when
        // F enters the village, avoiding a needless device-memory spike.
        eprintln!(
            "[3d] Fantasy Village deferred until entry (pack={})",
            fantasy_village_asset_root().display()
        );
        let map_overlay = map_overlay::MapOverlay::new(&device, &queue, format);
        Ok(Self {
            target,
            target_view,
            target_format: format,
            scene: None,
            upscaler: velvet_render::Upscaler3D::new(&device, format),
            finishing: velvet_render::Finishing3D::default(),
            render_scale: 1.0,
            dynamic_budget: None,
            dynamic_ms: 0.0,
            dynamic_scale: 1.0,
            last_render: None,
            render_size: (width, height),
            device,
            queue,
            renderer,
            model,
            green_model,
            mage_model,
            maid_model,
            maid_alt_model,
            last_skin_sample: None,
            last_green_skin_sample: None,
            last_mage_skin_sample: None,
            last_maid_skin_sample: None,
            last_maid_alt_skin_sample: None,
            queen_meshes,
            green_queen_meshes,
            green_hair_mesh_index,
            green_hair_base_vertices,
            green_hair_anchor_indices,
            green_head_indices,
            mage_meshes,
            maid_meshes,
            maid_alt_meshes,
            walk_animation,
            run_animation,
            idle_animation,
            relax,
            green_relax,
            extra_outfits,
            green_walk_animation,
            green_run_animation,
            green_idle_animation,
            mage_dismissing_animation,
            mage_petting_animation,
            maid_walk_animation,
            maid_alt_walk_animation,
            doctor,
            dungeon_scene,
            nature_footprint,
            player_marker,
            nature_mote,
            celestial_orb,
            village_entry_marker,
            nature_draws,
            forest_lamps,
            forest_glows,
            forest_bounds,
            forest_puff,
            forest_air,
            water_life,
            fantasy_village_draws: Vec::new(),
            fantasy_village_bounds: Vec::new(),
            fantasy_village_windows: Vec::new(),
            fantasy_village_lamps: Vec::new(),
            fantasy_village_smoke: Vec::new(),
            fantasy_village_steam: Vec::new(),
            fantasy_village_puff: None,
            fantasy_village_mill: Vec::new(),
            fantasy_village_mill_frame: Mat4::IDENTITY,
            fantasy_village_mill_speed: 0.0,
            fantasy_village_load_attempted: false,
            brothel_scene: None,
            elixir_bottle,
            brothel_load_attempted: false,
            baths_scene: None,
            baths_load_attempted: false,
            castle_scene: None,
            castle_life: None,
            castle_life_tried: false,
            castle_load_attempted: false,
            penascal_scene: None,
            penascal_load_attempted: false,
            guide_meshes: None,
            boss_scene: None,
            boss_load_attempted: false,
            war_scene: None,
            war_minimap_serial: 0,
            map_overlay,
        })
    }

    // Full per-frame context in one call; grouping would churn all call
    // sites for no runtime gain.
    #[allow(clippy::too_many_arguments)]
    fn render(&mut self, frame: &Frame) {
        zone(frame.zone).render(self, frame);
    }

    /// What every place above ground does before it draws: the villa loads when she walks
    /// in, the water simulates, the springs move, and she and the people around her are
    /// posed. The walk's bob and whether the green dress is on.
    fn prepare_above_ground(&mut self, frame: &Frame) -> (f32, bool) {
        let Frame {
            player,
            elapsed,
            nature_mode,
            village_entered,
            brothel_state,
            baths_state,
            ..
        } = *frame;
        if nature_mode && village_entered {
            self.ensure_fantasy_village_loaded();
        }
        let forest_water =
            nature_mode && !village_entered && brothel_state.is_none() && baths_state.is_none();
        let villa_water =
            nature_mode && village_entered && brothel_state.is_none() && baths_state.is_none();
        let water_shown = if villa_water {
            Some(water_life::VILLA)
        } else if forest_water {
            Some(water_life::FOREST)
        } else {
            None
        };
        self.water_life
            .frame(&mut self.renderer, elapsed, player, water_shown);
        if baths_state.is_some() {
            self.ensure_baths_loaded();
        }
        if let Some(scene) = self.baths_scene.as_mut() {
            scene.tick(baths_state, &mut self.renderer, &self.queue, elapsed);
        }
        self.pose_characters(frame)
    }
}

pub(crate) struct App {
    model_file: PathBuf,
    world_id: String,
    spawn_id: String,
    session_path: PathBuf,
    next_autosave: Instant,
    window: Option<Arc<Window>>,
    state: Option<RenderState>,
    display: Option<WindowDisplay>,
    embedded: bool,
    pending_story_request: bool,
    /// Stable villa-building ID waiting for the VN layer.  Keeping this
    /// separate from the mage story request lets every house get an `F`
    /// affordance before its individual script is authored.
    pending_villa_interaction: Option<String>,
    /// Act 1 is over: the host shows its thanks (taken once).
    thanks_pending: bool,
    /// The portraits' gallery asked for (the host opens it over the world).
    gallery_pending: bool,
    /// Sound, opened on first need (the first descent); `audio_tried`
    /// stops retrying when there is no sound device.
    audio: Option<game_audio::GameAudio>,
    audio_tried: bool,
    /// The sounds being made on another thread while the world loads in.
    audio_boot: Option<std::thread::JoinHandle<game_audio::Synth>>,
    movement_smoke: bool,
    seen_interactions: BTreeSet<String>,
    /// What she has done and earned (saved with the session).
    achievements: achievements::Progress,
    /// Where the QA walker found her at the start (the castle's spawn).
    qa_start: Vec3,
    /// Zones whose files are being read ahead (she is near their door).
    prefetched: BTreeSet<&'static str>,
    /// The achievements screen is open (L).
    achievements_open: bool,
    /// Shown instead of the real content (the capture's preview of a fuller catalog).
    achievements_preview: Option<map_overlay::AchievementsView>,
    /// "Achievement unlocked" notices waiting to show, and when the current one ends.
    achievement_toasts: VecDeque<Prompt>,
    achievement_toast_until: f32,
    last_frame: Instant,
    next_frame_at: Instant,
    /// The last frames handed to the GPU (at most two are let wait in its queue).
    frames_in_flight: std::collections::VecDeque<wgpu::SubmissionIndex>,
    /// A camera that replaces hers (the maps are baked from above).
    pub(crate) camera_override: Option<Camera3D>,
    elapsed: f32,
    player: Vec3,
    dungeon: dungeon::Dungeon,
    dungeon_mode: bool,
    dungeon_mouse_captured: bool,
    dungeon_return: Option<DungeonReturnState>,
    dungeon_pitch: f32,
    camera_focus: Vec3,
    player_yaw: f32,
    iso: IsoCamera3D,
    move_forward: bool,
    move_back: bool,
    move_left: bool,
    move_right: bool,
    sprint: bool,
    nature_mode: bool,
    village_entered: bool,
    map_open: bool,
    wardrobe_open: bool,
    wardrobe_selection: u8,
    outfit: u8,
    /// What she came to the springs in, waiting in the changing room while she wears the bikini.
    outfit_before_baths: Option<u8>,
    shop_open: bool,
    venue: venue::Menu,
    /// Inside the brothel: where she stands there and what has opened.
    brothel: Option<brothel::State>,
    /// At the hot springs: in the valley or one of its rooms.
    baths: Option<baths::State>,
    /// Inside the castle (its prototype's rooms): where she is, which walls are down, the maids.
    castle: Option<castle3d::State>,
    /// In el Peñascal: where she is, which cliffs are down.
    penascal: Option<penascal3d::State>,
    /// Varrak's arrival is playing: the fight starts when it ends.
    pending_boss_fight: bool,
    /// The fight started from the R button before the story reached him: winning it changes
    /// nothing.
    boss_practice: bool,
    /// The missions' notices; the followed one's guide and the way it last found.
    missions: missions::Tracker,
    guide: Option<guide::Guide>,
    route: Option<guide::Route>,
    /// The fight with Varrak (over el Peñascal while it lasts), and the mouse's doings in it.
    boss: Option<varrak_fight::Fight>,
    boss_input: BossInput,
    /// The war table open (its page, a battle, the campaign).
    war: Option<zone_war::WarSession>,
    /// The campaign under way (it rides in the saves).
    war_campaign: Option<demon_queen_war::rules::Campaign>,
    /// Men per regiment in the war's battles.
    war_scale: f32,
    /// The story's variables and seen scenes (saved; shared with the visual novel).
    story: guion::Vars,
    /// A conversation playing in the dialogue box.
    talk: Option<guion::Conversation>,
    /// The story as it was when that conversation began: what is saved while it plays, so a
    /// conversation cut short (the game closed halfway) plays again instead of counting as seen.
    talk_story_before: Option<guion::Vars>,
    /// A guion scene on the full-screen stage (its backdrop, cast and flourishes).
    stage: Option<stage::Stage>,
    /// The stage's clip (`@clip`) and its newest frame, waiting for the GPU.
    stage_clip: Option<stage::StageClip>,
    stage_frame: Option<image::RgbaImage>,
    /// The option picked in a choice.
    talk_choice: usize,
    /// The bed scene playing in the middle of the screen (its minigame).
    bed_scene: Option<bed_scene::BedScene>,
    /// A brothel girl's minigame playing on the same stage (Alba's words, Noa's silence).
    play_scene: Option<play_scene::PlayScene>,
    /// Rosalía's elixir going down (its close shot).
    drink: Option<drink::Drink>,
    /// Chiquita's comments floating on the screen.
    floats: Vec<floats::Float>,
    /// In her own bed, its menu open.
    alcove: Option<alcove::Alcove>,
    /// The court: audiences, works, lessons, training (`court.rs`).
    court: Option<court::Court>,
    /// A date on the stage: the dates' page comes back after it, with what it brought.
    date_after: Option<dates::DateAfter>,
    /// What each of the court told this morning, and whether its page is still to be seen.
    court_report: Vec<court_days::Report>,
    court_morning_pending: bool,
    /// A letter from Maren came this morning.
    court_letter: bool,
    /// A night's touch game (`caricias.rs`), its clip's last frame to upload, its clip's sound.
    caricias: Option<caricias::Caricias>,
    caricias_frame: Option<image::RgbaImage>,
    caricias_sound: Option<std::path::PathBuf>,
    /// The hypnotist's stall in the wood, while it is open.
    mage: Option<mage::MageMenu>,
    /// Chiquita out on the screen in 3D.
    mascot: mascot::Mascot,
    /// When the line on show began (it types itself out from then).
    talk_line_at: f32,
    /// A guion scene to open in the visual novel (taken by the host).
    pending_guion_vn: Option<String>,
    /// Rosalía has already lifted the rope this session.
    brothel_rope_open: bool,
    shop_tab: shop::Tab,
    shop_category: shop::Category,
    shop_selection: usize,
    shop_quantity: u32,
    shop_stock: BTreeMap<String, u32>,
    shop_notice: Option<String>,
    shop_preview_rotation: Option<(f32, f32)>,
    shop_preview_dragging: bool,
    shop_preview_drag_anchor: Option<(f32, f32)>,
    /// The shop's page beyond the shelves (a haggle, the fusion altar) and when it and the shop
    /// opened (their entrances), the shopkeeper's last word.
    shop_page: shop_games::Page,
    shop_page_at: f32,
    shop_opened_at: f32,
    shop_said: Option<(String, f32)>,
    haggle: Option<shop_games::Haggle>,
    /// Products he will not haggle over again this visit.
    shop_refused: Vec<usize>,
    fuse: shop_games::Fuse,
    cursor_position: Option<(f32, f32)>,
    gold: u32,
    inventory: BTreeMap<String, u32>,
    /// Where each item lies in the grid backpack, and what is worn.
    backpack: backpack::Backpack,
    /// A line the Queen thinks aloud over her head, until the given time.
    queen_thought: Option<(&'static str, f32)>,
    environment_cycle: DayNightCycle3D,
    environment_cycle_enabled: bool,
    footprints: Vec<Footprint>,
    footstep_distance: f32,
    footstep_side: f32,
    /// Distance walked since the last audible step.
    step_sound_distance: f32,
    fps_elapsed: f32,
    fps_frames: u32,
    fps_display: f32,
}

impl App {
    fn new(model_file: PathBuf, world_id: String, spawn_id: String, session_path: PathBuf) -> Self {
        Self::new_seeded(model_file, world_id, spawn_id, session_path, None)
    }

    /// As `new`, a fresh dungeon coming from `seed` when one is given.
    fn new_seeded(
        model_file: PathBuf,
        world_id: String,
        spawn_id: String,
        session_path: PathBuf,
        seed: Option<u64>,
    ) -> Self {
        let Start {
            dungeon,
            dungeon_resume,
            player,
            player_yaw,
            nature_mode,
            village_entered,
            outfit,
            seen_interactions,
            achievement_progress,
            story,
            gold,
            inventory,
            backpack,
            shop_stock,
            venue,
            inside,
            brothel_rope_open,
            war: war_saved,
            war_scale,
        } = starting_point(&world_id, &spawn_id, &session_path, seed);
        let mut app = Self {
            model_file,
            world_id,
            spawn_id,
            session_path,
            next_autosave: Instant::now() + Duration::from_secs(1),
            window: None,
            state: None,
            display: None,
            embedded: false,
            pending_story_request: false,
            pending_villa_interaction: None,
            thanks_pending: false,
            gallery_pending: false,
            audio: None,
            audio_tried: false,
            audio_boot: None,
            movement_smoke: false,
            seen_interactions,
            achievements: achievement_progress,
            qa_start: player,
            prefetched: BTreeSet::new(),
            achievement_toasts: VecDeque::new(),
            achievements_open: false,
            achievements_preview: None,
            achievement_toast_until: 0.0,
            last_frame: Instant::now(),
            next_frame_at: Instant::now(),
            frames_in_flight: std::collections::VecDeque::new(),
            camera_override: None,
            elapsed: 0.0,
            player,
            dungeon,
            dungeon_mode: false,
            dungeon_mouse_captured: false,
            dungeon_return: None,
            dungeon_pitch: 0.0,
            camera_focus: player,
            player_yaw,
            // Start at the maximum authored height so the whole room reads
            // immediately. Q/E still provides the full close inspection range.
            iso: IsoCamera3D::new(ISO, std::f32::consts::FRAC_PI_4),
            move_forward: false,
            move_back: false,
            move_left: false,
            move_right: false,
            sprint: false,
            nature_mode,
            village_entered,
            map_open: false,
            wardrobe_open: false,
            wardrobe_selection: outfit,
            outfit,
            outfit_before_baths: None,
            shop_open: false,
            venue,
            brothel: None,
            baths: None,
            castle: None,
            penascal: None,
            pending_boss_fight: false,
            boss_practice: false,
            missions: missions::Tracker::default(),
            guide: None,
            route: None,
            boss: None,
            boss_input: BossInput::default(),
            war: None,
            war_campaign: war_saved,
            war_scale,
            story,
            talk: None,
            talk_story_before: None,
            bed_scene: None,
            play_scene: None,
            drink: None,
            floats: Vec::new(),
            alcove: None,
            court: None,
            date_after: None,
            court_report: Vec::new(),
            court_morning_pending: false,
            court_letter: false,
            caricias: None,
            caricias_frame: None,
            caricias_sound: None,
            mage: None,
            mascot: mascot::Mascot::default(),
            talk_choice: 0,
            talk_line_at: 0.0,
            stage: None,
            stage_clip: None,
            stage_frame: None,
            pending_guion_vn: None,
            brothel_rope_open,
            shop_tab: shop::Tab::Buy,
            shop_category: shop::Category::All,
            shop_selection: 0,
            shop_quantity: 1,
            shop_stock,
            shop_notice: None,
            shop_preview_rotation: None,
            shop_preview_dragging: false,
            shop_preview_drag_anchor: None,
            shop_page: shop_games::Page::List,
            shop_page_at: 0.0,
            shop_opened_at: 0.0,
            shop_said: None,
            haggle: None,
            shop_refused: Vec::new(),
            fuse: shop_games::Fuse::default(),
            cursor_position: None,
            gold,
            inventory,
            backpack,
            queen_thought: None,
            // One clock drives both the royal room and nature test.  B swaps
            // the palette; C pauses/resumes time without touching assets.
            // Keep the prototype day long enough to see daylight, blue hour,
            // sunset, night, and sunrise as a continuous sequence.
            environment_cycle: DayNightCycle3D::new(EnvironmentPreset3D::Lived, 420.0),
            environment_cycle_enabled: true,
            footprints: Vec::with_capacity(28),
            footstep_distance: 0.0,
            footstep_side: 1.0,
            step_sound_distance: 0.0,
            fps_elapsed: 0.0,
            fps_frames: 0,
            fps_display: 60.0,
        };
        if dungeon_resume {
            let controller = app.dungeon.controller.clone();
            app.enter_dungeon_unchecked();
            app.dungeon.controller = controller;
            app.player = app.dungeon.controller.position;
            app.camera_focus = app.player;
        } else if let Some(inside) = inside {
            app.resume_inside(&inside);
        }
        app
    }

    /// The world for a visual novel's way in; a fresh dungeon from `seed` when given (the seed a
    /// background load was made for, see `WinitApp::preload_world`).
    pub(crate) fn for_entry(
        world: String,
        spawn: String,
        session: PathBuf,
        window: Arc<Window>,
        seed: Option<u64>,
    ) -> Self {
        let mut app = Self::new_seeded(model_path(), world, spawn, session, seed);
        app.embedded = true;
        app.window = Some(window);
        app
    }

    /// The outfit the Queen wears: the wardrobe's slot (0 the royal dress, 1 the green, 2 the bikini…).
    pub(crate) fn outfit(&self) -> u8 {
        self.outfit
    }

    pub(crate) fn prepare(gpu: GpuContext, dungeon_seed: u64) -> Result<RenderState> {
        RenderState::load(gpu, &model_path(), dungeon_seed)
    }

    pub(crate) fn attach(&mut self, window: Arc<Window>, state: RenderState) {
        self.state = Some(state);
        self.window = Some(window);
        if self.dungeon_mode && !self.modal_is_open() {
            self.dungeon_mouse_captured = self.capture_dungeon_mouse();
        }
        self.last_frame = Instant::now();
        self.next_frame_at = self.last_frame;
        // QA: VELVET_WAR_TABLE opens the war table at once (the full game's way in).
        if std::env::var_os("VELVET_WAR_TABLE").is_some() {
            self.open_war_table();
        }
    }

    pub(crate) fn texture_view(&self) -> Option<&wgpu::TextureView> {
        self.state.as_ref().map(|state| &state.target_view)
    }

    /// Waits until the GPU has finished everything submitted (for frame timing in QA).
    pub(crate) fn wait_for_gpu(&self) {
        if let Some(state) = self.state.as_ref() {
            let _ = state.device.poll(wgpu::Maintain::Wait);
        }
    }

    pub(crate) fn capture(&self, path: &Path) -> Result<()> {
        // As drawn: 1920×1080, or the size `VELVET_RENDER_SIZE` asks for.
        let (pixels, w, h) = self.capture_frame()?;
        image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)?;
        Ok(())
    }

    /// The world's frame as drawn (RGBA, and its size): the frozen world under a menu.
    pub(crate) fn capture_frame(&self) -> Result<(Vec<u8>, u32, u32)> {
        let state = self.state.as_ref().context("3D not loaded")?;
        let pixels = velvet_render::read_texture_rgba(&state.device, &state.queue, &state.target)?;
        anyhow::ensure!(!velvet_render::frame_is_blank(&pixels), "3D frame is blank");
        Ok((pixels, state.render_size.0, state.render_size.1))
    }

    /// Readback only when opening the pause menu; the live world is retained and frozen.
    pub(crate) fn capture_pixels(&self) -> Result<Vec<u8>> {
        let state = self.state.as_ref().context("3D not loaded")?;
        let pixels = velvet_render::read_texture_rgba(&state.device, &state.queue, &state.target)?;
        anyhow::ensure!(!velvet_render::frame_is_blank(&pixels), "3D frame is blank");
        if state.render_size == (WINDOW_WIDTH, WINDOW_HEIGHT) {
            return Ok(pixels);
        }
        // Drawn at the screen's size: brought to the logical frame the callers expect.
        let (w, h) = state.render_size;
        let image = image::RgbaImage::from_raw(w, h, pixels).context("3D frame size")?;
        Ok(image::imageops::resize(
            &image,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            image::imageops::FilterType::Triangle,
        )
        .into_raw())
    }

    pub(crate) fn clear_input(&mut self) {
        self.dungeon.jump_requested = false;
        self.dungeon.auto_walk = false;
        self.move_forward = false;
        self.move_back = false;
        self.move_left = false;
        self.move_right = false;
        self.sprint = false;
        // Nothing held in the fight either (a seal held through a pause stayed on).
        self.boss_input = BossInput::default();
        if let Some(w) = self.war.as_mut() {
            w.keys = Default::default();
            w.press = None;
        }
        self.shop_preview_dragging = false;
        self.shop_preview_drag_anchor = None;
        self.last_frame = Instant::now();
    }

    /// Hide world overlays before the host captures the frozen background for
    /// the global ESC menu. Otherwise the villa map (and its labels) remains
    /// visible behind the backpack panel.
    pub(crate) fn close_overlays_for_pause(&mut self) {
        self.bed_scene_interrupt();
        if let Some(audio) = &mut self.audio {
            audio.set_paused(true);
        }
        self.map_open = false;
        if self.shop_open {
            self.close_shop();
        }
        self.wardrobe_open = false;
        self.clear_input();
        if self.dungeon_mode || self.boss.is_some() {
            self.release_dungeon_mouse();
        }
    }

    /// Esc (or Back) shuts an open map before it opens the pause menu.
    /// The host's "Menú · ESC" button steps aside while a panel with its own Close is up
    /// (a modal, or the full map on M with its "ESC Cerrar").
    pub(crate) fn covers_menu_button(&self) -> bool {
        // (the war has its own controls in that corner)
        self.modal_is_open() || (self.map_open && !self.dungeon_mode) || self.war.is_some()
    }

    pub(crate) fn close_map_if_open(&mut self) -> bool {
        if self.map_open && !self.dungeon_mode {
            self.map_open = false;
            true
        } else {
            false
        }
    }

    /// The right button is the world's own (the fight's seal), not the menu's «back».
    pub(crate) fn owns_right_button(&self) -> bool {
        self.boss.is_some() || self.war.is_some()
    }

    pub(crate) fn resume_after_pause(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.set_paused(false);
        }
        self.clear_input();
        // (not back to looking about at the Tendero's counter)
        if (self.dungeon_mode && !self.modal_is_open()) || self.boss.is_some() {
            self.dungeon_mouse_captured = self.capture_dungeon_mouse();
        }
    }

    pub(crate) fn modal_is_open(&self) -> bool {
        self.war_modal()
            || self.wardrobe_open
            || self.shop_open
            || self.achievements_open
            || self.bed_scene_open()
            || self.drink.is_some()
            || self.alcove.is_some()
            || self.court.is_some()
            || self.caricias.is_some()
            || self.mage.is_some()
            || self.brothel.as_ref().is_some_and(|b| b.escort.is_some())
            || (self.dungeon_mode
                && (self.dungeon.seed_input.is_some()
                    || self.dungeon.map_open
                    || self.dungeon.stall.open))
    }

    /// Consumed by the host once the player presses `F` beside a villa
    /// building.  IDs map to the authored footprints in `village.rs` and can
    /// be connected to `open_world_vn` when their scripts are ready.
    /// Act 1 has just ended: the host puts up its thanks.
    pub(crate) fn take_thanks(&mut self) -> bool {
        std::mem::take(&mut self.thanks_pending)
    }

    /// The castle's portraits were asked for: the host puts up the gallery over the world.
    pub(crate) fn take_gallery(&mut self) -> bool {
        std::mem::take(&mut self.gallery_pending)
    }

    /// The language changed: the signs on show drawn again in it.
    pub(crate) fn relocalize_signs(&mut self) {
        if let Some(state) = self.state.as_mut() {
            state.relocalize_signs();
        }
    }

    pub(crate) fn take_villa_interaction(&mut self) -> Option<String> {
        self.pending_villa_interaction.take()
    }

    /// QA (the smoke's zone round): castle → wood → village → wood → castle, one hop per call.
    pub(crate) fn qa_next_zone(&mut self) -> &'static str {
        if !self.nature_mode {
            self.enter_nature_from_castle();
            "forest"
        } else if !self.village_entered {
            let (spawn, _) = village::spawn();
            self.player = spawn;
            self.camera_focus = spawn;
            self.village_entered = true;
            eprintln!("[3d] entered Fantasy Village");
            "village"
        } else {
            // Out of the village by the wood's own way back: to the castle.
            self.enter_castle_from_nature();
            "castle"
        }
    }

    pub(crate) fn set_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => self.move_forward = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.move_back = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.move_left = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.move_right = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.sprint = pressed,
            _ => {}
        }
    }

    fn tick(&mut self, dt: f32) {
        let zone = self.zone();
        if zone.id() != ZoneId::Dungeon {
            self.tick_world(dt);
        }
        zone.tick(self, dt);
        self.tick_act_one();
        self.tick_missions();
        self.tick_guide();
        self.tick_alcove();
        self.tick_court();
        self.tick_caricias(dt);
        self.tick_mage();
        self.tick_shop();
        self.tick_bed_scene(dt);
        self.tick_drink();
        self.tick_floats();
        self.tick_stage();
        self.tick_mascot(dt);
    }

    /// What every place above ground shares each step: the soundtrack and ambience, the
    /// hour, the camera settling and the footprints fading.
    fn tick_world(&mut self, dt: f32) {
        // Above ground: the soundtrack, the place's own ambience, and the cave's
        // air fading out.
        self.update_audio_above_ground(dt);
        if self.environment_cycle_enabled {
            self.advance_world_clock(dt);
        }

        self.iso.ease(dt);

        for footprint in &mut self.footprints {
            footprint.age += dt;
        }
        self.footprints.retain(|footprint| footprint.age < 12.0);
    }

    /// Shared by the native host and deterministic, save-free visual QA.
    /// What F does where the Queen stands, for the on-screen prompt: `None`
    /// when nothing is in reach or a menu is open.
    /// What F does where the Queen stands: each place has its own icon,
    /// colour, verb and name.
    fn interaction_prompt(&self) -> Option<Prompt> {
        if self.dungeon_mode
            || self.map_open
            || self.shop_open
            || self.wardrobe_open
            || self.venue.open
            || self.talk.is_some()
            || self.stage.is_some()
            || self.bed_scene_open()
            || self.drink.is_some()
            || self.alcove.is_some()
            || self.court.is_some()
            || self.caricias.is_some()
            || self.mage.is_some()
            || self.brothel.as_ref().is_some_and(|b| b.escort.is_some())
        {
            return None;
        }
        self.zone().prompt(self)
    }

    fn render_frame(&mut self) {
        // The world at the size the window shows it (or 1920×1080, as the settings say).
        if let (Some(window), Some(state)) = (self.window.as_ref(), self.state.as_mut()) {
            let inner = window.inner_size();
            state.set_render_size(prefs::render_size((inner.width, inner.height)));
        } else if let (Some(size), Some(state)) = (prefs::forced_render_size(), self.state.as_mut())
        {
            state.set_render_size(size);
        }
        // The graphics settings (QA: `VELVET_GFX_PRESET=0..3` tries a preset).
        if let Some(state) = self.state.as_mut() {
            let g = std::env::var("VELVET_GFX_PRESET")
                .ok()
                .and_then(|v| v.parse::<u8>().ok())
                .map_or_else(prefs::graphics, prefs::Graphics::preset);
            state.set_graphics(
                g.quality(),
                g.finishing(),
                g.render_scale as f32 / 100.0,
                g.budget_ms(),
            );
        }
        // Places to go in are noted afresh each frame by the zone that shows them; any other
        // place shows none (the last ones would stay up over the castle otherwise).
        let _ = hud::take_world_markers_raw();
        if let Some(state) = self.state.as_mut() {
            state.map_overlay.set_markers(Vec::new());
        }
        let achievements_view = self.achievements_open.then(|| {
            self.achievements_preview
                .clone()
                .unwrap_or_else(|| self.achievements_view())
        });
        let wardrobe_view = self.wardrobe_view();
        let scene = self.bed_scene_view();
        let dialogue_view = if scene.is_some() {
            None
        } else {
            self.dialogue_view()
        };
        let drink_view = self.drink_view();
        let floats_view = self.floats_view();
        let alcove_view = self.alcove_view();
        let court_view = self.court_view();
        let caricias_view = self.caricias_view();
        let caricias_frame = self
            .caricias_frame
            .take()
            .or_else(|| self.stage_frame.take());
        let mage_view = self.mage_view();
        let shop_view = self.shop_view();
        let mascot_view = self.mascot_view();
        let stage_view = self.stage_view();
        if let Some(state) = self.state.as_mut() {
            state.map_overlay.set_stage(stage_view);
            state.map_overlay.set_achievements(achievements_view);
            state.map_overlay.set_wardrobe(wardrobe_view);
            state
                .map_overlay
                .set_dialogue_age((self.elapsed - self.talk_line_at).max(0.0));
            state.map_overlay.set_dialogue(dialogue_view);
            state.map_overlay.set_drink(drink_view);
            state.map_overlay.set_floats(floats_view);
            state.map_overlay.set_alcove(alcove_view);
            state.map_overlay.set_court(court_view);
            state.map_overlay.set_caricias(caricias_view);
            if let Some(frame) = caricias_frame {
                state.map_overlay.upload_scene_frame(
                    &state.device,
                    &state.queue,
                    frame.width(),
                    frame.height(),
                    frame.as_raw(),
                );
            }
            state.map_overlay.set_mage(mage_view);
            state.map_overlay.set_shop(shop_view);
            state.map_overlay.set_mascot(mascot_view);
            match scene {
                Some((view, frame)) => {
                    if let Some(frame) = frame {
                        state.map_overlay.upload_scene_frame(
                            &state.device,
                            &state.queue,
                            frame.width(),
                            frame.height(),
                            frame.as_raw(),
                        );
                    }
                    state.map_overlay.set_scene(Some(view));
                }
                None => state.map_overlay.set_scene(None),
            }
        }
        // An achievement's notice takes the prompt's place for a few seconds.
        if self.elapsed >= self.achievement_toast_until && !self.achievement_toasts.is_empty() {
            if self.achievement_toast_until > 0.0 {
                self.achievement_toasts.pop_front();
            }
            self.achievement_toast_until = if self.achievement_toasts.is_empty() {
                0.0
            } else {
                self.elapsed + 4.0
            };
        }
        let action_prompt = match self.achievement_toasts.front() {
            Some(notice) if self.achievement_toast_until > 0.0 => Some(notice.clone()),
            _ => self.interaction_prompt(),
        };
        // Her own steps, or those of the walk up behind a girl of the brothel.
        let escorted = self
            .brothel
            .as_ref()
            .and_then(|b| b.escort.as_ref())
            .is_some_and(|e| e.queen_moving);
        let moving =
            self.move_forward || self.move_back || self.move_left || self.move_right || escorted;
        let camera = self.camera();
        // The shop reads the pointer for its hover; the maps for their buttons.
        let pointer = self.shop_pointer_position();
        let zone = self.zone_id();
        let drink = self.drink_clip_time();
        let lying = self.alcove_lying();
        let mission_view = self.mission_view(camera);
        let varrak_awake = self.varrak_awake();
        if let Some(state) = &mut self.state {
            state.map_overlay.set_missions(mission_view);
            state.render(&Frame {
                zone,
                camera,
                player: self.player,
                yaw: self.player_yaw,
                moving,
                running: moving && self.sprint,
                elapsed: self.elapsed,
                footprints: &self.footprints,
                queen_thought: self
                    .queen_thought
                    .as_ref()
                    .map(|(text, until)| (*text, ((*until - self.elapsed) / 0.5).clamp(0.0, 1.0))),
                nature_mode: self.nature_mode,
                village_entered: self.village_entered,
                dungeon_run: self.dungeon_mode.then_some(&self.dungeon),
                map_open: self.map_open,
                wardrobe_open: self.wardrobe_open,
                wardrobe_selection: self.wardrobe_selection,
                outfit: self.outfit,
                shop_open: self.shop_open,
                shop_tab: self.shop_tab,
                shop_category: self.shop_category,
                shop_selection: self.shop_selection,
                shop_quantity: self.shop_quantity,
                gold: self.gold,
                inventory: &self.inventory,
                shop_stock: &self.shop_stock,
                shop_notice: self.shop_notice.as_deref(),
                shop_pointer: pointer,
                shop_preview_rotation: self.shop_preview_rotation,
                venue: &self.venue,
                environment: self.environment_cycle.sample(),
                action_prompt: action_prompt.as_ref(),
                brothel_state: self.brothel.as_ref(),
                baths_state: self.baths.as_ref(),
                castle_state: self.castle.as_ref(),
                penascal_state: self.penascal.as_ref(),
                varrak_awake,
                mission: self
                    .guide
                    .as_ref()
                    .filter(|_| !matches!(zone, ZoneId::Dungeon | ZoneId::Boss)),
                boss: self.boss.as_ref(),
                war: self.war.as_ref(),
                war_campaign: self.war_campaign.as_ref(),
                drink,
                lying,
            });
            // The fight wants to know where his chest is (the spark goes there) and his head.
            if let (Some(scene), Some(fight)) = (state.boss_scene.as_ref(), self.boss.as_mut()) {
                fight.chest = Some(scene.chest);
                fight.over = Some(scene.over);
            }
            // At most two frames queued for the GPU. A window that is hidden or minimised no
            // longer blocks on its vsync, and nothing else stopped frames piling up in the queue
            // (with everything they hold: gigabytes in a minute).
            let marker = state.queue.submit(std::iter::empty());
            self.frames_in_flight.push_back(marker);
            while self.frames_in_flight.len() > 2 {
                if let Some(oldest) = self.frames_in_flight.pop_front() {
                    let _ = state
                        .device
                        .poll(wgpu::Maintain::WaitForSubmissionIndex(oldest));
                }
            }
            // Reclaim completed transient animation/map uploads without waiting.
            let _ = state.device.poll(wgpu::Maintain::Poll);
        }
    }
}

#[allow(dead_code)] // Used by the optional preview binary, not the VN host.
pub(crate) fn run_standalone() -> Result<()> {
    let model_file = model_path();
    let args: Vec<String> = std::env::args().collect();
    let argument_after = |flag: &str, fallback: &str| {
        args.iter()
            .position(|argument| argument == flag)
            .and_then(|index| args.get(index + 1))
            .filter(|value| !value.starts_with("--"))
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };
    let world = argument_after("--world", "act1_castle");
    if args.iter().any(|a| a == "--export-sounds") {
        let dir = argument_after("--export-sounds", "temp/sounds");
        let count = game_audio::export(Path::new(&dir))?;
        eprintln!("[audio] {count} sounds written to {dir}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--load-cycle") {
        // QA: load and release the whole 3D world repeatedly, as VN↔3D
        // trips do, to watch for memory that is never given back.
        let cycles = argument_after("--load-cycle", "6")
            .parse::<usize>()
            .unwrap_or(6);
        return toon_capture::load_cycle(cycles);
    }
    if args.iter().any(|a| a == "--capture-toon") {
        return toon_capture::capture(Path::new(&argument_after(
            "--capture-toon",
            "temp/toon-review/worlds",
        )));
    }
    let spawn = argument_after("--spawn", "bedroom_inside");
    let session_path = args
        .iter()
        .position(|argument| argument == "--session")
        .and_then(|index| args.get(index + 1))
        .filter(|value| !value.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(exploration3d_state::path);
    eprintln!("[3d] modelo: {}", model_file.display());
    eprintln!(
        "[3d] entrada: world={} spawn={} session={}",
        world,
        spawn,
        session_path.display()
    );
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(model_file, world, spawn, session_path);
    app.movement_smoke = args.iter().any(|arg| arg == "--movement-smoke");
    anyhow::ensure!(
        !app.movement_smoke || args.iter().any(|arg| arg == "--session"),
        "--movement-smoke requires an explicit --session test save"
    );
    event_loop.run_app(&mut app)?;
    exploration3d_state::background::flush();
    Ok(())
}

#[cfg(test)]
mod shop_integration_tests {
    use super::*;

    #[test]
    fn shop_filters_trade_the_displayed_item_and_preserve_the_save() {
        // No production saves, window, GPU or player position are touched.
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let session = std::env::temp_dir().join(format!(
            "demon-queen-shop-test-{}-{stamp}.json",
            std::process::id()
        ));
        let mut app = App::new(
            PathBuf::new(),
            "shop-test".into(),
            "bedroom_inside".into(),
            session.clone(),
        );
        app.open_shop();
        assert!(app.modal_is_open());
        app.change_shop_page(1);
        // (The stone sword on the first page pushed the ring over to the second.)
        assert_eq!(
            shop::catalog().products[app.selected_shop_product_index().unwrap()].id,
            "anillo_misterioso"
        );
        // (The hidden ones, like the cloth, are not on the shelves.)
        app.select_shop_category(shop::Category::Materials);
        assert_eq!(
            shop::catalog().products[app.selected_shop_product_index().unwrap()].id,
            "esencia_carmesi"
        );
        app.adjust_shop_quantity(1);
        app.commit_shop_transaction();
        assert_eq!(app.gold, 115);
        assert_eq!(app.inventory.get("esencia_carmesi"), Some(&2));
        app.toggle_shop_tab();
        assert_eq!(app.shop_quantity, 1);
        app.commit_shop_transaction();
        assert_eq!(app.gold, 147);
        assert_eq!(app.inventory.get("esencia_carmesi"), Some(&1));
        // Tab goes round the three: to sell, to the fusion altar, back to buying.
        app.toggle_shop_tab();
        assert_eq!(app.shop_page, shop_games::Page::Fuse);
        app.toggle_shop_tab();
        assert_eq!(app.shop_page, shop_games::Page::List);
        assert_eq!(app.shop_tab, shop::Tab::Buy);
        app.select_shop_category(shop::Category::Equipment);
        let sword = &shop::catalog().products[app.selected_shop_product_index().unwrap()];
        assert_eq!(sword.id, "espada_de_madera");
        app.shop_stock.insert(sword.id.clone(), 2);
        app.adjust_shop_quantity(98);
        assert_eq!(app.shop_quantity, 2);
        app.commit_shop_transaction();
        assert_eq!(app.gold, 67);
        assert_eq!(app.inventory.get("espada_de_madera"), Some(&2));
        app.commit_shop_transaction(); // Out of stock must never charge twice.
        assert_eq!(app.gold, 67);
        app.select_shop_category(shop::Category::Special);
        app.gold = 0;
        app.commit_shop_transaction();
        assert_eq!(app.gold, 0);
        assert!(!app.inventory.contains_key("anillo_misterioso"));
        let saved = exploration3d_state::load_from(&session).unwrap().unwrap();
        assert_eq!(saved.gold, 67);
        assert_eq!(saved.inventory.get("espada_de_madera"), Some(&2));
        app.close_shop();
        assert!(!app.modal_is_open());
        std::fs::remove_file(session).unwrap();
    }

    #[test]
    fn shop_picking_matches_letterbox_on_narrow_and_wide_windows() {
        for (w, h) in [(1920, 1080), (1280, 800), (2560, 1080), (800, 600)] {
            let viewport =
                velvet_render::SoftwareViewport::letterbox(w, h, WINDOW_WIDTH, WINDOW_HEIGHT)
                    .unwrap();
            let physical_x =
                viewport.x as f64 + 700.0 / WINDOW_WIDTH as f64 * viewport.width as f64;
            let physical_y =
                viewport.y as f64 + 287.0 / WINDOW_HEIGHT as f64 * viewport.height as f64;
            let (x, y) = viewport
                .physical_to_virtual(physical_x, physical_y)
                .unwrap();
            let pick = map_overlay::shop_pick(
                shop::Tab::Buy,
                shop::Category::All,
                0,
                &BTreeMap::new(),
                0,
                0,
                0,
            );
            assert_eq!(
                map_overlay::shop_hit_test(x, y, &pick),
                Some(map_overlay::ShopHit::Item(0))
            );
            if viewport.y > 0 {
                assert!(viewport.physical_to_virtual(10.0, 0.0).is_none());
            }
            if viewport.x > 0 {
                assert!(viewport.physical_to_virtual(0.0, 10.0).is_none());
            }
        }
    }
}

#[cfg(test)]
mod dungeon_mode_tests {
    use super::*;

    #[test]
    fn first_person_entry_and_return_preserve_the_surface_checkpoint() {
        let mut app = App::new(
            PathBuf::new(),
            "dungeon-test".into(),
            "bedroom_inside".into(),
            PathBuf::new(),
        );
        app.player = Vec3::new(-12.5, 0.0, 27.0);
        app.player_yaw = 0.34;
        app.nature_mode = true;
        app.village_entered = true;
        let surface = app.snapshot();

        app.enter_dungeon_unchecked();
        assert!(app.dungeon_mode);
        assert_eq!(app.snapshot().x, surface.x);
        assert_eq!(app.snapshot().z, surface.z);
        let camera = app.camera();
        assert_eq!(camera.orthographic_height, None);
        assert!((camera.position.y - app.dungeon.controller.position.y - 1.62).abs() < 0.001);
        // Entrance orientation is seed-dependent, not always world +X; the hand-made mine sets its own.
        let expected = if app.dungeon.authored.is_some() {
            let yaw = app.dungeon.spawn_yaw();
            Vec3::new(-yaw.sin(), 0.0, -yaw.cos())
        } else {
            velvet_dungeon::horizontal(
                app.dungeon.map.rooms[0].sockets[0].position - app.dungeon.spawn_position(),
            )
            .normalize_or_zero()
        };
        assert!(
            (camera.target - camera.position)
                .normalize_or_zero()
                .dot(expected)
                > 0.999
        );

        app.leave_dungeon();
        assert!(!app.dungeon_mode);
        assert_eq!(app.player, Vec3::new(-12.5, 0.0, 27.0));
        assert!(app.nature_mode && app.village_entered);
    }

    #[test]
    fn falling_to_a_champion_brings_her_up_into_its_scene() {
        let mut app = App::new(
            PathBuf::new(),
            "dungeon-defeat-test".into(),
            "bedroom_inside".into(),
            PathBuf::new(),
        );
        app.player = Vec3::new(-12.5, 0.0, 27.0);
        app.nature_mode = true;
        app.village_entered = true;
        app.enter_dungeon_unchecked();
        // (what the mine notes when she falls beside the smith: tested in the dungeon crate)
        app.dungeon.fallen_to = Some("herrero");
        app.tick_dungeon(1.0 / 60.0);
        assert!(!app.dungeon_mode, "carried up");
        assert_eq!(app.player, Vec3::new(-12.5, 0.0, 27.0));
        let lib = guion::library();
        assert_eq!(
            app.talk.as_ref().map(|t| t.scene_id(lib)),
            Some("mina.herrero")
        );
        // every champion has its scene, with its two choices
        for boss in [
            "herrero",
            "bibliotecaria",
            "senor",
            "centinela",
            "custodio",
            "guardian",
            "corazon",
        ] {
            for scene in [
                boss.to_owned(),
                format!("{boss}_zafarse"),
                format!("{boss}_vuelta"),
            ] {
                assert!(
                    lib.find(&format!("mina.{scene}"), None).is_some(),
                    "mina.{scene}"
                );
            }
        }
    }

    fn backpack_world(name: &str) -> App {
        App::new(
            PathBuf::new(),
            name.into(),
            "bedroom_inside".into(),
            PathBuf::new(),
        )
    }

    /// A walk through free 0.5 m cells from `from` to the cell nearest `to` (breadth first).
    fn villa_route(from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let cell = |p: Vec3| ((p.x / 0.5).round() as i32, (p.z / 0.5).round() as i32);
        let at = |c: (i32, i32)| Vec3::new(c.0 as f32 * 0.5, 0.0, c.1 as f32 * 0.5);
        let (start, goal) = (cell(from), cell(to));
        let mut came = std::collections::HashMap::from([(start, start)]);
        let mut queue = std::collections::VecDeque::from([start]);
        let mut best = start;
        while let Some(c) = queue.pop_front() {
            if (at(c) - to).length() < (at(best) - to).length() {
                best = c;
            }
            if c == goal {
                break;
            }
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (c.0 + di, c.1 + dj);
                if came.contains_key(&next) || village::blocked(at(next)) || came.len() > 400_000 {
                    continue;
                }
                came.insert(next, c);
                queue.push_back(next);
            }
        }
        let mut path = vec![to];
        let mut c = best;
        while c != start {
            path.push(at(c));
            c = came[&c];
        }
        path.reverse();
        Some(path)
    }

    /// Walks her along `path` with the keys, as the player would (W north, D east): false if
    /// she gets stuck.
    fn villa_walk(app: &mut App, path: &[Vec3]) -> bool {
        for waypoint in path {
            let mut steps = 0;
            loop {
                let d = *waypoint - app.player;
                if Vec3::new(d.x, 0.0, d.z).length() < 0.06 {
                    break;
                }
                app.move_forward = d.z < -0.03;
                app.move_back = d.z > 0.03;
                app.move_right = d.x > 0.03;
                app.move_left = d.x < -0.03;
                app.tick(1.0 / 60.0);
                steps += 1;
                if steps > 600 {
                    app.clear_input();
                    return false;
                }
            }
        }
        app.clear_input();
        true
    }

    #[test]
    fn the_brothel_opens_at_night_to_the_cloak_until_the_token() {
        let mut app = backpack_world("brothel-door");
        app.environment_cycle.set_phase(0.5);
        app.try_enter_brothel();
        assert!(app.brothel.is_none(), "not by day");
        assert_eq!(
            app.missions.shown.as_ref().map(|s| s.0),
            Some(missions::NOTE)
        );
        app.environment_cycle.set_phase(0.95);
        app.try_enter_brothel();
        assert!(app.brothel.is_none(), "not without the cloak");
        app.inventory.insert(rules::CAPE_ITEM.to_owned(), 1);
        app.on_bought(rules::CAPE_ITEM);
        app.try_enter_brothel();
        assert!(app.brothel.is_some(), "at night in the cloak");
        assert_eq!(Some(app.outfit), rules::outfit_slot("capa"));
        app.leave_brothel();
        // With Rosalía's token, any time of day and dressed as she likes.
        app.story.values.insert("burdel_bienvenida".to_owned(), 1);
        app.inventory.remove(rules::CAPE_ITEM);
        app.outfit = 0;
        app.environment_cycle.set_phase(0.5);
        app.try_enter_brothel();
        assert!(app.brothel.is_some());
        assert_eq!(app.outfit, 0);
    }

    #[test]
    fn the_mage_talks_to_the_green_dress_and_the_bikini_stays_at_the_springs() {
        let mut app = backpack_world("rules");
        app.outfit = 0;
        assert!(!app.mage_will_talk(), "he only stammers at the royal dress");
        app.outfit = 1;
        assert!(app.mage_will_talk());
        assert!(
            !app.mage_in_bikini(),
            "the green dress gets his stall, not the ring scene"
        );
        let bikini = rules::outfit_slot("bikini").expect("the bikini");
        app.outfit = bikini;
        assert!(
            app.mage_in_bikini(),
            "in the bikini he stammers until they have met"
        );
        assert!(app.talk.is_none());
        app.left_baths_in_bikini();
        assert_eq!(app.outfit, 0, "out of the springs too soon: dressed again");
        assert!(!app.wardrobe_allows(bikini));
        app.story.values.insert(ero::ORGASMOS.to_owned(), 4);
        app.outfit = bikini;
        app.left_baths_in_bikini();
        assert_eq!(app.outfit, bikini, "awake enough: she may");
        assert!(app.wardrobe_allows(bikini));
    }

    #[test]
    fn the_springs_changing_room_puts_the_bikini_on_and_gives_her_clothes_back() {
        let mut app = backpack_world("changing-room");
        app.story.values.insert("termas_bienvenida".to_owned(), 1);
        let bikini = rules::outfit_slot("bikini").expect("the bikini");
        app.outfit = 1;
        app.enter_baths();
        assert_eq!(app.outfit, 1, "not hers yet: the welcome gives it");
        app.leave_baths();
        app.achievements.grant_outfit("bikini");
        app.enter_baths();
        assert_eq!(app.outfit, bikini, "in the bikini at the springs");
        let saved = app.snapshot();
        assert_eq!(saved.inside.as_ref().and_then(|i| i.outfit_before), Some(1));
        app.leave_baths();
        assert_eq!(app.outfit, 1, "the green dress back on the way out");
        assert_eq!(
            app.story.get("termas_bikini_aviso"),
            1,
            "Chiquita says it once"
        );
        app.enter_baths();
        app.leave_baths();
        assert_eq!(app.outfit, 1);
    }

    #[test]
    fn the_thanks_comes_up_once_when_the_last_scene_of_act_one_has_played() {
        let mut app = backpack_world("thanks");
        app.tick_missions();
        assert!(!app.take_thanks(), "not before the act ends");
        app.story.values.insert("ajedrez_hecho".to_owned(), 1);
        app.tick_missions();
        assert!(app.take_thanks());
        assert_eq!(app.story.get(missions::THANKS_SHOWN), 1);
        app.tick_missions();
        assert!(!app.take_thanks(), "only once");
    }

    #[test]
    fn the_villa_is_walked_from_its_gate_to_every_door_and_back() {
        let mut app = backpack_world("villa-walk");
        app.nature_mode = true;
        app.village_entered = true;
        let (spawn, _) = village::spawn();
        app.player = spawn;
        app.iso.yaw_target = 0.0;
        app.iso.yaw = 0.0;
        for anchor in village::building_anchors() {
            let (door, out) = village::door_anchor(anchor.footprint_index).expect("a door");
            let target = door + out * 1.15;
            let path = villa_route(app.player, target).expect("a route");
            assert!(
                villa_walk(&mut app, &path),
                "stuck on the way to {}",
                anchor.id
            );
            let prompt = app
                .interaction_prompt()
                .unwrap_or_else(|| panic!("no prompt at {}", anchor.id));
            // Its name; a door with nothing behind it yet says so too.
            let expected = if village::in_development(anchor.id).is_some() {
                format!("{} · En desarrollo", anchor.label)
            } else {
                anchor.label.to_owned()
            };
            assert_eq!(
                prompt.detail.as_deref(),
                Some(expected.as_str()),
                "the prompt at {}",
                anchor.id
            );
        }
        // Back to the gate: F there leads out to the forest trail, clear of the threshold.
        let gate = village::entrance();
        let path = villa_route(app.player, gate).expect("a route to the gate");
        assert!(villa_walk(&mut app, &path), "stuck on the way to the gate");
        let prompt = app.interaction_prompt().expect("the gate's prompt");
        assert_eq!(prompt.verb, "Volver al bosque");
        app.leave_village_to_nature();
        assert!(!app.village_entered);
        assert!(
            village_entry_distance(app.player) > VILLAGE_INTERACT_RADIUS,
            "no instant way back in"
        );
    }

    #[test]
    fn escape_closes_an_open_map_before_anything_else_and_never_touches_the_dungeon_map() {
        let mut app = backpack_world("map-escape");
        assert!(!app.close_map_if_open(), "nothing to close");
        app.map_open = true;
        assert!(app.close_map_if_open());
        assert!(!app.map_open);
        assert!(
            !app.close_map_if_open(),
            "the second Esc is for the pause menu"
        );
        // In the caves the map belongs to the expedition, which handles its own keys.
        app.dungeon_mode = true;
        app.map_open = true;
        assert!(!app.close_map_if_open());
        assert!(app.map_open);
    }

    #[test]
    fn the_map_buttons_are_reachable_at_the_logical_canvas_size() {
        let close = map_overlay::map_hit_test(
            WINDOW_WIDTH as f32,
            WINDOW_HEIGHT as f32,
            true,
            1760.0,
            70.0,
        );
        assert_eq!(close, Some(map_overlay::MapHit::Close));
        let minimap = map_overlay::map_hit_test(
            WINDOW_WIDTH as f32,
            WINDOW_HEIGHT as f32,
            false,
            1700.0,
            300.0,
        );
        assert_eq!(minimap, Some(map_overlay::MapHit::Expand));
    }

    #[test]
    fn the_mine_needs_a_weapon_and_the_queen_says_so() {
        let mut app = backpack_world("mine-gate");
        assert!(!app.has_weapon());
        app.enter_dungeon();
        assert!(!app.dungeon_mode, "no weapon, no way down");
        assert_eq!(
            app.queen_thought.map(|t| t.0),
            Some("Necesito algo para defenderme primero.")
        );
        assert!(
            app.queen_thought.unwrap().1 > app.elapsed,
            "the thought lasts a moment"
        );
        app.inventory.insert("espada_de_madera".into(), 1);
        assert!(app.has_weapon());
        app.enter_dungeon();
        assert!(app.dungeon_mode);
    }

    #[test]
    fn the_relic_sword_or_an_equipped_weapon_also_opens_the_way() {
        let mut relic = backpack_world("mine-gate-relic");
        relic.inventory.insert(RELIC_SWORD_ID.into(), 1);
        assert!(relic.has_weapon());
        let mut worn = backpack_world("mine-gate-worn");
        worn.backpack.equipment.weapon = Some("espada_de_madera".into());
        assert!(worn.has_weapon());
    }

    #[test]
    fn buying_the_sword_wields_it_and_enables_the_mine() {
        let mut app = backpack_world("mine-gate-buy");
        app.gold = 245;
        app.shop_tab = shop::Tab::Buy;
        app.shop_category = shop::Category::Equipment;
        let visible = app.shop_product_indices();
        app.shop_selection = visible
            .iter()
            .position(|index| shop::catalog().products[*index].id == "espada_de_madera")
            .expect("the wooden sword is for sale");
        app.shop_quantity = 1;
        app.commit_shop_transaction();
        assert_eq!(app.inventory.get("espada_de_madera"), Some(&1));
        assert_eq!(
            app.backpack.equipment.weapon.as_deref(),
            Some("espada_de_madera")
        );
        assert!(app.has_weapon());
        assert_eq!(app.gold, 245 - 40);
    }

    #[test]
    fn a_guion_gift_lands_in_the_pack_and_is_wielded() {
        // Nemi's wooden sword in the first act: `@var objeto_espada_de_madera = 1`.
        let mut app = backpack_world("guion-gift");
        assert!(!app.has_weapon());
        app.story.values.insert("objeto_espada_de_madera".into(), 1);
        app.end_talk();
        assert_eq!(app.inventory.get("espada_de_madera"), Some(&1));
        assert_eq!(
            app.backpack.equipment.weapon.as_deref(),
            Some("espada_de_madera")
        );
        assert!(app.has_weapon());
        assert_eq!(app.story.get("objeto_espada_de_madera"), 0, "given once");
        app.end_talk();
        assert_eq!(
            app.inventory.get("espada_de_madera"),
            Some(&1),
            "and only once"
        );
    }

    #[test]
    fn the_menu_backpack_follows_the_live_counts_and_edits_commit_back() {
        let mut app = backpack_world("backpack-counts");
        app.inventory.insert("hierbas_curativas".into(), 2);
        app.inventory.insert("pocion_pequena".into(), 1);
        let (pack, craft) = app.backpack_for_menu();
        assert!(craft, "combining is allowed outside the caves");
        assert_eq!(pack.count("hierbas_curativas"), 2);
        assert!(pack.is_valid());

        // Combine in the menu, then commit: the world's counts follow.
        let mut edited = pack.clone();
        let herb = edited
            .placed
            .iter()
            .position(|p| p.id == "hierbas_curativas")
            .unwrap();
        let potion = edited
            .placed
            .iter()
            .position(|p| p.id == "pocion_pequena")
            .unwrap();
        assert_eq!(
            edited.combine(herb, potion).as_deref(),
            Ok("pocion_reforzada")
        );
        app.commit_backpack(edited);
        assert_eq!(app.inventory.get("hierbas_curativas"), Some(&1));
        assert_eq!(app.inventory.get("pocion_reforzada"), Some(&1));
        assert!(!app.inventory.contains_key("pocion_pequena"));
        // The crafted potion shows in the next menu, with its own name.
        let (_, items) = app.inventory_for_menu();
        let crafted = items.iter().find(|i| i.id == "pocion_reforzada").unwrap();
        assert_eq!(crafted.quantity, 1);
        assert_eq!(crafted.name, "Poción reforzada");
    }

    #[test]
    fn selling_or_gaining_items_reshapes_the_saved_layout_on_the_next_open() {
        let mut app = backpack_world("backpack-drift");
        app.inventory.insert("hueso_viejo".into(), 7);
        let (pack, _) = app.backpack_for_menu();
        app.commit_backpack(pack);
        // The shop sells four bones and a chest brings a potion.
        app.inventory.insert("hueso_viejo".into(), 3);
        app.inventory.insert("pocion_pequena".into(), 1);
        let (pack, _) = app.backpack_for_menu();
        assert_eq!(pack.count("hueso_viejo"), 3);
        assert_eq!(pack.count("pocion_pequena"), 1);
        assert!(pack.is_valid());
        // And the layout is part of the checkpoint.
        app.commit_backpack(pack.clone());
        assert_eq!(app.snapshot().backpack, Some(pack));
    }

    #[test]
    fn combining_is_locked_in_the_caves_and_expedition_finds_are_not_spent() {
        let mut app = backpack_world("backpack-cave");
        app.dungeon.unbanked_potions = 2;
        app.dungeon_mode = true;
        let (pack, craft) = app.backpack_for_menu();
        assert!(!craft);
        assert_eq!(
            pack.count("pocion_pequena"),
            2,
            "unbanked potions still show in the pack"
        );
        app.commit_backpack(pack);
        assert!(
            app.inventory.is_empty(),
            "the layout is stored but counts wait for the bank"
        );
        assert_eq!(app.dungeon.unbanked_potions, 2);
    }

    #[test]
    fn a_sword_taken_off_in_the_caves_leaves_the_hand_empty_and_is_not_doubled() {
        let mut app = backpack_world("backpack-unequip-cave");
        app.inventory.insert("espada_de_madera".into(), 1);
        app.dungeon_mode = true;
        let (mut pack, _) = app.backpack_for_menu();
        let wooden = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_de_madera")
            .unwrap();
        pack.equip(wooden).unwrap();
        app.commit_backpack(pack);
        assert!(app.dungeon.armed);
        let (mut pack, _) = app.backpack_for_menu();
        pack.unequip(backpack::Slot::Weapon).unwrap();
        app.commit_backpack(pack);
        assert!(!app.dungeon.armed, "taken off: the fist is empty");
        let (pack, _) = app.backpack_for_menu();
        assert_eq!(pack.count("espada_de_madera"), 1, "one sword, in the bag");
        assert_eq!(pack.equipment.weapon, None);
        // Worn again: in her hand again.
        let mut pack = pack;
        let wooden = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_de_madera")
            .unwrap();
        pack.equip(wooden).unwrap();
        app.commit_backpack(pack);
        assert!(app.dungeon.armed);
    }

    #[test]
    fn slime_coats_either_sword_green_or_blue_and_the_stone_one_is_listed_once() {
        let mut app = backpack_world("backpack-slime");
        app.inventory.insert("espada_de_madera".into(), 1);
        app.inventory.insert(RELIC_SWORD_ID.into(), 1);
        app.inventory.insert("gelatina_azul".into(), 1);
        app.inventory.insert("gelatina_verde".into(), 1);
        let (_, items) = app.inventory_for_menu();
        assert_eq!(items.iter().filter(|i| i.id == RELIC_SWORD_ID).count(), 1);
        let (mut pack, craft) = app.backpack_for_menu();
        assert!(craft);
        let at = |pack: &backpack::Backpack, id: &str| {
            pack.placed.iter().position(|p| p.id == id).unwrap()
        };
        let (w, b) = (at(&pack, "espada_de_madera"), at(&pack, "gelatina_azul"));
        assert_eq!(pack.combine(w, b).as_deref(), Ok("espada_pegajosa_azul"));
        let (r, g) = (at(&pack, RELIC_SWORD_ID), at(&pack, "gelatina_verde"));
        assert_eq!(pack.combine(r, g).as_deref(), Ok("espada_piedra_pegajosa"));
        app.commit_backpack(pack);
        assert_eq!(app.inventory.get("espada_pegajosa_azul"), Some(&1));
        assert_eq!(app.inventory.get("espada_piedra_pegajosa"), Some(&1));
        // Worn: its goo on the blade, its slowing touch; the stone one still cuts deeper.
        let (mut pack, _) = app.backpack_for_menu();
        let blue = at(&pack, "espada_pegajosa_azul");
        pack.equip(blue).unwrap();
        app.commit_backpack(pack);
        assert_eq!(
            app.dungeon.sword_goo,
            backpack::sword_goo("espada_pegajosa_azul")
        );
        assert!(app.dungeon.sword_goo.unwrap()[2] > 0.9, "blue");
        assert_eq!(app.dungeon.weapon_fx.slow, 3.0);
        assert!(!app.dungeon.relic_sword);
        let (mut pack, _) = app.backpack_for_menu();
        let stone = at(&pack, "espada_piedra_pegajosa");
        pack.equip(stone).unwrap();
        app.commit_backpack(pack);
        assert!(app.dungeon.relic_sword);
        assert!(app.dungeon.sword_goo.unwrap()[1] > 0.8, "green");
    }

    #[test]
    fn the_worn_weapon_decides_whether_the_relic_bonus_applies() {
        let mut app = backpack_world("backpack-weapon");
        app.inventory.insert(RELIC_SWORD_ID.into(), 1);
        app.inventory.insert("espada_de_madera".into(), 1);
        app.apply_equipment();
        assert!(
            app.dungeon.relic_sword,
            "nothing worn: the relic is used as before"
        );
        let (mut pack, _) = app.backpack_for_menu();
        let wooden = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_de_madera")
            .unwrap();
        pack.equip(wooden).unwrap();
        app.commit_backpack(pack);
        assert!(
            !app.dungeon.relic_sword,
            "choosing the wooden sword gives up the bonus"
        );
        let (mut pack, _) = app.backpack_for_menu();
        let relic = pack
            .placed
            .iter()
            .position(|p| p.id == RELIC_SWORD_ID)
            .unwrap();
        pack.equip(relic).unwrap();
        app.commit_backpack(pack);
        assert!(app.dungeon.relic_sword);
        assert_eq!(
            app.backpack.equipment.weapon.as_deref(),
            Some(RELIC_SWORD_ID)
        );
    }

    #[test]
    fn a_relic_found_with_nothing_wielded_is_taken_in_hand_when_banked() {
        let mut app = backpack_world("backpack-relic");
        app.dungeon.unbanked_relic_sword = true;
        app.dungeon.relic_sword = true;
        app.dungeon_mode = true;
        app.dungeon_mode = false;
        assert!(app.bank_dungeon_loot());
        assert_eq!(
            app.backpack.equipment.weapon.as_deref(),
            Some(RELIC_SWORD_ID)
        );
        assert_eq!(app.inventory.get(RELIC_SWORD_ID), Some(&1));
    }

    #[test]
    fn dungeon_chest_supplies_enter_backpack_and_can_be_sold() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let session = std::env::temp_dir().join(format!(
            "demon-queen-dungeon-loot-test-{}-{stamp}.json",
            std::process::id()
        ));
        let mut app = App::new(
            PathBuf::new(),
            "dungeon-loot-test".into(),
            "bedroom_inside".into(),
            session.clone(),
        );
        app.dungeon.unbanked_potions = 1;
        app.dungeon.unbanked_herbs = 2;
        app.dungeon.gold = 17;
        app.dungeon.materials.insert("hueso_viejo".into(), 3);
        app.dungeon_mode = true;

        let (visible_gold, expedition_backpack) = app.inventory_for_menu();
        assert_eq!(visible_gold, app.gold + 17);
        assert_eq!(
            expedition_backpack
                .iter()
                .find(|item| item.id == "pocion_pequena")
                .unwrap()
                .quantity,
            1,
            "newly collected potions should appear before leaving the dungeon"
        );
        assert_eq!(
            expedition_backpack
                .iter()
                .find(|item| item.id == "hierbas_curativas")
                .unwrap()
                .quantity,
            2,
            "newly collected herbs should appear before leaving the dungeon"
        );
        assert_eq!(
            expedition_backpack
                .iter()
                .find(|item| item.id == "hueso_viejo")
                .unwrap()
                .quantity,
            3,
            "bones gathered in the calabozo show in the pack"
        );
        app.dungeon_mode = false;

        assert!(app.bank_dungeon_loot());
        assert!(
            !app.bank_dungeon_loot(),
            "claimed supplies must not duplicate"
        );
        assert_eq!(app.inventory.get("pocion_pequena"), Some(&1));
        assert_eq!(app.inventory.get("hierbas_curativas"), Some(&2));
        let (_, backpack) = app.inventory_for_menu();
        assert_eq!(
            backpack
                .iter()
                .find(|item| item.id == "pocion_pequena")
                .unwrap()
                .quantity,
            1
        );
        assert_eq!(
            backpack
                .iter()
                .find(|item| item.id == "hierbas_curativas")
                .unwrap()
                .quantity,
            2
        );

        app.shop_tab = shop::Tab::Sell;
        app.shop_category = shop::Category::Consumables;
        let select_product = |app: &mut App, id: &str| {
            let visible = app.shop_product_indices();
            app.shop_selection = visible
                .iter()
                .position(|index| shop::catalog().products[*index].id == id)
                .expect("loot item is listed in the sell tab");
        };
        select_product(&mut app, "pocion_pequena");
        app.commit_shop_transaction();
        assert!(!app.inventory.contains_key("pocion_pequena"));
        assert_eq!(app.gold, 253);
        select_product(&mut app, "hierbas_curativas");
        app.adjust_shop_quantity(1);
        app.commit_shop_transaction();
        assert!(!app.inventory.contains_key("hierbas_curativas"));
        assert_eq!(app.gold, 261);

        // Materials: the shop buys them for a pittance but never sells them.
        assert_eq!(app.inventory.get("hueso_viejo"), Some(&3));
        assert!(
            shop::visible_indices(shop::Tab::Buy, shop::Category::All, &app.inventory)
                .iter()
                .all(|index| !shop::catalog().products[*index].sell_only)
        );
        app.shop_category = shop::Category::Materials;
        select_product(&mut app, "hueso_viejo");
        app.adjust_shop_quantity(2);
        app.commit_shop_transaction();
        assert!(!app.inventory.contains_key("hueso_viejo"));
        assert_eq!(app.gold, 264, "three old bones fetch three gold");

        let saved = exploration3d_state::load_from(&session).unwrap().unwrap();
        assert!(!saved.inventory.contains_key("pocion_pequena"));
        assert!(!saved.inventory.contains_key("hierbas_curativas"));
        assert_eq!(saved.gold, 264);
        std::fs::remove_file(session).unwrap();
    }

    #[test]
    fn q_in_the_mine_cures_from_the_bag_once_the_tonics_are_gone() {
        let session =
            std::env::temp_dir().join(format!("demon-queen-cure-test-{}.json", std::process::id()));
        let mut app = App::new(
            PathBuf::new(),
            "cure-test".into(),
            "bedroom_inside".into(),
            session,
        );
        app.inventory.insert("pocion_reforzada".into(), 1);
        app.inventory.insert("hierbas_curativas".into(), 2);
        app.dungeon_mode = true;
        app.dungeon.potions = 0;
        app.dungeon.herbs = 0;
        app.dungeon.health = 30.0;
        // 70 missing: the strong potion (the herbs would not do).
        app.quick_cure();
        assert_eq!(app.dungeon.health, 100.0);
        assert!(!app.inventory.contains_key("pocion_reforzada"));
        // 15 missing: one herb.
        app.dungeon.health = 85.0;
        app.quick_cure();
        assert_eq!(app.dungeon.health, 100.0);
        assert_eq!(app.inventory.get("hierbas_curativas"), Some(&1));
        // From the menu: a herb used there heals and leaves the bag.
        app.dungeon.health = 50.0;
        app.use_from_menu(&["hierbas_curativas".to_owned()]);
        assert_eq!(app.dungeon.health, 70.0);
        assert!(!app.inventory.contains_key("hierbas_curativas"));
    }

    #[test]
    fn a_worn_sticky_sword_reaches_the_mine() {
        let session = std::env::temp_dir().join(format!(
            "demon-queen-sticky-test-{}.json",
            std::process::id()
        ));
        let mut app = App::new(
            PathBuf::new(),
            "sticky-test".into(),
            "bedroom_inside".into(),
            session,
        );
        app.inventory.insert("espada_pegajosa".into(), 1);
        let mut pack = app.backpack.clone();
        pack.reconcile(&app.inventory.clone());
        let at = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_pegajosa")
            .unwrap();
        pack.equip(at).unwrap();
        app.commit_backpack(pack);
        assert!(app.dungeon.weapon_fx.slow > 0.0);
        assert!(!app.dungeon.relic_sword);
        assert!(app.has_weapon());
    }

    #[test]
    fn a_game_saved_inside_a_place_starts_inside_it() {
        for (place, x, z, room) in [
            ("burdel", 0.0f32, 0.0f32, None),
            ("termas", -6.0, 8.0, None),
            ("penascal", 1.5, 6.5, None::<String>),
        ] {
            let session = std::env::temp_dir().join(format!(
                "demon-queen-inside-{place}-{}.json",
                std::process::id()
            ));
            // Saved by the world itself: in, then the snapshot.
            let mut app = App::new(
                PathBuf::new(),
                "inside-test".into(),
                "bedroom_inside".into(),
                session.clone(),
            );
            app.nature_mode = true;
            app.village_entered = true;
            app.resume_inside(&exploration3d_state::Inside {
                place: place.into(),
                x,
                z,
                yaw: 1.0,
                level: 0,
                room: room.clone(),
                outfit_before: None,
            });
            let at = app.player;
            app.gold = 777;
            let mut state = app.snapshot();
            assert_eq!(state.inside.as_ref().map(|i| i.place.as_str()), Some(place));
            exploration3d_state::write_to(&session, &mut state).unwrap();
            let back = App::new(
                PathBuf::new(),
                "inside-test".into(),
                "bedroom_inside".into(),
                session.clone(),
            );
            let inside = match place {
                "burdel" => back.brothel.is_some(),
                "termas" => back.baths.is_some(),
                _ => back.penascal.is_some(),
            };
            assert!(inside, "{place}: not back inside");
            assert!(
                (back.player - at).length() < 0.05,
                "{place}: {:?} vs {:?}",
                back.player,
                at
            );
            assert_eq!(back.gold, 777);
            std::fs::remove_file(session).unwrap();
        }
    }
}
