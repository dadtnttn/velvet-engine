//! Screen-space navigation map for the Demon Queen exploration preview.
//!
//! The map is drawn as a vector atlas instead of a pasted image or second 3D
//! scene. Roads, house footprints and foliage come from world data; the full
//! villa overview opens with `M`, while the compact radar follows the player.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use bytemuck::{Pod, Zeroable};
use velvet_image::rasterize_simple_svg;
use velvet_math::Vec3;

use super::dungeon::prompt_icons;

mod alcove_overlay;
#[path = "map_kit.rs"]
mod map_kit;
#[path = "map_places.rs"]
mod map_places;
pub(crate) use alcove_overlay::{hit as alcove_hit, AlcoveCard, AlcoveTease, AlcoveView};
mod dialogue_overlay;
mod drink_overlay;
pub(crate) use drink_overlay::DrinkView;
mod mission_overlay;
pub(crate) use mission_overlay::MissionView;
mod boss_overlay;
pub(crate) use boss_overlay::{BossCard, BossView};
mod float_overlay;
pub(crate) use float_overlay::FloatLine;
mod glass_fx;
pub(crate) use glass_fx::Crack;
pub(crate) mod mascot;
mod mascot_hair;
pub(crate) use mascot::MascotView;
mod game_overlay;
mod hipno_overlay;
mod mage_overlay;
mod persona;
mod persona_screens;
pub(crate) use game_overlay::{chip_hit as game_chip_hit, pose_hit as game_pose_hit};
pub(crate) use mage_overlay::{hit as mage_hit, MageLine, MageOption, MageView};
mod scene_overlay;
pub(crate) use scene_overlay::{
    pose_hit as scene_pose_hit, prompt_hit as scene_prompt_hit, GameChip, GameRules, GameShout,
    GameView, HypnoView, SceneLine, ScenePrompt, SceneResult, SceneView,
};

/// The intimate scenes' picture on the screen (x, y, width, height in the logical 1080p).
pub(crate) fn scene_picture_rect(width: f32, height: f32) -> [f32; 4] {
    let r = scene_overlay::picture_rect(width, height);
    [r.x, r.y, r.w, r.h]
}
mod stage_overlay;
mod ui_paint;
pub(crate) use stage_overlay::{StageActor, StageItem, StagePanel, StageView};
mod shop_overlay;
pub(crate) use shop_overlay::{
    favor_hit as scene_favor_hit, FavorDeal, FavorTile, FuseShow, FuseView, HaggleView,
    Pick as ShopPick, ShopView,
};
#[path = "shop_preview.rs"]
mod shop_preview;
use map_kit::Place;
pub(crate) use map_kit::{hit_test as map_hit_test, MapHit};
pub(crate) mod venue_overlay;

/// (The font atlas is 4096 square, or 8192 for a big script: see `FontAtlas::new`.)
/// Mip levels of the font atlas (text is drawn well below its raster size).
const FONT_ATLAS_MIPS: u32 = 4;
const FONT_RASTER_SIZE: f32 = 64.0;
/// The glyphs are kept as distance fields (0.5 the outline, this many raster pixels each way),
/// so text stays sharp drawn at any size: a stamp four times the raster size has clean edges.
const FONT_SPREAD: u32 = 6;
const ICON_ATLAS_SIZE: u32 = 4096;
const ICON_RASTER_SIZE: u32 = 256;

const MAP_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) text: f32,
    @location(4) shape: vec4<f32>,
    @location(5) color2: vec4<f32>,
    @location(6) extra: vec4<f32>,
};
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) text: f32,
    @location(3) shape: vec4<f32>,
    @location(4) color2: vec4<f32>,
    @location(5) extra: vec4<f32>,
};

@group(0) @binding(0) var font_texture: texture_2d<f32>;
@group(0) @binding(1) var font_sampler: sampler;
@group(1) @binding(0) var icon_texture: texture_2d<f32>;
@group(1) @binding(1) var icon_sampler: sampler;
@group(2) @binding(0) var portrait_texture: texture_2d<f32>;
@group(2) @binding(1) var portrait_sampler: sampler;
@group(3) @binding(0) var picture_texture: texture_2d<f32>;
@group(3) @binding(1) var picture_sampler: sampler;

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.clip = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    output.text = input.text;
    output.shape = input.shape;
    output.color2 = input.color2;
    output.extra = input.extra;
    return output;
}

// ── Shapes drawn from their signed distance (negative inside), in their own local pixels.

