//! Drawing a stage on the graphics card: the pictures become textures (with their smaller copies, sent again only when
//! painted again), each card is a quad, the world through the camera with depth, then the screen cards on top. A lens
//! adds a pass between the two: the world again, magnified, inside its circle.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use velvet_math::Mat4;

use crate::stage::{Blend, CardId, PictureId, Space, Stage, MAX_LIGHTS};

/// The globals of a frame, as the shader reads them.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    screen_proj: [[f32; 4]; 4],
    eye: [f32; 4],
    ambient: [f32; 4],
    fog_color: [f32; 4],
    fog: [f32; 4],
    light_position: [[f32; 4]; MAX_LIGHTS],
    light_color: [[f32; 4]; MAX_LIGHTS],
    /// Through a lens: its centre and radius in the target's pixels, and 1 (0 when not).
    lens: [f32; 4],
}

/// A card, as the shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    shape: [f32; 4],
    uv: [f32; 4],
    tint: [f32; 4],
    flags: [f32; 4],
}

/// A picture on the graphics card.
struct GpuPicture {
    bind: wgpu::BindGroup,
    version: u64,
}

/// The multisampled colour and the depth of the frames, for a target size.
struct Targets {
    size: (u32, u32),
    color: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    World,
    WorldAdd,
    Screen,
    ScreenAdd,
}

