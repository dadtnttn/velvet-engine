//! The intimate scenes' stage, drawn in code around the clip: the picture in a gold frame with
//! the house's horns on its corners and a crest that glows with the heat; the lines said as cards
//! with the speaker's face; the seal's vial (a medallion beating faster as the pleasure climbs,
//! liquid light with bubbles and a meniscus, gems where the next lines come); under the picture
//! the key to hold and the fuse of how long she lets you; and at the end, the card of what changed.
//! When she asks something, the answers come over the picture's foot on tilted tiles with the time
//! left to answer; in a scene with postures, their names under the key, ←/→ to change.
use super::persona::*;
use super::ui_paint::*;
use super::*;

/// What the stage shows this frame. The frame itself is uploaded apart ([`ScenePicture`]).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct SceneView {
    /// The pleasure, 0..=1000 (`None`: no vial).
    pub heat: Option<u16>,
    /// What is happening (over the key).
    pub phase: String,
    /// The key is held now.
    pub holding: bool,
    /// How close holding is to being stopped, 0..=1000 (`None`: nobody stops you here).
    pub hold: Option<u16>,
    /// She stopped you: let go and press again.
    pub braked: bool,
    /// Where on the vial the next lines come, 0..=1000.
    pub marks: Vec<u16>,
    /// The last lines said, oldest first.
    pub lines: Vec<SceneLine>,
    /// Narration or a sound, as a subtitle over the picture.
    pub caption: Option<String>,
    /// The key is part of the game now (it builds); after the burst only Esc counts.
    pub keys: bool,
    /// What Esc does now («parar», «saltar»).
    pub esc: String,
    /// The closing card.
    pub result: Option<SceneResult>,
    /// A new frame was uploaded (keeps the stage's signature moving).
    pub frame: u32,
    /// The burst of the climax lighting the room, 0..=1000.
    pub flash: u16,
    /// The light that answers the held key, 0..=1000 (it swells while held, ebbs after).
    pub glow: u16,
    /// A question open: its answers.
    pub prompt: Option<ScenePrompt>,
    /// The eager loop's postures, the one playing, and whether they can be changed now.
    pub poses: Vec<String>,
    pub pose: u8,
    pub pose_active: bool,
    /// The climax heard over black: no picture.
    pub black: bool,
    /// A girl's minigame (Alba's words, Noa's silence): it takes the key's place.
    pub game: Option<GameView>,
    /// A favour of the village shop: the scene plays as a page of the shop instead of this stage.
    pub deal: Option<super::shop_overlay::FavorDeal>,
}

/// A brothel girl's minigame on the stage (`explore/play_scene.rs`): what it shows instead of the
/// key to hold, and the light it throws.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct GameView {
    /// 0: Alba's words; 1: Noa's silence.
    pub kind: u8,
    /// The girl: her name, colour and face (for the rules card).
    pub who: String,
    pub accent: [u8; 3],
    pub face: Option<PortraitRef>,
    /// How to play, while it is up.
    pub rules: Option<GameRules>,
    /// Her shout over the picture.
    pub shout: Option<GameShout>,
    /// The tiles to pick from (her words, the copla's rhymes), each with its state.
    pub chips: Vec<GameChip>,
    /// The time left to pick, 0..=1000 (`None`: no fuse).
    pub chips_left: Option<u16>,
    /// Since the tiles came (ms): they come in one by one.
    pub chips_age: u32,
    /// Right answers in a row.
    pub streak: u16,
    /// A stamp across the picture («¡RACHA ×3!», «¡TOC, TOC!»): its words, colour, age (ms).
    pub stamp: Option<(String, [u8; 3], u32)>,
    /// The copla on show (Alba's finale), the missing word last.
    pub verse: Option<String>,
    /// Noa: the noise 0..=1000 and the line the wall takes before they knock, 0..=1000.
    pub noise: Option<u16>,
    pub limit: u16,
    /// Her air, 0..=1000 (`None`: her mouth is free).
    pub air: Option<u16>,
    /// Quiet enough to count double (the «sigilo»).
    pub sneaky: bool,
    /// The neighbour: 0 asleep, 1 stirring, 2 listening, 3 knocking; and since when (ms).
    pub neighbour: u8,
    pub neighbour_ms: u32,
    /// The postures: names and whether each is open; the one on show; ←/→ work now.
    pub poses: Vec<(String, bool)>,
    pub pose: u8,
    pub pose_active: bool,
    /// The keys on the plate: the key, what it does, lit now.
    pub keys: Vec<(String, String, bool)>,
    /// The room's light now (the posture's colour), and an event's flash: colour and strength
    /// 0..=1000.
    pub tint: [u8; 3],
    pub pulse_tint: [u8; 3],
    pub pulse: u16,
    /// The stage shakes this many pixels (×10).
    pub shake: u16,
    /// The hypnotist's game: his ring and her gaze over the picture, the second column.
    pub hypno: Option<HypnoView>,
    /// A shift in Rosalía's house: her beat's ring and his stamina.
    pub turno: Option<TurnoView>,
}

/// «Al ritmo de la Reina» on the stage (`explore/turno.rs`).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct TurnoView {
    /// Her beat is going round (not during a bridge, the pick or the ending).
    pub beating: bool,
    /// How far the ring has closed on the next beat, 0..=1000.
    pub ring: u16,
    /// Since the last beat on time and off it (ms; `u32::MAX`: none yet).
    pub hit_ms: u32,
    pub miss_ms: u32,
    /// His stamina, 0..=1000: full, it is over.
    pub stamina: u16,
    /// Shift held (holding back).
    pub held: bool,
    /// On the side view; the clip has both views.
    pub side: bool,
    pub views: bool,
    /// The tip so far (gold).
    pub tip: u16,
    /// Beats on time in a row.
    pub streak: u16,
    /// The pips under the picture: one a loop and the last for the ending, and the one he is on.
    pub at: u8,
    pub steps: u8,
    /// Who he is, what the shift is, and his face (on his stamina's medallion).
    pub name: String,
    pub act: String,
    pub face: Option<PortraitRef>,
    /// Her loop's name thrown over the picture when it changes, and since when (ms).
    pub cut: String,
    pub cut_ms: u32,
    /// Since the take on show began, when it has both views (ms; `u32::MAX`: one view only).
    pub view_new: u32,
    /// Her first loop and he has not caught her beat yet: the hint by the ring.
    pub teach: bool,
}

/// «No mires el anillo» on the stage (`explore/hipno.rs`).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct HypnoView {
    /// 0 resisting (the ring and her gaze over the picture), 1 her mouth busy, 2 over.
    pub mode: u8,
    pub stage: u8,
    /// The ring: where its string hangs from and where it is (per mille of the picture, plus 200
    /// so a little outside still fits), how its face turns to us (0..=1000), how bright it burns
    /// after a whisper or an order (0..=1000).
    pub pivot: [u16; 2],
    pub ring: [u16; 2],
    pub turn: u16,
    pub burn: u16,
    /// The trance swirling round it (0..=1000).
    pub trance: u16,
    /// Her gaze across the picture (per mille plus 200, as the ring), on the ring now, held there
    /// on purpose.
    pub gaze: u16,
    pub on_ring: bool,
    pub staring: bool,
    /// Chiquita in front of the ring; Chiquita hypnotised herself.
    pub covered: bool,
    pub dazed: bool,
    /// What he whispers, drifting round the ring: the words, their age (ms), a seed for the drift.
    pub whispers: Vec<(String, u32, u16)>,
    /// His order on show: its words and age (ms).
    pub order: Option<(String, u32)>,
    /// The second column, «Despertar» or «Lucidez», 0..=1000.
    pub awake: u16,
    pub awake_label: String,
    /// Since the last E (ms).
    pub mash_ms: u32,
    /// Smelling salts left, the amulet worn, since the salts were sniffed (ms).
    pub salts: u8,
    pub amulet: bool,
    pub salt_ms: u32,
}

/// How a minigame is played: a title and rows of a key (or a word) and what it does.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct GameRules {
    pub title: String,
    pub rows: Vec<(String, String)>,
    /// Since it came up, and until it goes (ms): it slides in and fades out.
    pub age_ms: u32,
    pub left_ms: u32,
}

/// What she shouts: the words (garbled the hotter she is), wild when it shakes, its age (ms).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct GameShout {
    pub text: String,
    pub wild: bool,
    pub age_ms: u32,
}

/// One tile: its words and 0 open, 1 the right one, 2 the wrong pick, 3 put out; a word over it
/// («OBEDECER») and its colour (0 the game's, 1 violet, 2 sea green, 3 gold).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct GameChip {
    pub label: String,
    pub state: u8,
    pub tag: String,
    pub tone: u8,
}

