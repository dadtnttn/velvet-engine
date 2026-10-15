//! Small vector HUD and glyph atlas. No per-frame rasterization or full-screen
//! texture upload; the diagnostic map is drawn from live logical/surface data.
use super::{scene::RenderStats, Dungeon};
use ab_glyph::{point, Font, FontArc, PxScale, ScaleFont};
use wgpu::util::DeviceExt;
#[path = "hud_skin.rs"]
mod skin;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    p: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}
struct Glyph {
    uv: [f32; 4],
    size: [f32; 2],
    offset: [f32; 2],
    advance: f32,
}
/// The HUD's typefaces, as the rest of the game's menus use them: Nunito ExtraBold for labels,
/// numbers and keys, a book serif for the names of places, Nunito italic for asides.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Sans = 0,
    Serif = 1,
    Italic = 2,
}
pub struct Hud {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    capacity: usize,
    glyphs: Vec<std::collections::BTreeMap<char, Glyph>>,
    /// The face text is set in now (see [`Hud::in_face`]).
    face: std::cell::Cell<Face>,
    /// The language's own font under the faces (another script, shaped ones).
    script: Option<ScriptGlyphs>,
    /// The language these glyphs were made for, and the target format (to make them again).
    generation: u64,
    format: wgpu::TextureFormat,
}
/// The language's font: by character what the faces lack, by glyph id what is shaped.
struct ScriptGlyphs {
    chars: std::collections::BTreeMap<char, Glyph>,
    ids: std::collections::HashMap<u16, Glyph>,
    bytes: std::sync::Arc<Vec<u8>>,
    index: u32,
    /// Font units to raster units.
    k: f32,
    shift: f32,
    shaped: bool,
}
/// Side of the faces' block of the atlas: one 1024-pixel square per face (a language with its own
/// script doubles the atlas and fills the rest with its glyphs).
const ATLAS: u32 = 2048;
const SHADER: &str = r#"
struct In { @location(0) p:vec2<f32>, @location(1) uv:vec2<f32>, @location(2) color:vec4<f32> };
struct Out { @builtin(position) p:vec4<f32>, @location(0) uv:vec2<f32>, @location(1) color:vec4<f32> };
@group(0) @binding(0) var atlas:texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler:sampler;
@vertex fn vs(v:In)->Out{var o:Out;o.p=vec4<f32>(v.p,0.0,1.0);o.uv=v.uv;o.color=v.color;return o;}
@fragment fn fs(v:Out)->@location(0) vec4<f32>{return vec4<f32>(v.color.rgb,v.color.a*textureSample(atlas,atlas_sampler,v.uv).r);}
"#;
impl Hud {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let windows = std::env::var("WINDIR").unwrap_or_else(|_| "C:/Windows".into());
        let fonts = super::assets::root().join("../../fonts");
        let read = |paths: &[std::path::PathBuf]| {
            paths.iter().find_map(|p| {
                std::fs::read(p)
                    .ok()
                    .and_then(|bytes| FontArc::try_from_vec(bytes).ok())
            })
        };
        let serif = read(&[
            format!("{windows}/Fonts/georgia.ttf").into(),
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf".into(),
            format!("{windows}/Fonts/segoeui.ttf").into(),
        ])
        .ok_or_else(|| anyhow::anyhow!("no HUD font available"))?;
        let sans = read(&[
            fonts.join("Nunito-ExtraBold.ttf"),
            format!("{windows}/Fonts/segoeuib.ttf").into(),
        ])
        .unwrap_or_else(|| serif.clone());
        let italic = read(&[
            fonts.join("Nunito-BoldItalic.ttf"),
            format!("{windows}/Fonts/segoeuiz.ttf").into(),
        ])
        .unwrap_or_else(|| sans.clone());
        let language = crate::i18n::language();
        let generation = crate::i18n::generation();
        let texts = crate::i18n::catalog_texts();
        let extra: Vec<char> = {
            let mut set = std::collections::BTreeSet::new();
            for t in &texts {
                for c in t.chars() {
                    if !c.is_control() && c as u32 > 255 {
                        set.insert(c);
                    }
                }
            }
            set.into_iter().collect()
        };
        let script_source = if extra.is_empty() { None } else { crate::i18n::script_font(&language) };
        let atlas = if script_source.is_some() { ATLAS * 2 } else { ATLAS };
        let side = atlas as usize;
        let mut data = vec![0u8; side * side];
        // The solid corner the shapes sample (at 0.0015 of the atlas, whatever its size).
        for y in 0..12 {
            for x in 0..12 {
                data[y * side + x] = 255;
            }
        }
        let mut glyphs = Vec::new();
        for (face, font) in [sans, serif, italic].into_iter().enumerate() {
            let (ox, oy) = ((face as u32 % 2) * 1024, (face as u32 / 2) * 1024);
            let scaled = font.as_scaled(PxScale::from(38.0));
            // Every face set so the middle of its capitals sits where the layout expects it
            // (0.58 of the size below the top), whatever its own ascent.
            let cap = font
                .outline_glyph(font.glyph_id('H').with_scale(38.0))
                .map(|o| o.px_bounds().height())
                .unwrap_or(26.0);
            let shift = 38.0 * 0.58 - (scaled.ascent() - cap * 0.5);
            let mut map = std::collections::BTreeMap::new();
            for code in 32..=255u32 {
                let ch = char::from_u32(code).unwrap();
                let index = code - 31;
                let x = ox + (index % 16) * 64;
                let y = oy + (index / 16) * 64;
                let id = font.glyph_id(ch);
                let g = id.with_scale_and_position(38.0, point(0.0, scaled.ascent()));
                let mut entry = Glyph {
                    uv: [0.0; 4],
                    size: [0.0; 2],
                    offset: [0.0; 2],
                    advance: scaled.h_advance(id),
                };
                if let Some(outline) = font.outline_glyph(g) {
                    let bounds = outline.px_bounds();
                    let w = (bounds.width().ceil() as u32).min(60);
                    let h = (bounds.height().ceil() as u32).min(60);
                    outline.draw(|gx, gy, coverage| {
                        if gx < 60 && gy < 60 {
                            data[(y + gy) as usize * side + (x + gx) as usize] =
                                (coverage * 255.0) as u8;
                        }
                    });
                    let k = atlas as f32;
                    entry = Glyph {
                        uv: [x as f32 / k, y as f32 / k, (x + w) as f32 / k, (y + h) as f32 / k],
                        size: [w as f32, h as f32],
                        offset: [bounds.min.x, bounds.min.y + shift],
                        advance: scaled.h_advance(id),
                    };
                }
                map.insert(ch, entry);
            }
            glyphs.push(map);
        }
        // The language's glyphs, in 64-pixel cells over the rest of the atlas.
        let mut script = None;
        if let Some((bytes, index, shaped)) = script_source {
            if let Ok(font) = ab_glyph::FontVec::try_from_vec_and_index(bytes.as_ref().clone(), index) {
                let scaled = font.as_scaled(PxScale::from(38.0));
                let k = 38.0 / font.height_unscaled().max(1.0);
                let cap = font
                    .outline_glyph(font.glyph_id('H').with_scale(38.0))
                    .map(|o| o.px_bounds().height())
                    .unwrap_or(scaled.ascent() * 0.72);
                let shift = 38.0 * 0.58 - (scaled.ascent() - cap * 0.5);
                let cells: Vec<(u32, u32)> = (0..atlas / 64)
                    .flat_map(|cy| (0..atlas / 64).map(move |cx| (cx * 64, cy * 64)))
                    .filter(|&(x, y)| x >= ATLAS || y >= ATLAS)
                    .collect();
                let mut jobs: Vec<(Option<char>, ab_glyph::GlyphId)> = extra
                    .iter()
                    .filter(|c| !(shaped && crate::i18n::needs_shaping(**c)))
                    .map(|&c| (Some(c), font.glyph_id(c)))
                    .filter(|(_, id)| id.0 != 0)
                    .collect();
                if shaped {
                    let mut ids = std::collections::BTreeSet::new();
                    let base: String = (32u8..=126).map(char::from).collect();
                    for t in texts.iter().map(String::as_str).chain([base.as_str()]) {
                        for line in t.split('\n') {
                            for (id, ..) in crate::i18n::shape_line(&bytes, index, k, line) {
                                ids.insert(id);
                            }
                        }
                    }
                    jobs.extend(ids.into_iter().map(|id| (None, ab_glyph::GlyphId(id))));
                }
                let mut chars = std::collections::BTreeMap::new();
                let mut ids = std::collections::HashMap::new();
                for ((character, id), (x, y)) in jobs.into_iter().zip(cells) {
                    let g = id.with_scale_and_position(38.0, point(0.0, scaled.ascent()));
                    let mut entry = Glyph { uv: [0.0; 4], size: [0.0; 2], offset: [0.0; 2], advance: scaled.h_advance(id) };
                    if let Some(outline) = font.outline_glyph(g) {
                        let bounds = outline.px_bounds();
                        let w = (bounds.width().ceil() as u32).min(60);
                        let h = (bounds.height().ceil() as u32).min(60);
                        outline.draw(|gx, gy, coverage| {
                            if gx < 60 && gy < 60 {
                                data[(y + gy) as usize * side + (x + gx) as usize] = (coverage * 255.0) as u8;
                            }
                        });
                        let a = atlas as f32;
                        entry = Glyph {
                            uv: [x as f32 / a, y as f32 / a, (x + w) as f32 / a, (y + h) as f32 / a],
                            size: [w as f32, h as f32],
                            offset: [bounds.min.x, bounds.min.y + shift],
                            advance: scaled.h_advance(id),
                        };
                    }
                    match character {
                        Some(c) => {
                            chars.insert(c, entry);
                        }
                        None => {
                            ids.insert(id.0, entry);
                        }
                    }
                }
                script = Some(ScriptGlyphs { chars, ids, bytes, index, k, shift, shaped });
            }
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dungeon-hud-font"),
            size: wgpu::Extent3d {
                width: atlas,
                height: atlas,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
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
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas),
                rows_per_image: Some(atlas),
            },
            texture.size(),
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dungeon-hud"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dungeon-hud"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let capacity = 4096;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dungeon-hud-vertices"),
            size: (capacity * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            pipeline,
            bind,
            buffer,
            capacity,
            glyphs,
            face: std::cell::Cell::new(Face::Sans),
            script,
            generation,
            format,
        })
    }
    /// One line laid out in the face text is set in (the language's font for what it lacks; a
    /// shaped script through the shaper): glyphs with their x and y offset, and the line's advance.
    fn layout(&self, line: &str) -> (Vec<(&Glyph, f32, f32)>, f32) {
        let glyphs = self.face_glyphs();
        let mut placed = Vec::new();
        let mut pen = 0.0;
        match &self.script {
            Some(sc) if sc.shaped && line.chars().any(crate::i18n::needs_shaping) => {
                for (id, advance, dx, dy) in crate::i18n::shape_line(&sc.bytes, sc.index, sc.k, line) {
                    if let Some(g) = sc.ids.get(&id) {
                        placed.push((g, pen + dx, dy));
                    }
                    pen += advance;
                }
                let _ = sc.shift;
            }
            _ => {
                for c in line.chars() {
                    let g = glyphs
                        .get(&c)
                        .filter(|g| g.size[0] > 0.0 || c == ' ' || g.advance > 0.0 && (c as u32) < 256)
                        .or_else(|| self.script.as_ref().and_then(|s| s.chars.get(&c)))
                        .or_else(|| glyphs.get(&'?'));
                    if let Some(g) = g {
                        placed.push((g, pen, 0.0));
                        pen += g.advance;
                    }
                }
            }
        }
        (placed, pen)
    }
    /// The widest line of `text` at `size` (as drawn: translated).
    fn measure(&self, text: &str, size: f32) -> f32 {
        let text = &*crate::i18n::tr(text);
        text.split('\n')
            .map(|line| self.layout(line).1 * size / 38.0)
            .fold(0.0, f32::max)
    }
    /// The glyphs of the face text is set in now.
    fn face_glyphs(&self) -> &std::collections::BTreeMap<char, Glyph> {
        &self.glyphs[self.face.get() as usize]
    }
    /// Draws `paint` with text set in `face`.
    fn in_face<R>(&self, face: Face, paint: impl FnOnce() -> R) -> R {
        let before = self.face.replace(face);
        let out = paint();
        self.face.set(before);
        out
    }
    fn text(&self, v: &mut Vec<Vertex>, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) {
        let text = &*crate::i18n::tr(text);
        let mut py = y;
        let scale = size / 38.0;
        for line in text.split('\n') {
            let (placed, _) = self.layout(line);
            for (g, pen, dy) in placed {
                if g.size[0] > 0.0 {
                    rect_uv(
                        v,
                        [
                            x + (pen + g.offset[0]) * scale,
                            py + (dy + g.offset[1]) * scale,
                            g.size[0] * scale,
                            g.size[1] * scale,
                        ],
                        g.uv,
                        color,
                    );
                }
            }
            py += size * 1.5;
        }
    }
    pub fn render(
        &mut self,
        run: &Dungeon,
        stats: RenderStats,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        // QA: `VELVET_NO_HUD=1` draws the world alone (pictures of the places for the guide).
        if std::env::var("VELVET_NO_HUD").is_ok_and(|v| v == "1") {
            return;
        }
        // Another language: its glyphs made again.
        if self.generation != crate::i18n::generation() {
            if let Ok(fresh) = Hud::new(device, queue, self.format) {
                *self = fresh;
            }
        }
        let mut v = Vec::with_capacity(4096);
        // Modal screens own the whole frame; the play chrome stays hidden.
        let modal = run.debug || run.map_open || run.seed_input.is_some();
        if !modal {
            self.chrome(run, &mut v);
        }
        // The map keeps its chart inside a viewport: those vertices are drawn
        // under a scissor so zooming never spills over the panels.
        let mut clip = None;
        if run.debug || run.map_open {
            clip = self.map_overlay(run, stats, &mut v);
        }
        if let Some(seed) = &run.seed_input {
            self.seed_dialog(run, seed, &mut v);
        }
        for p in &mut v {
            p.p = [p.p[0] / 960.0 - 1.0, 1.0 - p.p[1] / 540.0];
        }
        if v.len() > self.capacity {
            self.capacity = v.len().next_power_of_two();
            self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("dungeon-hud-grow"),
                contents: &vec![0u8; self.capacity * std::mem::size_of::<Vertex>()],
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&v));
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dungeon-hud"),
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
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_vertex_buffer(0, self.buffer.slice(..));
            match clip {
                Some((start, end, r)) if start < end && end <= v.len() => {
                    let (kx, ky) = (size.0 as f32 / 1920.0, size.1 as f32 / 1080.0);
                    let x = ((r[0] * kx).floor().max(0.0) as u32).min(size.0.saturating_sub(1));
                    let y = ((r[1] * ky).floor().max(0.0) as u32).min(size.1.saturating_sub(1));
                    let w = ((r[2] * kx).ceil() as u32).min(size.0 - x).max(1);
                    let h = ((r[3] * ky).ceil() as u32).min(size.1 - y).max(1);
                    pass.draw(0..start as u32, 0..1);
                    pass.set_scissor_rect(x, y, w, h);
                    pass.draw(start as u32..end as u32, 0..1);
                    pass.set_scissor_rect(0, 0, size.0, size.1);
                    pass.draw(end as u32..v.len() as u32, 0..1);
                }
                _ => pass.draw(0..v.len() as u32, 0..1),
            }
        }
        queue.submit(Some(encoder.finish()));
    }
}
fn rect(v: &mut Vec<Vertex>, r: [f32; 4], c: [f32; 4]) {
    rect_uv(v, r, [0.0015, 0.0015, 0.0015, 0.0015], c);
}
fn rect_uv(v: &mut Vec<Vertex>, r: [f32; 4], uv: [f32; 4], color: [f32; 4]) {
    for (x, y, u, w) in [
        (r[0], r[1], uv[0], uv[1]),
        (r[0] + r[2], r[1], uv[2], uv[1]),
        (r[0] + r[2], r[1] + r[3], uv[2], uv[3]),
        (r[0], r[1], uv[0], uv[1]),
        (r[0] + r[2], r[1] + r[3], uv[2], uv[3]),
        (r[0], r[1] + r[3], uv[0], uv[3]),
    ] {
        v.push(Vertex {
            p: [x, y],
            uv: [u, w],
            color,
        });
    }
}
fn triangle(v: &mut Vec<Vertex>, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 4]) {
    for p in [a, b, c] {
        v.push(Vertex {
            p,
            uv: [0.0015; 2],
            color,
        });
    }
}