fn sd_round_box(p: vec2<f32>, half: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(half.x, half.y));
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sd_rhombus(p_in: vec2<f32>, b: vec2<f32>) -> f32 {
    let p = abs(p_in);
    let h = clamp(((b.x - 2.0 * p.x) * b.x - (b.y - 2.0 * p.y) * b.y) / dot(b, b), -1.0, 1.0);
    let d = length(p - 0.5 * b * vec2<f32>(1.0 - h, 1.0 + h));
    return d * sign(p.x * b.y + p.y * b.x - b.x * b.y);
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Coverage of a pixel by a shape at distance `d`, the edge `aa` pixels soft.
fn cover(d: f32, aa: f32) -> f32 {
    return clamp(0.5 - d / aa, 0.0, 1.0);
}

fn premul(c: vec4<f32>, k: f32) -> vec4<f32> {
    let a = c.a * k;
    return vec4<f32>(c.rgb * a, a);
}

fn over(dst: vec4<f32>, src: vec4<f32>) -> vec4<f32> {
    return src + dst * (1.0 - src.a);
}

fn straight(c: vec4<f32>) -> vec4<f32> {
    if (c.a <= 0.00001) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return vec4<f32>(c.rgb / c.a, c.a);
}

// Metal lit from above (t: -1 at the top of the shape, 1 at its foot): bright crown, a
// specular band, a darker foot.
fn metal(base: vec3<f32>, t: f32) -> vec3<f32> {
    let shade = mix(1.38, 0.58, clamp(t * 0.5 + 0.5, 0.0, 1.0));
    let spec = exp(-pow((t + 0.42) * 3.0, 2.0)) * 0.6;
    return base * shade + vec3<f32>(spec) * (base + vec3<f32>(0.22, 0.2, 0.16));
}

// A panel: velvet glass with a lit top edge, a shaded foot, a grain, maybe a sweeping gloss;
// a metal rim (shape.w wide) and a fine metal line extra.x inside it.
fn shade_box(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let half = input.shape.xy;
    let d = sd_round_box(p, half, input.shape.z);
    let t = clamp(p.y / max(half.y, 1.0), -1.0, 1.0);
    let bevel = input.extra.z;
    var rgb = input.color.rgb;
    let rim = 1.0 - smoothstep(0.0, 10.0, -d);
    rgb = rgb + vec3<f32>(rim * bevel * max(-t, 0.0) * 0.20);
    rgb = rgb * (1.0 - rim * bevel * max(t, 0.0) * 0.30);
    // Toward the sides the glass deepens.
    let side = clamp(abs(p.x) / max(half.x, 1.0), 0.0, 1.0);
    rgb = rgb * (1.0 - bevel * 0.16 * side * side);
    let grain = vnoise(p * 0.42) * 0.55 + vnoise(p * 1.7) * 0.45;
    rgb = rgb * (1.0 + (grain - 0.5) * 0.12 * bevel);
    if (input.extra.w > 0.0) {
        let x = p.x / max(half.x, 1.0) + p.y / max(half.y, 1.0) * 0.3;
        let c = mix(-1.9, 1.9, fract(input.extra.w));
        rgb = rgb + vec3<f32>(exp(-pow((x - c) * 2.6, 2.0)) * 0.18);
    }
    var acc = premul(vec4<f32>(rgb, input.color.a), cover(d, aa));
    let bw = input.shape.w;
    if (bw > 0.0) {
        let band = cover(d, aa) - cover(d + bw, aa);
        acc = over(acc, premul(vec4<f32>(metal(input.color2.rgb, t), input.color2.a), band));
    }
    if (input.extra.x > 0.0) {
        let line = clamp(1.0 - abs(d + input.extra.x) / max(aa * 1.1, 0.8), 0.0, 1.0);
        acc = over(acc, premul(vec4<f32>(metal(input.color2.rgb, t), input.color2.a * input.extra.y), line));
    }
    return straight(acc);
}

// A soft shadow or glow of a rounded box (shape.w: its blur); extra.x: 1 only outside the box,
// 2 only inside it (a vignette: strongest at the edge, fading inwards).
fn shade_soft(input: VertexOut) -> vec4<f32> {
    let d = sd_round_box(input.uv, input.shape.xy, input.shape.z);
    let blur = max(input.shape.w, 0.5);
    if (input.extra.x > 1.5) {
        let inside = clamp(-d, 0.0, 1.0) * exp(-(d * d) / (blur * blur) * 2.4);
        return vec4<f32>(input.color.rgb, input.color.a * select(0.0, inside, d <= 0.0));
    }
    var a = 1.0;
    if (d > 0.0) {
        a = exp(-(d * d) / (blur * blur) * 2.4);
    }
    if (input.extra.x > 0.5) {
        a = a * smoothstep(-blur * 0.7, 0.0, d);
    }
    return vec4<f32>(input.color.rgb, input.color.a * a);
}

// A wire with round ends (shape.x: half its length, shape.y: its radius, shape.z: a glow).
fn shade_wire(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let r = max(input.shape.y, 0.3);
    let q = vec2<f32>(max(abs(p.x) - input.shape.x, 0.0), p.y);
    let d = length(q) - r;
    var rgb = input.color.rgb;
    if (input.extra.x > 0.0) {
        // A gold thread: bright along its middle, darker at its sides.
        let across = clamp(length(q) / r, 0.0, 1.0);
        rgb = rgb * mix(1.42, 0.66, across * across) + vec3<f32>(0.1, 0.08, 0.04) * (1.0 - across);
    }
    var a = cover(d, max(aa, 0.6));
    if (input.shape.z > 0.0) {
        a = max(a, exp(-pow(max(d, 0.0) / input.shape.z, 2.0) * 2.0) * 0.5);
    }
    return vec4<f32>(rgb, input.color.a * a);
}

// A cut gem in a metal setting (shape.xy: its half sizes, shape.z: the setting, shape.w: glow).
fn shade_gem(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let b = input.shape.xy;
    let d = sd_rhombus(p, b);
    let inner = d + input.shape.z;
    let f = p / b;
    var lit = 0.58;
    if (f.y < 0.0) {
        lit = lit + 0.32;
    }
    if (f.x < 0.0) {
        lit = lit + 0.14;
    }
    var stone = input.color.rgb * lit;
    let table = 1.0 - smoothstep(0.0, 0.5, abs(f.x) + abs(f.y));
    stone = stone + input.color.rgb * table * 0.5;
    let glint = exp(-pow(length(f - vec2<f32>(-0.2, -0.32)) * 4.2, 2.0));
    stone = stone + vec3<f32>(glint * 0.95);
    var acc = premul(vec4<f32>(metal(input.color2.rgb, clamp(f.y, -1.0, 1.0)), input.color2.a), cover(d, aa));
    acc = over(acc, premul(vec4<f32>(stone, input.color.a), cover(inner, aa)));
    if (input.shape.w > 0.0) {
        let g = exp(-pow(max(d, 0.0) / input.shape.w, 2.0) * 2.0) * 0.6 * input.color.a;
        acc = over(vec4<f32>(input.color.rgb * g, g), acc);
    }
    return straight(acc);
}

// A picture (face or scene) masked to a rounded box, deepening toward its edge (shape.w).
fn shade_masked(input: VertexOut, aa: f32, texel: vec4<f32>) -> vec4<f32> {
    let d = sd_round_box(input.uv, input.shape.xy, input.shape.z);
    let inside = smoothstep(0.0, max(input.shape.w, 0.001), -d);
    let rgb = texel.rgb * input.color.rgb * mix(0.62, 1.0, inside);
    return vec4<f32>(rgb, texel.a * input.color.a * cover(d, aa));
}

// A crescent of metal: a disc (shape.x) less a disc (shape.y) at shape.zw.
fn shade_crescent(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let d1 = length(p) - input.shape.x;
    let d2 = length(p - input.shape.zw) - input.shape.y;
    let d = max(d1, -d2);
    let t = clamp(p.y / max(input.shape.x, 1.0), -1.0, 1.0);
    var acc = premul(vec4<f32>(metal(input.color.rgb, t), input.color.a), cover(d, aa));
    let edge = cover(d, aa) - cover(d + 1.1, aa);
    acc = over(acc, premul(input.color2, edge));
    return straight(acc);
}

// A four-pointed sparkle (shape.x: its reach, shape.y: how thin, shape.z: its glow).
fn shade_star(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let r = max(input.shape.x, 1.0);
    let k = input.shape.y;
    let d = min(sd_rhombus(p, vec2<f32>(r, r * k)), sd_rhombus(p, vec2<f32>(r * k, r)));
    let glow = exp(-length(p) / r * 3.2) * input.shape.z;
    let a = max(cover(d, aa), glow);
    return vec4<f32>(input.color.rgb, input.color.a * clamp(a, 0.0, 1.0));
}

// A ring (shape.x: radius, shape.y: width) filled clockwise from shape.w turns past the top for
// shape.z of a turn, over a track (color2).
fn shade_arc(input: VertexOut, aa: f32) -> vec4<f32> {
    let p = input.uv;
    let d = abs(length(p) - input.shape.x) - input.shape.y * 0.5;
    let ang = fract(atan2(p.x, -p.y) / 6.2831853 + 1.0 - input.shape.w);
    let along = (input.shape.z - ang) * input.shape.x * 6.2831853;
    let lit = clamp(along / max(aa, 0.75) + 0.5, 0.0, 1.0);
    var acc = premul(input.color2, cover(d, aa));
    acc = over(acc, premul(input.color, cover(d, aa) * lit));
    return straight(acc);
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    // Taken in uniform control flow: how many local pixels a screen pixel spans (the edges'
    // softness), and the glyph under this pixel (the atlas has mips, so this needs derivatives).
    let fw = fwidth(input.uv);
    let aa = max(0.5 * (fw.x + fw.y), 0.0001);
    let glyph = textureSample(font_texture, font_sampler, input.uv).r;
    let mode = input.text;
    if (mode > 14.5) {
        // The stage's pictures: straight alpha, with their mips.
        let texel = textureSample(picture_texture, picture_sampler, input.uv);
        return vec4<f32>(texel.rgb * input.color.rgb, texel.a * input.color.a);
    }
    if (mode > 13.5) {
        // Chiquita's picture (premultiplied by its own coverage), laid 1:1 over the screen.
        let texel = textureSampleLevel(picture_texture, picture_sampler, input.uv, 0.0);
        if (texel.a < 0.002) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        return vec4<f32>(texel.rgb / texel.a * input.color.rgb, texel.a * input.color.a);
    }
    if (mode > 12.5) {
        return shade_arc(input, aa);
    }
    if (mode > 11.5) {
        return shade_star(input, aa);
    }
    if (mode > 10.5) {
        return shade_crescent(input, aa);
    }
    if (mode > 9.5) {
        let texel = textureSampleLevel(picture_texture, picture_sampler, input.extra.xy, 0.0);
        return shade_masked(input, aa, vec4<f32>(texel.rgb, 1.0));
    }
    if (mode > 8.5) {
        let texel = textureSampleLevel(portrait_texture, portrait_sampler, input.extra.xy, 0.0);
        return shade_masked(input, aa, texel);
    }
    if (mode > 7.5) {
        return shade_gem(input, aa);
    }
    if (mode > 6.5) {
        return shade_wire(input, aa);
    }
    if (mode > 5.5) {
        return shade_soft(input);
    }
    if (mode > 4.5) {
        return shade_box(input, aa);
    }
    if (mode > 3.5) {
        let texel = textureSample(picture_texture, picture_sampler, input.uv);
        return vec4<f32>(texel.rgb * input.color.rgb, input.color.a);
    }
    if (mode > 2.5) {
        let texel = textureSample(portrait_texture, portrait_sampler, input.uv);
        return vec4<f32>(texel.rgb * input.color.rgb, texel.a * input.color.a);
    }
    if (mode > 1.5) {
        let texel = textureSample(icon_texture, icon_sampler, input.uv);
        return vec4<f32>(texel.rgb * input.color.rgb, texel.a * input.color.a);
    }
    var coverage = 1.0;
    if (mode > 0.5) {
        // The glyphs are distance fields: 0.5 is the outline, softened over about a pixel.
        let edge = max(fwidth(glyph) * 0.7, 0.002);
        coverage = smoothstep(0.5 - edge, 0.5 + edge, glyph);
        // A rim round the letters (shape.x: how far out, in field units; color2: its colour),
        // traced from the same field: round, even and clean however thick.
        if (input.shape.x > 0.0) {
            let out = smoothstep(0.5 - input.shape.x - edge, 0.5 - input.shape.x + edge, glyph);
            let fill = premul(input.color, coverage);
            let rim = premul(input.color2, out);
            return straight(over(rim, fill));
        }
    }
    return vec4<f32>(input.color.rgb, input.color.a * coverage);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MapVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    /// What the pixel is: 0 flat, 1 glyph, 2 icon, 3 face, 4 picture, 5.. the painter's shapes.
    text: f32,
    /// The painter's shapes: their size and corner, a second colour, more parameters.
    shape: [f32; 4],
    color2: [f32; 4],
    extra: [f32; 4],
}

impl MapVertex {
    const BLANK: Self = Self {
        position: [0.0; 2],
        uv: [0.0; 2],
        color: [0.0; 4],
        text: 0.0,
        shape: [0.0; 4],
        color2: [0.0; 4],
        extra: [0.0; 4],
    };
}

/// CPU time spent laying out and uploading the overlay (nanoseconds, summed): for profiling.
pub(crate) static OVERLAY_CPU_NS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Adds the time since it was made to `OVERLAY_CPU_NS` when dropped.
struct CpuTimer(std::time::Instant);

impl Drop for CpuTimer {
    fn drop(&mut self) {
        OVERLAY_CPU_NS.fetch_add(self.0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Clone, Copy)]
struct GlyphEntry {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    size: [f32; 2],
    bearing: [f32; 2],
    advance: f32,
}

pub(crate) struct FontAtlas {
    bind_group: wgpu::BindGroup,
    glyphs: HashMap<char, GlyphEntry>,
    ascent: f32,
    shop_faces: Vec<FontFace>,
    /// The language's own font under the game's faces (see [`ScriptFace`]).
    script: Option<ScriptFace>,
    /// The language generation these glyphs were made for (`i18n::generation`).
    pub(crate) generation: u64,
    /// Lines already laid out (shaping is not free), by face and text.
    runs: std::sync::Mutex<HashMap<(u8, String), std::sync::Arc<Run>>>,
}

/// One line of text laid out: each glyph with its pen position along the line and its offset
/// from the baseline (raster units), and the line's advance.
pub(crate) struct Run {
    pub(crate) glyphs: Vec<(GlyphEntry, f32, f32)>,
    pub(crate) advance: f32,
}

/// The language's own font, under the game's faces: the glyphs they lack (another script, accents
/// past Latin-1) by character, and for scripts whose letters join and reorder (Devanagari, Thai,
/// Hebrew) the shaped glyphs of every text the catalog draws, by glyph id.
struct ScriptFace {
    glyphs: HashMap<char, GlyphEntry>,
    ids: HashMap<u16, GlyphEntry>,
    bytes: std::sync::Arc<Vec<u8>>,
    index: u32,
    /// Its height in font units (to turn shaped positions into raster units).
    height_units: f32,
    shaped: bool,
}

use demon_queen_dungeon::i18n::script_font;

/// `line` shaped in the script font, in raster units.
fn shape_line(bytes: &[u8], index: u32, height_units: f32, line: &str) -> Vec<(u16, f32, f32, f32)> {
    demon_queen_dungeon::i18n::shape_line(bytes, index, FONT_RASTER_SIZE / height_units.max(1.0), line)
}

impl FontAtlas {
    /// One line laid out in `glyphs` (a face's), falling back to the language's font for what the
    /// face lacks; lines in a shaped script go through the shaper.
    pub(crate) fn run(&self, face_key: u8, glyphs: &HashMap<char, GlyphEntry>, line: &str) -> std::sync::Arc<Run> {
        let key = (face_key, line.to_owned());
        if let Some(run) = self.runs.lock().unwrap().get(&key) {
            return run.clone();
        }
        let mut placed = Vec::new();
        let mut pen = 0.0;
        match &self.script {
            Some(sc) if sc.shaped && line.chars().any(demon_queen_dungeon::i18n::needs_shaping) => {
                for (id, advance, dx, dy) in shape_line(&sc.bytes, sc.index, sc.height_units, line) {
                    if let Some(g) = sc.ids.get(&id) {
                        placed.push((*g, pen + dx, dy));
                    }
                    pen += advance;
                }
            }
            _ => {
                for c in line.chars() {
                    let g = glyphs
                        .get(&c)
                        .or_else(|| self.script.as_ref().and_then(|s| s.glyphs.get(&c)))
                        .or_else(|| glyphs.get(&'?'));
                    if let Some(g) = g {
                        placed.push((*g, pen, 0.0));
                        pen += g.advance;
                    }
                }
            }
        }
        let run = std::sync::Arc::new(Run { glyphs: placed, advance: pen });
        let mut runs = self.runs.lock().unwrap();
        if runs.len() > 20_000 {
            runs.clear();
        }
        runs.insert(key, run.clone());
        run
    }
}

struct FontFace {
    glyphs: HashMap<char, GlyphEntry>,
    ascent: f32,
}

#[derive(Clone, Copy)]
struct IconEntry {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

struct IconAtlas {
    bind_group: wgpu::BindGroup,
    icons: HashMap<String, IconEntry>,
}

/// The places' pictures, baked from the world seen from straight above (`demon-queen-3d
/// --capture-toon <dir> --bake-maps`, copied into `data/maps/`: `<name>.png` and `maps.json`
/// with each one's world rectangle). One is held at a time: the one of the place on the map.
struct MapPictures {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Each picture's world rectangle: x0, x1, z0, z1 (north, -z, at the top).
    rects: HashMap<String, [f32; 4]>,
    current: Option<String>,
    bind_group: wgpu::BindGroup,
    failed: std::collections::HashSet<String>,
    root: PathBuf,
}

impl MapPictures {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let root = crate::exploration3d_state::data_dir().join("maps");
        let rects = std::fs::read_to_string(root.join("maps.json"))
            .ok()
            .and_then(|text| {
                serde_json::from_str::<HashMap<String, HashMap<String, f32>>>(&text).ok()
            })
            .map(|maps| {
                maps.into_iter()
                    .filter_map(|(name, r)| {
                        Some((
                            name,
                            [*r.get("x0")?, *r.get("x1")?, *r.get("z0")?, *r.get("z1")?],
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-map-picture-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = Self::upload(device, queue, layout, &sampler, 1, 1, &[0, 0, 0, 255]);
        Self {
            layout: layout.clone(),
            sampler,
            rects,
            current: None,
            bind_group,
            failed: Default::default(),
            root,
        }
    }

    /// The picture as a texture with its mip chain (the map shows it far smaller than baked).
    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> wgpu::BindGroup {
        let levels = 32 - width.max(height).leading_zeros();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("demon-queen-map-picture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut level = image::RgbaImage::from_raw(width, height, rgba.to_vec())
            .unwrap_or_else(|| image::RgbaImage::new(width, height));
        for mip in 0..levels {
            let (w, h) = (level.width(), level.height());
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                level.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            if mip + 1 < levels {
                level = image::imageops::resize(
                    &level,
                    (w / 2).max(1),
                    (h / 2).max(1),
                    image::imageops::FilterType::Triangle,
                );
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demon-queen-map-picture-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    /// The world rectangle of `name`'s picture, which is now the one bound; none when there is
    /// no such picture (the map is then drawn the old way).
    fn ensure(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        name: &str,
    ) -> Option<[f32; 4]> {
        let rect = *self.rects.get(name)?;
        if self.current.as_deref() == Some(name) {
            return Some(rect);
        }
        if self.failed.contains(name) {
            return None;
        }
        let path = self.root.join(format!("{name}.png"));
        let Ok(picture) = velvet_avif::open_image(path.as_ref()) else {
            eprintln!("[map] picture missing: {}", path.display());
            self.failed.insert(name.to_owned());
            return None;
        };
        let picture = picture.to_rgba8();
        self.bind_group = Self::upload(
            device,
            queue,
            &self.layout,
            &self.sampler,
            picture.width(),
            picture.height(),
            picture.as_raw(),
        );
        self.current = Some(name.to_owned());
        Some(rect)
    }
}

/// The part of a picture (world rectangle `picture`, north up) that lies inside the map's
/// bounds (in map space: x, and -z up), drawn over the map rectangle.
fn push_map_picture(
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    picture: [f32; 4],
    color: [f32; 4],
) {
    let [x0, x1, z0, z1] = picture;
    // In map space the picture runs from -z1 (its foot) up to -z0 (its top).
    let (ax0, ax1) = (x0.max(bounds.min_x), x1.min(bounds.max_x));
    let (az0, az1) = ((-z1).max(bounds.min_z), (-z0).min(bounds.max_z));
    if ax1 <= ax0 || az1 <= az0 {
        return;
    }
    let rect = map_world_rect_coords(ax0, ax1, az0, az1, bounds, map);
    let u = |x: f32| (x - x0) / (x1 - x0);
    let v = |map_z: f32| (-z0 - map_z) / (z1 - z0);
    let (uv_min, uv_max) = ([u(ax0), v(az1)], [u(ax1), v(az0)]);
    let (rx0, ry0, rx1, ry1) = (rect.x, rect.y, rect.x + rect.w, rect.y + rect.h);
    for (position, uv) in [
        ([rx0, ry0], [uv_min[0], uv_min[1]]),
        ([rx1, ry0], [uv_max[0], uv_min[1]]),
        ([rx1, ry1], [uv_max[0], uv_max[1]]),
        ([rx0, ry0], [uv_min[0], uv_min[1]]),
        ([rx1, ry1], [uv_max[0], uv_max[1]]),
        ([rx0, ry1], [uv_min[0], uv_max[1]]),
    ] {
        vertices.push(MapVertex {
            position,
            uv,
            color,
            text: 4.0,
            ..MapVertex::BLANK
        });
    }
}

const PORTRAIT_ATLAS_SIZE: u32 = 2048;
const PORTRAIT_CELL: u32 = 512;

/// The dialogue box's portraits: sixteen 512 px cells, each filled the first time its portrait is
/// shown (decoded from the game's art, cropped to the head and shoulders); the oldest cell is
/// reused once all are taken.
struct PortraitAtlas {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    cells: HashMap<PortraitRef, u32>,
    order: Vec<PortraitRef>,
    failed: std::collections::HashSet<PortraitRef>,
    root: PathBuf,
}

impl PortraitAtlas {
    fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("demon-queen-portrait-atlas"),
            size: wgpu::Extent3d {
                width: PORTRAIT_ATLAS_SIZE,
                height: PORTRAIT_ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-portrait-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demon-queen-portrait-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let root = crate::exploration3d_state::data_dir().join("assets");
        Self {
            texture,
            bind_group,
            cells: HashMap::new(),
            order: Vec::new(),
            failed: Default::default(),
            root,
        }
    }

    /// Where `portrait` sits in the atlas (uv min, uv max), uploading it the first time.
    fn ensure(
        &mut self,
        queue: &wgpu::Queue,
        portrait: &PortraitRef,
    ) -> Option<([f32; 2], [f32; 2])> {
        let per_row = PORTRAIT_ATLAS_SIZE / PORTRAIT_CELL;
        let capacity = per_row * per_row;
        let cell = match self.cells.get(portrait) {
            Some(&cell) => cell,
            None => {
                if self.failed.contains(portrait) {
                    return None;
                }
                let path = self.root.join(format!("{}.png", portrait.image));
                let Ok(image) = velvet_avif::open_image(path.as_ref()) else {
                    eprintln!("[dialogue] portrait missing: {}", path.display());
                    self.failed.insert(portrait.clone());
                    return None;
                };
                let [x, y, w, h] = portrait.crop;
                let (iw, ih) = (image.width(), image.height());
                let x = x.min(iw.saturating_sub(1));
                let y = y.min(ih.saturating_sub(1));
                let w = w.max(1).min(iw - x);
                let h = h.max(1).min(ih - y);
                let rgba = image
                    .crop_imm(x, y, w, h)
                    .resize_exact(
                        PORTRAIT_CELL,
                        PORTRAIT_CELL,
                        image::imageops::FilterType::Lanczos3,
                    )
                    .to_rgba8();
                let cell = if (self.cells.len() as u32) < capacity {
                    self.cells.len() as u32
                } else {
                    let oldest = self.order.remove(0);
                    self.cells.remove(&oldest).unwrap_or(0)
                };
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: (cell % per_row) * PORTRAIT_CELL,
                            y: (cell / per_row) * PORTRAIT_CELL,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    rgba.as_raw(),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(PORTRAIT_CELL * 4),
                        rows_per_image: Some(PORTRAIT_CELL),
                    },
                    wgpu::Extent3d {
                        width: PORTRAIT_CELL,
                        height: PORTRAIT_CELL,
                        depth_or_array_layers: 1,
                    },
                );
                self.cells.insert(portrait.clone(), cell);
                self.order.push(portrait.clone());
                cell
            }
        };
        let size = PORTRAIT_ATLAS_SIZE as f32;
        let x0 = ((cell % per_row) * PORTRAIT_CELL) as f32 / size;
        let y0 = ((cell / per_row) * PORTRAIT_CELL) as f32 / size;
        let span = PORTRAIT_CELL as f32 / size;
        Some(([x0, y0], [x0 + span, y0 + span]))
    }
}

impl IconAtlas {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        asset_root: &Path,
    ) -> Self {
        let mut pixels = vec![0_u8; (ICON_ATLAS_SIZE * ICON_ATLAS_SIZE * 4) as usize];
        let mut icons = HashMap::new();
        let venue_root = asset_root
            .parent()
            .map(|p| p.join("venue"))
            .unwrap_or_else(|| asset_root.to_path_buf());
        let mut assets: Vec<(&str, &str)> = super::shop::catalog()
            .products
            .iter()
            .filter_map(|p| {
                // Vector art only: the shop draws its icons from SVG.
                p.icon_svg
                    .as_deref()
                    .or(p.icon_image.as_deref())
                    .map(|file| (p.id.as_str(), file))
            })
            .collect();
        assets.extend(
            super::backpack::EXTRAS
                .iter()
                .filter(|e| !e.icon_svg.is_empty())
                .map(|e| (e.id, e.icon_svg)),
        );
        assets.extend([
            ("coin", "coin.svg"),
            ("tab_buy", "tab_buy.svg"),
            ("tab_sell", "tab_sell.svg"),
            ("cart", "cart.svg"),
            ("sell", "sell.svg"),
            ("plus", "plus.svg"),
            ("minus", "minus.svg"),
            ("close", "close.svg"),
            ("category_all", "category_all.svg"),
            ("category_equipment", "category_equipment.svg"),
            ("category_materials", "category_materials.svg"),
            ("category_special", "category_special.svg"),
            // Venue decor & icons
            ("venue_sign", "sign.png"),
            ("venue_roses", "decor_roses.png"),
            ("heart", "heart.svg"),
            ("work", "work.svg"),
            ("bed", "bed.svg"),
            ("key", "key.svg"),
            ("chat", "chat.svg"),
            ("book", "book.svg"),
            ("muscle", "muscle.svg"),
            ("cup", "cup.svg"),
            ("lightning", "lightning.svg"),
            ("smile", "smile.svg"),
            ("alert", "alert.svg"),
            ("doc", "doc.svg"),
            ("star_full", "star_full.svg"),
            ("star_empty", "star_empty.svg"),
            // Venue rooms
            ("room_modest", "room_modest.png"),
            ("room_comfortable", "room_comfortable.png"),
            ("room_suite", "room_suite.png"),
            ("room_discreet", "room_discreet.png"),
            ("room_modest_large", "room_modest_large.png"),
            ("room_comfortable_large", "room_comfortable_large.png"),
            ("room_suite_large", "room_suite_large.png"),
            ("room_discreet_large", "room_discreet_large.png"),
            // Venue companions
            ("comp_1", "comp_1.png"),
            ("comp_2", "comp_2.png"),
            ("comp_3", "comp_3.png"),
            ("comp_4", "comp_4.png"),
            ("comp_1_large", "comp_1_large.png"),
            ("comp_2_large", "comp_2_large.png"),
            ("comp_3_large", "comp_3_large.png"),
            ("comp_4_large", "comp_4_large.png"),
            // Venue clients
            ("client_merchant", "client_merchant.png"),
            ("client_guard", "client_guard.png"),
            ("client_noble", "client_noble.png"),
            ("client_scholar", "client_scholar.png"),
            ("client_pilgrim", "client_pilgrim.png"),
            ("client_merchant_large", "client_merchant_large.png"),
            ("client_guard_large", "client_guard_large.png"),
            ("client_noble_large", "client_noble_large.png"),
            ("client_scholar_large", "client_scholar_large.png"),
            ("client_pilgrim_large", "client_pilgrim_large.png"),
        ]);
        let cell = 320_u32;
        let inset = 32_u32;
        // Shop sprites are prefiltered at their final 1080p logical size, then
        // sampled 1:1. Keep existing venue entries and the same single GPU atlas.
        let mut jobs: Vec<_> = assets
            .iter()
            .map(|&(id, file)| (id.to_owned(), file, ICON_RASTER_SIZE, ICON_RASTER_SIZE))
            .collect();
        for product in &super::shop::catalog().products {
            if let Some(file) = product
                .icon_svg
                .as_deref()
                .or(product.icon_image.as_deref())
            {
                for size in [74, 276] {
                    jobs.push((format!("{}@{size}x{size}", product.id), file, size, size));
                }
            }
        }
        for (id, sizes) in [
            ("tab_buy", &[36][..]),
            ("tab_sell", &[32, 36, 76, 128][..]),
            ("coin", &[28, 30, 34][..]),
            ("plus", &[32][..]),
            ("minus", &[32][..]),
            ("close", &[36][..]),
            ("category_all", &[32][..]),
            ("category_equipment", &[32][..]),
            ("category_materials", &[32][..]),
            ("category_special", &[32][..]),
        ] {
            if let Some(&(_, file)) = assets.iter().find(|&&(key, _)| key == id) {
                for &size in sizes {
                    jobs.push((format!("{id}@{size}x{size}"), file, size, size));
                }
            }
        }
        let mut sources = HashMap::new();
        for (index, (id, file, raster_w, raster_h)) in jobs.into_iter().enumerate() {
            let path = if venue_root.join(file).exists() {
                venue_root.join(file)
            } else {
                asset_root.join(file)
            };
            let (bitmap, vector) = sources.entry(path.clone()).or_insert_with(|| {
                // The shop's own icons are vector art: a raster with the same
                // name never replaces them. Venue art is still PNG-first.
                let shop_vector =
                    path.extension().is_some_and(|e| e == "svg") && path.starts_with(asset_root);
                let bitmap = if shop_vector {
                    None
                } else {
                    velvet_avif::open_image(&path.with_extension("png")).ok()
                };
                let vector = if bitmap.is_none() {
                    std::fs::read_to_string(&path).ok()
                } else {
                    None
                };
                (bitmap, vector)
            });
            let rgba = bitmap
                .as_ref()
                .map(|image| {
                    image
                        .resize_exact(raster_w, raster_h, image::imageops::FilterType::Lanczos3)
                        .to_rgba8()
                        .into_raw()
                })
                .or_else(|| {
                    // Supersample the small vector controls once for antialiased edges.
                    let svg = vector.as_deref()?;
                    let img = rasterize_simple_svg(svg, raster_w * 3, raster_h * 3).ok()?;
                    let rgba = image::RgbaImage::from_raw(raster_w * 3, raster_h * 3, img.pixels)?;
                    Some(
                        image::imageops::resize(
                            &rgba,
                            raster_w,
                            raster_h,
                            image::imageops::FilterType::Lanczos3,
                        )
                        .into_raw(),
                    )
                });
            let Some(rgba) = rgba else {
                eprintln!("[shop-ui] icon missing: {}", path.display());
                continue;
            };
            let col = index as u32 % 12;
            let row = index as u32 / 12;
            let x0 = inset + col * cell;
            let y0 = inset + row * cell;
            if x0 + raster_w > ICON_ATLAS_SIZE || y0 + raster_h > ICON_ATLAS_SIZE {
                continue;
            }
            for y in 0..raster_h {
                let src_start = (y * raster_w * 4) as usize;
                let dst_start = (((y0 + y) * ICON_ATLAS_SIZE + x0) * 4) as usize;
                let len = (raster_w * 4) as usize;
                pixels[dst_start..dst_start + len]
                    .copy_from_slice(&rgba[src_start..src_start + len]);
            }
            icons.insert(
                id.to_owned(),
                IconEntry {
                    uv_min: [
                        x0 as f32 / ICON_ATLAS_SIZE as f32,
                        y0 as f32 / ICON_ATLAS_SIZE as f32,
                    ],
                    uv_max: [
                        (x0 + raster_w) as f32 / ICON_ATLAS_SIZE as f32,
                        (y0 + raster_h) as f32 / ICON_ATLAS_SIZE as f32,
                    ],
                },
            );
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("demon-queen-shop-icon-atlas"),
            size: wgpu::Extent3d {
                width: ICON_ATLAS_SIZE,
                height: ICON_ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ICON_ATLAS_SIZE * 4),
                rows_per_image: Some(ICON_ATLAS_SIZE),
            },
            wgpu::Extent3d {
                width: ICON_ATLAS_SIZE,
                height: ICON_ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-shop-icon-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demon-queen-shop-icon-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self { bind_group, icons }
    }

    fn get(&self, id: &str) -> Option<IconEntry> {
        self.icons.get(id).copied()
    }

    fn get_sized(&self, id: &str, w: u32, h: u32) -> Option<IconEntry> {
        self.get(&format!("{id}@{w}x{h}")).or_else(|| self.get(id))
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// A place to go in on the screen (a door, a path, a stall): over it, its icon on a slanted card
/// in the Persona manner, an arrow down at the spot, and the F key once she is in reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldMarker {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) icon: prompt_icons::Icon,
    /// 0..1 as she comes near.
    pub(crate) fade: f32,
    /// In reach: F does it.
    pub(crate) ready: bool,
}

/// Screen-space speech bubble anchored to a non-player character.
///
/// The world renderer projects the NPC's head into logical 1920×1080 pixels;
/// this overlay owns only the readable bubble treatment. Keeping the data
/// separate from the player prevents accidental player speech indicators.
#[derive(Clone, Copy)]
pub(crate) struct NpcSpeechBubble {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) speaker: &'static str,
    pub(crate) text: &'static str,
    pub(crate) opacity: f32,
    pub(crate) accent: [f32; 4],
}

impl Rect {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShopHit {
    TabBuy,
    TabSell,
    Category(super::shop::Category),
    PagePrevious,
    PageNext,
    Item(usize),
    QuantityMinus,
    QuantityPlus,
    Action,
    Close,
    /// The fusion altar's tab, the lit product's haggle.
    TabFuse,
    Haggle,
    /// A haggle's tactic tile.
    Tactic(u8),
    /// The altar: one of her things, a socket, its two buttons.
    FuseItem(usize),
    FuseSlot(u8),
    FuseGo,
    FuseClear,
    /// «Otro trato»'s arrows: the other favours she can offer.
    FavorPrev,
    FavorNext,
}

fn shop_hit_signature(hit: Option<ShopHit>) -> u16 {
    match hit {
        None => 0,
        Some(ShopHit::TabBuy) => 1,
        Some(ShopHit::TabSell) => 2,
        Some(ShopHit::Item(index)) => 100 + index.min(999) as u16,
        Some(ShopHit::QuantityMinus) => 3,
        Some(ShopHit::QuantityPlus) => 4,
        Some(ShopHit::Action) => 5,
        Some(ShopHit::Close) => 6,
        Some(ShopHit::Category(category)) => 10 + category as u16,
        Some(ShopHit::PagePrevious) => 20,
        Some(ShopHit::PageNext) => 21,
        Some(ShopHit::TabFuse) => 22,
        Some(ShopHit::Haggle) => 23,
        Some(ShopHit::FuseGo) => 24,
        Some(ShopHit::FuseClear) => 25,
        Some(ShopHit::Tactic(i)) => 30 + u16::from(i.min(9)),
        Some(ShopHit::FuseSlot(i)) => 40 + u16::from(i.min(9)),
        Some(ShopHit::FuseItem(index)) => 2000 + index.min(999) as u16,
        Some(ShopHit::FavorPrev) => 50,
        Some(ShopHit::FavorNext) => 51,
    }
}

/// What lies under the pointer (the logical 1080p) on the shop's page on show.
pub(crate) fn shop_hit_test(x: f32, y: f32, pick: &ShopPick) -> Option<ShopHit> {
    shop_overlay::Layout::hit(x, y, pick)
}

/// What picking needs: the shelves as they are filtered, and the page on show.
#[allow(clippy::too_many_arguments)]
pub(crate) fn shop_pick(
    tab: super::shop::Tab,
    category: super::shop::Category,
    selection: usize,
    inventory: &BTreeMap<String, u32>,
    page: u8,
    fuse_count: usize,
    fuse_cursor: usize,
) -> ShopPick {
    ShopPick {
        page,
        selection,
        count: super::shop::visible_indices(tab, category, inventory).len(),
        fuse_count,
        fuse_cursor,
        favor: false,
        favor_choices: 0,
    }
}

pub(crate) fn shop_preview_hit_test(x: f32, y: f32, has_model: bool) -> bool {
    has_model && shop_overlay::Layout::PREVIEW.contains(x, y)
}

#[derive(Clone, Copy)]
struct MapBounds {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
}

/// A glyph's coverage (`w`×`h`, 0..1) as a signed distance field in bytes: 128 on the outline,
/// higher inside, `spread` pixels to 0 or 255. Exact Euclidean distances to the nearest pixel of
/// the other side (Felzenszwalb's transform, by rows then columns), refined at the edge by the
/// coverage itself.
fn distance_field(coverage: &[f32], w: usize, h: usize, spread: f32) -> Vec<u8> {
    const FAR: f64 = 1.0e12;
    // Squared distance along one line to the nearest zero of `f` (0 at the set, FAR elsewhere).
    fn line(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
        let n = f.len();
        let parabola = |q: usize, p: usize| {
            ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * q as f64 - 2.0 * p as f64)
        };
        let mut k = 0;
        v[0] = 0;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        for q in 1..n {
            let mut s = parabola(q, v[k]);
            while s <= z[k] {
                k -= 1;
                s = parabola(q, v[k]);
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        for q in 0..n {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            let p = v[k];
            d[q] = (q as f64 - p as f64).powi(2) + f[p];
        }
    }
    let squared = |inside: bool| -> Vec<f64> {
        let mut grid: Vec<f64> = coverage
            .iter()
            .map(|&c| if (c >= 0.5) == inside { 0.0 } else { FAR })
            .collect();
        let n = w.max(h);
        let (mut f, mut d) = (vec![0.0; n], vec![0.0; n]);
        let (mut v, mut z) = (vec![0usize; n], vec![0.0f64; n + 1]);
        for x in 0..w {
            for y in 0..h {
                f[y] = grid[y * w + x];
            }
            line(&f[..h], &mut d[..h], &mut v, &mut z);
            for y in 0..h {
                grid[y * w + x] = d[y];
            }
        }
        for y in 0..h {
            f[..w].copy_from_slice(&grid[y * w..y * w + w]);
            line(&f[..w], &mut d[..w], &mut v, &mut z);
            grid[y * w..y * w + w].copy_from_slice(&d[..w]);
        }
        grid
    };
    let to_ink = squared(true);
    let to_paper = squared(false);
    coverage
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let signed = if c >= 0.5 {
                to_paper[i].sqrt() as f32 - 0.5
            } else {
                0.5 - to_ink[i].sqrt() as f32
            };
            // On the edge itself the coverage says where the outline falls within the pixel.
            let signed = if c > 0.0 && c < 1.0 && signed.abs() <= 1.0 {
                c - 0.5
            } else {
                signed
            };
            ((0.5 + signed / (2.0 * spread)).clamp(0.0, 1.0) * 255.0).round() as u8
        })
        .collect()
}

impl FontAtlas {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout) -> Self {
        let fonts = [
            load_ui_font(),
            load_shop_font("georgiab.ttf", "LiberationSerif-Bold.ttf"),
            load_shop_font("georgia.ttf", "LiberationSerif-Regular.ttf"),
            load_shop_font("georgiai.ttf", "LiberationSerif-Italic.ttf"),
            load_shop_font("segoeui.ttf", "LiberationSans-Regular.ttf"),
            load_shop_font("constanb.ttf", "LiberationSerif-Bold.ttf"),
            load_shop_font("constani.ttf", "LiberationSerif-Italic.ttf"),
            // The stage's faces, as the web scenes have them (OFL fonts shipped in data/fonts).
            load_game_font("Anton-Regular.ttf"),
            load_game_font("Nunito-ExtraBold.ttf"),
            load_game_font("Nunito-BoldItalic.ttf"),
            load_game_font("Nunito-SemiBoldItalic.ttf"),
        ];
        let scale = PxScale::from(FONT_RASTER_SIZE);
        let mut characters = (32_u8..=126_u8).map(char::from).collect::<Vec<_>>();
        characters.extend("ÁÉÍÓÚÑÜáéíóúñü¿¡“”•—◆★☆…«»‘’×–·−".chars());
        // The language's own characters: the game's faces take those they have, the language's
        // font the rest (and, for a shaped script, the shaped glyphs of every text).
        let language = demon_queen_dungeon::i18n::language();
        let generation = demon_queen_dungeon::i18n::generation();
        let texts = demon_queen_dungeon::i18n::catalog_texts();
        let mut extra: Vec<char> = {
            let mut set = std::collections::BTreeSet::new();
            for t in &texts {
                for c in t.chars() {
                    if !c.is_control() && !characters.contains(&c) {
                        set.insert(c);
                    }
                }
            }
            set.into_iter().collect()
        };
        let script_source = if extra.is_empty() { None } else { script_font(&language) };
        let script_font_arc = script_source.as_ref().and_then(|(bytes, index, _)| {
            ab_glyph::FontVec::try_from_vec_and_index(bytes.as_ref().clone(), *index)
                .ok()
                .map(FontArc::new)
        });
        // Roughly how much room the glyphs need: a bigger atlas for a big script.
        // A small alphabet (Cyrillic, Hebrew…) drawn finer: the stamps blow its letters up big.
        let script_raster = if extra.len() > 600 {
            48.0
        } else if extra.len() > 220 {
            FONT_RASTER_SIZE
        } else {
            FONT_RASTER_SIZE * 1.5
        };
        let estimate = fonts.len() as f32 * (characters.len() + extra.len().min(400)) as f32 * 4200.0
            + extra.len() as f32 * (script_raster + 22.0).powi(2) * 1.2;
        let atlas_size: u32 = if estimate > 15_000_000.0 { 8192 } else { 4096 };
        let mut pixels = vec![0_u8; (atlas_size * atlas_size) as usize];
        extra.retain(|c| !demon_queen_dungeon::i18n::needs_shaping(*c) || script_source.as_ref().is_some_and(|s| !s.2));
        // Room around each glyph so the smaller mips do not bleed into their neighbours.
        let padding = 10_u32;
        let mut pen_x = padding;
        let mut pen_y = padding;
        let mut row_height = 0_u32;

        let mut faces = Vec::new();
        for font in fonts {
            let scaled = font.as_scaled(scale);
            let ascent = scaled.ascent();
            let mut glyphs = HashMap::new();
            let own: Vec<char> = characters
                .iter()
                .copied()
                .chain(extra.iter().copied().filter(|c| font.glyph_id(*c).0 != 0))
                .collect();
            for &character in &own {
                let glyph_id = font.glyph_id(character);
                let advance = scaled.h_advance(glyph_id);
                let glyph = glyph_id.with_scale_and_position(scale, point(0.0, 0.0));
                let Some(outlined) = font.outline_glyph(glyph) else {
                    glyphs.insert(
                        character,
                        GlyphEntry {
                            uv_min: [0.0; 2],
                            uv_max: [0.0; 2],
                            size: [0.0; 2],
                            bearing: [0.0; 2],
                            advance,
                        },
                    );
                    continue;
                };
                let bounds = outlined.px_bounds();
                let ink_w = (bounds.max.x - bounds.min.x).ceil().max(1.0) as u32;
                let ink_h = (bounds.max.y - bounds.min.y).ceil().max(1.0) as u32;
                // The glyph's field reaches FONT_SPREAD past its ink on every side.
                let width = ink_w + 2 * FONT_SPREAD;
                let height = ink_h + 2 * FONT_SPREAD;
                if pen_x + width + padding > atlas_size {
                    pen_x = padding;
                    pen_y += row_height + padding;
                    row_height = 0;
                }
                if pen_y + height + padding > atlas_size {
                    eprintln!("[fonts] atlas full");
                    break;
                }
                let mut coverage = vec![0.0_f32; (width * height) as usize];
                outlined.draw(|x, y, c| {
                    let i = ((y + FONT_SPREAD) * width + x + FONT_SPREAD) as usize;
                    if let Some(cell) = coverage.get_mut(i) {
                        *cell = c.clamp(0.0, 1.0);
                    }
                });
                let field = distance_field(&coverage, width as usize, height as usize, FONT_SPREAD as f32);
                for y in 0..height {
                    for x in 0..width {
                        let index = ((pen_y + y) * atlas_size + pen_x + x) as usize;
                        pixels[index] = field[(y * width + x) as usize];
                    }
                }
                let bounds = ab_glyph::Rect {
                    min: point(
                        bounds.min.x - FONT_SPREAD as f32,
                        bounds.min.y - FONT_SPREAD as f32,
                    ),
                    max: bounds.max,
                };
                glyphs.insert(
                    character,
                    GlyphEntry {
                        uv_min: [
                            pen_x as f32 / atlas_size as f32,
                            pen_y as f32 / atlas_size as f32,
                        ],
                        uv_max: [
                            (pen_x + width) as f32 / atlas_size as f32,
                            (pen_y + height) as f32 / atlas_size as f32,
                        ],
                        size: [width as f32, height as f32],
                        bearing: [bounds.min.x, bounds.min.y],
                        advance,
                    },
                );
                pen_x += width + padding;
                row_height = row_height.max(height);
            }
            faces.push(FontFace { glyphs, ascent });
        }
        // The language's font: by character what no face drew, by glyph id what is shaped.
        let mut script = None;
        if let (Some((bytes, index, shaped)), Some(sfont)) = (script_source, script_font_arc) {
            let height_units = sfont.height_unscaled();
            let up = FONT_RASTER_SIZE / script_raster;
            // The field spans as much of a letter as the faces' do (their rims read the same).
            let spread = (FONT_SPREAD as f32 / up).round() as u32;
            let sscale = PxScale::from(script_raster);
            let full = sfont.as_scaled(scale);
            let mut by_char = HashMap::new();
            let mut by_id = HashMap::new();
            let mut jobs: Vec<(Option<char>, ab_glyph::GlyphId)> = Vec::new();
            for &c in &extra {
                let id = sfont.glyph_id(c);
                if id.0 != 0 && !faces.iter().all(|f| f.glyphs.contains_key(&c)) {
                    jobs.push((Some(c), id));
                }
            }
            if shaped {
                let mut ids = std::collections::BTreeSet::new();
                let base: String = (32_u8..=126_u8).map(char::from).collect();
                for t in texts.iter().map(String::as_str).chain([base.as_str()]) {
                    for line in t.split('\n') {
                        for (id, ..) in shape_line(&bytes, index, height_units, line) {
                            ids.insert(id);
                        }
                    }
                }
                jobs.extend(ids.into_iter().map(|id| (None, ab_glyph::GlyphId(id))));
            }
            for (character, glyph_id) in jobs {
                let advance = full.h_advance(glyph_id);
                let glyph = glyph_id.with_scale_and_position(sscale, point(0.0, 0.0));
                let entry = match sfont.outline_glyph(glyph) {
                    None => GlyphEntry { uv_min: [0.0; 2], uv_max: [0.0; 2], size: [0.0; 2], bearing: [0.0; 2], advance },
                    Some(outlined) => {
                        let bounds = outlined.px_bounds();
                        let ink_w = (bounds.max.x - bounds.min.x).ceil().max(1.0) as u32;
                        let ink_h = (bounds.max.y - bounds.min.y).ceil().max(1.0) as u32;
                        let width = ink_w + 2 * spread;
                        let height = ink_h + 2 * spread;
                        if pen_x + width + padding > atlas_size {
                            pen_x = padding;
                            pen_y += row_height + padding;
                            row_height = 0;
                        }
                        if pen_y + height + padding > atlas_size {
                            eprintln!("[fonts] atlas full ({language})");
                            break;
                        }
                        let mut coverage = vec![0.0_f32; (width * height) as usize];
                        outlined.draw(|x, y, c| {
                            let i = ((y + FONT_SPREAD) * width + x + FONT_SPREAD) as usize;
                            if let Some(cell) = coverage.get_mut(i) {
                                *cell = c.clamp(0.0, 1.0);
                            }
                        });
                        let field = distance_field(&coverage, width as usize, height as usize, FONT_SPREAD as f32);
                        for y in 0..height {
                            for x in 0..width {
                                pixels[((pen_y + y) * atlas_size + pen_x + x) as usize] = field[(y * width + x) as usize];
                            }
                        }
                        let entry = GlyphEntry {
                            uv_min: [pen_x as f32 / atlas_size as f32, pen_y as f32 / atlas_size as f32],
                            uv_max: [(pen_x + width) as f32 / atlas_size as f32, (pen_y + height) as f32 / atlas_size as f32],
                            size: [width as f32 * up, height as f32 * up],
                            bearing: [(bounds.min.x - FONT_SPREAD as f32) * up, (bounds.min.y - FONT_SPREAD as f32) * up],
                            advance,
                        };
                        pen_x += width + padding;
                        row_height = row_height.max(height);
                        entry
                    }
                };
                match character {
                    Some(c) => {
                        by_char.insert(c, entry);
                    }
                    None => {
                        by_id.insert(glyph_id.0, entry);
                    }
                }
            }
            eprintln!(
                "[fonts] {language}: {} glyphs by character, {} shaped, atlas {atlas_size}",
                by_char.len(),
                by_id.len()
            );
            script = Some(ScriptFace { glyphs: by_char, ids: by_id, bytes, index, height_units, shaped });
        }
        let primary = faces.remove(0);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("demon-queen-map-font-atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: FONT_ATLAS_MIPS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // The glyphs and their mips (each a 2×2 average of the one above), so text drawn small
        // stays smooth instead of shimmering.
        let mut level = pixels;
        let mut side = atlas_size;
        for mip in 0..FONT_ATLAS_MIPS {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &level,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(side),
                    rows_per_image: Some(side),
                },
                wgpu::Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: 1,
                },
            );
            if mip + 1 < FONT_ATLAS_MIPS {
                let next = side / 2;
                let mut smaller = vec![0_u8; (next * next) as usize];
                for y in 0..next {
                    for x in 0..next {
                        let at = |dx: u32, dy: u32| {
                            level[((y * 2 + dy) * side + x * 2 + dx) as usize] as u32
                        };
                        smaller[(y * next + x) as usize] =
                            ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                    }
                }
                level = smaller;
                side = next;
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-map-font-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demon-queen-map-font-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Self {
            bind_group,
            glyphs: primary.glyphs,
            ascent: primary.ascent,
            shop_faces: faces,
            script,
            generation,
            runs: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

fn load_ui_font() -> FontArc {
    let mut candidates = Vec::<PathBuf>::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        candidates.push(fonts.join("seguisb.ttf"));
        candidates.push(fonts.join("segoeuib.ttf"));
        candidates.push(fonts.join("segoeui.ttf"));
        candidates.push(fonts.join("arialbd.ttf"));
        candidates.push(fonts.join("arial.ttf"));
    }
    candidates.extend([
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf"),
    ]);
    for path in candidates {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if let Ok(font) = FontArc::try_from_vec(bytes) {
            return font;
        }
    }
    panic!("no system sans-serif font available for the navigation map");
}

/// A font shipped with the game (`data/fonts`), else the interface's own.
fn load_game_font(file: &str) -> FontArc {
    let path = crate::exploration3d_state::data_dir()
        .join("fonts")
        .join(file);
    std::fs::read(&path)
        .ok()
        .and_then(|b| FontArc::try_from_vec(b).ok())
        .unwrap_or_else(|| {
            eprintln!("[fonts] missing {}", path.display());
            load_ui_font()
        })
}

fn load_shop_font(windows: &str, linux: &str) -> FontArc {
    // Load installed typefaces without distributing proprietary font files.
    let mut paths = Vec::new();
    if let Some(dir) = std::env::var_os("WINDIR") {
        paths.push(PathBuf::from(dir).join("Fonts").join(windows));
    }
    paths.push(PathBuf::from("/usr/share/fonts/truetype/liberation2").join(linux));
    paths.push(PathBuf::from("/usr/share/fonts/truetype/liberation").join(linux));
    paths
        .into_iter()
        .find_map(|p| {
            std::fs::read(p)
                .ok()
                .and_then(|b| FontArc::try_from_vec(b).ok())
        })
        .unwrap_or_else(load_ui_font)
}

/// One achievement as the achievements screen shows it.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AchievementRow {
    pub title: String,
    pub detail: String,
    /// What it gives ("Atuendo: Bikini · +50 oro"); empty when nothing.
    pub reward: String,
    pub done: bool,
    pub icon: Option<prompt_icons::Icon>,
}

/// The achievements screen: every achievement, and a few counts of what she has done.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct AchievementsView {
    pub rows: Vec<AchievementRow>,
    pub stats: Vec<(String, String)>,
}

/// The wardrobe's outfits as the catalog names them, and which are not earned yet.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct WardrobeView {
    pub outfits: Vec<(String, String)>,
    pub locked: Vec<bool>,
    /// The Queen in each outfit (her status cut-out), when there is one.
    pub pictures: Vec<Option<PortraitRef>>,
}

pub struct MapOverlay {
    /// Nothing drawn at all (a frame baked into a map has no HUD).
    pub hidden: bool,
    /// She is in el Peñascal (its map, not the wood's).
    pub penascal: bool,
    /// The followed mission: its panel, its way, its mark, the notices.
    missions: Option<MissionView>,
    /// The places to go in on the screen this frame.
    markers: Vec<WorldMarker>,
    /// The fight with Varrak's HUD (`None`: no fight).
    boss: Option<BossView>,
    /// Open: the achievements screen (over everything else).
    achievements: Option<AchievementsView>,
    wardrobe_view: WardrobeView,
    /// A conversation's box at the foot of the screen (a guion playing in the world).
    dialogue: Option<DialogueView>,
    dialogue_age: f32,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    vertex_count: u32,
    last_signature: Option<MapSignature>,
    font: FontAtlas,
    /// Kept to make the font atlas again when the language changes.
    font_layout: wgpu::BindGroupLayout,
    icons: IconAtlas,
    portraits: PortraitAtlas,
    /// The place's picture under the map's marks.
    pictures: MapPictures,
    /// The bed scene playing in the middle of the screen (over everything else).
    scene: Option<SceneView>,
    scene_picture: scene_overlay::ScenePicture,
    liquid_physics: shop_preview::LiquidPhysics,
    /// Rosalía's elixir going down: the 2D layer over its close shot.
    drink: Option<DrinkView>,
    /// Chiquita's comments floating on the right.
    floats: Vec<FloatLine>,
    /// Her own bed's menu.
    alcove: Option<AlcoveView>,
    /// The hypnotist's stall in the wood.
    mage: Option<MageView>,
    /// The shop's page, its haggle and its altar.
    shop: Option<ShopView>,
    /// Chiquita in 3D over everything (made the first time she shows), where she is, and the
    /// layer drawn over the rest with her, her glass and her words.
    mascot: Option<mascot::Mascot>,
    mascot_tried: bool,
    mascot_view: Option<MascotView>,
    picture_layout: wgpu::BindGroupLayout,
    top_buffer: wgpu::Buffer,
    top_capacity: usize,
    top_count: u32,
    /// A guion scene played full screen (`None`: no stage up).
    stage: Option<StageView>,
    stage_pictures: stage_overlay::StagePictures,
    stage_buffer: wgpu::Buffer,
    stage_capacity: usize,
    /// The stage pass's runs: picture, over the shapes, first vertex, count.
    stage_runs: Vec<(String, bool, u32, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MapSignature {
    width: u32,
    height: u32,
    player_x: u32,
    player_z: u32,
    player_yaw: u32,
    nature_mode: bool,
    village_entered: bool,
    expanded: bool,
    wardrobe_open: bool,
    wardrobe_selection: u8,
    outfit: u8,
    shop_open: bool,
    venue_hash: u64,
    shop_tab: u8,
    shop_category: super::shop::Category,
    shop_selection: usize,
    shop_quantity: u32,
    gold: u32,
    inventory_hash: u64,
    shop_stock_hash: u64,
    shop_notice_hash: u64,
    shop_hover_code: u16,
    shop_anim_tick: u32,
    shop_preview_rotation: Option<(u32, u32)>,
    speech_hash: u64,
    prompt_hash: u64,
    map_hover: u8,
    brothel_level: Option<(u8, bool)>,
    baths: bool,
    penascal: bool,
    /// The achievements screen and the wardrobe's names and locks.
    modal_hash: u64,
    /// While a conversation or a scene is up its ornaments move: every frame is new.
    ui_anim_tick: u32,
}

impl MapOverlay {
    /// Opens the achievements screen with this content (`None` closes it).
    pub fn set_achievements(&mut self, view: Option<AchievementsView>) {
        self.achievements = view;
    }

    /// The wardrobe's outfit names and locks.
    pub fn set_wardrobe(&mut self, view: WardrobeView) {
        self.wardrobe_view = view;
    }

    /// The followed mission's panel, way and mark (`None`: none open).
    pub(crate) fn set_markers(&mut self, markers: Vec<WorldMarker>) {
        self.markers = markers;
    }

    pub fn set_missions(&mut self, view: Option<MissionView>) {
        self.missions = view;
    }

    /// The fight's HUD (`None`: no fight).
    pub fn set_boss(&mut self, view: Option<BossView>) {
        self.boss = view;
    }

    /// A guion scene on the full-screen stage (`None`: none).
    pub fn set_stage(&mut self, view: Option<StageView>) {
        self.stage = view;
    }

    /// The conversation's box (`None`: no one is talking).
    pub fn set_dialogue(&mut self, view: Option<DialogueView>) {
        self.dialogue = view;
    }

    /// How long the conversation's line has been up (seconds; its box and cards come in by it).
    pub fn set_dialogue_age(&mut self, age: f32) {
        self.dialogue_age = age;
    }

    /// The bed scene's panel (`None`: no scene playing).
    pub fn set_scene(&mut self, view: Option<SceneView>) {
        self.scene = view;
    }

    /// The elixir's close shot, its 2D layer (`None`: she is not drinking).
    pub fn set_drink(&mut self, view: Option<DrinkView>) {
        self.drink = view;
    }

    /// Chiquita's comments in the air.
    pub fn set_floats(&mut self, floats: Vec<FloatLine>) {
        self.floats = floats;
    }

    /// Her own bed's menu (`None`: not in bed, or something plays over her).
    pub fn set_mage(&mut self, view: Option<MageView>) {
        self.mage = view;
    }

    /// The village shop beyond its shelves (`None`: closed, or Rosalía's book is open).
    pub fn set_shop(&mut self, view: Option<ShopView>) {
        self.shop = view;
    }

    pub fn set_alcove(&mut self, view: Option<AlcoveView>) {
        self.alcove = view;
    }

    /// Chiquita over the screen (`None`: she is not out).
    pub fn set_mascot(&mut self, view: Option<MascotView>) {
        self.mascot_view = view;
    }

    /// Her last 3D picture (width, height, sRGB bytes premultiplied by her coverage), for cutting
    /// her portraits from (`None` before she has been drawn).
    pub fn mascot_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<u8>)> {
        self.mascot.as_ref()?.snapshot(device, queue)
    }

    /// The scene's next frame (RGBA8), shown from the next draw.
    pub fn upload_scene_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        self.scene_picture
            .upload(device, queue, width, height, rgba);
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("demon-queen-map-overlay-shader"),
            source: wgpu::ShaderSource::Wgsl(MAP_SHADER.into()),
        });
        let font_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("demon-queen-map-font-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let icon_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("demon-queen-shop-icon-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("demon-queen-map-overlay-layout"),
            bind_group_layouts: &[&font_layout, &icon_layout, &icon_layout, &icon_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("demon-queen-map-overlay-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MapVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                        2 => Float32x4,
                        3 => Float32,
                        4 => Float32x4,
                        5 => Float32x4,
                        6 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let vertex_capacity = 256;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demon-queen-map-overlay-vertices"),
            size: (vertex_capacity * std::mem::size_of::<MapVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let font = FontAtlas::new(device, queue, &font_layout);
        // The shop's art (and the venue's, beside it in assets/ui).
        let shop_asset_root = crate::exploration3d_state::game_dir().join("assets/ui/shop");
        let icons = IconAtlas::new(device, queue, &icon_layout, &shop_asset_root);
        let portraits = PortraitAtlas::new(device, &icon_layout);
        let pictures = MapPictures::new(device, queue, &icon_layout);
        Self {
            hidden: false,
            penascal: false,
            missions: None,
            markers: Vec::new(),
            boss: None,
            pipeline,
            vertex_buffer,
            vertex_capacity,
            vertex_count: 0,
            last_signature: None,
            achievements: None,
            wardrobe_view: WardrobeView::default(),
            dialogue: None,
            dialogue_age: 0.0,
            font,
            font_layout,
            icons,
            portraits,
            pictures,
            scene: None,
            scene_picture: scene_overlay::ScenePicture::new(device, &icon_layout),
            liquid_physics: shop_preview::LiquidPhysics::default(),
            drink: None,
            floats: Vec::new(),
            alcove: None,
            mage: None,
            shop: None,
            mascot: None,
            mascot_tried: false,
            mascot_view: None,
            picture_layout: icon_layout.clone(),
            top_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("demon-queen-map-overlay-top"),
                size: (256 * std::mem::size_of::<MapVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            top_capacity: 256,
            top_count: 0,
            stage: None,
            stage_pictures: stage_overlay::StagePictures::new(device, &icon_layout),
            stage_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("demon-queen-map-overlay-stage"),
                size: (256 * std::mem::size_of::<MapVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            stage_capacity: 256,
            stage_runs: Vec::new(),
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        player: Vec3,
        player_yaw: f32,
        nature_mode: bool,
        village_entered: bool,
        expanded: bool,
        wardrobe_open: bool,
        wardrobe_selection: u8,
        outfit: u8,
        shop_open: bool,
        shop_tab: super::shop::Tab,
        shop_category: super::shop::Category,
        shop_selection: usize,
        shop_quantity: u32,
        gold: u32,
        inventory: &BTreeMap<String, u32>,
        shop_stock: &BTreeMap<String, u32>,
        shop_notice: Option<&str>,
        shop_pointer: Option<(f32, f32)>,
        ui_time: f32,
        shop_preview_rotation: Option<(f32, f32)>,
        speech_bubbles: &[NpcSpeechBubble],
        venue: Option<&super::venue::Menu>,
        action_prompt: Option<&prompt_icons::Prompt>,
        brothel_level: Option<(u8, bool)>,
        baths: bool,
    ) {
        // QA: `VELVET_NO_HUD=1` draws the world alone (pictures of the places for the guide).
        if self.hidden || std::env::var("VELVET_NO_HUD").is_ok_and(|v| v == "1") {
            return;
        }
        let _timer = CpuTimer(std::time::Instant::now());
        // Another language: its glyphs (another script, maybe shaped) in a new atlas.
        if self.font.generation != demon_queen_dungeon::i18n::generation() {
            self.font = FontAtlas::new(device, queue, &self.font_layout);
            self.last_signature = None;
        }
        let width = size.0.max(1) as f32;
        let height = size.1.max(1) as f32;
        // Chiquita's picture for this frame (her renderer made the first time she shows).
        let mascot_shown = self.mascot_view.as_ref().is_some_and(MascotView::shown);
        if mascot_shown && self.mascot.is_none() && !self.mascot_tried {
            self.mascot_tried = true;
            self.mascot = mascot::Mascot::new(device, queue, &self.picture_layout);
        }
        let mascot_on = mascot_shown && self.mascot.is_some();
        if let (true, Some(m), Some(view)) =
            (mascot_on, self.mascot.as_mut(), self.mascot_view.as_ref())
        {
            m.render(device, queue, view);
        }
        let shop_hover = if shop_open {
            let mut pick = shop_pick(
                shop_tab,
                shop_category,
                shop_selection,
                inventory,
                self.shop.as_ref().map_or(0, |v| v.page),
                self.shop
                    .as_ref()
                    .and_then(|v| v.fuse.as_ref())
                    .map_or(0, |f| f.items.len()),
                self.shop
                    .as_ref()
                    .and_then(|v| v.fuse.as_ref())
                    .map_or(0, |f| f.cursor),
            );
            if let Some(fv) = self
                .shop
                .as_ref()
                .and_then(|v| v.haggle.as_ref())
                .and_then(|h| h.favor.as_ref())
            {
                pick.favor = true;
                pick.favor_choices = fv.choices;
            }
            shop_pointer.and_then(|(x, y)| shop_hit_test(x, y, &pick))
        } else {
            None
        };
        // Rosalía's offer turns the elixir on its stage too.
        let offer_open =
            venue.is_some_and(|menu| menu.open && menu.page == super::venue::Page::Offer);
        let liquid_preview_active = std::env::var_os("VELVET_REDUCED_MOTION").is_none()
            && (offer_open
                || (shop_open
                    && !venue.is_some_and(|menu| menu.open)
                    && super::shop::visible_indices(shop_tab, shop_category, inventory)
                        .get(shop_selection)
                        .and_then(|&i| super::shop::catalog().products[i].preview_model.as_deref())
                        .is_some_and(shop_preview::has_animated_potion_liquid)));
        let liquid_motion = if liquid_preview_active {
            self.liquid_physics.update(
                ui_time,
                Some(shop_preview::display_rotation(
                    ui_time,
                    if offer_open {
                        None
                    } else {
                        shop_preview_rotation
                    },
                )),
            )
        } else {
            self.liquid_physics.reset();
            shop_preview::LiquidMotion::default()
        };
        // The Close button and the minimap answer the pointer.
        let map_hover = if shop_open || wardrobe_open || venue.is_some_and(|menu| menu.open) {
            None
        } else {
            shop_pointer.and_then(|(x, y)| map_kit::hit_test(width, height, expanded, x, y))
        };
        // The scene's speakers' faces, too.
        let scene_faces: Vec<Option<([f32; 2], [f32; 2])>> = match &self.scene {
            Some(view) => view
                .lines
                .iter()
                .map(|l| {
                    l.portrait
                        .as_ref()
                        .and_then(|p| self.portraits.ensure(queue, p))
                })
                .collect(),
            None => Vec::new(),
        };
        let quote_face = self
            .scene
            .as_ref()
            .and_then(|view| view.result.as_ref())
            .and_then(|result| result.quote.portrait.clone())
            .and_then(|p| self.portraits.ensure(queue, &p));
        // A favour of the shop: the shopkeeper's face of the moment (and the one fading out), and
        // what the pointer is on.
        let favor_faces = match self.scene.as_ref().and_then(|view| view.deal.as_ref()) {
            Some(deal) => (
                deal.face
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p)),
                deal.face_from
                    .clone()
                    .filter(|_| deal.face_ms < 400)
                    .and_then(|p| self.portraits.ensure(queue, &p)),
            ),
            None => (None, None),
        };
        let favor_hover = self
            .scene
            .as_ref()
            .filter(|view| view.deal.is_some())
            .and_then(|view| {
                shop_pointer.and_then(|(x, y)| shop_overlay::favor_hit(x, y, view.result.is_some()))
            });
        // A minigame's girl, for its rules card.
        let game_face = self
            .scene
            .as_ref()
            .and_then(|view| view.game.as_ref())
            .and_then(|game| game.face.clone())
            .and_then(|p| self.portraits.ensure(queue, &p));
        // Rosalía's book: each girl's face, and Rosalía's own.
        let book_face = |portraits: &mut PortraitAtlas, who: &str| {
            super::guion::portraits()
                .find(who, None)
                .map(|p| PortraitRef {
                    image: p.image,
                    crop: p.crop,
                })
                .and_then(|p| portraits.ensure(queue, &p))
        };
        let (venue_faces, host_face) = match venue.filter(|menu| menu.open) {
            Some(menu) => (
                menu.entries()
                    .iter()
                    .map(|e| book_face(&mut self.portraits, &e.id))
                    .collect::<Vec<_>>(),
                book_face(&mut self.portraits, "rosalia"),
            ),
            None => (Vec::new(), None),
        };
        // Chiquita's face and the Queen's for her bed's menu.
        // The hypnotist's figure and the Queen's face for his stall.
        let mage_faces = match self.mage.as_ref() {
            Some(view) => (
                view.mage
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p)),
                view.queen
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p)),
            ),
            None => (None, None),
        };
        // The wardrobe's Queens, one per outfit.
        let wardrobe_faces: Vec<Option<([f32; 2], [f32; 2])>> = if wardrobe_open {
            self.wardrobe_view
                .pictures
                .clone()
                .iter()
                .map(|p| p.as_ref().and_then(|p| self.portraits.ensure(queue, p)))
                .collect()
        } else {
            Vec::new()
        };
        let shop_face = self
            .shop
            .as_ref()
            .and_then(|view| view.merchant.clone())
            .and_then(|p| self.portraits.ensure(queue, &p));
        // The shopkeeper's face before the one on show, while the new one fades in over it.
        let shop_face_from = self
            .shop
            .as_ref()
            .filter(|view| view.merchant_ms < 400)
            .and_then(|view| view.merchant_from.clone())
            .and_then(|p| self.portraits.ensure(queue, &p));
        let alcove_faces = match self.alcove.as_ref() {
            Some(view) => (
                view.chiquita
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p)),
                view.queen
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p)),
            ),
            None => (None, None),
        };
        // Chiquita's face beside her comments.
        let float_faces: Vec<Option<([f32; 2], [f32; 2])>> = self
            .floats
            .iter()
            .map(|f| {
                f.portrait
                    .clone()
                    .and_then(|p| self.portraits.ensure(queue, &p))
            })
            .collect();
        // The speaker's portrait is in the atlas before the frame is built.
        let portrait_uv = match self.dialogue.as_ref().and_then(|d| d.portrait.clone()) {
            Some(p) => self.portraits.ensure(queue, &p),
            None => None,
        };
        let modal_hash = {
            let mut h = DefaultHasher::new();
            self.achievements.hash(&mut h);
            self.wardrobe_view.hash(&mut h);
            self.dialogue.hash(&mut h);
            self.scene.hash(&mut h);
            self.drink.hash(&mut h);
            self.floats.hash(&mut h);
            self.alcove.hash(&mut h);
            self.mage.hash(&mut h);
            if self.mage.is_some() {
                shop_pointer
                    .map(|(x, y)| (x.to_bits(), y.to_bits()))
                    .hash(&mut h);
            }
            self.mascot_view.hash(&mut h);
            self.missions.hash(&mut h);
            self.boss.hash(&mut h);
            h.finish()
        };
        let signature = MapSignature {
            modal_hash,
            venue_hash: {
                let mut h = DefaultHasher::new();
                venue.hash(&mut h);
                if venue.is_some_and(|menu| menu.open) {
                    shop_pointer
                        .map(|(x, y)| (x.to_bits(), y.to_bits()))
                        .hash(&mut h);
                }
                h.finish()
            },
            width: size.0.max(1),
            height: size.1.max(1),
            // The player pin and nearby-place distances need sub-metre
            // updates, not a full geometry rebuild for every float bit change.
            // This keeps the vector atlas light while the retained marker
            // still moves smoothly at map scale.
            player_x: map_signature_step(player.x, 0.075),
            player_z: map_signature_step(player.z, 0.075),
            player_yaw: map_signature_step(player_yaw, 0.04),
            nature_mode,
            village_entered,
            expanded,
            wardrobe_open,
            wardrobe_selection,
            outfit,
            shop_open,
            shop_tab: shop_tab.as_u8(),
            shop_category,
            shop_selection,
            shop_quantity,
            gold,
            inventory_hash: hash_inventory(inventory),
            shop_stock_hash: hash_inventory(shop_stock),
            shop_notice_hash: hash_notice(shop_notice),
            shop_hover_code: shop_hit_signature(shop_hover),
            shop_anim_tick: if shop_open
                && (venue.is_some_and(|menu| menu.open)
                    || (std::env::var_os("VELVET_REDUCED_MOTION").is_none()
                        && super::shop::visible_indices(shop_tab, shop_category, inventory)
                            .get(shop_selection)
                            .is_some_and(|&i| {
                                super::shop::catalog().products[i].preview_model.is_some()
                            }))) {
                (ui_time * 30.0) as u32
            } else {
                0
            },
            shop_preview_rotation: shop_preview_rotation
                .map(|(yaw, pitch)| (yaw.to_bits(), pitch.to_bits())),
            speech_hash: speech_hash(speech_bubbles),
            prompt_hash: prompt_hash(action_prompt),
            map_hover: map_kit::hit_signature(map_hover),
            brothel_level,
            baths,
            penascal: self.penascal,
            ui_anim_tick: if self.scene.is_some()
                || self.missions.as_ref().is_some_and(MissionView::animating)
                || self.boss.is_some()
                || self.stage.is_some()
                || self.dialogue.is_some()
                || self.drink.is_some()
                || self.alcove.is_some()
                || self.mage.is_some()
                || self.shop.is_some()
                || self.mascot_view.is_some()
                || !self.floats.is_empty()
                || !self.markers.is_empty()
                || venue.is_some_and(|menu| menu.open)
            {
                (ui_time * 60.0) as u32
            } else {
                0
            },
        };
        let font = &self.font;
        let icons = &self.icons;
        if self.last_signature != Some(signature) {
            let mut vertices = Vec::with_capacity(if expanded { 24_000 } else { 8_000 });
            // The stage's pictures (loaded the first time each shows), in runs by picture.
            let mut stage_vertices: Vec<MapVertex> = Vec::new();
            self.stage_runs.clear();
            if let Some(view) = self.stage.as_ref() {
                // The scene's pictures load ahead; what has arrived goes up a few a frame.
                self.stage_pictures.prefetch(&view.preload);
                self.stage_pictures.pump(device, queue);
                let mut names: Vec<&str> = Vec::new();
                names.extend(view.bg_prev.as_deref());
                names.extend(view.bg.as_deref());
                for a in &view.actors {
                    names.push(&a.image);
                    names.extend(a.prev.as_deref());
                    if let Some((frames, _, before)) = &a.spin {
                        names.extend(frames.iter().map(String::as_str));
                        names.push(before);
                    }
                }
                names.extend(view.panels.iter().flatten().map(|p| p.image.as_str()));
                names.extend(view.reveal.as_ref().map(|(p, _)| p.as_str()));
                names.extend(view.item.as_ref().map(|i| i.icon.as_str()));
                for name in names {
                    self.stage_pictures.ensure(device, queue, name);
                }
                let pictures = &self.stage_pictures;
                for run in stage_overlay::pictures(view, &|n| pictures.info(n), width, height) {
                    let start = stage_vertices.len() as u32;
                    stage_vertices.extend(run.vertices);
                    self.stage_runs.push((
                        run.image,
                        run.over,
                        start,
                        stage_vertices.len() as u32 - start,
                    ));
                }
            }
            if let Some(view) = &self.stage {
                // The stage looks as the web scenes do: its own box, cards and flourishes.
                stage_overlay::draw_ui(font, &mut vertices, width, height, view, ui_time);
                // While a beat plays (a title, a reveal, a turn, an item shown off) the box waits.
                if let Some(d) = self.dialogue.as_ref().filter(|_| !view.waiting) {
                    stage_overlay::draw_box(
                        font,
                        &mut vertices,
                        width,
                        height,
                        d,
                        portrait_uv,
                        view.line_age,
                        view.box_age,
                        ui_time,
                        mascot_on,
                        view.theme,
                    );
                }
                stage_overlay::draw_top(font, &mut vertices, width, height, view, ui_time);
            } else if let Some(view) = &self.scene {
                match view.deal.as_ref() {
                    // A favour of the shop plays as a page of the shop.
                    Some(deal) => shop_overlay::draw_favor(
                        font,
                        icons,
                        &mut vertices,
                        view,
                        deal,
                        favor_faces.0,
                        favor_faces.1,
                        self.scene_picture.has_frame(),
                        favor_hover,
                        ui_time,
                    ),
                    None => scene_overlay::draw_scene(
                        font,
                        &mut vertices,
                        width,
                        height,
                        view,
                        &scene_faces,
                        quote_face,
                        game_face,
                        self.scene_picture.has_frame(),
                        ui_time,
                    ),
                }
            } else if let Some(view) = &self.drink {
                drink_overlay::draw(font, &mut vertices, width, height, view, ui_time);
            } else if let Some(view) = &self.mage {
                mage_overlay::draw(
                    font,
                    &mut vertices,
                    width,
                    height,
                    view,
                    mage_faces.0,
                    mage_faces.1,
                    shop_pointer,
                    ui_time,
                );
            } else if let Some(view) = &self.alcove {
                alcove_overlay::draw(
                    font,
                    &mut vertices,
                    width,
                    height,
                    view,
                    alcove_faces.0,
                    alcove_faces.1,
                    ui_time,
                );
            } else if let Some(menu) = venue.filter(|v| v.open) {
                venue_overlay::draw(
                    font,
                    &mut vertices,
                    width,
                    height,
                    menu,
                    gold,
                    shop_pointer,
                    &venue_faces,
                    host_face,
                    ui_time,
                    liquid_motion,
                );
            } else if shop_open {
                let frame = shop_overlay::ShopFrame {
                    tab: shop_tab,
                    category: shop_category,
                    selection: shop_selection,
                    quantity: shop_quantity,
                    gold,
                    inventory,
                    stock: shop_stock,
                    notice: shop_notice,
                    hover: shop_hover,
                    time: ui_time,
                    preview_rotation: shop_preview_rotation,
                    liquid_motion,
                };
                shop_overlay::draw(
                    font,
                    icons,
                    &mut vertices,
                    &frame,
                    self.shop.as_ref(),
                    shop_face,
                    shop_face_from,
                );
            } else if let Some(view) = &self.boss {
                boss_overlay::draw(font, &mut vertices, width, height, view);
            } else if let Some(view) = &self.achievements {
                persona_screens::draw_achievements(font, &mut vertices, width, height, view, ui_time);
            } else if wardrobe_open {
                persona_screens::draw_wardrobe(
                    font,
                    &mut vertices,
                    width,
                    height,
                    wardrobe_selection,
                    outfit,
                    &self.wardrobe_view,
                    &wardrobe_faces,
                    ui_time,
                );
            } else {
                // Draw NPC speech first. The compact map panel is intentionally
                // composited over it if the two occupy the same screen area;
                // comments never become a second interactive UI layer.
                if !expanded {
                    if let Some(view) = &self.dialogue {
                        if classic_dialogue() {
                            dialogue_overlay::draw_dialogue(
                                font,
                                &mut vertices,
                                width,
                                height,
                                view,
                                portrait_uv,
                                ui_time,
                            );
                        } else {
                            // Every conversation wears the web scenes' look, as the stage does.
                            stage_overlay::draw_box(
                                font,
                                &mut vertices,
                                width,
                                height,
                                view,
                                portrait_uv,
                                self.dialogue_age,
                                self.dialogue_age,
                                ui_time,
                                false,
                                0,
                            );
                        }
                    } else {
                        for marker in &self.markers {
                            draw_world_marker(font, &mut vertices, marker, ui_time);
                        }
                        for layout in layout_npc_speech_bubbles(font, width, height, speech_bubbles)
                        {
                            draw_npc_speech_bubble(font, &mut vertices, &layout);
                        }
                        if let Some(view) = &self.missions {
                            mission_overlay::draw_marker(font, &mut vertices, view, ui_time);
                        }
                        if let Some(action) = action_prompt {
                            draw_action_prompt(font, &mut vertices, width, height, action);
                        }
                    }
                }
                // While someone is talking the compact map steps aside (the box and the
                // thought card need the screen); the big map, open on purpose, stays.
                if expanded || self.dialogue.is_none() {
                    let place = if brothel_level.is_some() {
                        Place::Brothel
                    } else if baths {
                        Place::Baths
                    } else if self.penascal {
                        Place::Penascal
                    } else if village_entered {
                        Place::Village
                    } else if nature_mode {
                        Place::Forest
                    } else {
                        Place::Castle
                    };
                    let (title, subtitle) = match place {
                        Place::Village => ("Mapa de la villa", "Aldea fantástica"),
                        Place::Forest => ("Mapa exterior", "Bosque Oscuro"),
                        Place::Castle => ("Mapa del Refugio", "Castillo de la Reina"),
                        Place::Baths => ("Mapa de las termas", "Baños termales"),
                        Place::Penascal => ("Mapa del Peñascal", "La aldea de los trasgos"),
                        Place::Brothel => (
                            "Mapa del burdel",
                            if brothel_level.is_some_and(|(level, _)| level == 1) {
                                "Planta alta · habitaciones"
                            } else {
                                "Planta baja · Corazón Carmesí"
                            },
                        ),
                    };
                    let frame = map_kit::draw_frame(
                        font,
                        &mut vertices,
                        width,
                        height,
                        expanded,
                        place,
                        title,
                        subtitle,
                        map_hover,
                    );
                    // The full map has a column each side of the chart.
                    let columns = if expanded {
                        Some(village_map_columns(frame.content))
                    } else {
                        None
                    };
                    let map_content = columns.map(|(_, map, _)| map).unwrap_or(frame.content);
                    let map_bounds = match place {
                        Place::Village => {
                            if expanded {
                                village_overview_bounds(super::fantasy_village_center())
                            } else {
                                // The compact map remains a nearby, player-centered
                                // radar; the expanded map switches to a complete
                                // cartographic view of the authored settlement.
                                focused_village_bounds(player, map_content)
                            }
                        }
                        Place::Forest => {
                            if expanded {
                                MapBounds {
                                    min_x: -27.5,
                                    max_x: 27.5,
                                    min_z: -27.5,
                                    max_z: 27.5,
                                }
                            } else {
                                map_places::focused_forest_bounds(player, map_content)
                            }
                        }
                        Place::Castle => map_places::castle_bounds(),
                        Place::Penascal => {
                            let [x0, x1, z0, z1] = super::penascal3d::plan().bounds();
                            if expanded {
                                // Flipped z: north (-z) up.
                                MapBounds {
                                    min_x: x0 - 1.0,
                                    max_x: x1 + 1.0,
                                    min_z: -z1 - 1.0,
                                    max_z: -z0 + 1.0,
                                }
                            } else {
                                let half_h = 13.0;
                                let half_w =
                                    half_h * (map_content.w / map_content.h.max(1.0)).max(0.35);
                                MapBounds {
                                    min_x: player.x - half_w,
                                    max_x: player.x + half_w,
                                    min_z: -player.z - half_h,
                                    max_z: -player.z + half_h,
                                }
                            }
                        }
                        Place::Baths => {
                            if expanded {
                                MapBounds {
                                    min_x: -22.0,
                                    max_x: 22.0,
                                    min_z: -20.0,
                                    max_z: 20.0,
                                }
                            } else {
                                let half_h = 11.0;
                                let half_w =
                                    half_h * (map_content.w / map_content.h.max(1.0)).max(0.35);
                                MapBounds {
                                    min_x: player.x - half_w,
                                    max_x: player.x + half_w,
                                    min_z: -player.z - half_h,
                                    max_z: -player.z + half_h,
                                }
                            }
                        }
                        Place::Brothel => {
                            let b = super::brothel::plan().bounds;
                            // Flipped z: north (-z) up, as the house is seen in the game.
                            MapBounds {
                                min_x: b.x0 - 0.6,
                                max_x: b.x1 + 0.6,
                                min_z: -b.z1 - 0.6,
                                max_z: -b.z0 + 0.6,
                            }
                        }
                    };
                    // The place's picture, baked from above, when there is one. With it the villa and
                    // the forest are drawn north up (their old drawing had +z up: a mirror image).
                    let picture_name = match place {
                        Place::Village => Some("villa"),
                        Place::Forest => Some("bosque"),
                        Place::Baths => Some("termas"),
                        Place::Castle => Some(if map_places::castle_level(player) == 1 {
                            "castillo-1"
                        } else {
                            "castillo-0"
                        }),
                        Place::Penascal => Some("penascal"),
                        Place::Brothel => None,
                    };
                    let picture =
                        picture_name.and_then(|name| self.pictures.ensure(device, queue, name));
                    let map_bounds =
                        if picture.is_some() && matches!(place, Place::Village | Place::Forest) {
                            MapBounds {
                                min_x: map_bounds.min_x,
                                max_x: map_bounds.max_x,
                                min_z: -map_bounds.max_z,
                                max_z: -map_bounds.min_z,
                            }
                        } else {
                            map_bounds
                        };
                    let map_rect = fit_map_rect(map_content, map_bounds);
                    match place {
                        Place::Village if picture.is_some() => {
                            draw_village_picture(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                picture.unwrap_or_default(),
                                player,
                                player_yaw,
                                expanded,
                            );
                            push_border(&mut vertices, map_rect, 1.5, [0.48, 0.68, 0.62, 0.88]);
                            if let Some((left, _, right)) = columns {
                                map_places::draw_village_orientation(
                                    font,
                                    &mut vertices,
                                    left,
                                    player,
                                );
                                map_places::draw_village_places(font, &mut vertices, right, player);
                            }
                        }
                        Place::Village => {
                            push_rect(&mut vertices, map_rect, [0.27, 0.19, 0.12, 0.96]);
                            push_border(&mut vertices, map_rect, 1.5, [0.48, 0.68, 0.62, 0.88]);
                            draw_village_map(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                player,
                                player_yaw,
                                expanded,
                            );
                            map_kit::vignette(&mut vertices, map_rect, 0.30);
                            if let Some((left, _, right)) = columns {
                                map_places::draw_village_orientation(
                                    font,
                                    &mut vertices,
                                    left,
                                    player,
                                );
                                map_places::draw_village_places(font, &mut vertices, right, player);
                            }
                        }
                        Place::Forest => {
                            match picture {
                                Some(picture) => map_places::draw_forest_picture(
                                    font,
                                    &mut vertices,
                                    map_rect,
                                    map_bounds,
                                    picture,
                                    player,
                                    player_yaw,
                                    expanded,
                                ),
                                // Without its baked picture: a plain panel.
                                None => {
                                    push_rect(&mut vertices, map_rect, [0.10, 0.14, 0.10, 0.96])
                                }
                            }
                            if let Some((left, _, right)) = columns {
                                map_places::draw_forest_orientation(
                                    font,
                                    &mut vertices,
                                    left,
                                    player,
                                );
                                map_places::draw_forest_places(font, &mut vertices, right, player);
                            }
                        }
                        Place::Brothel => {
                            let (level, rope_open) = brothel_level.unwrap_or((0, false));
                            map_places::draw_brothel(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                player,
                                player_yaw,
                                expanded,
                                level,
                                rope_open,
                            );
                            if let Some((left, _, right)) = columns {
                                map_places::draw_brothel_orientation(
                                    font,
                                    &mut vertices,
                                    left,
                                    player,
                                    level,
                                );
                                map_places::draw_brothel_places(
                                    font,
                                    &mut vertices,
                                    right,
                                    player,
                                    level,
                                    rope_open,
                                );
                            }
                        }
                        Place::Baths => {
                            map_places::draw_baths(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                player,
                                player_yaw,
                                expanded,
                                picture,
                            );
                        }
                        Place::Castle => {
                            map_places::draw_castle(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                player,
                                player_yaw,
                                expanded,
                                picture,
                            );
                            if let Some((left, _, right)) = columns {
                                map_places::draw_castle_orientation(
                                    font,
                                    &mut vertices,
                                    left,
                                    player,
                                );
                                map_places::draw_castle_rooms(font, &mut vertices, right, player);
                            }
                        }
                        Place::Penascal => {
                            map_places::draw_penascal(
                                font,
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                player,
                                player_yaw,
                                expanded,
                                picture,
                            );
                        }
                    }
                    // The followed mission's way over the map (the villa's old drawing is the
                    // only one with +z up), her pin again over it, and its panel under the
                    // corner map.
                    if let Some(view) = &self.missions {
                        let flip = !(place == Place::Village && picture.is_none());
                        mission_overlay::draw_route(
                            &mut vertices,
                            map_rect,
                            map_bounds,
                            view,
                            expanded,
                            flip,
                        );
                        if !view.route.is_empty() {
                            map_kit::player_pin(
                                &mut vertices,
                                map_rect,
                                map_bounds,
                                if flip {
                                    Vec3::new(player.x, player.y, -player.z)
                                } else {
                                    player
                                },
                                if flip {
                                    std::f32::consts::PI - player_yaw
                                } else {
                                    player_yaw
                                },
                                expanded,
                            );
                        }
                        if !expanded {
                            mission_overlay::draw_panel(font, &mut vertices, width, height, view);
                        }
                    }
                }
                if let Some(view) = &self.missions {
                    mission_overlay::draw_notice(font, &mut vertices, width, view, ui_time);
                }
            }

            // Over everything else: Chiquita, the glass she broke, her words.
            let mut top = Vec::new();
            let view = self.mascot_view.as_ref();
            if let (true, Some(view)) = (mascot_on, view) {
                mascot::quad(
                    &mut top,
                    width,
                    height,
                    view.alpha.min(1000) as f32 / 1000.0,
                );
            }
            if let Some(crack) = view.and_then(|v| v.crack.as_ref()) {
                glass_fx::draw(&mut top, crack, ui_time);
            }
            let anchor = view.filter(|_| mascot_on).map(|v| {
                (
                    [v.feet[0] as f32, v.feet[1] as f32],
                    v.height as f32 * v.squash[1] as f32 / 1000.0,
                )
            });
            float_overlay::draw(
                font,
                &mut top,
                width,
                height,
                &self.floats,
                &float_faces,
                anchor,
                ui_time,
            );
            // The blow shakes the screen for a moment.
            if let Some(crack) = view.and_then(|v| v.crack.as_ref()) {
                let age = crack.age_ms as f32 / 1000.0;
                if crack.fall_ms.is_none() && age < 0.32 {
                    let k = (1.0 - age / 0.32).powi(2) * 9.0;
                    let shake = [(age * 71.0).sin() * k, (age * 53.0).cos() * k * 0.7];
                    for vertex in vertices.iter_mut().chain(top.iter_mut()) {
                        vertex.position[0] += shake[0];
                        vertex.position[1] += shake[1];
                    }
                }
            }
            // Build every primitive in physical pixels first, then convert once
            // here to the clip-space coordinates consumed by the overlay shader.
            for vertex in vertices
                .iter_mut()
                .chain(top.iter_mut())
                .chain(stage_vertices.iter_mut())
            {
                vertex.position[0] = vertex.position[0] / width * 2.0 - 1.0;
                vertex.position[1] = 1.0 - vertex.position[1] / height * 2.0;
            }
            if stage_vertices.len() > self.stage_capacity {
                self.stage_capacity = stage_vertices.len().next_power_of_two();
                self.stage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("demon-queen-map-overlay-stage"),
                    size: (self.stage_capacity * std::mem::size_of::<MapVertex>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            if !stage_vertices.is_empty() {
                queue.write_buffer(&self.stage_buffer, 0, bytemuck::cast_slice(&stage_vertices));
            }
            if !vertices.is_empty() {
                self.ensure_capacity(device, vertices.len());
                queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            }
            self.vertex_count = vertices.len() as u32;
            if top.len() > self.top_capacity {
                self.top_capacity = top.len().next_power_of_two();
                self.top_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("demon-queen-map-overlay-top"),
                    size: (self.top_capacity * std::mem::size_of::<MapVertex>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            if !top.is_empty() {
                queue.write_buffer(&self.top_buffer, 0, bytemuck::cast_slice(&top));
            }
            self.top_count = top.len() as u32;
            self.last_signature = Some(signature);
        }
        if self.vertex_count == 0 && self.top_count == 0 && self.stage_runs.is_empty() {
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("demon-queen-map-overlay-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("demon-queen-map-overlay-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.font.bind_group, &[]);
            pass.set_bind_group(1, &self.icons.bind_group, &[]);
            pass.set_bind_group(2, &self.portraits.bind_group, &[]);
            // The stage's pictures under the shapes: each run with its own picture.
            for (image, over, start, count) in &self.stage_runs {
                if *over || *count == 0 {
                    continue;
                }
                if let Some(group) = self.stage_pictures.group(image) {
                    pass.set_bind_group(3, group, &[]);
                    pass.set_vertex_buffer(0, self.stage_buffer.slice(..));
                    pass.draw(*start..start + count, 0..1);
                }
            }
            pass.set_bind_group(
                3,
                if self.scene.is_some() {
                    &self.scene_picture.bind_group
                } else {
                    &self.pictures.bind_group
                },
                &[],
            );
            if self.vertex_count > 0 {
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..self.vertex_count, 0..1);
            }
            // The stage's pictures that sit on its cards, over the shapes.
            for (image, over, start, count) in &self.stage_runs {
                if !*over || *count == 0 {
                    continue;
                }
                if let Some(group) = self.stage_pictures.group(image) {
                    pass.set_bind_group(3, group, &[]);
                    pass.set_vertex_buffer(0, self.stage_buffer.slice(..));
                    pass.draw(*start..start + count, 0..1);
                }
            }
            // The top layer samples Chiquita's picture where the scene's would be.
            if self.top_count > 0 {
                if let (true, Some(m)) = (mascot_on, self.mascot.as_ref()) {
                    pass.set_bind_group(3, &m.picture_group, &[]);
                }
                pass.set_vertex_buffer(0, self.top_buffer.slice(..));
                pass.draw(0..self.top_count, 0..1);
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, vertex_count: usize) {
        if vertex_count <= self.vertex_capacity {
            return;
        }
        self.vertex_capacity = vertex_count.next_power_of_two();
        self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("demon-queen-map-overlay-vertices"),
            size: (self.vertex_capacity * std::mem::size_of::<MapVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}

fn panel_rect(width: f32, height: f32, expanded: bool) -> Rect {
    if expanded {
        let w = (width * 0.97).min(width - 24.0).max(360.0);
        let h = (height * 0.96).min(height - 24.0).max(280.0);
        Rect::new((width - w) * 0.5, (height - h) * 0.5, w, h)
    } else {
        let w = (width * 0.29).clamp(520.0, 620.0).min(width - 24.0);
        let h = (height * 0.40).clamp(430.0, 500.0).min(height - 24.0);
        Rect::new(width - w - 18.0, 18.0, w, h)
    }
}

/// The three outfits of the Queen's wardrobe: their name, a short line, and the
/// palette used to paint their figure on the card.
#[derive(Clone, Copy)]
struct OutfitLook {
    name: &'static str,
    detail: &'static str,
    dress: [f32; 4],
    dress_dark: [f32; 4],
    trim: [f32; 4],
    glow: [f32; 4],
}

/// The looks of `n` outfits: the two built in, then a rose one for each outfit with its own model.
fn outfit_looks_n(n: usize) -> Vec<OutfitLook> {
    let mut looks = outfit_looks().to_vec();
    while looks.len() < n.max(2) {
        looks.push(OutfitLook {
            name: "Atuendo",
            detail: "Modelo propio",
            dress: [0.62, 0.30, 0.42, 1.0],
            dress_dark: [0.36, 0.14, 0.24, 1.0],
            trim: [0.98, 0.82, 0.70, 1.0],
            glow: [0.94, 0.52, 0.66, 1.0],
        });
    }
    looks
}

fn outfit_looks() -> [OutfitLook; 2] {
    [
        OutfitLook {
            name: "Regalia real",
            detail: "La paleta original de la Reina",
            dress: [0.26, 0.13, 0.40, 1.0],
            dress_dark: [0.15, 0.07, 0.25, 1.0],
            trim: [0.94, 0.74, 0.36, 1.0],
            glow: [0.62, 0.36, 0.90, 1.0],
        },
        OutfitLook {
            name: "Vestido verde",
            detail: "Modelo y animaciones nuevas",
            dress: [0.14, 0.50, 0.30, 1.0],
            dress_dark: [0.07, 0.30, 0.18, 1.0],
            trim: [0.82, 0.94, 0.62, 1.0],
            glow: [0.36, 0.86, 0.52, 1.0],
        },
    ]
}

fn wardrobe_quad(vertices: &mut Vec<MapVertex>, p: [[f32; 2]; 4], color: [f32; 4]) {
    push_triangle(vertices, [p[0], p[1], p[2]], color);
    push_triangle(vertices, [p[0], p[2], p[3]], color);
}

/// A small standing figure of the Queen wearing `look`, `height` px tall with
/// her feet at `base`.
fn draw_outfit_figure(
    vertices: &mut Vec<MapVertex>,
    cx: f32,
    base: f32,
    height: f32,
    index: usize,
    look: &OutfitLook,
) {
    let h = height;
    let top = base - h;
    let skin = [0.95, 0.80, 0.72, 1.0];
    let hair = [0.10, 0.06, 0.12, 1.0];
    // Floor shadow.
    push_rounded_rect(
        vertices,
        Rect::new(cx - h * 0.34, base - h * 0.02, h * 0.68, h * 0.05),
        h * 0.025,
        [0.0, 0.0, 0.0, 0.35],
    );
    // Hair behind the head and shoulders.
    push_circle(vertices, cx, top + h * 0.10, h * 0.098, hair, 20);
    wardrobe_quad(
        vertices,
        [
            [cx - h * 0.095, top + h * 0.10],
            [cx + h * 0.095, top + h * 0.10],
            [cx + h * 0.10, top + h * 0.34],
            [cx - h * 0.10, top + h * 0.34],
        ],
        hair,
    );
    // Skirt: a wide flare with a darker fold in the middle.
    let waist = top + h * 0.42;
    let hem = base - h * 0.02;
    wardrobe_quad(
        vertices,
        [
            [cx - h * 0.075, waist],
            [cx + h * 0.075, waist],
            [cx + h * 0.31, hem],
            [cx - h * 0.31, hem],
        ],
        look.dress,
    );
    wardrobe_quad(
        vertices,
        [
            [cx - h * 0.02, waist],
            [cx + h * 0.075, waist],
            [cx + h * 0.12, hem],
            [cx - h * 0.06, hem],
        ],
        look.dress_dark,
    );
    push_segment(
        vertices,
        cx - h * 0.31,
        hem,
        cx + h * 0.31,
        hem,
        h * 0.018,
        look.trim,
    );
    // Arms hang from the shoulders.
    for side in [-1.0f32, 1.0] {
        push_segment(
            vertices,
            cx + side * h * 0.115,
            top + h * 0.245,
            cx + side * h * 0.165,
            top + h * 0.47,
            h * 0.05,
            skin,
        );
    }
    // Neck and bodice.
    wardrobe_quad(
        vertices,
        [
            [cx - h * 0.024, top + h * 0.17],
            [cx + h * 0.024, top + h * 0.17],
            [cx + h * 0.028, top + h * 0.23],
            [cx - h * 0.028, top + h * 0.23],
        ],
        skin,
    );
    wardrobe_quad(
        vertices,
        [
            [cx - h * 0.125, top + h * 0.23],
            [cx + h * 0.125, top + h * 0.23],
            [cx + h * 0.075, waist],
            [cx - h * 0.075, waist],
        ],
        look.dress,
    );
    push_triangle(
        vertices,
        [
            [cx - h * 0.05, top + h * 0.23],
            [cx + h * 0.05, top + h * 0.23],
            [cx, top + h * 0.30],
        ],
        skin,
    );
    push_segment(
        vertices,
        cx - h * 0.078,
        waist,
        cx + h * 0.078,
        waist,
        h * 0.022,
        look.trim,
    );
    // Head.
    push_circle(vertices, cx, top + h * 0.115, h * 0.075, skin, 20);
    // What she wears on her head: each outfit has its own.
    match index {
        0 => {
            // A gold crown of three points.
            let y = top + h * 0.045;
            wardrobe_quad(
                vertices,
                [
                    [cx - h * 0.055, y],
                    [cx + h * 0.055, y],
                    [cx + h * 0.055, y + h * 0.02],
                    [cx - h * 0.055, y + h * 0.02],
                ],
                look.trim,
            );
            for dx in [-0.05f32, 0.0, 0.05] {
                push_triangle(
                    vertices,
                    [
                        [cx + h * (dx - 0.014), y],
                        [cx + h * (dx + 0.014), y],
                        [
                            cx + h * dx,
                            y - h * (0.03 + if dx == 0.0 { 0.014 } else { 0.0 }),
                        ],
                    ],
                    look.trim,
                );
            }
        }
        1 => {
            // A circlet of leaves.
            for k in 0..5 {
                let dx = (k as f32 - 2.0) * 0.032;
                push_circle(
                    vertices,
                    cx + h * dx,
                    top + h * (0.048 + dx.abs() * 0.4),
                    h * 0.014,
                    look.trim,
                    8,
                );
            }
        }
        _ => {
            // A slim tiara with a ruby.
            push_segment(
                vertices,
                cx - h * 0.05,
                top + h * 0.055,
                cx + h * 0.05,
                top + h * 0.055,
                h * 0.012,
                look.trim,
            );
            push_circle(
                vertices,
                cx,
                top + h * 0.048,
                h * 0.014,
                [1.0, 0.26, 0.36, 1.0],
                10,
            );
        }
    }
    // Eyes, so she is not a mask.
    for side in [-1.0f32, 1.0] {
        push_circle(
            vertices,
            cx + side * h * 0.028,
            top + h * 0.118,
            h * 0.008,
            [0.12, 0.06, 0.12, 1.0],
            8,
        );
    }
}

fn draw_wardrobe(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    selection: u8,
    equipped: u8,
    view: &WardrobeView,
) {
    let tone = map_kit::tone(Place::Castle);
    let panel_width = (width * 0.68).clamp(660.0, 1_020.0).min(width - 36.0);
    let panel_height = (height * 0.74).clamp(470.0, 690.0).min(height - 36.0);
    let panel = Rect::new(
        (width - panel_width) * 0.5,
        (height - panel_height) * 0.5,
        panel_width,
        panel_height,
    );
    push_rect(
        vertices,
        Rect::new(0.0, 0.0, width, height),
        [0.006, 0.008, 0.022, 0.72],
    );
    map_kit::soft_shadow(vertices, panel, 26.0, 26.0, 0.9);
    map_kit::round_gradient(vertices, panel, 26.0, tone.top, tone.bottom);
    push_rounded_border(vertices, panel, 26.0, 2.6, tone.rim);
    push_rounded_border(
        vertices,
        Rect::new(panel.x + 7.0, panel.y + 7.0, panel.w - 14.0, panel.h - 14.0),
        20.0,
        1.0,
        tone.rim_soft,
    );

    // Header: the wardrobe's badge, its name, and how many outfits there are.
    let header = Rect::new(panel.x + 12.0, panel.y + 12.0, panel.w - 24.0, 92.0);
    map_kit::round_gradient(vertices, header, 18.0, tone.band_top, tone.band_bottom);
    push_rounded_border(vertices, header, 18.0, 1.0, tone.rim_soft);
    push_prompt_badge(
        vertices,
        prompt_icons::Icon::Wardrobe,
        header.x + 52.0,
        header.y + header.h * 0.5,
        30.0,
        1.0,
    );
    push_label_shadowed(
        font,
        vertices,
        "Ropero de la Reina",
        header.x + 100.0,
        header.y + 16.0,
        32.0,
        tone.title,
    );
    push_label(
        font,
        vertices,
        "Elige un atuendo para la exploración",
        header.x + 102.0,
        header.y + 58.0,
        16.0,
        tone.subtitle,
    );
    let count = view.outfits.len().max(2);
    let badge = Rect::new(header.x + header.w - 132.0, header.y + 30.0, 108.0, 32.0);
    map_kit::pill(vertices, badge, [0.10, 0.05, 0.12, 0.96], tone.rim);
    push_label_centered(
        font,
        vertices,
        &format!("{} atuendos", count),
        badge.x + badge.w * 0.5,
        badge.y + 7.0,
        15.0,
        tone.title,
    );

    // Cards.
    let looks = outfit_looks_n(count);
    let cards_top = header.y + header.h + 22.0;
    let cards_bottom = panel.y + panel.h - 84.0;
    let gap = 18.0;
    let card_width = ((panel.w - 48.0) - gap * (count as f32 - 1.0)) / count as f32;
    let card_height = (cards_bottom - cards_top).max(200.0);
    for (index, look) in looks.iter().enumerate() {
        let card = Rect::new(
            panel.x + 24.0 + index as f32 * (card_width + gap),
            cards_top,
            card_width,
            card_height,
        );
        let selected = index as u8 == selection % count as u8;
        let worn = index as u8 == equipped % count as u8;
        if selected {
            // A halo in the outfit's own colour, only colour, never geometry.
            for k in 1..=4 {
                let grow = k as f32 * 3.0;
                push_rounded_border(
                    vertices,
                    Rect::new(
                        card.x - grow,
                        card.y - grow,
                        card.w + grow * 2.0,
                        card.h + grow * 2.0,
                    ),
                    20.0 + grow,
                    3.0,
                    map_kit::with_alpha(look.glow, 0.30 - k as f32 * 0.06),
                );
            }
        }
        let (fill_top, fill_bottom) = if selected {
            ([0.16, 0.10, 0.20, 0.99], [0.08, 0.05, 0.11, 0.99])
        } else {
            ([0.085, 0.052, 0.110, 0.98], [0.050, 0.030, 0.068, 0.98])
        };
        map_kit::round_gradient(vertices, card, 20.0, fill_top, fill_bottom);
        push_rounded_border(
            vertices,
            card,
            20.0,
            if selected { 2.8 } else { 1.2 },
            if selected {
                [1.0, 0.78, 0.32, 1.0]
            } else {
                map_kit::with_alpha(tone.rim_soft, 0.75)
            },
        );

        // The stage: a pool of light behind her.
        let stage = Rect::new(card.x + 12.0, card.y + 12.0, card.w - 24.0, card.h - 132.0);
        map_kit::round_gradient(
            vertices,
            stage,
            14.0,
            [0.03, 0.02, 0.05, 0.98],
            map_kit::with_alpha(look.dress_dark, if selected { 0.95 } else { 0.70 }),
        );
        push_circle(
            vertices,
            stage.x + stage.w * 0.5,
            stage.y + stage.h * 0.55,
            stage.h * 0.42,
            map_kit::with_alpha(look.glow, if selected { 0.16 } else { 0.08 }),
            32,
        );
        push_circle(
            vertices,
            stage.x + stage.w * 0.5,
            stage.y + stage.h * 0.55,
            stage.h * 0.28,
            map_kit::with_alpha(look.glow, if selected { 0.14 } else { 0.06 }),
            32,
        );
        draw_outfit_figure(
            vertices,
            stage.x + stage.w * 0.5,
            stage.y + stage.h - 12.0,
            stage.h - 30.0,
            index,
            look,
        );

        // Key cap in the corner.
        let key = format!("{}", index + 1);
        let key = key.as_str();
        let cap = Rect::new(card.x + 20.0, card.y + 20.0, 30.0, 30.0);
        push_rounded_rect(vertices, cap, 8.0, [0.16, 0.06, 0.16, 0.96]);
        push_rounded_border(vertices, cap, 8.0, 1.2, [0.88, 0.62, 0.42, 0.92]);
        push_label_centered(
            font,
            vertices,
            key,
            cap.x + cap.w * 0.5,
            cap.y + 5.0,
            17.0,
            [1.0, 0.90, 0.72, 1.0],
        );

        // Name, line, and the state (the catalog's name when it gives one).
        let text_y = stage.y + stage.h + 14.0;
        let (name, detail) = view
            .outfits
            .get(index)
            .map(|(n, d)| {
                (
                    n.as_str(),
                    if d.is_empty() {
                        look.detail
                    } else {
                        d.as_str()
                    },
                )
            })
            .unwrap_or((look.name, look.detail));
        let locked = view.locked.get(index).copied().unwrap_or(false);
        push_label_centered(
            font,
            vertices,
            name,
            card.x + card.w * 0.5,
            text_y,
            21.0,
            if selected { tone.title } else { tone.ink },
        );
        push_label_centered(
            font,
            vertices,
            if locked {
                "Se gana con un logro"
            } else {
                detail
            },
            card.x + card.w * 0.5,
            text_y + 32.0,
            13.5,
            tone.soft,
        );
        if locked {
            // Veiled: the figure behind a dark glass, a padlock on it.
            push_rounded_rect(vertices, stage, 14.0, [0.02, 0.01, 0.04, 0.72]);
            let (cx, cy) = (stage.x + stage.w * 0.5, stage.y + stage.h * 0.5);
            push_rounded_rect(
                vertices,
                Rect::new(cx - 22.0, cy - 4.0, 44.0, 34.0),
                6.0,
                [0.80, 0.64, 0.36, 0.95],
            );
            push_rounded_border(
                vertices,
                Rect::new(cx - 14.0, cy - 26.0, 28.0, 30.0),
                12.0,
                5.0,
                [0.80, 0.64, 0.36, 0.95],
            );
            push_circle(vertices, cx, cy + 10.0, 4.5, [0.10, 0.06, 0.12, 1.0], 16);
        }
        if worn {
            let pill_rect = Rect::new(
                card.x + card.w * 0.5 - 62.0,
                card.y + card.h - 40.0,
                124.0,
                26.0,
            );
            map_kit::pill(
                vertices,
                pill_rect,
                [0.05, 0.20, 0.11, 0.98],
                [0.42, 0.95, 0.60, 0.95],
            );
            push_label_centered(
                font,
                vertices,
                "EQUIPADO",
                pill_rect.x + pill_rect.w * 0.5,
                pill_rect.y + 5.0,
                13.0,
                [0.62, 1.0, 0.76, 1.0],
            );
        } else if selected {
            let pill_rect = Rect::new(
                card.x + card.w * 0.5 - 62.0,
                card.y + card.h - 40.0,
                124.0,
                26.0,
            );
            map_kit::pill(
                vertices,
                pill_rect,
                [0.20, 0.13, 0.04, 0.98],
                [1.0, 0.78, 0.32, 0.95],
            );
            push_label_centered(
                font,
                vertices,
                "ENTER Equipar",
                pill_rect.x + pill_rect.w * 0.5,
                pill_rect.y + 5.0,
                13.0,
                [1.0, 0.90, 0.60, 1.0],
            );
        }
    }

    // Footer with the keys.
    let footer = Rect::new(
        panel.x + 24.0,
        panel.y + panel.h - 68.0,
        panel.w - 48.0,
        46.0,
    );
    push_rounded_rect(vertices, footer, 14.0, [0.0, 0.0, 0.0, 0.32]);
    push_rounded_border(
        vertices,
        footer,
        14.0,
        1.0,
        map_kit::with_alpha(tone.rim_soft, 0.7),
    );
    let items: [(&str, &str); 4] = [
        ("Flechas", "Cambiar"),
        ("1 2", "Elegir"),
        ("ENTER", "Equipar"),
        ("F / ESC", "Cerrar"),
    ];
    let widths: Vec<f32> = items
        .iter()
        .map(|(key, caption)| {
            (label_width(font, key, 14.0 * 0.86) + 16.0).max(21.0)
                + 8.0
                + label_width(font, caption, 14.0)
        })
        .collect();
    let spacing = 34.0;
    let total: f32 = widths.iter().sum::<f32>() + spacing * (items.len() as f32 - 1.0);
    let mut x = footer.x + (footer.w - total) * 0.5;
    for ((key, caption), w) in items.iter().zip(widths) {
        map_kit::key_caption(
            font,
            vertices,
            x,
            footer.y + footer.h * 0.5,
            key,
            caption,
            14.0,
            tone.ink,
        );
        x += w + spacing;
    }
}

fn hash_inventory(inventory: &BTreeMap<String, u32>) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (id, amount) in inventory {
        id.hash(&mut hasher);
        amount.hash(&mut hasher);
    }
    hasher.finish()
}

fn map_signature_step(value: f32, step: f32) -> u32 {
    ((value / step).round() as i32) as u32
}

fn hash_notice(notice: Option<&str>) -> u64 {
    let mut hasher = DefaultHasher::new();
    notice.unwrap_or_default().hash(&mut hasher);
    hasher.finish()
}

fn push_shop_coin(icons: &IconAtlas, vertices: &mut Vec<MapVertex>, x: f32, y: f32, radius: f32) {
    if let Some(icon) = icons.get("coin") {
        let size = radius * 2.8;
        push_icon_rect(
            vertices,
            Rect::new(x - size * 0.5, y - size * 0.5, size, size),
            icon,
            [1.0, 1.0, 1.0, 1.0],
        );
        return;
    }

    push_circle(
        vertices,
        x + 1.4,
        y + 1.8,
        radius + 1.4,
        [0.30, 0.16, 0.04, 0.38],
        18,
    );
    push_circle(vertices, x, y, radius + 1.0, [0.57, 0.31, 0.05, 1.0], 18);
    push_circle(vertices, x, y, radius, [1.0, 0.67, 0.12, 1.0], 18);
    push_circle(
        vertices,
        x - radius * 0.18,
        y - radius * 0.20,
        radius * 0.54,
        [1.0, 0.78, 0.27, 0.96],
        16,
    );
}

fn push_shop_corner_ornament(vertices: &mut Vec<MapVertex>, x: f32, y: f32, sx: f32, sy: f32) {
    let gold = [0.92, 0.70, 0.31, 0.96];
    let soft = [0.62, 0.45, 0.20, 0.70];
    push_segment(vertices, x, y, x + sx * 24.0, y, 2.2, gold);
    push_segment(vertices, x, y, x, y + sy * 24.0, 2.2, gold);
    push_segment(
        vertices,
        x + sx * 8.0,
        y + sy * 8.0,
        x + sx * 22.0,
        y + sy * 22.0,
        1.4,
        soft,
    );
    push_triangle(
        vertices,
        [
            [x + sx * 7.0, y + sy * 2.0],
            [x + sx * 16.0, y + sy * 6.0],
            [x + sx * 7.0, y + sy * 10.0],
        ],
        gold,
    );
    push_triangle(
        vertices,
        [
            [x + sx * 2.0, y + sy * 7.0],
            [x + sx * 6.0, y + sy * 16.0],
            [x + sx * 10.0, y + sy * 7.0],
        ],
        gold,
    );
}

fn push_shop_button(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    label: &str,
    active: bool,
    disabled: bool,
    hovered: bool,
    ui_time: f32,
    size: f32,
) {
    let pulse = if hovered && !disabled {
        0.5 + 0.5 * (ui_time * 8.0).sin().abs()
    } else {
        0.0
    };
    let grow = if hovered && !disabled {
        2.0 + pulse * 1.5
    } else {
        0.0
    };
    let draw_rect = Rect::new(
        rect.x - grow,
        rect.y - grow,
        rect.w + grow * 2.0,
        rect.h + grow * 2.0,
    );
    let fill = if disabled {
        [0.38, 0.39, 0.35, 0.72]
    } else if hovered {
        if active {
            [0.17 + pulse * 0.025, 0.38 + pulse * 0.035, 0.29, 1.0]
        } else {
            [0.06 + pulse * 0.025, 0.17 + pulse * 0.035, 0.16, 1.0]
        }
    } else if active {
        [0.12, 0.30, 0.24, 0.99]
    } else {
        [0.035, 0.09, 0.10, 0.98]
    };
    let border = if disabled {
        [0.42, 0.44, 0.40, 0.60]
    } else if hovered {
        [1.0, 0.78 + pulse * 0.12, 0.31, 1.0]
    } else if active {
        [1.0, 0.76, 0.28, 1.0]
    } else {
        [0.48, 0.68, 0.62, 0.86]
    };
    if hovered && !disabled {
        push_border(
            vertices,
            Rect::new(
                draw_rect.x - 3.0,
                draw_rect.y - 3.0,
                draw_rect.w + 6.0,
                draw_rect.h + 6.0,
            ),
            1.5 + pulse,
            [0.94, 0.69, 0.23, 0.20 + pulse * 0.28],
        );
    }
    push_rect(vertices, draw_rect, fill);
    push_border(
        vertices,
        draw_rect,
        if active || hovered { 2.5 } else { 1.2 },
        border,
    );
    if (active || hovered) && !disabled {
        push_border(
            vertices,
            Rect::new(
                draw_rect.x + 5.0,
                draw_rect.y + 5.0,
                draw_rect.w - 10.0,
                draw_rect.h - 10.0,
            ),
            1.0,
            [0.78, 0.61, 0.30, 0.56 + pulse * 0.18],
        );
    }
    let color = if disabled {
        [0.70, 0.70, 0.64, 0.82]
    } else if hovered {
        [1.0, 0.95, 0.77, 1.0]
    } else {
        [0.98, 0.91, 0.71, 1.0]
    };
    let text_size = if hovered && !disabled {
        size + 0.8 + pulse * 0.7
    } else {
        size
    };
    push_label_centered(
        font,
        vertices,
        label,
        draw_rect.x + draw_rect.w * 0.5,
        draw_rect.y + (draw_rect.h - text_size) * 0.5,
        text_size,
        color,
    );
}

fn fit_map_rect(content: Rect, bounds: MapBounds) -> Rect {
    let world_w = (bounds.max_x - bounds.min_x).max(0.001);
    let world_h = (bounds.max_z - bounds.min_z).max(0.001);
    let aspect = world_w / world_h;
    let mut w = content.w.max(1.0);
    let mut h = w / aspect;
    if h > content.h {
        h = content.h.max(1.0);
        w = h * aspect;
    }
    Rect::new(
        content.x + (content.w - w) * 0.5,
        content.y + (content.h - h) * 0.5,
        w,
        h,
    )
}

fn focused_village_bounds(player: Vec3, content: Rect) -> MapBounds {
    // Keep the compact radar player-centered and tight enough that nearby
    // houses stay readable while moving. The full map uses overview bounds.
    let view_height = 12.5;
    let aspect = (content.w / content.h.max(1.0)).max(0.35);
    let half_height = view_height * 0.5;
    let half_width = half_height * aspect;
    MapBounds {
        min_x: player.x - half_width,
        max_x: player.x + half_width,
        min_z: player.z - half_height,
        max_z: player.z + half_height,
    }
}

fn village_overview_bounds(_center: Vec3) -> MapBounds {
    // The whole clearing, its gate and the first rows of the forest round it.
    // The compact radar uses its tighter player-centered bounds instead.
    let (middle, radius) = super::village::clearing();
    let radius = radius + 4.0;
    MapBounds {
        min_x: middle.x - radius,
        max_x: middle.x + radius,
        min_z: middle.z - radius,
        max_z: middle.z + radius,
    }
}

fn village_map_columns(content: Rect) -> (Rect, Rect, Rect) {
    let gap = 18.0;
    let left_width = (content.w * 0.17).clamp(232.0, 300.0);
    let right_width = (content.w * 0.19).clamp(280.0, 340.0);
    let map_width = (content.w - left_width - right_width - gap * 2.0).max(320.0);
    let left = Rect::new(content.x, content.y, left_width, content.h);
    let map = Rect::new(
        content.x + left_width + gap,
        content.y,
        map_width,
        content.h,
    );
    let right = Rect::new(map.x + map.w + gap, content.y, right_width, content.h);
    (left, map, right)
}

fn map_point(x: f32, z: f32, bounds: MapBounds, map: Rect) -> (f32, f32) {
    let tx = ((x - bounds.min_x) / (bounds.max_x - bounds.min_x)).clamp(0.0, 1.0);
    let tz = ((z - bounds.min_z) / (bounds.max_z - bounds.min_z)).clamp(0.0, 1.0);
    (map.x + tx * map.w, map.y + (1.0 - tz) * map.h)
}

fn map_world_rect_coords(
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
    bounds: MapBounds,
    map: Rect,
) -> Rect {
    let (x0, y0) = map_point(min_x, max_z, bounds, map);
    let (x1, y1) = map_point(max_x, min_z, bounds, map);
    Rect::new(x0, y0, (x1 - x0).max(1.0), (y1 - y0).max(1.0))
}

fn draw_village_map(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    player_yaw: f32,
    expanded: bool,
) {
    let layout = super::village::layout();
    let center = super::fantasy_village_center();
    push_rect(vertices, map, [0.23, 0.32, 0.17, 1.0]);

    // The undergrowth and tree line use the exact same deterministic world
    // placements as the 3D renderer. Tiny leaf/grass marks give the parchment
    // map texture without adding a bitmap or duplicating meshes.
    if expanded {
        for plant in super::village::vegetation_placements() {
            if !village_point_visible(plant.x, plant.z, bounds) {
                continue;
            }
            let (x, y) = map_point(plant.x, plant.z, bounds, map);
            draw_village_vegetation(vertices, x, y, plant.asset, plant.scale, map, bounds);
        }
    }

    // The dashed circle is the clearing's edge, where the forest begins; the
    // streets and buildings below come from the villa's exported plan.
    let (clearing, boundary) = super::village::clearing();
    for index in (0..144).step_by(2) {
        let start = index as f32 / 144.0 * std::f32::consts::TAU;
        let end = (index + 1) as f32 / 144.0 * std::f32::consts::TAU;
        push_map_segment(
            vertices,
            map,
            bounds,
            clearing.x + start.cos() * boundary,
            clearing.z + start.sin() * boundary,
            clearing.x + end.cos() * boundary,
            clearing.z + end.sin() * boundary,
            if expanded { 2.0 } else { 1.4 },
            [0.78, 0.78, 0.49, if expanded { 0.76 } else { 0.50 }],
        );
    }
    // The brook, from the hot springs west under the bridge.
    let (brook, brook_width) = super::village::brook_path();
    for pair in brook.windows(2) {
        push_map_segment(
            vertices,
            map,
            bounds,
            pair[0][0],
            pair[0][1],
            pair[1][0],
            pair[1][1],
            if expanded {
                brook_width * 7.0
            } else {
                brook_width * 4.0
            },
            [0.36, 0.66, 0.78, 0.92],
        );
    }

    // The streets as one shape: every piece grown by the kerb first, dark, then
    // every piece itself, light, so the joins show no seams and the edge reads
    // as a kerb all round.
    for road in &layout.roads {
        if village_polygon_visible(&road.outline, center, bounds) {
            let screen = village_polygon_screen(&road.outline, center, bounds, map);
            fill_map_polygon(vertices, &screen, [0.47, 0.35, 0.22, 0.96]);
        }
    }
    for road in &layout.roads {
        if village_polygon_visible(&road.polygon, center, bounds) {
            let screen = village_polygon_screen(&road.polygon, center, bounds, map);
            fill_map_polygon(vertices, &screen, [0.78, 0.62, 0.37, 1.0]);
        }
    }

    let footprint_count = layout.footprints.len().max(1) as f32;
    let village_label_center = layout
        .footprints
        .iter()
        .map(|house| village_footprint_center(&house.polygon))
        .fold((0.0, 0.0), |sum, point| (sum.0 + point.0, sum.1 + point.1));
    let village_label_center = (
        center.x + village_label_center.0 / footprint_count,
        center.z + village_label_center.1 / footprint_count,
    );

    for house in &layout.footprints {
        if !village_polygon_visible(&house.polygon, center, bounds) {
            continue;
        }
        let (local_x, local_z) = village_footprint_center(&house.polygon);
        let x = local_x;
        let z = local_z;
        let world_x = center.x + x;
        let world_z = center.z + z;
        let screen = village_polygon_screen(&house.polygon, center, bounds, map);
        draw_village_building(vertices, &screen, house.name.as_str());
        let (x, y) = map_point(world_x, world_z, bounds, map);
        let Some(label) = village_map_label(house.name.as_str()) else {
            continue;
        };
        if expanded {
            let (label_x, label_y) = if house.name == "Casa del Burgomaestre" {
                (x, y - 54.0)
            } else if world_x < village_label_center.0 {
                (x - 124.0, y)
            } else {
                (x + 124.0, y)
            };
            draw_village_sign(font, vertices, map, x, y, label_x, label_y, label);
        } else {
            push_label_centered_bold_shadowed(
                font,
                vertices,
                label,
                x,
                y,
                18.5,
                [0.008, 0.008, 0.010, 1.0],
            );
        }
    }

    let entry = super::village::entrance();
    if village_point_visible(entry.x, entry.z, bounds) {
        let (x, y) = map_point(entry.x, entry.z, bounds, map);
        push_circle(
            vertices,
            x,
            y,
            if expanded { 13.0 } else { 8.0 },
            [0.07, 0.30, 0.18, 0.96],
            24,
        );
        push_border(
            vertices,
            Rect::new(x - 9.0, y - 9.0, 18.0, 18.0),
            1.2,
            [0.87, 0.75, 0.40, 0.92],
        );
        push_triangle(
            vertices,
            [[x, y - 6.0], [x - 4.0, y + 3.0], [x + 4.0, y + 3.0]],
            [0.98, 0.91, 0.68, 1.0],
        );
        push_label_shadowed(
            font,
            vertices,
            "F · Entrada",
            x,
            y + if expanded { 20.0 } else { 12.0 },
            if expanded { 16.0 } else { 13.0 },
            [0.96, 0.94, 0.78, 1.0],
        );
    }
    draw_player_marker(vertices, map, bounds, player, player_yaw, expanded);
    draw_compass_rose(font, vertices, map, expanded);
}

/// The villa from its picture, north up (`bounds` in map space: x, and -z up): the clearing seen
/// from above, a sign for every house, the gate, her pin.
#[allow(clippy::too_many_arguments)]
fn draw_village_picture(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    picture: [f32; 4],
    player: Vec3,
    player_yaw: f32,
    expanded: bool,
) {
    let place = Place::Village;
    push_rect(vertices, map, [0.10, 0.17, 0.08, 1.0]);
    push_map_picture(vertices, map, bounds, picture, [1.0, 1.0, 1.0, 1.0]);
    map_kit::vignette(vertices, map, 0.22);
    let layout = super::village::layout();
    let center = super::fantasy_village_center();
    let inside = |x: f32, z: f32| {
        x > bounds.min_x && x < bounds.max_x && -z > bounds.min_z && -z < bounds.max_z
    };
    let count = layout.footprints.len().max(1) as f32;
    let middle_x = center.x
        + layout
            .footprints
            .iter()
            .map(|h| village_footprint_center(&h.polygon).0)
            .sum::<f32>()
            / count;
    for house in &layout.footprints {
        let (lx, lz) = village_footprint_center(&house.polygon);
        let (wx, wz) = (center.x + lx, center.z + lz);
        if !inside(wx, wz) {
            continue;
        }
        let (x, y) = map_point(wx, -wz, bounds, map);
        let Some(label) = village_map_label(house.name.as_str()) else {
            continue;
        };
        if expanded {
            // A sign beside the house, on the side away from the square.
            let label_x = if wx < middle_x { x - 124.0 } else { x + 124.0 };
            draw_village_sign(font, vertices, map, x, y, label_x, y, label);
        } else {
            push_label_centered_bold_shadowed(
                font,
                vertices,
                label,
                x,
                y,
                16.0,
                [1.0, 0.97, 0.86, 1.0],
            );
        }
    }
    let entry = super::village::entrance();
    if inside(entry.x, entry.z) {
        map_kit::landmark(
            font,
            vertices,
            map,
            bounds,
            (entry.x, -entry.z),
            prompt_icons::Icon::Forest,
            "Al bosque",
            place,
            expanded,
            false,
        );
    }
    map_kit::player_pin(
        vertices,
        map,
        bounds,
        Vec3::new(player.x, player.y, -player.z),
        std::f32::consts::PI - player_yaw,
        expanded,
    );
    map_kit::compass(font, vertices, map, place, expanded);
    if expanded {
        map_kit::scale_bar(font, vertices, map, bounds, place, expanded);
    }
}

fn village_polygon_screen(
    points: &[[f32; 2]],
    center: Vec3,
    bounds: MapBounds,
    map: Rect,
) -> Vec<[f32; 2]> {
    points
        .iter()
        .map(|point| {
            let (x, y) = map_point(center.x + point[0], center.z + point[1], bounds, map);
            [x, y]
        })
        .collect()
}

fn fill_map_polygon(vertices: &mut Vec<MapVertex>, points: &[[f32; 2]], color: [f32; 4]) {
    for index in 1..points.len().saturating_sub(1) {
        push_triangle(
            vertices,
            [points[0], points[index], points[index + 1]],
            color,
        );
    }
}

fn village_footprint_center(points: &[[f32; 2]]) -> (f32, f32) {
    let count = points.len().max(1) as f32;
    (
        points.iter().map(|point| point[0]).sum::<f32>() / count,
        points.iter().map(|point| point[1]).sum::<f32>() / count,
    )
}

/// A house's name on the village's map; `None` for the doors with nothing behind them yet (the
/// map names only the places with something to do).
fn village_map_label(name: &str) -> Option<&str> {
    match name {
        "Casa del Burgomaestre" | "Casa de aldeanos A" | "Casa de aldeanos B" | "Taberna" => None,
        "Baños termales" => Some("Termas"),
        other => Some(other),
    }
}

fn draw_village_building(vertices: &mut Vec<MapVertex>, footprint: &[[f32; 2]], name: &str) {
    if footprint.len() < 3 {
        return;
    }
    let mut shadow = footprint.to_vec();
    for point in &mut shadow {
        point[0] += 3.0;
        point[1] += 4.0;
    }
    fill_map_polygon(vertices, &shadow, [0.025, 0.055, 0.045, 0.58]);
    fill_map_polygon(vertices, footprint, [0.87, 0.76, 0.55, 1.0]);

    let (roof_color, roof_light, roof_dark) = village_roof_colors(name);
    let (cx, cy) = footprint.iter().fold((0.0, 0.0), |center, point| {
        (center.0 + point[0], center.1 + point[1])
    });
    let (cx, cy) = (cx / footprint.len() as f32, cy / footprint.len() as f32);
    let roof_screen: Vec<[f32; 2]> = footprint
        .iter()
        .map(|point| [cx + (point[0] - cx) * 0.86, cy + (point[1] - cy) * 0.86])
        .collect();
    fill_map_polygon(vertices, &roof_screen, roof_color);

    // Flat triangular facets give each footprint a miniature low-poly roof;
    // the ridge and eaves stay inside the exported building outline.
    let screen_center = roof_screen.iter().fold([0.0, 0.0], |center, point| {
        [center[0] + point[0], center[1] + point[1]]
    });
    let screen_center = [
        screen_center[0] / roof_screen.len() as f32,
        screen_center[1] / roof_screen.len() as f32,
    ];
    for index in (0..roof_screen.len()).step_by(2) {
        push_triangle(
            vertices,
            [
                screen_center,
                roof_screen[index],
                roof_screen[(index + 1) % roof_screen.len()],
            ],
            if index % 4 == 0 {
                roof_light
            } else {
                roof_dark
            },
        );
    }
    for index in 0..footprint.len() {
        let a = footprint[index];
        let b = footprint[(index + 1) % footprint.len()];
        push_segment(
            vertices,
            a[0],
            a[1],
            b[0],
            b[1],
            1.6,
            [0.18, 0.14, 0.11, 0.96],
        );
        let a = roof_screen[index];
        let b = roof_screen[(index + 1) % roof_screen.len()];
        push_segment(
            vertices,
            a[0],
            a[1],
            b[0],
            b[1],
            1.0,
            [0.14, 0.12, 0.11, 0.88],
        );
    }
    let min_x = roof_screen
        .iter()
        .map(|point| point[0])
        .fold(f32::INFINITY, f32::min);
    let max_x = roof_screen
        .iter()
        .map(|point| point[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = roof_screen
        .iter()
        .map(|point| point[1])
        .fold(f32::INFINITY, f32::min);
    let max_y = roof_screen
        .iter()
        .map(|point| point[1])
        .fold(f32::NEG_INFINITY, f32::max);
    if max_x - min_x >= max_y - min_y {
        push_segment(
            vertices,
            min_x + 4.0,
            screen_center[1],
            max_x - 4.0,
            screen_center[1],
            2.0,
            roof_light,
        );
    } else {
        push_segment(
            vertices,
            screen_center[0],
            min_y + 4.0,
            screen_center[0],
            max_y - 4.0,
            2.0,
            roof_light,
        );
    }

    if name == "Baños termales" {
        push_circle(
            vertices,
            screen_center[0],
            screen_center[1],
            5.0,
            [0.28, 0.70, 0.73, 0.92],
            16,
        );
        push_circle(
            vertices,
            screen_center[0] - 8.0,
            screen_center[1] + 5.0,
            3.0,
            [0.35, 0.75, 0.76, 0.90],
            12,
        );
    } else {
        let window_size = ((max_x - min_x).min(max_y - min_y) * 0.14).clamp(2.4, 5.0);
        for offset in [-1.0, 1.0] {
            push_rect(
                vertices,
                Rect::new(
                    screen_center[0] + offset * (max_x - min_x).min(24.0) * 0.22
                        - window_size * 0.5,
                    screen_center[1] - window_size * 0.5,
                    window_size,
                    window_size,
                ),
                [1.0, 0.79, 0.38, 0.96],
            );
        }
        push_rect(
            vertices,
            Rect::new(
                screen_center[0] - window_size * 0.42,
                max_y - window_size * 1.25,
                window_size * 0.84,
                window_size * 1.25,
            ),
            [0.20, 0.14, 0.12, 1.0],
        );
    }
}

fn draw_village_sign(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    anchor_x: f32,
    anchor_y: f32,
    center_x: f32,
    center_y: f32,
    label: &str,
) {
    // A dot on the house, a leader of the village's colour, the name on its Persona tag.
    let place = Place::Village;
    let accent = ui_paint::rgba(map_kit::accent(place), 0.95);
    let size = 28.0;
    let tag = map_kit::name_tag(font, vertices, map, center_x, center_y, label, size, place);
    let edge_x = if tag.x + tag.w * 0.5 < anchor_x { tag.x + tag.w } else { tag.x };
    push_segment(vertices, anchor_x, anchor_y, edge_x, tag.y + tag.h * 0.5, 3.0, accent);
    push_circle(vertices, anchor_x, anchor_y, 6.0, accent, 16);
    // Over the leader's end.
    map_kit::name_tag(font, vertices, map, center_x, center_y, label, size, place);
}

fn draw_village_vegetation(
    vertices: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    asset: &str,
    scale: f32,
    map: Rect,
    bounds: MapBounds,
) {
    let px_per_unit = map.w / (bounds.max_x - bounds.min_x).max(0.001);
    match asset {
        "CommonTree_1" | "CommonTree_3" | "Pine_2" => {
            let size = (scale * px_per_unit * 0.105).clamp(0.7, 1.65);
            if asset == "Pine_2" {
                draw_pine_icon(vertices, x, y, size);
            } else {
                draw_map_broadleaf(vertices, x, y, size, asset == "CommonTree_3");
            }
        }
        "Flower_3_Group" => {
            let radius = (scale * px_per_unit * 0.45).clamp(1.6, 3.4);
            for offset in [[-radius, 0.0], [radius, -0.7], [0.0, radius]] {
                push_circle(
                    vertices,
                    x + offset[0],
                    y + offset[1],
                    radius * 0.42,
                    [0.97, 0.73, 0.56, 0.95],
                    8,
                );
            }
            push_circle(vertices, x, y, radius * 0.26, [0.97, 0.88, 0.54, 1.0], 8);
        }
        "Bush_Common" => {
            let radius = (scale * px_per_unit * 0.65).clamp(1.8, 4.5);
            push_circle(vertices, x, y, radius, [0.17, 0.42, 0.24, 0.92], 10);
            push_circle(
                vertices,
                x - radius * 0.55,
                y + radius * 0.15,
                radius * 0.66,
                [0.24, 0.51, 0.28, 0.96],
                10,
            );
            push_circle(
                vertices,
                x + radius * 0.52,
                y + radius * 0.05,
                radius * 0.58,
                [0.32, 0.58, 0.31, 0.94],
                10,
            );
        }
        "Fern_1" => {
            let length = (scale * px_per_unit * 1.3).clamp(2.5, 6.0);
            for side in [-1.0, 0.0, 1.0] {
                push_triangle(
                    vertices,
                    [
                        [x, y],
                        [x + side * length * 0.65 - length * 0.2, y - length],
                        [x + side * length, y - length * 0.2],
                    ],
                    [0.31, 0.61, 0.30, 0.92],
                );
            }
        }
        _ => {
            let length = (scale * px_per_unit * 1.0).clamp(2.2, 5.4);
            push_segment(
                vertices,
                x,
                y + length * 0.3,
                x - length * 0.35,
                y - length * 0.7,
                1.1,
                [0.48, 0.67, 0.34, 0.92],
            );
            push_segment(
                vertices,
                x,
                y + length * 0.3,
                x + length * 0.35,
                y - length * 0.8,
                1.1,
                [0.56, 0.73, 0.38, 0.92],
            );
        }
    }
}

fn draw_map_broadleaf(vertices: &mut Vec<MapVertex>, x: f32, y: f32, scale: f32, lighter: bool) {
    let r = 8.5 * scale;
    push_circle(
        vertices,
        x + 2.0,
        y + 5.0,
        r + 2.0,
        [0.04, 0.09, 0.05, 0.45],
        16,
    );
    push_rect(
        vertices,
        Rect::new(x - 2.0 * scale, y + 2.0 * scale, 4.0 * scale, 9.0 * scale),
        [0.33, 0.23, 0.13, 0.94],
    );
    let (base, light, dark) = if lighter {
        (
            [0.30, 0.54, 0.26, 1.0],
            [0.46, 0.68, 0.32, 1.0],
            [0.16, 0.38, 0.22, 1.0],
        )
    } else {
        (
            [0.15, 0.42, 0.25, 1.0],
            [0.27, 0.56, 0.30, 1.0],
            [0.09, 0.31, 0.20, 1.0],
        )
    };
    push_circle(vertices, x, y, r, base, 16);
    push_circle(vertices, x - r * 0.48, y + r * 0.06, r * 0.59, dark, 12);
    push_circle(vertices, x + r * 0.42, y - r * 0.15, r * 0.57, light, 12);
    push_triangle(
        vertices,
        [
            [x - 2.0 * scale, y - 2.0 * scale],
            [x + 1.0 * scale, y - r],
            [x + 5.0 * scale, y + 1.0 * scale],
        ],
        [light[0], light[1], light[2], 0.72],
    );
}

/// What pressing F does here, at the bottom of the screen: a large key and
/// the action, on a small plate in the villa map's colours.
/// A filled simple polygon (any winding, may be concave).
fn push_polygon(vertices: &mut Vec<MapVertex>, points: &[[f32; 2]], color: [f32; 4]) {
    for [a, b, c] in prompt_icons::triangulate(points) {
        push_triangle(vertices, [points[a], points[b], points[c]], color);
    }
}

/// Outline of a rounded rectangle, clockwise on screen.
fn rounded_outline(rect: Rect, radius: f32) -> Vec<[f32; 2]> {
    let r = radius.min(rect.w * 0.5).min(rect.h * 0.5).max(0.0);
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.x + rect.w, rect.y + rect.h);
    let mut points = Vec::with_capacity(28);
    for (cx, cy, start) in [
        (x1 - r, y0 + r, -std::f32::consts::FRAC_PI_2),
        (x1 - r, y1 - r, 0.0),
        (x0 + r, y1 - r, std::f32::consts::FRAC_PI_2),
        (x0 + r, y0 + r, std::f32::consts::PI),
    ] {
        for k in 0..=6 {
            let a = start + k as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
            points.push([cx + a.cos() * r, cy + a.sin() * r]);
        }
    }
    points
}

fn push_rounded_rect(vertices: &mut Vec<MapVertex>, rect: Rect, radius: f32, color: [f32; 4]) {
    let points = rounded_outline(rect, radius);
    let centre = [rect.x + rect.w * 0.5, rect.y + rect.h * 0.5];
    for i in 0..points.len() {
        push_triangle(
            vertices,
            [centre, points[i], points[(i + 1) % points.len()]],
            color,
        );
    }
}

/// A rounded rectangle's rim of `width`, drawn as a band (no fill).
fn push_rounded_border(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    radius: f32,
    width: f32,
    color: [f32; 4],
) {
    let outer = rounded_outline(rect, radius);
    let inner = rounded_outline(
        Rect::new(
            rect.x + width,
            rect.y + width,
            rect.w - width * 2.0,
            rect.h - width * 2.0,
        ),
        (radius - width).max(0.0),
    );
    for i in 0..outer.len() {
        let j = (i + 1) % outer.len();
        push_triangle(vertices, [outer[i], outer[j], inner[j]], color);
        push_triangle(vertices, [outer[i], inner[j], inner[i]], color);
    }
}

fn push_ring(
    vertices: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    radius: f32,
    width: f32,
    color: [f32; 4],
) {
    let segments = 40;
    for k in 0..segments {
        let a0 = k as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (k + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let (r0, r1) = (radius - width * 0.5, radius + width * 0.5);
        let p = |a: f32, r: f32| [x + a.cos() * r, y + a.sin() * r];
        push_triangle(vertices, [p(a0, r0), p(a0, r1), p(a1, r1)], color);
        push_triangle(vertices, [p(a0, r0), p(a1, r1), p(a1, r0)], color);
    }
}

/// An interaction icon of `size` pixels centred on (x, y): ink parts in
/// `ink`, cut-outs in `face` (the colour behind it).
fn push_prompt_icon(
    vertices: &mut Vec<MapVertex>,
    icon: prompt_icons::Icon,
    x: f32,
    y: f32,
    size: f32,
    ink: [f32; 4],
    face: [f32; 4],
) {
    for part in prompt_icons::shape(icon) {
        let points: Vec<[f32; 2]> = part
            .points
            .iter()
            .map(|p| [x + p[0] * size, y + p[1] * size])
            .collect();
        push_polygon(vertices, &points, if part.ink > 0.5 { ink } else { face });
    }
}

/// A place to go in, over it on the screen: a slanted ink card with a wedge of the interaction's
/// colour, its icon in cream, a white rim and a halo once she is in reach, an arrow bobbing down at
/// the spot and the F key on a cap at its corner.
fn draw_world_marker(font: &FontAtlas, v: &mut Vec<MapVertex>, m: &WorldMarker, time: f32) {
    use ui_paint::{keycap, rgba};
    let a = m.fade.clamp(0.0, 1.0);
    if a <= 0.01 {
        return;
    }
    let accent = m.icon.accent();
    let to = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
    let hex = (to(accent[0]) << 16) | (to(accent[1]) << 8) | to(accent[2]);
    let bob = (time * 2.6).sin() * 4.0;
    let scale = 0.82 + 0.18 * a + if m.ready { 0.05 * (time * 5.0).sin().abs() } else { 0.0 };
    let (cx, cy) = (m.x, m.y + bob);
    let (w, h, lean) = (80.0 * scale, 64.0 * scale, 15.0 * scale);
    let q = |dx: f32, dy: f32, ww: f32, hh: f32| -> [[f32; 2]; 4] {
        let (x, y) = (cx - ww * 0.5 + dx, cy - hh * 0.5 + dy);
        [[x + lean, y], [x + ww + lean, y], [x + ww - lean, y + hh], [x - lean, y + hh]]
    };
    let fill = |v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], color: [f32; 4]| {
        push_triangle(v, [c[0], c[1], c[2]], color);
        push_triangle(v, [c[0], c[2], c[3]], color);
        push_outline_fringe(v, &c, color);
    };
    if m.ready {
        let pulse = 0.5 + 0.5 * (time * 4.0).sin();
        push_circle(v, cx, cy, 58.0 * scale, rgba(hex, (0.14 + 0.1 * pulse) * a), 40);
    }
    // The arrow down at the spot: ink under, the colour over, a white edge.
    let tip = cy + h * 0.5 + 30.0 * scale + (time * 3.2).sin() * 4.0;
    let arrow = |v: &mut Vec<MapVertex>, grow: f32, dy: f32, color: [f32; 4]| {
        let top = cy + h * 0.5 + 8.0 * scale + dy;
        push_triangle(
            v,
            [
                [cx - 16.0 * scale - grow, top - grow * 0.5],
                [cx + 16.0 * scale + grow, top - grow * 0.5],
                [cx, tip + dy + grow],
            ],
            color,
        );
    };
    arrow(v, 3.0, 4.0, rgba(0x000000, 0.5 * a));
    arrow(v, 3.0, 0.0, rgba(0xffffff, a));
    arrow(v, 0.0, 0.0, rgba(hex, a));
    // The card: shadow, ink plate, the colour's wedge, the rim.
    fill(v, q(7.0, 7.0, w, h), rgba(0x000000, 0.55 * a));
    fill(v, q(0.0, 0.0, w, h), rgba(0x0e0816, 0.96 * a));
    let wedge = {
        let c = q(0.0, 0.0, w, h);
        [c[0], [c[0][0] + 14.0 * scale, c[0][1]], [c[3][0] + 14.0 * scale, c[3][1]], c[3]]
    };
    fill(v, wedge, rgba(hex, a));
    let rim = q(0.0, 0.0, w, h);
    let edge = if m.ready { rgba(0xffffff, a) } else { rgba(hex, 0.75 * a) };
    for k in 0..4 {
        let (p0, p1) = (rim[k], rim[(k + 1) % 4]);
        let d = [p1[0] - p0[0], p1[1] - p0[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.001);
        let n = [-d[1] / len, d[0] / len];
        let t = if m.ready { 2.4 } else { 1.4 };
        let quad = [
            p0,
            p1,
            [p1[0] - n[0] * t, p1[1] - n[1] * t],
            [p0[0] - n[0] * t, p0[1] - n[1] * t],
        ];
        push_triangle(v, [quad[0], quad[1], quad[2]], edge);
        push_triangle(v, [quad[0], quad[2], quad[3]], edge);
    }
    // The icon in cream, its cut-outs in the plate's ink.
    push_prompt_icon(
        v,
        m.icon,
        cx + 5.0 * scale,
        cy,
        50.0 * scale,
        [1.0, 0.95, 0.88, a],
        [0.055, 0.031, 0.086, a],
    );
    // In reach: F on its cap at the card's corner.
    if m.ready {
        let k = 34.0 * scale;
        keycap(
            font,
            v,
            Rect::new(cx + w * 0.5 - 4.0, cy - h * 0.5 - k * 0.55, k, k),
            "F",
            18.0 * scale,
            true,
            false,
            hex,
            a,
        );
    }
}

/// A round medallion holding an interaction's icon, rimmed in its colour.
fn push_prompt_badge(
    vertices: &mut Vec<MapVertex>,
    icon: prompt_icons::Icon,
    x: f32,
    y: f32,
    radius: f32,
    opacity: f32,
) {
    let accent = icon.accent();
    let face = [0.045, 0.035, 0.07, 0.98 * opacity];
    push_circle(
        vertices,
        x + 1.5,
        y + 3.0,
        radius + 2.0,
        [0.0, 0.0, 0.0, 0.35 * opacity],
        40,
    );
    push_circle(vertices, x, y, radius, face, 40);
    // Soft inner glow in the icon's colour.
    push_circle(
        vertices,
        x,
        y,
        radius * 0.82,
        [
            accent[0] * 0.16,
            accent[1] * 0.16,
            accent[2] * 0.16,
            0.9 * opacity,
        ],
        40,
    );
    push_ring(
        vertices,
        x,
        y,
        radius - 1.5,
        3.0,
        [accent[0], accent[1], accent[2], opacity],
    );
    push_ring(
        vertices,
        x,
        y,
        radius - 6.0,
        1.0,
        [accent[0], accent[1], accent[2], 0.35 * opacity],
    );
    push_prompt_icon(
        vertices,
        icon,
        x,
        y,
        radius * 1.18,
        [
            accent[0].min(1.0),
            accent[1].min(1.0),
            accent[2].min(1.0),
            opacity,
        ],
        [
            accent[0] * 0.16,
            accent[1] * 0.16,
            accent[2] * 0.16,
            opacity,
        ],
    );
}

/// A raised keyboard key with its letter.
fn push_keycap(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    key: &str,
    x: f32,
    y: f32,
    size: f32,
) {
    let cap = Rect::new(x, y, size, size);
    push_rounded_rect(
        vertices,
        Rect::new(cap.x, cap.y + 4.0, cap.w, cap.h),
        9.0,
        [0.30, 0.19, 0.07, 1.0],
    );
    push_rounded_rect(
        vertices,
        Rect::new(cap.x, cap.y, cap.w, cap.h - 3.0),
        9.0,
        [0.96, 0.78, 0.40, 1.0],
    );
    push_rounded_rect(
        vertices,
        Rect::new(cap.x + 3.5, cap.y + 3.0, cap.w - 7.0, cap.h - 11.0),
        7.0,
        [0.16, 0.09, 0.19, 1.0],
    );
    let text = size * 0.52;
    push_label_centered_bold_shadowed(
        font,
        vertices,
        key,
        cap.x + cap.w * 0.5,
        cap.y + (cap.h - 7.0 - text) * 0.5 - 1.0,
        text,
        [1.0, 0.88, 0.55, 1.0],
    );
}

/// The on-screen F prompt: key, the interaction's own icon and colour, the
/// action in large type and what it acts on beneath.
fn draw_action_prompt(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    prompt: &prompt_icons::Prompt,
) {
    // In the game's Persona manner: a slanted dark plate with a wedge of the interaction's colour,
    // the key on its cap, the interaction's badge, the verb big with an ink rim and its detail.
    use ui_paint::{keycap, rgba, text_outline, text_shadow, text_width, Face};
    let accent = prompt.icon.accent();
    let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    let hex = (to(accent[0]) << 16) | (to(accent[1]) << 8) | to(accent[2]);
    let (verb_size, detail_size) = (30.0, 18.0);
    let verb_w = text_width(font, Face::Title, &prompt.verb, verb_size);
    let detail_w = prompt
        .detail
        .as_deref()
        .map_or(0.0, |d| text_width(font, Face::Italic, d, detail_size));
    let key_space = if prompt.keyed { 62.0 } else { 0.0 };
    let badge = 26.0;
    let text_x0 = 34.0 + key_space + badge * 2.0 + 18.0;
    let w = (text_x0 + verb_w.max(detail_w) + 44.0).min(width - 80.0);
    let h = 76.0;
    let lean = 14.0;
    // A second action on its own plate to the right: both centred together.
    let second = prompt.second.as_ref().map(|(key, verb)| {
        let kw = text_width(font, Face::Title, key, 20.0) + 26.0;
        (key, verb, kw, kw + 30.0 + text_width(font, Face::Title, verb, 26.0) + 40.0)
    });
    let second_w = second.as_ref().map_or(0.0, |s| s.3 + 22.0);
    let (x, y) = ((width - w - second_w) * 0.5, height - 190.0);
    let q = |x: f32, y: f32, w: f32, h: f32, lean: f32| -> [[f32; 2]; 4] {
        [[x + lean, y], [x + w + lean, y], [x + w - lean, y + h], [x - lean, y + h]]
    };
    let fill = |v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], color: [f32; 4]| {
        push_triangle(v, [c[0], c[1], c[2]], color);
        push_triangle(v, [c[0], c[2], c[3]], color);
        push_outline_fringe(v, &c, color);
    };
    fill(vertices, q(x + 9.0, y + 9.0, w, h, lean), rgba(0x000000, 0.55));
    fill(vertices, q(x, y, w, h, lean), rgba(0x0e0816, 0.95));
    fill(vertices, q(x, y, 18.0, h, lean), rgba(hex, 1.0));
    let top = q(x, y, w, h, lean);
    push_fringe(vertices, top[0], top[1], [0.0, -1.0], 2.5, [accent[0], accent[1], accent[2], 0.9]);
    let mid = y + h * 0.5;
    let mut cx = x + 34.0;
    if prompt.keyed {
        keycap(font, vertices, Rect::new(cx, mid - 22.0, 46.0, 44.0), "F", 22.0, false, false, hex, 1.0);
        cx += key_space;
    }
    push_prompt_badge(vertices, prompt.icon, cx + badge, mid, badge, 1.0);
    if let Some((key, verb, kw, sw)) = second {
        // Red for a fight: the plate, its wedge, the key on its cap, the verb.
        let red = 0xff4a5a;
        let sx = x + w + 22.0;
        let sy = y + 8.0;
        let sh = h - 16.0;
        fill(vertices, q(sx + 8.0, sy + 8.0, sw, sh, lean), rgba(0x000000, 0.5));
        fill(vertices, q(sx, sy, sw, sh, lean), rgba(0x1a0610, 0.95));
        fill(vertices, q(sx, sy, 14.0, sh, lean), rgba(red, 1.0));
        let smid = sy + sh * 0.5;
        keycap(font, vertices, Rect::new(sx + 28.0, smid - 19.0, kw, 38.0), key, 20.0, false, false, red, 1.0);
        text_outline(
            font,
            vertices,
            Face::Title,
            verb,
            sx + 28.0 + kw + 16.0,
            smid - 26.0 * 0.58,
            26.0,
            rgba(0xfff0f0, 1.0),
            rgba(0x000000, 0.85),
        );
    }
    let tx = cx + badge * 2.0 + 18.0;
    let max = x + w - 26.0 - tx;
    let fit = |s: &str, face: Face, size: f32| {
        let w = text_width(font, face, s, size);
        if w <= max { size } else { (size * max / w).max(size * 0.6) }
    };
    match prompt.detail.as_deref() {
        Some(detail) => {
            let vs = fit(&prompt.verb, Face::Title, verb_size);
            text_outline(font, vertices, Face::Title, &prompt.verb, tx, mid - vs - 2.0, vs, rgba(0xfff6ee, 1.0), rgba(0x000000, 0.85));
            let ds = fit(detail, Face::Italic, detail_size);
            text_shadow(
                font,
                vertices,
                Face::Italic,
                detail,
                tx + 2.0,
                mid + 4.0,
                ds,
                [0.6 + accent[0] * 0.4, 0.6 + accent[1] * 0.4, 0.6 + accent[2] * 0.4, 1.0],
            );
        }
        None => {
            let vs = fit(&prompt.verb, Face::Title, verb_size);
            text_outline(font, vertices, Face::Title, &prompt.verb, tx, mid - vs * 0.58, vs, rgba(0xfff6ee, 1.0), rgba(0x000000, 0.85));
        }
    }
}

fn prompt_hash(prompt: Option<&prompt_icons::Prompt>) -> u64 {
    let mut hasher = DefaultHasher::new();
    if let Some(prompt) = prompt {
        prompt.icon.hash(&mut hasher);
        prompt.text().hash(&mut hasher);
    }
    hasher.finish()
}

fn village_roof_colors(name: &str) -> ([f32; 4], [f32; 4], [f32; 4]) {
    match name {
        "Casa del Burgomaestre" => (
            [0.24, 0.34, 0.42, 1.0],
            [0.35, 0.48, 0.54, 1.0],
            [0.16, 0.23, 0.31, 1.0],
        ),
        "Burdel" => (
            [0.48, 0.20, 0.23, 1.0],
            [0.67, 0.32, 0.31, 1.0],
            [0.32, 0.13, 0.18, 1.0],
        ),
        "Tienda" => (
            [0.16, 0.39, 0.31, 1.0],
            [0.29, 0.55, 0.43, 1.0],
            [0.09, 0.26, 0.23, 1.0],
        ),
        "Taberna" => (
            [0.52, 0.27, 0.17, 1.0],
            [0.70, 0.40, 0.22, 1.0],
            [0.33, 0.17, 0.13, 1.0],
        ),
        "Baños termales" => (
            [0.28, 0.38, 0.39, 1.0],
            [0.48, 0.64, 0.62, 1.0],
            [0.18, 0.27, 0.30, 1.0],
        ),
        "Casa de aldeanos B" => (
            [0.32, 0.40, 0.44, 1.0],
            [0.46, 0.57, 0.57, 1.0],
            [0.20, 0.29, 0.35, 1.0],
        ),
        _ => (
            [0.56, 0.31, 0.19, 1.0],
            [0.74, 0.45, 0.26, 1.0],
            [0.35, 0.19, 0.13, 1.0],
        ),
    }
}

fn village_point_visible(x: f32, z: f32, bounds: MapBounds) -> bool {
    x >= bounds.min_x && x <= bounds.max_x && z >= bounds.min_z && z <= bounds.max_z
}

fn village_polygon_visible(points: &[[f32; 2]], center: Vec3, bounds: MapBounds) -> bool {
    let Some(first) = points.first() else {
        return false;
    };
    let mut min_x = center.x + first[0];
    let mut max_x = min_x;
    let mut min_z = center.z + first[1];
    let mut max_z = min_z;
    for point in &points[1..] {
        let x = center.x + point[0];
        let z = center.z + point[1];
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    max_x >= bounds.min_x && min_x <= bounds.max_x && max_z >= bounds.min_z && min_z <= bounds.max_z
}

fn draw_pine_icon(vertices: &mut Vec<MapVertex>, x: f32, y: f32, scale: f32) {
    push_rect(
        vertices,
        Rect::new(x - 1.5 * scale, y + 5.0 * scale, 3.0 * scale, 7.0 * scale),
        [0.34, 0.23, 0.13, 0.88],
    );
    let canopy = [0.18, 0.43, 0.29, 0.88];
    let canopy_dark = [0.11, 0.32, 0.23, 0.94];
    push_triangle(
        vertices,
        [
            [x, y - 11.0 * scale],
            [x - 7.0 * scale, y + 3.0 * scale],
            [x + 7.0 * scale, y + 3.0 * scale],
        ],
        canopy,
    );
    push_triangle(
        vertices,
        [
            [x, y - 4.0 * scale],
            [x - 8.5 * scale, y + 8.0 * scale],
            [x + 8.5 * scale, y + 8.0 * scale],
        ],
        canopy_dark,
    );
}

fn draw_compass_rose(font: &FontAtlas, vertices: &mut Vec<MapVertex>, map: Rect, expanded: bool) {
    let radius = if expanded { 22.0 } else { 17.0 };
    let x = map.x + radius + 13.0;
    let y = map.y + radius + 13.0;
    push_circle(vertices, x, y, radius, [0.025, 0.07, 0.075, 0.82], 24);
    push_circle(vertices, x, y, radius - 2.0, [0.08, 0.18, 0.17, 0.96], 24);
    push_triangle(
        vertices,
        [
            [x, y - radius + 3.0],
            [x - 5.0, y + 2.0],
            [x + 5.0, y + 2.0],
        ],
        [1.0, 0.76, 0.28, 1.0],
    );
    push_triangle(
        vertices,
        [
            [x, y + radius - 4.0],
            [x - 4.0, y - 1.0],
            [x + 4.0, y - 1.0],
        ],
        [0.62, 0.78, 0.72, 0.84],
    );
    push_label_centered(
        font,
        vertices,
        "N",
        x,
        y - 7.0,
        if expanded { 16.0 } else { 13.0 },
        [0.035, 0.065, 0.07, 1.0],
    );
}

fn draw_player_marker(
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    player: Vec3,
    player_yaw: f32,
    expanded: bool,
) {
    let (x, y) = map_point(player.x, player.z, bounds, map);
    let radius = if expanded { 7.0 } else { 4.5 };
    push_circle(vertices, x, y, radius + 3.0, [0.02, 0.03, 0.05, 0.78], 12);
    push_circle(vertices, x, y, radius, [1.0, 0.27, 0.58, 1.0], 12);
    let heading_x = player_yaw.sin();
    let heading_y = -player_yaw.cos();
    push_segment(
        vertices,
        x,
        y,
        x + heading_x * (radius + 8.0),
        y + heading_y * (radius + 8.0),
        if expanded { 2.2 } else { 1.5 },
        [1.0, 0.84, 0.39, 1.0],
    );
}

fn push_map_segment(
    vertices: &mut Vec<MapVertex>,
    map: Rect,
    bounds: MapBounds,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    thickness: f32,
    color: [f32; 4],
) {
    let (x0, y0) = map_point(x0, z0, bounds, map);
    let (x1, y1) = map_point(x1, z1, bounds, map);
    push_segment(vertices, x0, y0, x1, y1, thickness, color);
}

fn push_segment(
    vertices: &mut Vec<MapVertex>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: [f32; 4],
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let length = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = -dy / length * thickness * 0.5;
    let ny = dx / length * thickness * 0.5;
    push_vertices(
        vertices,
        [
            [x0 + nx, y0 + ny],
            [x1 + nx, y1 + ny],
            [x1 - nx, y1 - ny],
            [x0 + nx, y0 + ny],
            [x1 - nx, y1 - ny],
            [x0 - nx, y0 - ny],
        ],
        color,
    );
}

fn push_circle(
    vertices: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    radius: f32,
    color: [f32; 4],
    segments: usize,
) {
    for index in 0..segments {
        let a0 = index as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        push_triangle(
            vertices,
            [
                [x, y],
                [x + a0.cos() * radius, y + a0.sin() * radius],
                [x + a1.cos() * radius, y + a1.sin() * radius],
            ],
            color,
        );
    }
}

fn push_border(vertices: &mut Vec<MapVertex>, rect: Rect, thickness: f32, color: [f32; 4]) {
    let t = thickness.max(0.5);
    push_rect(vertices, Rect::new(rect.x, rect.y, rect.w, t), color);
    push_rect(
        vertices,
        Rect::new(rect.x, rect.y + rect.h - t, rect.w, t),
        color,
    );
    push_rect(vertices, Rect::new(rect.x, rect.y, t, rect.h), color);
    push_rect(
        vertices,
        Rect::new(rect.x + rect.w - t, rect.y, t, rect.h),
        color,
    );
}

fn push_rect(vertices: &mut Vec<MapVertex>, rect: Rect, color: [f32; 4]) {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w;
    let y1 = rect.y + rect.h;
    push_vertices(
        vertices,
        [[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]],
        color,
    );
}

fn speech_hash(bubbles: &[NpcSpeechBubble]) -> u64 {
    // FNV-1a keeps cache invalidation deterministic without retaining the
    // current text or allocating a second UI model every frame.
    let mut hash = 14_695_981_039_346_656_037_u64;
    for bubble in bubbles {
        for value in [
            bubble.x.to_bits(),
            bubble.y.to_bits(),
            bubble.opacity.to_bits(),
        ] {
            hash ^= u64::from(value);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        for channel in bubble.accent {
            hash ^= u64::from(channel.to_bits());
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        for byte in bubble.speaker.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        for byte in bubble.text.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    hash
}

/// Where one speech bubble sits on screen, before drawing.
struct BubbleLayout {
    bubble: NpcSpeechBubble,
    rect: Rect,
    text: String,
    text_size: f32,
    name_size: f32,
}

fn layout_npc_speech_bubble(
    font: &FontAtlas,
    width: f32,
    height: f32,
    bubble: NpcSpeechBubble,
) -> Option<BubbleLayout> {
    let opacity = bubble.opacity.clamp(0.0, 1.0);
    if opacity <= 0.01 || !bubble.x.is_finite() || !bubble.y.is_finite() {
        return None;
    }
    let name_size = fit_label_size(font, bubble.speaker, 260.0, 16.0, 12.0);
    // Readable raster size: long lines wrap and grow the bubble instead of
    // shrinking into an unreadable strip.
    let text_size = 20.0;
    let max_text_width = (width - 72.0).clamp(220.0, 460.0);
    let text = wrap_label_text(font, bubble.text, text_size, max_text_width);
    let text_width = label_width(font, &text, text_size);
    let name_width = label_width(font, bubble.speaker, name_size) + 60.0;
    let bubble_width = (text_width + 48.0).max(name_width).clamp(170.0, 520.0);
    let lines = text.lines().count().max(1) as f32;
    let bubble_height = (text_size * 1.28 * lines + 42.0).clamp(66.0, (height - 60.0).max(66.0));
    let tail = 18.0;
    let half = bubble_width * 0.5;
    let centre = bubble.x.clamp(half + 12.0, width - half - 12.0);
    let top = (bubble.y - bubble_height - tail)
        .clamp(28.0, (height - bubble_height - tail - 12.0).max(28.0));
    Some(BubbleLayout {
        bubble,
        rect: Rect::new(centre - half, top, bubble_width, bubble_height),
        text,
        text_size,
        name_size,
    })
}

/// Lay out every bubble, lifting later ones clear of those they overlap so
/// two speakers side by side never talk over each other.
fn layout_npc_speech_bubbles(
    font: &FontAtlas,
    width: f32,
    height: f32,
    bubbles: &[NpcSpeechBubble],
) -> Vec<BubbleLayout> {
    let mut layouts: Vec<BubbleLayout> = bubbles
        .iter()
        .filter_map(|b| layout_npc_speech_bubble(font, width, height, *b))
        .collect();
    // Nearer the bottom of the screen first: those keep their place.
    layouts.sort_by(|a, b| b.bubble.y.total_cmp(&a.bubble.y));
    for i in 1..layouts.len() {
        for _ in 0..4 {
            let mut moved = false;
            for j in 0..i {
                let (a, b) = (layouts[j].rect, layouts[i].rect);
                let gap = 14.0;
                let overlap_x = a.x < b.x + b.w + gap && b.x < a.x + a.w + gap;
                let overlap_y = a.y < b.y + b.h + 20.0 && b.y < a.y + a.h + 20.0;
                if overlap_x && overlap_y {
                    layouts[i].rect.y = (a.y - b.h - 30.0).max(28.0);
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
    }
    layouts
}

/// `VELVET_DIALOGO_CLASICO=1`: the conversations' box as it was before the web look (to compare).
pub(crate) fn classic_dialogue() -> bool {
    static CLASSIC: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CLASSIC.get_or_init(|| std::env::var_os("VELVET_DIALOGO_CLASICO").is_some())
}

/// What the conversation box shows: who speaks, how much of the line is out, or the choice.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct DialogueView {
    /// The speaker's name (empty for narration and sounds).
    pub name: String,
    pub text: String,
    /// How many characters of `text` are shown (the line types itself out).
    pub shown: usize,
    /// 0 dialogue, 1 narration, 2 a sound, 3 a thought.
    pub kind: u8,
    /// The speaker's colour (0..255).
    pub accent: [u8; 3],
    /// A choice: its prompt and options (label, can be taken); empty when it is a line.
    pub prompt: Option<String>,
    pub choices: Vec<(String, bool)>,
    pub selected: usize,
    /// The speaker's face beside the box (`None`: no portrait).
    pub portrait: Option<PortraitRef>,
    /// Once the line is out, the mark that F goes on.
    pub continue_key: bool,
}

/// A portrait to show: an image of the game's art (a key under `data/assets`, no `.png`) and the
/// square holding the head and shoulders (`[x, y, width, height]`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct PortraitRef {
    pub image: String,
    pub crop: [u32; 4],
}

/// An sRGB colour (0xRRGGBB) as the overlay's vertex colour: the target is sRGB, so the shader
/// works in linear light.
fn srgb(hex: u32, alpha: f32) -> [f32; 4] {
    let channel = |shift: u32| {
        let c = ((hex >> shift) & 0xff) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(16), channel(8), channel(0), alpha]
}

/// Two sRGB colours mixed (0 all `a`, 1 all `b`).
fn mix_hex(a: u32, b: u32, t: f32) -> u32 {
    let ch = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x + (y - x) * t).round().clamp(0.0, 255.0) as u32) << shift
    };
    ch(16) | ch(8) | ch(0)
}

fn draw_npc_speech_bubble(font: &FontAtlas, vertices: &mut Vec<MapVertex>, layout: &BubbleLayout) {
    // The web scenes' look: a white card with a hard shadow in the speaker's colour, the name in
    // Anton on a dark plate turned a little over its top edge, the line in dark ink. (Measured and
    // placed as before: only how it is painted changed.)
    let bubble = layout.bubble;
    let opacity = bubble.opacity.clamp(0.0, 1.0);
    let rect = layout.rect;
    let accent = bubble.accent;
    let tint = |alpha: f32| [accent[0], accent[1], accent[2], alpha * opacity];
    let radius = 16.0;
    // The tail points at the speaker; it may stretch if the bubble was lifted.
    let tail_x = bubble.x.clamp(rect.x + 34.0, rect.x + rect.w - 34.0);
    let tail_tip = [
        bubble.x.clamp(rect.x + 10.0, rect.x + rect.w - 10.0),
        bubble.y.max(rect.y + rect.h + 8.0),
    ];
    let bottom = rect.y + rect.h;
    let tail = |dx: f32, dy: f32| {
        [
            [tail_x - 14.0 + dx, bottom - 2.0 + dy],
            [tail_x + 14.0 + dx, bottom - 2.0 + dy],
            [tail_tip[0] + dx, tail_tip[1] + dy],
        ]
    };
    // The shadow: the card and its tail again, down and to the right.
    push_rounded_rect(
        vertices,
        Rect::new(rect.x + 7.0, rect.y + 7.0, rect.w, rect.h),
        radius,
        tint(0.95),
    );
    push_triangle(vertices, tail(7.0, 7.0), tint(0.95));
    let white = ui_paint::rgba(0xf6f1ea, opacity);
    push_rounded_rect(vertices, rect, radius, white);
    push_triangle(vertices, tail(0.0, 0.0), white);
    // The name (Anton: the atlas sizes a face by its full height, hence the factor).
    let css = 18.0;
    let size = css * 1.51;
    let name = bubble.speaker.to_uppercase();
    let plate_w = ui_paint::text_width(font, ui_paint::Face::Display, &name, size) + 26.0;
    let plate = Rect::new(rect.x + 18.0, rect.y - 20.0, plate_w, 30.0);
    persona::turned(
        vertices,
        [plate.x + plate.w * 0.5, plate.y + plate.h * 0.5],
        -0.05,
        |v| {
            push_rect(
                v,
                Rect::new(plate.x + 4.0, plate.y + 4.0, plate.w, plate.h),
                tint(1.0),
            );
            push_rect(v, plate, ui_paint::rgba(0x0f070c, opacity));
            ui_paint::text(
                font,
                v,
                ui_paint::Face::Display,
                &name,
                plate.x + 13.0,
                plate.y + 3.0 - (size - css) * 0.5,
                size,
                ui_paint::rgba(0xf6f1ea, opacity),
            );
        },
    );
    // The line itself, in dark ink.
    push_label_centered(
        font,
        vertices,
        &layout.text,
        rect.x + rect.w * 0.5,
        rect.y + 22.0,
        layout.text_size,
        ui_paint::rgba(0x241a33, opacity),
    );
}

fn push_vertices(vertices: &mut Vec<MapVertex>, positions: [[f32; 2]; 6], color: [f32; 4]) {
    for position in positions {
        // Keep primitives in pixel space while composing the panel. One final
        // viewport conversion happens immediately before the GPU upload.
        vertices.push(MapVertex {
            position,
            uv: [0.0; 2],
            color,
            text: 0.0,
            ..MapVertex::BLANK
        });
    }
}

/// A soft edge outside a flat shape: from `a` to `b` (its outline, wound so `out` is outward) a
/// strip `width` pixels wide fading from `color` to clear. Flat triangles have hard, stepped
/// edges; this gives them the painter's shapes' smooth ones.
pub(super) fn push_fringe(
    vertices: &mut Vec<MapVertex>,
    a: [f32; 2],
    b: [f32; 2],
    out: [f32; 2],
    width: f32,
    color: [f32; 4],
) {
    let clear = [color[0], color[1], color[2], 0.0];
    let a2 = [a[0] + out[0] * width, a[1] + out[1] * width];
    let b2 = [b[0] + out[0] * width, b[1] + out[1] * width];
    for (position, c) in [(a, color), (b, color), (b2, clear), (a, color), (b2, clear), (a2, clear)] {
        vertices.push(MapVertex {
            position,
            uv: [0.0; 2],
            color: c,
            text: 0.0,
            ..MapVertex::BLANK
        });
    }
}

/// The soft edge round a convex outline (corners in order, either way round).
pub(super) fn push_outline_fringe(vertices: &mut Vec<MapVertex>, pts: &[[f32; 2]], color: [f32; 4]) {
    let n = pts.len();
    if n < 3 || color[3] <= 0.0 {
        return;
    }
    // Which way round: the sign of the area.
    let mut area = 0.0;
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        area += p[0] * q[1] - q[0] * p[1];
    }
    let turn = if area >= 0.0 { 1.0 } else { -1.0 };
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if len < 1e-3 {
            continue;
        }
        // Outward: to the right of a→b on screen (y down) when wound with positive area.
        let out = [-d[1] / len * turn, d[0] / len * turn];
        push_fringe(vertices, a, b, [-out[0], -out[1]], 1.1, color);
    }
}

fn push_triangle(vertices: &mut Vec<MapVertex>, positions: [[f32; 2]; 3], color: [f32; 4]) {
    for position in positions {
        vertices.push(MapVertex {
            position,
            uv: [0.0; 2],
            color,
            text: 0.0,
            ..MapVertex::BLANK
        });
    }
}

fn push_label_centered(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    text: &str,
    center_x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    push_label(
        font,
        vertices,
        text,
        center_x - label_width(font, text, size) * 0.5,
        y,
        size,
        color,
    );
}

fn push_label_centered_bold_shadowed(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    text: &str,
    center_x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let x = center_x - label_width(font, text, size) * 0.5;
    let shadow = (size * 0.075).clamp(1.2, 2.8);
    push_label(
        font,
        vertices,
        text,
        x + shadow,
        y + shadow,
        size,
        [0.005, 0.012, 0.014, 0.90],
    );
    let weight = (size * 0.035).clamp(0.55, 1.25);
    let stroke = [color[0], color[1], color[2], color[3] * 0.82];
    for (dx, dy) in [(-weight, 0.0), (weight, 0.0), (0.0, -weight), (0.0, weight)] {
        push_label(font, vertices, text, x + dx, y + dy, size, stroke);
    }
    push_label(font, vertices, text, x, y, size, color);
}

fn push_label_shadowed(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let offset = (size * 0.075).clamp(1.0, 2.0);
    push_label(
        font,
        vertices,
        text,
        x + offset,
        y + offset,
        size,
        [0.005, 0.012, 0.014, 0.82],
    );
    push_label(font, vertices, text, x, y, size, color);
}

fn push_label(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: [f32; 4],
) {
    let text = &*demon_queen_dungeon::i18n::tr(text);
    let size = size.max(7.0);
    let ratio = size / FONT_RASTER_SIZE;
    let mut baseline = y + font.ascent * ratio;
    for line in text.split('\n') {
        let run = font.run(0, &font.glyphs, line);
        for (glyph, pen, dy) in &run.glyphs {
            if glyph.size[0] > 0.0 && glyph.size[1] > 0.0 {
                let rect = Rect::new(
                    x + (pen + glyph.bearing[0]) * ratio,
                    baseline + (dy + glyph.bearing[1]) * ratio,
                    glyph.size[0] * ratio,
                    glyph.size[1] * ratio,
                );
                push_textured_rect(vertices, rect, glyph.uv_min, glyph.uv_max, color);
            }
        }
        baseline += size * 1.22;
    }
}

/// A portrait from the portrait atlas (the shader's third texture).
fn push_portrait_rect(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
) {
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.x + rect.w, rect.y + rect.h);
    for (position, uv) in [
        ([x0, y0], [uv_min[0], uv_min[1]]),
        ([x1, y0], [uv_max[0], uv_min[1]]),
        ([x1, y1], [uv_max[0], uv_max[1]]),
        ([x0, y0], [uv_min[0], uv_min[1]]),
        ([x1, y1], [uv_max[0], uv_max[1]]),
        ([x0, y1], [uv_min[0], uv_max[1]]),
    ] {
        vertices.push(MapVertex {
            position,
            uv,
            color,
            text: 3.0,
            ..MapVertex::BLANK
        });
    }
}

fn push_icon_rect(vertices: &mut Vec<MapVertex>, rect: Rect, icon: IconEntry, color: [f32; 4]) {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w;
    let y1 = rect.y + rect.h;
    for (position, uv) in [
        ([x0, y0], [icon.uv_min[0], icon.uv_min[1]]),
        ([x1, y0], [icon.uv_max[0], icon.uv_min[1]]),
        ([x1, y1], [icon.uv_max[0], icon.uv_max[1]]),
        ([x0, y0], [icon.uv_min[0], icon.uv_min[1]]),
        ([x1, y1], [icon.uv_max[0], icon.uv_max[1]]),
        ([x0, y1], [icon.uv_min[0], icon.uv_max[1]]),
    ] {
        vertices.push(MapVertex {
            position,
            uv,
            color,
            text: 2.0,
            ..MapVertex::BLANK
        });
    }
}

/// A glyph with a rim traced round it from its field: `rim` in screen pixels, `rim_color`.
pub(super) fn push_rimmed_glyph(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
    rim: f32,
    rim_color: [f32; 4],
    scale: f32,
) {
    // Screen pixels → field units: the field spans FONT_SPREAD raster pixels from 0.5 to 0 or 1,
    // and a raster pixel is `scale` screen pixels.
    let width = (rim / (scale * FONT_SPREAD as f32 * 2.0)).min(0.49);
    let start = vertices.len();
    push_textured_rect(vertices, rect, uv_min, uv_max, color);
    for vertex in &mut vertices[start..] {
        vertex.shape = [width, 0.0, 0.0, 0.0];
        vertex.color2 = rim_color;
    }
}

fn push_textured_rect(
    vertices: &mut Vec<MapVertex>,
    rect: Rect,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
) {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w;
    let y1 = rect.y + rect.h;
    for (position, uv) in [
        ([x0, y0], [uv_min[0], uv_min[1]]),
        ([x1, y0], [uv_max[0], uv_min[1]]),
        ([x1, y1], [uv_max[0], uv_max[1]]),
        ([x0, y0], [uv_min[0], uv_min[1]]),
        ([x1, y1], [uv_max[0], uv_max[1]]),
        ([x0, y1], [uv_min[0], uv_max[1]]),
    ] {
        vertices.push(MapVertex {
            position,
            uv,
            color,
            text: 1.0,
            ..MapVertex::BLANK
        });
    }
}

fn fit_label_size(
    font: &FontAtlas,
    text: &str,
    max_width: f32,
    preferred: f32,
    minimum: f32,
) -> f32 {
    let preferred_width = label_width(font, text, preferred).max(0.001);
    if preferred_width <= max_width {
        preferred
    } else {
        (preferred * max_width / preferred_width).clamp(minimum, preferred)
    }
}

pub(crate) fn label_width(font: &FontAtlas, text: &str, size: f32) -> f32 {
    let text = &*demon_queen_dungeon::i18n::tr(text);
    let ratio = size / FONT_RASTER_SIZE;
    text.split('\n')
        .map(|line| font.run(0, &font.glyphs, line).advance * ratio)
        .fold(0.0, f32::max)
}

/// Wrap dialogue at word boundaries while preserving explicit line breaks.
/// Oversized words are split by glyph so names/URLs cannot escape the panel.
fn wrap_label_text(font: &FontAtlas, text: &str, size: f32, max_width: f32) -> String {
    let text = &*demon_queen_dungeon::i18n::tr(text);
    let max_width = max_width.max(size * 4.0);
    let mut lines = Vec::new();
    for source_line in text.lines() {
        let mut current = String::new();
        for word in source_line.split_whitespace() {
            if label_width(font, word, size) > max_width {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let mut chunk = String::new();
                for character in word.chars() {
                    let mut candidate = chunk.clone();
                    candidate.push(character);
                    if !chunk.is_empty() && label_width(font, &candidate, size) > max_width {
                        lines.push(std::mem::take(&mut chunk));
                    }
                    chunk.push(character);
                }
                if !chunk.is_empty() {
                    current = chunk;
                }
                continue;
            }
            let candidate = if current.is_empty() {
                word.to_owned()
            } else {
                format!("{current} {word}")
            };
            if !current.is_empty() && label_width(font, &candidate, size) > max_width {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
            } else {
                current = candidate;
            }
        }
        lines.push(current);
    }
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

/// The achievements screen: a panel of every achievement (earned ones lit, hidden ones veiled)
/// and, beside it, a few counts of what she has done.
fn draw_achievements(
    font: &FontAtlas,
    vertices: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &AchievementsView,
) {
    let tone = map_kit::tone(Place::Castle);
    let panel_width = (width * 0.72).clamp(700.0, 1_120.0).min(width - 36.0);
    let panel_height = (height * 0.80).clamp(500.0, 760.0).min(height - 36.0);
    let panel = Rect::new(
        (width - panel_width) * 0.5,
        (height - panel_height) * 0.5,
        panel_width,
        panel_height,
    );
    push_rect(
        vertices,
        Rect::new(0.0, 0.0, width, height),
        [0.006, 0.008, 0.022, 0.72],
    );
    map_kit::soft_shadow(vertices, panel, 26.0, 26.0, 0.9);
    map_kit::round_gradient(vertices, panel, 26.0, tone.top, tone.bottom);
    push_rounded_border(vertices, panel, 26.0, 2.6, tone.rim);
    push_rounded_border(
        vertices,
        Rect::new(panel.x + 7.0, panel.y + 7.0, panel.w - 14.0, panel.h - 14.0),
        20.0,
        1.0,
        tone.rim_soft,
    );
    // Header: the crown, the title, how many are earned.
    let header = Rect::new(panel.x + 12.0, panel.y + 12.0, panel.w - 24.0, 92.0);
    map_kit::round_gradient(vertices, header, 18.0, tone.band_top, tone.band_bottom);
    push_rounded_border(vertices, header, 18.0, 1.0, tone.rim_soft);
    push_prompt_badge(
        vertices,
        prompt_icons::Icon::Crown,
        header.x + 52.0,
        header.y + header.h * 0.5,
        30.0,
        1.0,
    );
    push_label_shadowed(
        font,
        vertices,
        "Logros",
        header.x + 100.0,
        header.y + 16.0,
        32.0,
        tone.title,
    );
    push_label(
        font,
        vertices,
        "Lo que la Reina ha hecho, y lo que le ha valido",
        header.x + 102.0,
        header.y + 58.0,
        16.0,
        tone.subtitle,
    );
    let done = view.rows.iter().filter(|r| r.done).count();
    let badge = Rect::new(header.x + header.w - 150.0, header.y + 30.0, 126.0, 32.0);
    map_kit::pill(vertices, badge, [0.10, 0.05, 0.12, 0.96], tone.rim);
    push_label_centered(
        font,
        vertices,
        &format!("{} de {}", done, view.rows.len()),
        badge.x + badge.w * 0.5,
        badge.y + 7.0,
        15.0,
        tone.title,
    );
    // The list, on the left.
    let list = Rect::new(
        panel.x + 24.0,
        header.y + header.h + 18.0,
        panel.w * 0.64,
        panel.h - header.h - 70.0,
    );
    if view.rows.is_empty() {
        push_label(
            font,
            vertices,
            "Todavía no hay logros.",
            list.x + 8.0,
            list.y + 10.0,
            22.0,
            tone.title,
        );
        push_label(
            font,
            vertices,
            "Se irán añadiendo; lo que ya hace queda apuntado.",
            list.x + 8.0,
            list.y + 46.0,
            15.0,
            tone.soft,
        );
    }
    let row_h = 64.0;
    let rows_fit = ((list.h / (row_h + 8.0)).floor() as usize).max(1);
    for (k, row) in view.rows.iter().take(rows_fit).enumerate() {
        let r = Rect::new(list.x, list.y + k as f32 * (row_h + 8.0), list.w, row_h);
        let (top, bottom) = if row.done {
            ([0.17, 0.11, 0.20, 0.98], [0.10, 0.06, 0.13, 0.98])
        } else {
            ([0.07, 0.045, 0.09, 0.96], [0.045, 0.028, 0.06, 0.96])
        };
        map_kit::round_gradient(vertices, r, 14.0, top, bottom);
        push_rounded_border(
            vertices,
            r,
            14.0,
            if row.done { 1.8 } else { 1.0 },
            if row.done {
                [1.0, 0.78, 0.32, 0.95]
            } else {
                map_kit::with_alpha(tone.rim_soft, 0.6)
            },
        );
        push_prompt_badge(
            vertices,
            row.icon.unwrap_or(prompt_icons::Icon::Crown),
            r.x + 34.0,
            r.y + r.h * 0.5,
            22.0,
            if row.done { 1.0 } else { 0.35 },
        );
        push_label(
            font,
            vertices,
            &row.title,
            r.x + 66.0,
            r.y + 10.0,
            19.0,
            if row.done { tone.title } else { tone.ink },
        );
        push_label(
            font,
            vertices,
            &row.detail,
            r.x + 66.0,
            r.y + 36.0,
            13.5,
            tone.soft,
        );
        if !row.reward.is_empty() {
            let w = (row.reward.chars().count() as f32 * 7.4 + 24.0).min(r.w * 0.45);
            let pill = Rect::new(r.x + r.w - w - 12.0, r.y + 18.0, w, 26.0);
            map_kit::pill(
                vertices,
                pill,
                [0.10, 0.05, 0.12, 0.96],
                if row.done {
                    [1.0, 0.78, 0.32, 1.0]
                } else {
                    tone.rim_soft
                },
            );
            push_label_centered(
                font,
                vertices,
                &row.reward,
                pill.x + pill.w * 0.5,
                pill.y + 5.0,
                12.5,
                if row.done { tone.title } else { tone.soft },
            );
        }
    }
    if view.rows.len() > rows_fit {
        push_label(
            font,
            vertices,
            &format!("… y {} más", view.rows.len() - rows_fit),
            list.x + 8.0,
            list.y + list.h - 18.0,
            14.0,
            tone.soft,
        );
    }
    // The counts, on the right.
    let side = Rect::new(
        list.x + list.w + 20.0,
        list.y,
        panel.x + panel.w - 24.0 - (list.x + list.w + 20.0),
        list.h,
    );
    map_kit::round_gradient(
        vertices,
        side,
        16.0,
        [0.08, 0.05, 0.10, 0.96],
        [0.05, 0.03, 0.07, 0.96],
    );
    push_rounded_border(vertices, side, 16.0, 1.0, tone.rim_soft);
    push_label(
        font,
        vertices,
        "Su camino",
        side.x + 18.0,
        side.y + 14.0,
        19.0,
        tone.title,
    );
    for (k, (label, value)) in view.stats.iter().enumerate() {
        let y = side.y + 54.0 + k as f32 * 34.0;
        push_label(font, vertices, label, side.x + 18.0, y, 14.0, tone.soft);
        push_label(
            font,
            vertices,
            value,
            side.x + side.w - 18.0 - value.chars().count() as f32 * 8.0,
            y,
            14.0,
            tone.title,
        );
    }
    // Footer.
    push_label_centered(
        font,
        vertices,
        "L o Esc para cerrar",
        panel.x + panel.w * 0.5,
        panel.y + panel.h - 38.0,
        14.0,
        tone.soft,
    );
}