/// The answers to her question.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct ScenePrompt {
    pub answers: Vec<String>,
    /// The one lit (Enter takes it).
    pub lit: u8,
    /// Time left to answer, 0..=1000.
    pub left: u16,
    /// The asker's colour.
    pub accent: [u8; 3],
}

/// One line on a card.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct SceneLine {
    pub name: String,
    pub text: String,
    pub accent: [u8; 3],
    /// A thought (in parentheses, lilac, italic).
    pub thought: bool,
    pub portrait: Option<PortraitRef>,
    /// 0..=255: fades in when new.
    pub alpha: u8,
    /// 0 the newest, 1 and 2 older.
    pub age: u8,
}

/// What changed, when the scene is over.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct SceneResult {
    pub title: String,
    /// Whose scene it was («Nina», «La cama»).
    pub who: String,
    /// Finished (the seal is fed) or left half-way.
    pub complete: bool,
    /// Each value that changed and by how much.
    pub effects: Vec<(String, i32)>,
    pub seconds: u32,
    /// The last word, and who says it.
    pub quote: SceneLine,
    /// How long the card has been up, in milliseconds (it opens in steps).
    pub age_ms: u32,
}

/// The clip's frame as a texture, rewritten in place every time a new one is decoded.
pub(super) struct ScenePicture {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    texture: Option<(wgpu::Texture, u32, u32)>,
    pub(super) bind_group: wgpu::BindGroup,
}

impl ScenePicture {
    pub(super) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-scene-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let (_placeholder, bind_group) = Self::create(device, layout, &sampler, 1, 1);
        Self {
            layout: layout.clone(),
            sampler,
            texture: None,
            bind_group,
        }
    }

    fn create(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("demon-queen-scene-frame"),
            size: wgpu::Extent3d {
                width,
                height,
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("demon-queen-scene-group"),
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
        });
        (texture, bind_group)
    }

    /// Writes a decoded frame (RGBA8), making the texture first when the size changes.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        if rgba.len() != (width * height * 4) as usize {
            return;
        }
        if !matches!(&self.texture, Some((_, w, h)) if *w == width && *h == height) {
            let (texture, bind_group) =
                Self::create(device, &self.layout, &self.sampler, width, height);
            self.texture = Some((texture, width, height));
            self.bind_group = bind_group;
        }
        let Some((texture, _, _)) = &self.texture else {
            return;
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(super) fn has_frame(&self) -> bool {
        self.texture.is_some()
    }
}

/// The picture's place: a square centred near the top of the screen.
pub(crate) fn picture_rect(width: f32, height: f32) -> Rect {
    let s = height / 1080.0;
    let side = (700.0 * s).min(width - 80.0).max(64.0);
    Rect::new((width - side) * 0.5, 46.0 * s, side, side)
}

pub(super) fn hex3(c: [u8; 3]) -> u32 {
    ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32
}

pub(super) type Uv = ([f32; 2], [f32; 2]);

/// The whole stage. Over, the closing card stands beside the picture, never on it.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_scene(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &SceneView,
    faces: &[Option<Uv>],
    quote_face: Option<Uv>,
    game_faces: [Option<Uv>; 2],
    has_frame: bool,
    time: f32,
) {
    // A minigame can shake the whole stage (a knock on the wall, a cough, a wrong word).
    let shake = view.game.as_ref().map_or(0.0, |g| g.shake as f32 / 10.0) * height / 1080.0;
    let by = if shake > 0.05 {
        [
            (time * 71.0).sin() * shake,
            (time * 53.0 + 1.3).cos() * shake * 0.6,
        ]
    } else {
        [0.0, 0.0]
    };
    shifted(v, by, |v| {
        draw_stage(
            font, v, width, height, view, faces, quote_face, game_faces, has_frame, time,
        )
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_stage(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &SceneView,
    faces: &[Option<Uv>],
    quote_face: Option<Uv>,
    game_faces: [Option<Uv>; 2],
    has_frame: bool,
    time: f32,
) {
    let [game_face, client_face] = game_faces;
    let s = height / 1080.0;
    let pic = picture_rect(width, height);
    let frame = grow(pic, 12.0 * s);
    let heat = view.heat.unwrap_or(0) as f32 / 1000.0;
    let held = if view.braked {
        0.0
    } else {
        view.glow as f32 / 1000.0
    };
    // The world steps back.
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05020a, 0.5));
    let flash = view.flash as f32 / 1000.0;
    // The burst lights the whole room for a moment.
    if flash > 0.0 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0xff4a7a, 0.08 * flash),
        );
    }
    // After the burst (no keys, no flash) the room settles: the crimson glow dims to an ember.
    let glow = if view.keys || flash > 0.0 {
        heat
    } else {
        heat * 0.3
    };
    let beat = if view.braked {
        0.0
    } else {
        heartbeat(time, 54.0 + 76.0 * heat)
    };
    room_glow(v, frame, glow, held, beat, view.braked, s);
    if let Some(game) = &view.game {
        super::game_overlay::room_light(v, width, height, frame, game, heat, time, s);
    }
    draw_frame(
        v,
        pic,
        frame,
        heat,
        glow,
        flash,
        held,
        view.braked,
        has_frame && !view.black,
        time,
        s,
    );
    if let Some(game) = &view.game {
        super::game_overlay::picture_light(v, pic, game, has_frame && !view.black, time, s);
        super::game_overlay::over_picture(font, v, pic, game, view.result.is_some(), time, s);
    }
    if let Some(caption) = &view.caption {
        draw_caption(font, v, pic, caption, s);
    }
    match view
        .game
        .as_ref()
        .and_then(|g| g.rules.as_ref().map(|r| (g, r)))
    {
        Some((game, rules)) if view.result.is_none() => {
            super::game_overlay::draw_rules(font, v, frame, game, rules, game_face, time, s)
        }
        _ => draw_cards(font, v, frame, &view.lines, faces, time, s),
    }
    if view.heat.is_some() {
        draw_vial(font, v, frame, view, heat, time, s);
    }
    if let Some(game) = &view.game {
        super::game_overlay::draw_wall(font, v, frame, game, client_face, time, s);
    }
    match &view.result {
        Some(result) => draw_result(font, v, pic, frame, result, quote_face, time, s),
        None if view.game.is_some() => {
            let game = view.game.as_ref().expect("a game");
            super::game_overlay::draw_plate(font, v, pic, frame, view, game, time, s);
            if game.pose_active || !game.poses.is_empty() {
                super::game_overlay::draw_postures(font, v, pic, frame, game, time, s);
            }
        }
        None => {
            draw_keys(font, v, pic, frame, view, held, time, s);
            if let Some(prompt) = &view.prompt {
                draw_prompt(font, v, pic, prompt, time, s);
            }
            if view.pose_active {
                draw_poses(font, v, pic, frame, view, time, s);
            }
        }
    }
}

/// The room warms behind the picture: a broad glow round the frame, candle-gold at first, rose and
/// crimson as the pleasure climbs, swelling while the key is held and beating with the heart; lilac
/// and still when she has stopped you.
fn room_glow(
    v: &mut Vec<MapVertex>,
    frame: Rect,
    heat: f32,
    held: f32,
    beat: f32,
    braked: bool,
    s: f32,
) {
    if braked {
        soft(
            v,
            grow(frame, 20.0 * s),
            40.0 * s,
            110.0 * s,
            rgba(0x8a7aff, 0.12),
            false,
        );
        return;
    }
    let warm = mix(CANDLE, 0xff2a5a, heat);
    let strength = (0.05 + 0.16 * heat + 0.12 * held) * (0.72 + 0.28 * beat);
    soft(
        v,
        grow(frame, 30.0 * s),
        40.0 * s,
        (110.0 + 120.0 * heat) * s,
        rgba(warm, strength),
        false,
    );
    // Low and wide, as if from the bed: the floor of the frame catches more of it.
    soft(
        v,
        Rect::new(
            frame.x - 60.0 * s,
            frame.y + frame.h * 0.6,
            frame.w + 120.0 * s,
            frame.h * 0.5,
        ),
        60.0 * s,
        90.0 * s,
        rgba(mix(0xff6a9a, 0xff2a5a, heat), (0.04 + 0.1 * held) * heat),
        false,
    );
}

/// A point `u` (0..1) of the way round `r`'s edge, clockwise from its top-left corner.
fn perimeter_point(r: Rect, u: f32) -> [f32; 2] {
    let total = 2.0 * (r.w + r.h);
    let mut d = u.rem_euclid(1.0) * total;
    if d < r.w {
        return [r.x + d, r.y];
    }
    d -= r.w;
    if d < r.h {
        return [r.x + r.w, r.y + d];
    }
    d -= r.h;
    if d < r.w {
        return [r.x + r.w - d, r.y + r.h];
    }
    d -= r.w;
    [r.x, r.y + r.h - d]
}