/// Draws stages into a target of one texture format.
pub struct StageRenderer {
    samples: u32,
    picture_layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 4],
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    lens_globals: wgpu::Buffer,
    lens_globals_bind: wgpu::BindGroup,
    instances: wgpu::Buffer,
    capacity: usize,
    pictures: HashMap<PictureId, GpuPicture>,
    white: wgpu::BindGroup,
    targets: Option<Targets>,
}

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// sRGB to linear, for colours given in sRGB.
fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl StageRenderer {
    /// A renderer for targets of `format`, with `samples` samples a pixel (1 or 4).
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        samples: u32,
    ) -> StageRenderer {
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-stage globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let picture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-stage picture"),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-stage"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stage.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-stage"),
            bind_group_layouts: &[&globals_layout, &picture_layout],
            push_constant_ranges: &[],
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];
        let normal = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let pipeline =
            |blend: wgpu::BlendState, write_depth: bool, compare: wgpu::CompareFunction| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("velvet-stage"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Instance>() as u64,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &attributes,
                        }],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: DEPTH,
                        depth_write_enabled: write_depth,
                        depth_compare: compare,
                        stencil: Default::default(),
                        bias: Default::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                })
            };
        let pipelines = [
            pipeline(normal, true, wgpu::CompareFunction::LessEqual),
            pipeline(add, false, wgpu::CompareFunction::LessEqual),
            pipeline(normal, false, wgpu::CompareFunction::Always),
            pipeline(add, false, wgpu::CompareFunction::Always),
        ];
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-stage"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let uniforms = |label: &str| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<Globals>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &globals_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            (buffer, bind)
        };
        let (globals, globals_bind) = uniforms("velvet-stage globals");
        // The globals of the world seen through a lens.
        let (lens_globals, lens_globals_bind) = uniforms("velvet-stage lens");
        let capacity = 64;
        let instances = Self::instance_buffer(device, capacity);
        let white = Self::upload(
            device,
            queue,
            &picture_layout,
            &sampler,
            1,
            1,
            &[255, 255, 255, 255],
        );
        StageRenderer {
            samples,
            picture_layout,
            pipelines,
            sampler,
            globals,
            globals_bind,
            lens_globals,
            lens_globals_bind,
            instances,
            capacity,
            pictures: HashMap::new(),
            white,
            targets: None,
        }
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-stage cards"),
            size: (capacity * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// A texture with its smaller copies, from premultiplied RGBA pixels.
    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> wgpu::BindGroup {
        let levels = 32 - width.max(height).max(1).leading_zeros();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velvet-stage picture"),
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
        let mut level = pixels.to_vec();
        let (mut w, mut h) = (width, height);
        for mip in 0..levels {
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
                let (next, nw, nh) = halve(&level, w, h);
                level = next;
                w = nw;
                h = nh;
            }
        }
        let view = texture.create_view(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-stage picture"),
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

    /// Sends the pictures painted since the last frame, and forgets the ones taken off the stage.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, stage: &Stage) {
        for (id, picture) in stage.pictures() {
            if self
                .pictures
                .get(&id)
                .is_some_and(|g| g.version == picture.version)
            {
                continue;
            }
            let canvas = &picture.canvas;
            let bind = Self::upload(
                device,
                queue,
                &self.picture_layout,
                &self.sampler,
                canvas.pixel_width(),
                canvas.pixel_height(),
                canvas.pixels(),
            );
            self.pictures.insert(
                id,
                GpuPicture {
                    bind,
                    version: picture.version,
                },
            );
        }
        self.pictures.retain(|id, _| stage.picture(*id).is_some());
    }

    fn targets(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let texture = |format: wgpu::TextureFormat, label: &str| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0.max(1),
                        height: size.1.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: self.samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let color = (self.samples > 1).then(|| texture(format, "velvet-stage colour"));
        let depth = texture(DEPTH, "velvet-stage depth");
        self.targets = Some(Targets { size, color, depth });
    }

    /// Draws the stage into `target` (a view of a texture of `format` and `size`), inside `viewport` (x, y, width,
    /// height in the target's pixels); the rest is cleared black.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        viewport: [f32; 4],
        stage: &Stage,
    ) {
        self.prepare(device, queue, stage);
        self.targets(device, format, size);

        // The cards in drawing order: the backdrop of the stage's colour, the world far to near within each order,
        // then the screen.
        let view = stage.camera.view();
        let mut world: Vec<(i32, f32, CardId)> = Vec::new();
        let mut screen: Vec<(i32, usize, CardId)> = Vec::new();
        for (index, (id, card)) in stage.cards().enumerate() {
            if !stage.shown(id) || !self.pictures.contains_key(&card.picture) {
                continue;
            }
            match card.space {
                Space::World => {
                    let Some(matrix) = stage.matrix(id) else {
                        continue;
                    };
                    let centre = matrix.transform_point3(card.face_point(0.5, 0.5));
                    let depth = view.transform_point3(centre).z;
                    world.push((card.order, depth, id));
                }
                Space::Screen => screen.push((card.order, index, id)),
            }
        }
        // Farther first: in view space the farther, the more negative.
        world.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        screen.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut instances: Vec<Instance> = Vec::with_capacity(world.len() + screen.len() + 1);
        let mut draws: Vec<(Pass, Option<PictureId>)> = Vec::new();
        let clear = stage.clear.map(linear);
        instances.push(Instance {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            shape: [stage.screen.x, stage.screen.y, 0.0, 0.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: [clear[0], clear[1], clear[2], 1.0],
            flags: [0.0, 1.0, 0.0, 0.0],
        });
        draws.push((Pass::Screen, None));
        for (_, _, id) in &world {
            let Some(card) = stage.card(*id) else {
                continue;
            };
            let Some(matrix) = stage.matrix(*id) else {
                continue;
            };
            instances.push(Instance {
                model: matrix.to_cols_array_2d(),
                shape: [card.size.x, card.size.y, card.anchor.x, card.anchor.y],
                uv: card.uv,
                tint: card.tint,
                // A card that writes depth hides what is behind it: its nearly clear pixels are dropped (see the
                // shader), so the faint haze a cut-out leaves round a picture does not hide things it seems not to cover.
                flags: [
                    if card.lit { 1.0 } else { 0.0 },
                    0.0,
                    if card.blend == Blend::Add { 0.0 } else { 1.0 },
                    0.0,
                ],
            });
            draws.push((
                if card.blend == Blend::Add {
                    Pass::WorldAdd
                } else {
                    Pass::World
                },
                Some(card.picture),
            ));
        }
        for (_, _, id) in &screen {
            let Some(card) = stage.card(*id) else {
                continue;
            };
            let Some(matrix) = stage.matrix(*id) else {
                continue;
            };
            instances.push(Instance {
                model: matrix.to_cols_array_2d(),
                shape: [card.size.x, card.size.y, card.anchor.x, card.anchor.y],
                uv: card.uv,
                tint: card.tint,
                flags: [0.0, 1.0, 0.0, 0.0],
            });
            draws.push((
                if card.blend == Blend::Add {
                    Pass::ScreenAdd
                } else {
                    Pass::Screen
                },
                Some(card.picture),
            ));
        }

        // The globals.
        let lights = stage.active_lights();
        let mut light_position = [[0.0f32; 4]; MAX_LIGHTS];
        let mut light_color = [[0.0f32; 4]; MAX_LIGHTS];
        for (i, light) in lights.iter().enumerate() {
            light_position[i] = [
                light.position.x,
                light.position.y,
                light.position.z,
                light.range.max(1e-3),
            ];
            light_color[i] = [
                light.color[0] * light.intensity,
                light.color[1] * light.intensity,
                light.color[2] * light.intensity,
                0.0,
            ];
        }
        let (fog_color, fog) = match stage.fog {
            Some(f) => (
                [f.color[0], f.color[1], f.color[2], 1.0],
                [f.start, f.end, 1.0, lights.len() as f32],
            ),
            None => ([0.0; 4], [0.0, 1.0, 0.0, lights.len() as f32]),
        };
        let (w, h) = (stage.screen.x.max(1.0), stage.screen.y.max(1.0));
        let screen_proj = Mat4::from_cols(
            [2.0 / w, 0.0, 0.0, 0.0],
            [0.0, -2.0 / h, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        );
        let eye = stage.camera.eye;
        let globals = Globals {
            view_proj: stage.view_projection().to_cols_array_2d(),
            screen_proj: screen_proj.to_cols_array_2d(),
            eye: [eye.x, eye.y, eye.z, 1.0],
            ambient: [stage.ambient[0], stage.ambient[1], stage.ambient[2], 1.0],
            fog_color,
            fog,
            light_position,
            light_color,
            lens: [0.0; 4],
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        if instances.len() > self.capacity {
            self.capacity = instances.len().next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&instances));

        let targets = self.targets.as_ref().expect("made above");
        let (view, resolve) = match &targets.color {
            Some(msaa) => (msaa, Some(target)),
            None => (target, None),
        };
        let [x, y, vw, vh] = viewport;
        // A lens draws the backdrop and the world again, magnified, inside its circle and before the screen cards: its
        // globals carry the magnified projection and the circle in the target's pixels, and a scissor keeps the pass
        // to the circle's square. Without one, a single pass.
        let world_end = 1 + world.len();
        let mut passes: Vec<(Range<usize>, bool, Option<[u32; 4]>)> =
            vec![(0..draws.len(), false, None)];
        if let Some(lens) = stage.lens.filter(|l| l.radius > 0.5 && l.zoom > 0.0) {
            let (sx, sy) = (vw / w, vh / h);
            let (cx, cy) = (x + lens.center.x * sx, y + lens.center.y * sy);
            let r = lens.radius * sx.min(sy);
            let clamp = |v: f32, max: u32| v.clamp(0.0, max as f32) as u32;
            let (x0, y0) = (
                clamp((cx - r).floor(), size.0),
                clamp((cy - r).floor(), size.1),
            );
            let (x1, y1) = (
                clamp((cx + r).ceil(), size.0),
                clamp((cy + r).ceil(), size.1),
            );
            if x1 > x0 && y1 > y0 {
                let magnified = Globals {
                    view_proj: lens
                        .view_projection(stage.view_projection(), stage.screen)
                        .to_cols_array_2d(),
                    lens: [cx, cy, r, 1.0],
                    ..globals
                };
                queue.write_buffer(&self.lens_globals, 0, bytemuck::bytes_of(&magnified));
                passes = vec![
                    (0..world_end, false, None),
                    (0..world_end, true, Some([x0, y0, x1 - x0, y1 - y0])),
                    (world_end..draws.len(), false, None),
                ];
            }
        }
        let count = passes.len();
        for (k, (range, through, scissor)) in passes.into_iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-stage"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    // Resolved once, after the last pass.
                    resolve_target: if k + 1 == count { resolve } else { None },
                    ops: wgpu::Operations {
                        load: if k == 0 {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // Each pass starts its own depth: the lens sees the world anew, and the screen does not test it.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_viewport(x, y, vw.max(1.0), vh.max(1.0), 0.0, 1.0);
            if let Some([left, top, width, height]) = scissor {
                pass.set_scissor_rect(left, top, width, height);
            }
            let uniforms = if through {
                &self.lens_globals_bind
            } else {
                &self.globals_bind
            };
            pass.set_bind_group(0, uniforms, &[]);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            self.draw_range(&mut pass, &draws, range);
        }
    }

    /// Issues the draws of `range` (each draws the instance of its own index), changing pipelines as their kind
    /// changes.
    fn draw_range(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        draws: &[(Pass, Option<PictureId>)],
        range: Range<usize>,
    ) {
        let mut current: Option<Pass> = None;
        for (i, (kind, picture)) in draws.iter().enumerate().skip(range.start).take(range.len()) {
            if current != Some(*kind) {
                let index = match kind {
                    Pass::World => 0,
                    Pass::WorldAdd => 1,
                    Pass::Screen => 2,
                    Pass::ScreenAdd => 3,
                };
                pass.set_pipeline(&self.pipelines[index]);
                current = Some(*kind);
            }
            let bind = match picture {
                Some(id) => match self.pictures.get(id) {
                    Some(g) => &g.bind,
                    None => continue,
                },
                None => &self.white,
            };
            pass.set_bind_group(1, bind, &[]);
            pass.draw(0..6, i as u32..i as u32 + 1);
        }
    }
}

/// Half the size, each pixel the mean of the (up to) four under it.
fn halve(pixels: &[u8], width: u32, height: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((width / 2).max(1), (height / 2).max(1));
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        let (y0, y1) = ((y * 2).min(height - 1), (y * 2 + 1).min(height - 1));
        for x in 0..nw {
            let (x0, x1) = ((x * 2).min(width - 1), (x * 2 + 1).min(width - 1));
            for c in 0..4 {
                let at = |px: u32, py: u32| pixels[((py * width + px) * 4 + c) as usize] as u32;
                let sum = at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1);
                out[((y * nw + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
            }
        }
    }
    (out, nw, nh)
}

trait ColsArray {
    fn to_cols_array_2d(self) -> [[f32; 4]; 4];
}

impl ColsArray for Mat4 {
    fn to_cols_array_2d(self) -> [[f32; 4]; 4] {
        [self.x_axis, self.y_axis, self.z_axis, self.w_axis]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_averages_and_keeps_odd_edges() {
        let pixels: Vec<u8> = (0..3 * 2).flat_map(|i| [i as u8 * 10, 0, 0, 255]).collect();
        let (out, w, h) = halve(&pixels, 3, 2);
        assert_eq!((w, h), (1, 1));
        // The mean of pixels 0, 1, 3, 4 (values 0, 10, 30, 40).
        assert_eq!(out[0], 20);
        assert_eq!(out[3], 255);
    }
}