/// Candle bokeh drifting up through the picture: soft orbs of light, warm at first and rosier and
/// more of them as the pleasure climbs. Each stays inside the picture, fading in and out.
fn bokeh(v: &mut Vec<MapVertex>, pic: Rect, heat: f32, held: f32, time: f32, s: f32) {
    let count = 8 + (heat * 18.0 + held * 6.0) as usize;
    for i in 0..count {
        let fi = i as f32;
        let speed = 0.03 + 0.03 * ((fi * 1.37).sin() * 0.5 + 0.5) + 0.05 * heat;
        let phase = (time * speed + fi * 0.618).fract();
        let size = (2.5 + 4.5 * ((fi * 2.17).cos() * 0.5 + 0.5)) * s;
        let blur = size * 2.2;
        let margin = size + blur * 2.2 + 6.0 * s;
        let across = ((fi * 0.7548).fract() + 0.025 * (time * 0.4 + fi).sin()).rem_euclid(1.0);
        let x = pic.x + margin + (pic.w - 2.0 * margin) * across;
        let y = pic.y + pic.h - margin - (pic.h - 2.0 * margin) * phase;
        let life = (phase * (1.0 - phase) * 4.0).clamp(0.0, 1.0);
        let twinkle = 0.7 + 0.3 * (time * (1.3 + fi * 0.11)).sin();
        let tone = mix(
            0xffb878,
            0xff6aa8,
            (heat * 1.15 + 0.2 * (fi * 0.9).sin()).clamp(0.0, 1.0),
        );
        let a = (0.08 + 0.12 * heat + 0.06 * held) * life * twinkle;
        halo(v, [x, y], size, blur, rgba(tone, a));
        // A bright core: a point of light, not a smudge.
        disc(
            v,
            [x, y],
            size * 0.45,
            rgba(mix(tone, 0xffffff, 0.6), (a * 2.2).min(0.6)),
        );
    }
}

/// How far the answers' tiles lean.
const ANSWER_ANGLE: f32 = -0.03;

/// The answers' tiles: over the picture's foot, the first highest.
fn answer_tiles(pic: Rect, s: f32) -> [Rect; 3] {
    let h = 62.0 * s;
    let gap = 12.0 * s;
    let x = pic.x + 40.0 * s;
    let w = pic.w - 80.0 * s;
    let bottom = pic.y + pic.h - 34.0 * s;
    [0, 1, 2].map(|i| {
        Rect::new(
            x + i as f32 * 10.0 * s,
            bottom - (3 - i) as f32 * (h + gap) + gap,
            w,
            h,
        )
    })
}

/// The answer under the pointer, if any (the screen's logical 1080p).
pub(crate) fn prompt_hit(width: f32, height: f32, x: f32, y: f32) -> Option<usize> {
    let s = height / 1080.0;
    let pic = picture_rect(width, height);
    answer_tiles(pic, s).iter().position(|r| {
        let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
        let (sn, cs) = (-ANSWER_ANGLE).sin_cos();
        let (dx, dy) = (x - c[0], y - c[1]);
        r.contains(c[0] + dx * cs - dy * sn, c[1] + dx * sn + dy * cs)
    })
}

/// Her question's answers on tilted tiles over the picture's foot, the lit one in her colour with
/// the cursor at it, and a fuse of the time left.
fn draw_prompt(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    prompt: &ScenePrompt,
    time: f32,
    s: f32,
) {
    let accent = hex3(prompt.accent);
    // A scrim up from the picture's foot, so the tiles read over any frame.
    panel(
        v,
        Rect::new(pic.x, pic.y + pic.h * 0.5, pic.w, pic.h * 0.5),
        16.0 * s,
        Panel {
            top: rgba(SHADOW, 0.0),
            bottom: rgba(SHADOW, 0.82),
            ..Panel::fill(0, 0, 0.0)
        },
    );
    let tiles = answer_tiles(pic, s);
    for (i, (r, label)) in tiles.iter().zip(&prompt.answers).enumerate() {
        let lit = i == prompt.lit as usize;
        let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
        turned(v, c, ANSWER_ANGLE, |v| {
            soft(
                v,
                Rect::new(r.x + 8.0 * s, r.y + 10.0 * s, r.w, r.h),
                8.0 * s,
                10.0 * s,
                rgba(SHADOW, 0.7),
                false,
            );
            if lit {
                soft(
                    v,
                    *r,
                    8.0 * s,
                    18.0 * s,
                    rgba(accent, 0.45 + 0.12 * wave(time, 0.9)),
                    true,
                );
            }
            panel(
                v,
                grow(*r, 3.5 * s),
                9.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            let (top, bottom, rim) = if lit {
                (
                    mix(0x3a0f2a, accent, 0.45),
                    mix(0x14040c, accent, 0.18),
                    GOLD_HI,
                )
            } else {
                (0x22102a, 0x0e0610, GOLD_DEEP)
            };
            panel(
                v,
                *r,
                7.0 * s,
                Panel::fill(top, bottom, 0.97)
                    .rim(rim, 1.0, 2.0 * s)
                    .bevel(0.6),
            );
            let cap = Rect::new(
                r.x + 14.0 * s,
                r.y + (r.h - 38.0 * s) * 0.5,
                40.0 * s,
                38.0 * s,
            );
            keycap(
                font,
                v,
                cap,
                &format!("{}", i + 1),
                19.0 * s,
                lit,
                false,
                accent,
                1.0,
            );
            let size = 25.0 * s;
            let room = r.w - 90.0 * s;
            let mut fitted = size;
            while fitted > 17.0 * s && text_width(font, Face::Body, label, fitted) > room {
                fitted -= 0.5 * s;
            }
            text_outline(
                font,
                v,
                Face::Body,
                label,
                cap.x + cap.w + 16.0 * s,
                r.y + (r.h - fitted) * 0.5 - 3.0 * s,
                fitted,
                rgba(INK, 1.0),
                rgba(0x0b0508, 0.8),
            );
        });
        if lit {
            let (sn, cs) = ANSWER_ANGLE.sin_cos();
            let half = r.w * 0.5;
            cursor(
                v,
                [c[0] - half * cs - 10.0 * s, c[1] - half * sn],
                26.0 * s,
                time,
            );
        }
    }
    // The time left, a fuse over the first tile.
    let first = tiles[0];
    let left = prompt.left as f32 / 1000.0;
    let fuse = Rect::new(first.x, first.y - 22.0 * s, first.w, 7.0 * s);
    turned(
        v,
        [first.x + first.w * 0.5, first.y + first.h * 0.5],
        ANSWER_ANGLE,
        |v| {
            panel(
                v,
                fuse,
                3.5 * s,
                Panel::fill(0x1a0d22, 0x120818, 0.95).rim(GOLD_DEEP, 0.8, 1.0 * s),
            );
            if left > 0.01 {
                let lit = Rect::new(fuse.x, fuse.y, fuse.w * left, fuse.h);
                panel(
                    v,
                    lit,
                    3.5 * s,
                    Panel::fill(mix(CRIMSON, GOLD_HI, left), mix(0x8a1030, GOLD, left), 1.0),
                );
                sparkle(
                    v,
                    [lit.x + lit.w, fuse.y + fuse.h * 0.5],
                    9.0 * s,
                    rgba(0xfff2c8, 1.0),
                    0.8,
                );
            }
        },
    );
}

/// The postures' strip under the key plate: ◀ and ▶ either side, the one playing lit.
fn pose_strip(pic: Rect, frame: Rect, s: f32) -> (Rect, Rect, Rect) {
    let y = frame.y + frame.h + 22.0 * s + 152.0 * s + 14.0 * s;
    let strip = Rect::new(pic.x + pic.w * 0.5 - 250.0 * s, y, 500.0 * s, 52.0 * s);
    let left = Rect::new(strip.x - 64.0 * s, y, 54.0 * s, 52.0 * s);
    let right = Rect::new(strip.x + strip.w + 10.0 * s, y, 54.0 * s, 52.0 * s);
    (strip, left, right)
}

/// ◀ or ▶ under the pointer: −1 or +1.
pub(crate) fn pose_hit(width: f32, height: f32, x: f32, y: f32) -> Option<i32> {
    let s = height / 1080.0;
    let pic = picture_rect(width, height);
    let frame = grow(pic, 12.0 * s);
    let (_, left, right) = pose_strip(pic, frame, s);
    if left.contains(x, y) {
        Some(-1)
    } else if right.contains(x, y) {
        Some(1)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_poses(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    view: &SceneView,
    time: f32,
    s: f32,
) {
    let (strip, left, right) = pose_strip(pic, frame, s);
    turned(
        v,
        [strip.x + strip.w * 0.5, strip.y + strip.h * 0.5],
        -0.025,
        |v| {
            soft(
                v,
                Rect::new(strip.x + 8.0 * s, strip.y + 10.0 * s, strip.w, strip.h),
                8.0 * s,
                10.0 * s,
                rgba(SHADOW, 0.7),
                false,
            );
            panel(
                v,
                grow(strip, 3.5 * s),
                8.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(
                v,
                strip,
                6.0 * s,
                Panel::fill(0x2a0c22, 0x10040c, 0.97).rim(GOLD, 1.0, 2.0 * s),
            );
            let n = view.poses.len().max(1);
            let cell = strip.w / n as f32;
            for (i, name) in view.poses.iter().enumerate() {
                let lit = i == view.pose as usize;
                let r = Rect::new(
                    strip.x + cell * i as f32 + 6.0 * s,
                    strip.y + 6.0 * s,
                    cell - 12.0 * s,
                    strip.h - 12.0 * s,
                );
                if lit {
                    soft(
                        v,
                        r,
                        6.0 * s,
                        12.0 * s,
                        rgba(CRIMSON, 0.4 + 0.1 * wave(time, 0.8)),
                        true,
                    );
                    panel(
                        v,
                        r,
                        5.0 * s,
                        Panel::fill(0xd02a52, 0x7a1030, 1.0).sheen(time * 0.3),
                    );
                }
                let size = 24.0 * s;
                let label = name.to_uppercase();
                let tw = text_width(font, Face::Title, &label, size);
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &label,
                    r.x + (r.w - tw) * 0.5,
                    r.y + (r.h - size) * 0.5 - 3.0 * s,
                    size,
                    rgba(if lit { 0xffffff } else { DIM }, 1.0),
                    rgba(0x0b0508, 0.85),
                );
            }
        },
    );
    keycap(font, v, left, "A", 20.0 * s, false, false, CRIMSON, 1.0);
    keycap(font, v, right, "D", 20.0 * s, false, false, CRIMSON, 1.0);
    for (r, dir) in [(left, -1.0f32), (right, 1.0)] {
        let c = [
            r.x + r.w * 0.5 + dir * (r.w * 0.5 + 14.0 * s),
            r.y + r.h * 0.5,
        ];
        let tip = [c[0] + dir * 7.0 * s, c[1]];
        for side in [-1.0f32, 1.0] {
            wire(
                v,
                [c[0] - dir * 4.0 * s, c[1] + side * 9.0 * s],
                tip,
                3.0 * s,
                rgba(GOLD_HI, 0.95),
                rgba(GOLD_HI, 0.95),
                0.0,
                true,
            );
        }
    }
    text_shadow(
        font,
        v,
        Face::Italic,
        "postura",
        right.x + right.w + 40.0 * s,
        strip.y + 13.0 * s,
        20.0 * s,
        rgba(DIM, 1.0),
    );
}

/// The picture in its frame: a shadow, a glow of the heat, velvet and gold, horns on the upper
/// corners, a crest at the top beating with the heart. While the key is held, two sparks of light
/// chase each other round the gold rim, faster as it climbs; near the top, the rim glitters.
#[allow(clippy::too_many_arguments)]
fn draw_frame(
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    heat: f32,
    glow: f32,
    flash: f32,
    held: f32,
    braked: bool,
    has_frame: bool,
    time: f32,
    s: f32,
) {
    let cx = pic.x + pic.w * 0.5;
    let beat = if braked {
        0.0
    } else {
        heartbeat(time, 54.0 + 76.0 * heat)
    };
    let ember = mix(0x7a3cff, 0xff2a5a, heat);
    let candle = candle_flicker(time);
    soft(
        v,
        Rect::new(frame.x, frame.y + 20.0 * s, frame.w, frame.h),
        30.0 * s,
        36.0 * s,
        rgba(SHADOW, 0.85),
        false,
    );
    // The room's candles spill warm light around the frame, breathing as they flicker.
    soft(
        v,
        frame,
        28.0 * s,
        60.0 * s,
        rgba(CANDLE, 0.05 + 0.07 * candle),
        true,
    );
    if flash > 0.0 {
        soft(
            v,
            frame,
            28.0 * s,
            (90.0 + 80.0 * flash) * s,
            rgba(0xff6ea0, 0.45 * flash),
            true,
        );
    }
    soft(
        v,
        frame,
        28.0 * s,
        (22.0 + 14.0 * heat) * s,
        rgba(ember, (0.06 + 0.2 * heat) * (0.7 + 0.3 * beat)),
        true,
    );
    if held > 0.0 {
        soft(
            v,
            frame,
            28.0 * s,
            (16.0 + 18.0 * held) * s,
            rgba(0xff7ab0, held * (0.18 + 0.2 * beat)),
            true,
        );
    }
    panel(
        v,
        frame,
        26.0 * s,
        Panel::velvet(1.0, 3.6 * s).hairline(7.0 * s, 0.55),
    );
    if has_frame {
        picture(v, pic, 16.0 * s);
        scene_light(v, pic, glow, flash, held, beat, candle, braked, time, s);
    } else {
        panel(v, pic, 16.0 * s, Panel::fill(0x07030a, 0x07030a, 1.0));
        // Over black, only the burst's light.
        if flash > 0.0 {
            vignette(v, pic, 16.0 * s, pic.w * 0.4, rgba(0xff6ea0, 0.5 * flash));
        }
    }
    // A fine gold line hugging the picture.
    panel(
        v,
        grow(pic, 1.5 * s),
        17.5 * s,
        Panel::fill(0x000000, 0x000000, 0.0).rim(GOLD, 0.7, 1.4 * s),
    );
    for (at, sx, sy, horn) in [
        ([frame.x, frame.y], 1.0, 1.0, true),
        ([frame.x + frame.w, frame.y], -1.0, 1.0, true),
        ([frame.x, frame.y + frame.h], 1.0, -1.0, false),
        ([frame.x + frame.w, frame.y + frame.h], -1.0, -1.0, false),
    ] {
        corner(v, at, sx, sy, s * 1.1, CRIMSON, horn, 1.0);
    }
    let stone = if braked {
        0x8a7aa8
    } else {
        mix(0x9a2244, 0xff3a66, heat)
    };
    crest(
        v,
        [cx, frame.y - 4.0 * s],
        44.0 * s,
        stone,
        1.0,
        (5.0 + 12.0 * heat * beat) * s,
    );
    rule(
        v,
        [cx, frame.y + frame.h],
        130.0 * s,
        CRIMSON,
        s,
        false,
        1.0,
    );
    // The held key's light running round the rim: two sparks, each with a short tail.
    if held > 0.02 {
        let rim = grow(pic, 6.0 * s);
        let speed = 0.06 + 0.16 * heat;
        for start in [0.0f32, 0.5] {
            for k in 0..4 {
                let u = time * speed + start - k as f32 * 0.012;
                let p = perimeter_point(rim, u);
                let tail = 1.0 - k as f32 / 4.0;
                halo(
                    v,
                    p,
                    (5.0 * tail + 1.5) * s,
                    (10.0 * tail + 4.0) * s,
                    rgba(0xffd6e6, held * 0.75 * tail),
                );
            }
            sparkle(
                v,
                perimeter_point(rim, time * speed + start),
                (8.0 + 6.0 * beat) * s,
                rgba(0xffffff, held),
                0.9,
            );
        }
    }
    // Near the top the gold glitters all round.
    if heat > 0.72 && !braked {
        let near = ((heat - 0.72) / 0.28).clamp(0.0, 1.0);
        for i in 0..10 {
            let fi = i as f32;
            let tw = (time * (2.0 + fi * 0.37) + fi).sin() * 0.5 + 0.5;
            let p = perimeter_point(frame, (fi * 0.1 + 0.031 * fi * fi).fract());
            sparkle(v, p, (3.0 + 7.0 * tw) * s, rgba(0xfff0f6, near * tw), 0.8);
        }
    }
}

/// Warm candle light.
const CANDLE: u32 = 0xffa24a;

/// How bright the candles burn now, 0..1: a few slow waves and a quick tremble, never still.
fn candle_flicker(time: f32) -> f32 {
    let w = 0.5
        + 0.22 * (time * 1.7).sin()
        + 0.14 * (time * 4.3 + 1.3).sin()
        + 0.09 * (time * 9.1 + 0.4).sin()
        + 0.05 * (time * 17.9 + 2.2).sin();
    w.clamp(0.0, 1.0)
}

/// The light of the room laid over the clip: the candles' warmth and a little shadow at the
/// corners, candle bokeh drifting up, the seal's crimson climbing in from the edges and beating with
/// the pleasure; while the key is held a rose rim light breathing with each beat and a warm wash
/// from below; a cool lilac breath when she stops you; and the burst of the climax.
#[allow(clippy::too_many_arguments)]
fn scene_light(
    v: &mut Vec<MapVertex>,
    pic: Rect,
    heat: f32,
    flash: f32,
    held: f32,
    beat: f32,
    candle: f32,
    braked: bool,
    time: f32,
    s: f32,
) {
    let r = 16.0 * s;
    // The corners sink a little into shadow when the flames dip.
    vignette(
        v,
        pic,
        r,
        pic.w * 0.16,
        rgba(0x05020a, 0.18 - 0.08 * candle),
    );
    // Warm candlelight pooling in from the edges.
    vignette(v, pic, r, pic.w * 0.24, rgba(CANDLE, 0.03 + 0.05 * candle));
    bokeh(v, pic, heat, held, time, s);
    if braked {
        // She has stopped you: a cool breath over everything.
        vignette(v, pic, r, pic.w * 0.14, rgba(0xa898ff, 0.22));
        panel(v, pic, r, Panel::fill(0x8a7aff, 0x4a3a8a, 0.05));
    } else {
        if heat > 0.0 {
            vignette(
                v,
                pic,
                r,
                pic.w * (0.06 + 0.1 * heat),
                rgba(0xff2a5a, heat * (0.05 + 0.12 * beat)),
            );
        }
        if held > 0.0 {
            vignette(
                v,
                pic,
                r,
                pic.w * (0.03 + 0.04 * heat),
                rgba(0xff8ab8, held * (0.1 + 0.14 * beat)),
            );
            panel(
                v,
                Rect::new(pic.x, pic.y + pic.h * 0.5, pic.w, pic.h * 0.5),
                r,
                Panel {
                    top: rgba(0xff6a9a, 0.0),
                    bottom: rgba(0xff6a9a, 0.06 * held * (0.6 + 0.4 * beat)),
                    ..Panel::fill(0, 0, 0.0)
                },
            );
        }
    }
    if flash > 0.0 {
        panel(v, pic, r, Panel::fill(0xffeef4, 0xffd6e6, 0.09 * flash));
        vignette(v, pic, r, pic.w * 0.3, rgba(0xff6ea0, 0.4 * flash));
    }
}

/// Narration and sounds: a subtitle over the picture's foot, on a soft dark scrim.
fn draw_caption(font: &FontAtlas, v: &mut Vec<MapVertex>, pic: Rect, caption: &str, s: f32) {
    let size = 27.0 * s;
    let lines = wrap(font, Face::Italic, caption, size, pic.w - 110.0 * s);
    let line_h = size * 1.42;
    let band = lines.len() as f32 * line_h + 40.0 * s;
    panel(
        v,
        Rect::new(
            pic.x,
            pic.y + pic.h - band - 50.0 * s,
            pic.w,
            band + 50.0 * s,
        ),
        16.0 * s,
        Panel {
            top: rgba(SHADOW, 0.0),
            bottom: rgba(SHADOW, 0.88),
            ..Panel::fill(SHADOW, SHADOW, 0.0)
        },
    );
    let cx = pic.x + pic.w * 0.5;
    for (i, line) in lines.iter().enumerate() {
        let y = pic.y + pic.h - band + 12.0 * s + i as f32 * line_h;
        text_centered(font, v, Face::Italic, line, cx, y, size, rgba(CREAM, 1.0));
        if i == 0 {
            let w = text_width(font, Face::Italic, line, size);
            for side in [-1.0f32, 1.0] {
                sparkle(
                    v,
                    [cx + side * (w * 0.5 + 24.0 * s), y + size * 0.58],
                    7.0 * s,
                    rgba(GOLD_HI, 0.9),
                    0.5,
                );
            }
        }
    }
}

/// The lines said, as cards stacked left of the picture, the newest at the foot, lit.
fn draw_cards(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    lines: &[SceneLine],
    faces: &[Option<Uv>],
    time: f32,
    s: f32,
) {
    let right = frame.x - 36.0 * s;
    let width = (right - 54.0 * s).min(530.0 * s);
    if width < 260.0 * s {
        return;
    }
    let mut bottom = frame.y + frame.h;
    for (line, face_uv) in lines.iter().zip(faces.iter()).rev() {
        let a = line.alpha as f32 / 255.0;
        let newest = line.age == 0;
        let dim: f32 = [1.0, 0.68, 0.46][line.age.min(2) as usize];
        let accent = hex3(line.accent);
        let pad = 18.0 * s;
        let face_side = 106.0 * s;
        let text_x = pad + face_side + 24.0 * s;
        let size = 23.5 * s;
        let kind = if line.thought {
            Face::Italic
        } else {
            Face::Body
        };
        let body = if line.thought {
            format!("({})", line.text)
        } else {
            line.text.clone()
        };
        let wrapped = wrap(font, kind, &body, size, width - text_x - 26.0 * s);
        let h =
            (60.0 * s + wrapped.len() as f32 * size * 1.42 + 12.0 * s).max(face_side + pad * 2.0);
        let card = Rect::new(right - width, bottom - h, width, h);
        if card.y < frame.y - 6.0 * s {
            break;
        }
        soft(
            v,
            Rect::new(card.x, card.y + 12.0 * s, card.w, card.h),
            22.0 * s,
            18.0 * s,
            rgba(SHADOW, 0.7 * a),
            false,
        );
        if newest {
            let pulse = 0.8 + 0.2 * wave(time, 0.6);
            soft(
                v,
                card,
                22.0 * s,
                18.0 * s,
                rgba(accent, 0.30 * a * pulse),
                true,
            );
        }
        panel(
            v,
            card,
            22.0 * s,
            Panel {
                top: rgba(mix(VELVET_TOP, accent, 0.12), 0.96 * a),
                bottom: rgba(VELVET_BOTTOM, 0.97 * a),
                ..Panel::velvet(a, 2.2 * s)
            }
            .rim(
                if newest { GOLD_HI } else { GOLD },
                a * if newest { 1.0 } else { 0.7 },
                2.2 * s,
            )
            .hairline(6.0 * s, 0.35 * a),
        );
        // The speaker's face in a gold frame, a gem of their colour under it.
        let fr = Rect::new(card.x + pad, card.y + pad, face_side, face_side);
        panel(
            v,
            grow(fr, 3.0 * s),
            16.0 * s,
            Panel::fill(mix(0x1c0f28, accent, 0.34), mix(0x0c0612, accent, 0.12), a).rim(
                GOLD,
                a,
                2.4 * s,
            ),
        );
        match face_uv {
            Some(uv) => face(v, fr, 13.0 * s, *uv, a * dim.max(0.8)),
            None => {
                // No face yet: a medallion with the speaker's initial.
                let c = [fr.x + fr.w * 0.5, fr.y + fr.h * 0.5];
                halo(v, c, fr.w * 0.3, 14.0 * s, rgba(accent, 0.35 * a));
                gem(v, c, fr.w * 0.3, fr.h * 0.38, accent, a, 0.0);
                let initial: String = line.name.chars().take(1).collect();
                let w = text_width(font, Face::Title, &initial, 34.0 * s);
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &initial,
                    c[0] - w * 0.5,
                    c[1] - 21.0 * s,
                    34.0 * s,
                    rgba(INK, a),
                    rgba(0x12060e, 0.8 * a),
                );
            }
        }
        gem(
            v,
            [fr.x + fr.w * 0.5, fr.y + fr.h + 3.0 * s],
            6.0 * s,
            9.0 * s,
            accent,
            a,
            if newest { 5.0 * s } else { 0.0 },
        );
        // The name with a thread of their colour under it.
        let nx = card.x + text_x;
        let ny = card.y + 15.0 * s;
        let name_size = 23.0 * s;
        text_shadow(
            font,
            v,
            Face::Title,
            &line.name,
            nx,
            ny,
            name_size,
            rgba(mix(accent, 0xffffff, 0.42), a * dim.max(0.82)),
        );
        let nw = text_width(font, Face::Title, &line.name, name_size);
        wire(
            v,
            [nx, ny + name_size + 8.0 * s],
            [nx + nw + 40.0 * s, ny + name_size + 8.0 * s],
            1.8 * s,
            rgba(accent, 0.9 * a),
            rgba(accent, 0.0),
            0.0,
            false,
        );
        for (i, l) in wrapped.iter().enumerate() {
            let ink = if line.thought { LILAC } else { INK };
            text_shadow(
                font,
                v,
                kind,
                l,
                nx,
                card.y + 56.0 * s + i as f32 * size * 1.42,
                size,
                rgba(ink, a * dim),
            );
        }
        if newest {
            gem(
                v,
                [card.x + card.w, card.y + card.h * 0.5],
                8.0 * s,
                13.0 * s,
                accent,
                a,
                7.0 * s,
            );
        }
        bottom = card.y - 16.0 * s;
    }
}

/// The seal's vial, right of the picture.
#[allow(clippy::too_many_arguments)]
fn draw_vial(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    view: &SceneView,
    heat: f32,
    time: f32,
    s: f32,
) {
    let braked = view.braked;
    let mx = frame.x + frame.w + 36.0 * s + 60.0 * s;
    let column = Rect::new(mx - 60.0 * s, frame.y, 120.0 * s, frame.h);
    soft(
        v,
        Rect::new(column.x, column.y + 14.0 * s, column.w, column.h),
        32.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    panel(
        v,
        column,
        32.0 * s,
        Panel::velvet(0.95, 2.6 * s).hairline(6.0 * s, 0.4),
    );
    gem(
        v,
        [mx, column.y + column.h],
        7.0 * s,
        10.0 * s,
        CRIMSON,
        1.0,
        5.0 * s,
    );

    // The medallion of the seal, beating faster with the heat.
    let my = column.y + 84.0 * s;
    let mr = 46.0 * s;
    let beat = if braked {
        0.0
    } else {
        heartbeat(time, 54.0 + 80.0 * heat)
    };
    halo(
        v,
        [mx, my],
        mr,
        (22.0 + 16.0 * heat) * s,
        rgba(
            CRIMSON,
            (0.12 + 0.52 * heat) * (0.62 + 0.38 * beat) * if braked { 0.25 } else { 1.0 },
        ),
    );
    horns(v, [mx, my - mr * 0.78], mr * 1.7, GOLD, 1.0);
    panel(
        v,
        Rect::new(mx - mr, my - mr, mr * 2.0, mr * 2.0),
        mr,
        Panel::fill(
            if braked {
                0x2a2234
            } else {
                mix(0x3a0f24, 0xff4a6e, heat * 0.8)
            },
            if braked {
                0x120e18
            } else {
                mix(0x14040c, 0x8a1030, heat)
            },
            1.0,
        )
        .rim(GOLD, 1.0, 4.0 * s)
        .hairline(8.0 * s, 0.5)
        .bevel(1.0),
    );
    arc(
        v,
        [mx, my],
        mr - 16.0 * s,
        4.0 * s,
        0.0,
        heat,
        rgba(GOLD_HI, 0.95),
        rgba(GOLD_DEEP, 0.35),
    );
    let rune = if braked {
        0x8a80a0
    } else {
        mix(0xe0a8b8, 0xfff6ea, heat)
    };
    sparkle(
        v,
        [mx, my],
        20.0 * s * (0.92 + 0.12 * beat),
        rgba(rune, 1.0),
        0.2 + 0.7 * heat * beat,
    );
    if braked {
        padlock(v, [mx + mr * 0.66, my + mr * 0.62], 28.0 * s, LILAC, 1.0);
    }

    // The vial of liquid light.
    let vial_top = my + mr + 34.0 * s;
    let vial = Rect::new(
        mx - 26.0 * s,
        vial_top,
        52.0 * s,
        column.y + column.h - vial_top - 92.0 * s,
    );
    soft(v, vial, 26.0 * s, 10.0 * s, rgba(SHADOW, 0.6), false);
    panel(
        v,
        vial,
        26.0 * s,
        Panel::fill(0x1c0e2a, 0x0a0510, 0.94)
            .rim(GOLD, 1.0, 3.0 * s)
            .bevel(0.5),
    );
    let inner = grow(vial, -6.0 * s);
    let fill_h = inner.h * heat;
    if fill_h > 1.0 {
        let liquid = Rect::new(inner.x, inner.y + inner.h - fill_h, inner.w, fill_h);
        let (top_c, bottom_c) = if braked {
            (0xbca6da, 0x4a3a66)
        } else {
            (
                mix(0xff8fb0, 0xffe2a0, (heat - 0.72) * 3.5),
                mix(0x7a1038, 0xc0204a, heat),
            )
        };
        if !braked {
            soft(
                v,
                liquid,
                inner.w * 0.5,
                12.0 * s,
                rgba(CRIMSON, 0.32 * heat),
                true,
            );
        }
        panel(
            v,
            liquid,
            inner.w * 0.5,
            Panel::fill(top_c, bottom_c, 1.0)
                .bevel(0.4)
                .sheen(time * 0.22),
        );
        let tilt = wave(time, 0.35) * 2.0 * s;
        wire(
            v,
            [liquid.x + 7.0 * s, liquid.y + 4.0 * s - tilt],
            [liquid.x + liquid.w - 7.0 * s, liquid.y + 4.0 * s + tilt],
            3.0 * s,
            rgba(0xffffff, 0.6),
            rgba(0xffffff, 0.35),
            0.0,
            false,
        );
        if !braked {
            for i in 0..8 {
                let fi = i as f32;
                let speed = 0.14 + 0.05 * (fi * 1.7).sin().abs();
                let phase = (time * speed + fi * 0.137).rem_euclid(1.0);
                let y = liquid.y + liquid.h - phase * liquid.h;
                if y < liquid.y + 8.0 * s {
                    continue;
                }
                let x = liquid.x
                    + liquid.w * (0.28 + 0.44 * (fi * 0.618).fract())
                    + (time * 2.0 + fi).sin() * 3.0 * s;
                let r = (1.6 + (i % 3) as f32 * 0.9) * s;
                arc(
                    v,
                    [x, y],
                    r,
                    1.3 * s,
                    0.0,
                    1.0,
                    rgba(0xffffff, 0.55 * (1.0 - phase)),
                    [0.0; 4],
                );
            }
        }
    }
    wire(
        v,
        [vial.x + 14.0 * s, vial.y + 22.0 * s],
        [vial.x + 14.0 * s, vial.y + vial.h - 24.0 * s],
        4.0 * s,
        rgba(0xffffff, 0.18),
        rgba(0xffffff, 0.04),
        0.0,
        false,
    );
    for mark in &view.marks {
        let m = *mark as f32 / 1000.0;
        let y = inner.y + inner.h - inner.h * m;
        let lit = heat >= m;
        wire(
            v,
            [vial.x - 6.0 * s, y],
            [vial.x + 6.0 * s, y],
            2.0 * s,
            rgba(GOLD, 0.9),
            rgba(GOLD, 0.9),
            0.0,
            true,
        );
        gem(
            v,
            [vial.x - 14.0 * s, y],
            6.0 * s,
            9.0 * s,
            if lit { CRIMSON } else { 0x4a3a52 },
            1.0,
            if lit { 5.0 * s } else { 0.0 },
        );
    }

    // The plaque with the number.
    let plaque = Rect::new(
        mx - 48.0 * s,
        column.y + column.h - 74.0 * s,
        96.0 * s,
        48.0 * s,
    );
    panel(
        v,
        plaque,
        24.0 * s,
        Panel::fill(0x2c1640, 0x10081a, 1.0)
            .rim(GOLD, 1.0, 2.2 * s)
            .hairline(5.0 * s, 0.4)
            .bevel(1.0),
    );
    let (label, kind, ink, size) = if braked {
        ("¡Alto!".to_owned(), Face::Italic, LILAC, 22.0 * s)
    } else {
        (
            format!("{}%", (heat * 100.0).round() as i32),
            Face::Ui,
            INK,
            25.0 * s,
        )
    };
    text_centered(
        font,
        v,
        kind,
        &label,
        mx,
        plaque.y + (plaque.h - size) * 0.5 - 3.0 * s,
        size,
        rgba(ink, 1.0),
    );
}

/// Under the picture: what is happening, the key to hold (sparks rising from it while held), the
/// fuse, and Esc.
#[allow(clippy::too_many_arguments)]
fn draw_keys(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    view: &SceneView,
    held: f32,
    time: f32,
    s: f32,
) {
    let cx = pic.x + pic.w * 0.5;
    let plate = Rect::new(pic.x, frame.y + frame.h + 22.0 * s, pic.w, 152.0 * s);
    let fuse_p = view.hold.map_or(0.0, |h| h as f32 / 1000.0);
    let danger = ((fuse_p - 0.62) / 0.38).clamp(0.0, 1.0);
    soft(
        v,
        Rect::new(plate.x, plate.y + 12.0 * s, plate.w, plate.h),
        26.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if danger > 0.0 {
        soft(
            v,
            plate,
            26.0 * s,
            18.0 * s,
            rgba(CRIMSON, 0.42 * danger * (0.6 + 0.4 * wave(time, 2.6).abs())),
            true,
        );
    }
    panel(
        v,
        plate,
        26.0 * s,
        Panel::velvet(0.96, 2.6 * s).hairline(6.0 * s, 0.4),
    );
    for side in [plate.x, plate.x + plate.w] {
        gem(
            v,
            [side, plate.y + plate.h * 0.5],
            7.0 * s,
            11.0 * s,
            CRIMSON,
            1.0,
            5.0 * s,
        );
    }
    text_centered(
        font,
        v,
        Face::Italic,
        &view.phase,
        cx,
        plate.y + 14.0 * s,
        23.0 * s,
        rgba(0xeadcf2, 1.0),
    );
    let esc_w = 64.0 * s;
    if view.keys {
        let key = Rect::new(cx - 150.0 * s, plate.y + 54.0 * s, 300.0 * s, 62.0 * s);
        let label = if view.braked { "SUELTA" } else { "ESPACIO" };
        if held > 0.0 {
            soft(
                v,
                key,
                16.0 * s,
                26.0 * s,
                rgba(0xff6a9a, held * 0.35),
                true,
            );
            for i in 0..7 {
                let fi = i as f32;
                let phase = (time * (0.9 + 0.13 * fi) + fi * 0.29).fract();
                let x = key.x
                    + key.w * (0.12 + 0.76 * (fi * 0.618).fract())
                    + (time * 3.0 + fi).sin() * 4.0 * s;
                let y = key.y + 6.0 * s - phase * 30.0 * s;
                sparkle(
                    v,
                    [x, y],
                    (3.0 + 4.0 * (1.0 - phase)) * s,
                    rgba(0xffd0e4, held * (1.0 - phase)),
                    0.7,
                );
            }
        }
        keycap(
            font,
            v,
            key,
            label,
            25.0 * s,
            view.holding && !view.braked,
            view.braked,
            CRIMSON,
            1.0,
        );
        let hint = if view.braked {
            "y vuelve a pulsar"
        } else {
            "mantén"
        };
        let hw = text_width(font, Face::Italic, hint, 20.0 * s);
        text_shadow(
            font,
            v,
            Face::Italic,
            hint,
            key.x - 22.0 * s - hw,
            key.y + 19.0 * s,
            20.0 * s,
            rgba(DIM, 1.0),
        );
        let esc = Rect::new(key.x + key.w + 24.0 * s, key.y + 11.0 * s, esc_w, 40.0 * s);
        keycap(font, v, esc, "Esc", 18.0 * s, false, false, CRIMSON, 0.9);
        text_shadow(
            font,
            v,
            Face::Italic,
            &view.esc,
            esc.x + esc.w + 10.0 * s,
            esc.y + 8.0 * s,
            20.0 * s,
            rgba(DIM, 1.0),
        );
        if let Some(h) = view.hold {
            let p = h as f32 / 1000.0;
            let fuse = Rect::new(
                key.x + 12.0 * s,
                key.y + key.h + 16.0 * s,
                key.w - 24.0 * s,
                8.0 * s,
            );
            panel(
                v,
                fuse,
                4.0 * s,
                Panel::fill(0x1a0d22, 0x120818, 1.0).rim(GOLD_DEEP, 0.85, 1.2 * s),
            );
            if p > 0.002 && !view.braked {
                let lit = Rect::new(fuse.x, fuse.y, (fuse.w * p).max(8.0 * s), fuse.h);
                panel(
                    v,
                    lit,
                    4.0 * s,
                    Panel::fill(mix(GOLD_HI, CRIMSON, p), mix(GOLD, 0x8a1030, p), 1.0).bevel(0.4),
                );
                let tip = [lit.x + lit.w, fuse.y + fuse.h * 0.5];
                let flicker = 0.78 + 0.22 * wave(time, 7.0).abs();
                halo(
                    v,
                    tip,
                    4.0 * s,
                    12.0 * s,
                    rgba(mix(GOLD_HI, CRIMSON, p), 0.6),
                );
                sparkle(v, tip, 12.0 * s * flicker, rgba(0xfff2c8, 1.0), 0.8);
            }
        }
    } else {
        let esc = Rect::new(cx - esc_w - 8.0 * s, plate.y + 66.0 * s, esc_w, 40.0 * s);
        keycap(font, v, esc, "Esc", 18.0 * s, false, false, CRIMSON, 0.9);
        text_shadow(
            font,
            v,
            Face::Italic,
            &view.esc,
            esc.x + esc.w + 12.0 * s,
            esc.y + 8.0 * s,
            21.0 * s,
            rgba(DIM, 1.0),
        );
    }
}

/// The closing card, beside the picture and never on it: a tall velvet panel where the lines were,
/// sliding in from the left. The ending's title on a ribbon, whose scene it was and how long it
/// took, a row for each value that changed (they come in one by one and count up), and the last
/// word with its speaker's face. Under the picture, on its plate, the key to go on.
#[allow(clippy::too_many_arguments)]
fn draw_result(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    res: &SceneResult,
    quote_face: Option<Uv>,
    time: f32,
    s: f32,
) {
    let age = res.age_ms as f32 / 1000.0;
    let a = ease_out(age / 0.4);
    let (stone, bright, deep) = if res.complete {
        (CRIMSON, 0xff5a8e, 0x5a0c22)
    } else {
        (0x8a7aa8, 0xc8b8f0, 0x2a2244)
    };
    let right = frame.x - 36.0 * s;
    let w = (right - 54.0 * s).min(520.0 * s);
    let plate = Rect::new(pic.x, frame.y + frame.h + 22.0 * s, pic.w, 152.0 * s);
    let p = Rect::new(right - w, frame.y, w, plate.y + plate.h - frame.y);
    let n = res.effects.len();
    faded(v, a, |v| {
        shifted(v, [-(1.0 - a) * 150.0 * s, 0.0], |v| {
            soft(
                v,
                Rect::new(p.x, p.y + 20.0 * s, p.w, p.h),
                28.0 * s,
                32.0 * s,
                rgba(SHADOW, 0.85),
                false,
            );
            soft(
                v,
                p,
                28.0 * s,
                26.0 * s,
                rgba(stone, 0.26 + 0.06 * wave(time, 0.5)),
                true,
            );
            panel(
                v,
                p,
                26.0 * s,
                Panel::velvet(0.98, 3.2 * s).hairline(10.0 * s, 0.5),
            );
            for (at, sx, sy, horn) in [
                ([p.x, p.y], 1.0, 1.0, true),
                ([p.x + p.w, p.y], -1.0, 1.0, true),
                ([p.x, p.y + p.h], 1.0, -1.0, false),
                ([p.x + p.w, p.y + p.h], -1.0, -1.0, false),
            ] {
                corner(v, at, sx, sy, s, stone, horn, 1.0);
            }
            gem(
                v,
                [p.x + p.w, p.y + p.h * 0.5],
                8.0 * s,
                13.0 * s,
                stone,
                1.0,
                6.0 * s,
            );

            // The title on a ribbon thrown across the panel's head.
            let room = p.w - 70.0 * s;
            let mut size = 54.0 * s;
            while size > 38.0 * s && text_width(font, Face::Title, &res.title, size) > room {
                size -= 1.0 * s;
            }
            let lines = if text_width(font, Face::Title, &res.title, size) > room {
                wrap(font, Face::Title, &res.title, size, room)
            } else {
                vec![res.title.clone()]
            };
            let band_h = 44.0 * s + lines.len() as f32 * size * 1.12;
            let band = Rect::new(p.x - 18.0 * s, p.y + 40.0 * s, p.w + 36.0 * s, band_h);
            let stamp = ease_out((age - 0.15) / 0.3);
            faded(v, stamp, |v| {
                turned(
                    v,
                    [band.x + band.w * 0.5, band.y + band.h * 0.5],
                    -0.04,
                    |v| {
                        soft(
                            v,
                            Rect::new(band.x + 10.0 * s, band.y + 12.0 * s, band.w, band.h),
                            6.0 * s,
                            12.0 * s,
                            rgba(SHADOW, 0.7),
                            false,
                        );
                        panel(
                            v,
                            grow(band, 5.0 * s),
                            6.0 * s,
                            Panel::fill(0x0b0508, 0x0b0508, 1.0),
                        );
                        panel(
                            v,
                            band,
                            4.0 * s,
                            Panel::fill(mix(bright, 0xffffff, 0.08), mix(stone, deep, 0.4), 1.0)
                                .sheen(time * 0.25),
                        );
                        panel(
                            v,
                            Rect::new(band.x, band.y, band.w, 6.0 * s),
                            0.0,
                            Panel::fill(0xffffff, 0xffffff, 0.85),
                        );
                        for (i, line) in lines.iter().enumerate() {
                            let lw = text_width(font, Face::Title, line, size);
                            shout(
                                font,
                                v,
                                Face::Title,
                                line,
                                band.x + (band.w - lw) * 0.5,
                                band.y + 20.0 * s + i as f32 * size * 1.12,
                                size,
                                0xffffff,
                                deep,
                                1.0,
                            );
                        }
                    },
                );
            });
            let mut y = band.y + band.h + 30.0 * s;
            // Each part in the player's language before they are joined (the joined line has no
            // entry of its own).
            let who = format!(
                "{} · {}",
                demon_queen_dungeon::i18n::tr(&res.who),
                demon_queen_dungeon::i18n::tr(&format!("{} s", res.seconds))
            );
            let ww = text_width(font, Face::Italic, &who, 25.0 * s);
            text_shadow(
                font,
                v,
                Face::Italic,
                &who,
                p.x + (p.w - ww) * 0.5,
                y,
                25.0 * s,
                rgba(CREAM, 1.0),
            );
            y += 50.0 * s;
            rule(v, [p.x + p.w * 0.5, y], p.w * 0.36, stone, s, true, 1.0);
            y += 34.0 * s;

            // What changed, a row each, counting up as it lands.
            for (k, (label, delta)) in res.effects.iter().enumerate() {
                let start = 0.5 + k as f32 * 0.2;
                let appear = ease_out((age - start) / 0.3);
                let row = Rect::new(p.x + 26.0 * s, y, p.w - 52.0 * s, 66.0 * s);
                y += 80.0 * s;
                if appear <= 0.0 {
                    continue;
                }
                let up = *delta > 0;
                let (mark, mark_deep, fill) = if up {
                    (0xff9ab8, 0x8a1038, 0x3a0f2a)
                } else {
                    (0xa8c4ff, 0x1e2a6a, 0x141a3a)
                };
                faded(v, appear, |v| {
                    shifted(v, [-(1.0 - appear) * 40.0 * s, 0.0], |v| {
                        soft(
                            v,
                            Rect::new(row.x, row.y + 8.0 * s, row.w, row.h),
                            14.0 * s,
                            10.0 * s,
                            rgba(SHADOW, 0.6),
                            false,
                        );
                        panel(
                            v,
                            row,
                            14.0 * s,
                            Panel::fill(fill, 0x0b0508, 0.95)
                                .rim(mark, 0.85, 1.6 * s)
                                .bevel(0.8),
                        );
                        panel(
                            v,
                            Rect::new(row.x, row.y, 8.0 * s, row.h),
                            4.0 * s,
                            Panel::fill(mark, mark_deep, 1.0),
                        );
                        let badge = [row.x + 42.0 * s, row.y + row.h * 0.5];
                        halo(v, badge, 14.0 * s, 10.0 * s, rgba(mark, 0.35));
                        disc(v, badge, 17.0 * s, rgba(mix(fill, 0x000000, 0.3), 1.0));
                        arrow(v, badge, 19.0 * s, up, rgba(mark, 1.0));
                        text_shadow(
                            font,
                            v,
                            Face::Body,
                            label,
                            row.x + 74.0 * s,
                            row.y + (row.h - 26.0 * s) * 0.5 - 3.0 * s,
                            26.0 * s,
                            rgba(INK, 1.0),
                        );
                        let count =
                            (delta.abs() as f32 * ease_out((age - start) / 0.6)).round() as i32;
                        let value = format!("{}{}", if up { "+" } else { "−" }, count);
                        let vw = text_width(font, Face::Title, &value, 42.0 * s);
                        shout(
                            font,
                            v,
                            Face::Title,
                            &value,
                            row.x + row.w - 24.0 * s - vw,
                            row.y + (row.h - 42.0 * s) * 0.5 - 4.0 * s,
                            42.0 * s,
                            mix(mark, 0xffffff, 0.35),
                            mark_deep,
                            1.0,
                        );
                    });
                });
            }

            // The last word, at the panel's foot.
            let qa = ease_out((age - (0.6 + n as f32 * 0.2)) / 0.4);
            let accent = hex3(res.quote.accent);
            let q = Rect::new(
                p.x + 22.0 * s,
                p.y + p.h - 236.0 * s,
                p.w - 44.0 * s,
                210.0 * s,
            );
            // Between them, the seal it was all for: its ring of runes turning slowly, glowing
            // when it was fed.
            let room_left = q.y - y;
            if room_left > 120.0 * s {
                let c = [p.x + p.w * 0.5, y + room_left * 0.5 - 14.0 * s];
                let r = (room_left * 0.36).min(92.0 * s);
                let beat = if res.complete {
                    heartbeat(time, 60.0)
                } else {
                    0.0
                };
                halo(
                    v,
                    c,
                    r * 0.7,
                    30.0 * s,
                    rgba(stone, (0.12 + 0.12 * beat) * qa.max(a * 0.6)),
                );
                arc(
                    v,
                    c,
                    r,
                    3.0 * s,
                    time * 0.04,
                    0.9,
                    rgba(mix(stone, GOLD_HI, 0.5), 0.55),
                    rgba(stone, 0.15),
                );
                arc(
                    v,
                    c,
                    r * 0.72,
                    2.2 * s,
                    -time * 0.07,
                    0.75,
                    rgba(GOLD, 0.45),
                    [0.0; 4],
                );
                horns(v, [c[0], c[1] - r * 0.62], r * 1.25, GOLD, 0.7);
                for k in 0..6 {
                    let t = time * 0.04 * std::f32::consts::TAU
                        + k as f32 * std::f32::consts::TAU / 6.0;
                    sparkle(
                        v,
                        [c[0] + t.cos() * r, c[1] + t.sin() * r],
                        5.0 * s,
                        rgba(GOLD_HI, 0.6),
                        0.4,
                    );
                }
                sparkle(
                    v,
                    c,
                    (18.0 + 6.0 * beat) * s,
                    rgba(if res.complete { 0xffe0ea } else { 0xd8d0f0 }, 0.9),
                    0.6 + 0.3 * beat,
                );
            }
            faded(v, qa, |v| {
                panel(
                    v,
                    q,
                    18.0 * s,
                    Panel::fill(mix(0x1c0f28, accent, 0.18), 0x0c0612, 0.96)
                        .rim(GOLD, 0.9, 2.0 * s)
                        .hairline(6.0 * s, 0.3),
                );
                let fr = Rect::new(q.x + 20.0 * s, q.y + 24.0 * s, 92.0 * s, 92.0 * s);
                soft(v, fr, 46.0 * s, 12.0 * s, rgba(accent, 0.35), true);
                panel(
                    v,
                    grow(fr, 3.0 * s),
                    49.0 * s,
                    Panel::fill(mix(0x1c0f28, accent, 0.3), 0x0c0612, 1.0).rim(GOLD, 1.0, 2.2 * s),
                );
                match quote_face {
                    Some(uv) => face(v, fr, 46.0 * s, uv, 1.0),
                    None => gem(
                        v,
                        [fr.x + fr.w * 0.5, fr.y + fr.h * 0.5],
                        20.0 * s,
                        26.0 * s,
                        accent,
                        1.0,
                        0.0,
                    ),
                }
                let tx = fr.x + fr.w + 20.0 * s;
                text_shadow(
                    font,
                    v,
                    Face::Title,
                    &res.quote.name,
                    tx,
                    q.y + 24.0 * s,
                    25.0 * s,
                    rgba(mix(accent, 0xffffff, 0.42), 1.0),
                );
                let nw = text_width(font, Face::Title, &res.quote.name, 25.0 * s);
                wire(
                    v,
                    [tx, q.y + 58.0 * s],
                    [tx + nw + 40.0 * s, q.y + 58.0 * s],
                    1.8 * s,
                    rgba(accent, 0.9),
                    rgba(accent, 0.0),
                    0.0,
                    false,
                );
                let size = 23.0 * s;
                let lines = wrap(
                    font,
                    Face::Italic,
                    &res.quote.text,
                    size,
                    q.x + q.w - 20.0 * s - tx,
                );
                for (i, line) in lines.iter().take(4).enumerate() {
                    text_shadow(
                        font,
                        v,
                        Face::Italic,
                        line,
                        tx,
                        q.y + 72.0 * s + i as f32 * size * 1.38,
                        size,
                        rgba(CREAM, 1.0),
                    );
                }
            });
        });
    });

    // Under the picture: its plate, and the key to go on.
    soft(
        v,
        Rect::new(plate.x, plate.y + 12.0 * s, plate.w, plate.h),
        26.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    panel(
        v,
        plate,
        26.0 * s,
        Panel::velvet(0.96, 2.6 * s).hairline(6.0 * s, 0.4),
    );
    for side in [plate.x, plate.x + plate.w] {
        gem(
            v,
            [side, plate.y + plate.h * 0.5],
            7.0 * s,
            11.0 * s,
            stone,
            1.0,
            5.0 * s,
        );
    }
    let ready = age > 1.0;
    let cx = plate.x + plate.w * 0.5;
    text_centered(
        font,
        v,
        Face::Italic,
        if res.complete { "Calma" } else { "A medias" },
        cx,
        plate.y + 16.0 * s,
        23.0 * s,
        rgba(0xeadcf2, 1.0),
    );
    let key = Rect::new(cx - 120.0 * s, plate.y + 62.0 * s, 56.0 * s, 52.0 * s);
    let pulse = if ready {
        0.85 + 0.15 * wave(time, 0.9)
    } else {
        0.45
    };
    if ready {
        soft(v, key, 14.0 * s, 20.0 * s, rgba(bright, 0.3 * pulse), true);
    }
    keycap(font, v, key, "F", 24.0 * s, false, !ready, stone, 1.0);
    text_shadow(
        font,
        v,
        Face::Title,
        "Continuar",
        key.x + key.w + 22.0 * s,
        key.y + 10.0 * s,
        30.0 * s,
        rgba(CREAM, pulse),
    );
}
