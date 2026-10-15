//! The 3D frame from the size it was drawn at to the size it is shown at, with the graphics
//! settings' finishing passes: FXAA (edges smoothed from the frame's own luma), the upscale
//! (bilinear, or AMD FidelityFX Super Resolution 1's edge-adaptive EASU) and RCAS sharpening
//! (FSR 1's robust contrast-adaptive sharpening, usable at full size too).
//!
//! EASU and RCAS follow `ffx_fsr1.h` (AMD FidelityFX FSR 1.0, MIT licence): the same 12-tap
//! kernel, direction/length analysis, Lanczos-2 approximation with its deringing clamp, and the
//! same 5-tap sharpening lobe with its limit.

/// How the frame is brought up to the screen's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Upscale3D {
    /// Plain bilinear filtering.
    Bilinear,
    /// AMD FSR 1 (EASU): edge-adaptive, keeps lines crisp.
    #[default]
    Fsr1,
}

/// Edge smoothing done after the frame is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Antialias3D {
    /// None.
    #[default]
    None,
    /// Fast approximate antialiasing on the drawn frame.
    Fxaa,
}

/// The finishing passes of one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finishing3D {
    /// How a smaller frame is brought to the shown size.
    pub upscale: Upscale3D,
    /// Edge smoothing.
    pub antialias: Antialias3D,
    /// RCAS sharpening: 0 none, 1 strongest.
    pub sharpness: f32,
}

impl Default for Finishing3D {
    fn default() -> Self {
        Self {
            upscale: Upscale3D::Fsr1,
            antialias: Antialias3D::None,
            sharpness: 0.0,
        }
    }
}

impl Finishing3D {
    /// Whether a frame drawn at `drawn` and shown at `shown` needs any pass at all (else it can be
    /// drawn straight into the shown target).
    pub fn needed(&self, drawn: (u32, u32), shown: (u32, u32)) -> bool {
        drawn != shown || self.antialias != Antialias3D::None || self.sharpness > 0.0
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    /// input width, height, output width, height
    sizes: [f32; 4],
    /// x: RCAS strength (exp2(-stops)), y: 1 when RCAS runs
    sharp: [f32; 4],
}

struct Temp {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// The passes and their in-between pictures.
pub struct Upscaler3D {
    format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    fxaa: wgpu::RenderPipeline,
    easu: wgpu::RenderPipeline,
    bilinear: wgpu::RenderPipeline,
    rcas: wgpu::RenderPipeline,
    /// FXAA's result (drawn size) and the upscale's (shown size) when RCAS follows.
    temps: [Option<Temp>; 2],
}

impl Upscaler3D {
    /// Passes writing `format` (the shown target's).
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-upscale-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-upscale-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-upscale-uniform"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-upscale-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-upscale-pipeline-layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = |entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        Self {
            format,
            fxaa: pipeline("fs_fxaa"),
            easu: pipeline("fs_easu"),
            bilinear: pipeline("fs_bilinear"),
            rcas: pipeline("fs_rcas"),
            layout,
            sampler,
            uniform,
            temps: [None, None],
        }
    }

    fn temp(&mut self, device: &wgpu::Device, slot: usize, size: (u32, u32)) -> wgpu::TextureView {
        let fits = self.temps[slot]
            .as_ref()
            .is_some_and(|t| (t.texture.width(), t.texture.height()) == size);
        if !fits {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("velvet-upscale-temp"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            self.temps[slot] = Some(Temp { texture, view });
        }
        self.temps[slot].as_ref().expect("made above").view.clone()
    }

    /// `input` (drawn at `drawn`, sampled) into `output` (shown at `shown`) through the passes
    /// `finishing` asks for; submitted at once.
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &wgpu::TextureView,
        drawn: (u32, u32),
        output: &wgpu::TextureView,
        shown: (u32, u32),
        finishing: Finishing3D,
    ) {
        let sharp = finishing.sharpness.clamp(0.0, 1.0);
        // RCAS's stops: 0 is its strongest, 2 a gentle touch.
        let stops = (1.0 - sharp) * 2.0;
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Uniform {
                sizes: [
                    drawn.0 as f32,
                    drawn.1 as f32,
                    shown.0 as f32,
                    shown.1 as f32,
                ],
                sharp: [(-stops).exp2(), if sharp > 0.0 { 1.0 } else { 0.0 }, 0.0, 0.0],
            }),
        );
        // The in-between pictures first (they may be made anew).
        let sharpen = sharp > 0.0;
        let fxaa_out = (finishing.antialias == Antialias3D::Fxaa).then(|| self.temp(device, 0, drawn));
        let scaled = if sharpen { self.temp(device, 1, shown) } else { output.clone() };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("velvet-upscale"),
        });
        let group = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-upscale-group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };
        let pass = |encoder: &mut wgpu::CommandEncoder,
                    pipeline: &wgpu::RenderPipeline,
                    group: &wgpu::BindGroup,
                    target: &wgpu::TextureView| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-upscale-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        // 1. FXAA at the drawn size.
        let mut source = input.clone();
        if let Some(out) = fxaa_out {
            let g = group(&source);
            pass(&mut encoder, &self.fxaa, &g, &out);
            source = out;
        }
        // 2. To the shown size (the same size: a plain copy), into the output unless RCAS follows.
        let g = group(&source);
        let upscale = if drawn != shown && finishing.upscale == Upscale3D::Fsr1 {
            &self.easu
        } else {
            &self.bilinear
        };
        pass(&mut encoder, upscale, &g, &scaled);
        // 3. RCAS into the output.
        if sharpen {
            let g = group(&scaled);
            pass(&mut encoder, &self.rcas, &g, output);
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

const SHADER: &str = r#"
struct U {
    sizes: vec4<f32>,
    sharp: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct V {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> V {
    var out: V;
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

fn load(p: vec2<i32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(src));
    return textureLoad(src, clamp(p, vec2<i32>(0), size - vec2<i32>(1)), 0).rgb;
}

// ---- bilinear (also the same-size copy)
@fragment
fn fs_bilinear(in: V) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSampleLevel(src, samp, in.uv, 0.0).rgb, 1.0);
}

// ---- FXAA (Lottes): the edge's direction from the corners' luma, blended along it
fn fxaa_luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

@fragment
fn fs_fxaa(in: V) -> @location(0) vec4<f32> {
    let texel = 1.0 / u.sizes.xy;
    let uv = in.uv;
    let rgb_m = textureSampleLevel(src, samp, uv, 0.0).rgb;
    let nw = fxaa_luma(textureSampleLevel(src, samp, uv + vec2<f32>(-1.0, -1.0) * texel, 0.0).rgb);
    let ne = fxaa_luma(textureSampleLevel(src, samp, uv + vec2<f32>(1.0, -1.0) * texel, 0.0).rgb);
    let sw = fxaa_luma(textureSampleLevel(src, samp, uv + vec2<f32>(-1.0, 1.0) * texel, 0.0).rgb);
    let se = fxaa_luma(textureSampleLevel(src, samp, uv + vec2<f32>(1.0, 1.0) * texel, 0.0).rgb);
    let m = fxaa_luma(rgb_m);
    let luma_min = min(m, min(min(nw, ne), min(sw, se)));
    let luma_max = max(m, max(max(nw, ne), max(sw, se)));
    // Flat enough: left alone.
    if (luma_max - luma_min < max(0.0312, luma_max * 0.125)) {
        return vec4<f32>(rgb_m, 1.0);
    }
    var dir = vec2<f32>(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
    let reduce = max((nw + ne + sw + se) * (0.25 * (1.0 / 8.0)), 1.0 / 128.0);
    let rcp_min = 1.0 / (min(abs(dir.x), abs(dir.y)) + reduce);
    dir = clamp(dir * rcp_min, vec2<f32>(-8.0), vec2<f32>(8.0)) * texel;
    let rgb_a = 0.5 * (textureSampleLevel(src, samp, uv + dir * (1.0 / 3.0 - 0.5), 0.0).rgb
        + textureSampleLevel(src, samp, uv + dir * (2.0 / 3.0 - 0.5), 0.0).rgb);
    let rgb_b = rgb_a * 0.5 + 0.25 * (textureSampleLevel(src, samp, uv + dir * -0.5, 0.0).rgb
        + textureSampleLevel(src, samp, uv + dir * 0.5, 0.0).rgb);
    let luma_b = fxaa_luma(rgb_b);
    if (luma_b < luma_min || luma_b > luma_max) {
        return vec4<f32>(rgb_a, 1.0);
    }
    return vec4<f32>(rgb_b, 1.0);
}

// ---- FSR 1 EASU
fn easu_luma(c: vec3<f32>) -> f32 {
    return c.b * 0.5 + (c.r * 0.5 + c.g);
}

// One of the four bilinear corners' contribution to the edge's direction and length.
fn easu_set(dir: ptr<function, vec2<f32>>, len: ptr<function, f32>, w: f32,
            la: f32, lb: f32, lc: f32, ld: f32, le: f32) {
    let dc = ld - lc;
    let cb = lc - lb;
    var len_x = max(abs(dc), abs(cb));
    len_x = select(1.0 / len_x, 0.0, len_x == 0.0);
    let dir_x = ld - lb;
    (*dir).x = (*dir).x + dir_x * w;
    len_x = clamp(abs(dir_x) * len_x, 0.0, 1.0);
    len_x = len_x * len_x;
    *len = *len + len_x * w;
    let ec = le - lc;
    let ca = lc - la;
    var len_y = max(abs(ec), abs(ca));
    len_y = select(1.0 / len_y, 0.0, len_y == 0.0);
    let dir_y = le - la;
    (*dir).y = (*dir).y + dir_y * w;
    len_y = clamp(abs(dir_y) * len_y, 0.0, 1.0);
    len_y = len_y * len_y;
    *len = *len + len_y * w;
}

// One tap of the anisotropic Lanczos-2 approximation.
fn easu_tap(ac: ptr<function, vec3<f32>>, aw: ptr<function, f32>, off: vec2<f32>, dir: vec2<f32>,
            len2: vec2<f32>, lob: f32, clp: f32, c: vec3<f32>) {
    var v = vec2<f32>(off.x * dir.x + off.y * dir.y, off.x * -dir.y + off.y * dir.x);
    v = v * len2;
    let d2 = min(v.x * v.x + v.y * v.y, clp);
    var wb = (2.0 / 5.0) * d2 - 1.0;
    var wa = lob * d2 - 1.0;
    wb = wb * wb;
    wa = wa * wa;
    wb = (25.0 / 16.0) * wb - (25.0 / 16.0 - 1.0);
    let w = wb * wa;
    *ac = *ac + c * w;
    *aw = *aw + w;
}

@fragment
fn fs_easu(in: V) -> @location(0) vec4<f32> {
    // The output pixel's place on the input grid.
    let ip = floor(in.pos.xy);
    var pp = (ip + vec2<f32>(0.5)) * (u.sizes.xy / u.sizes.zw) - vec2<f32>(0.5);
    let fp = floor(pp);
    pp = pp - fp;
    let f0 = vec2<i32>(fp);
    //      b c
    //    e f g h
    //    i j k l
    //      n o
    let b = load(f0 + vec2<i32>(0, -1));
    let c = load(f0 + vec2<i32>(1, -1));
    let e = load(f0 + vec2<i32>(-1, 0));
    let f = load(f0 + vec2<i32>(0, 0));
    let g = load(f0 + vec2<i32>(1, 0));
    let h = load(f0 + vec2<i32>(2, 0));
    let i = load(f0 + vec2<i32>(-1, 1));
    let j = load(f0 + vec2<i32>(0, 1));
    let k = load(f0 + vec2<i32>(1, 1));
    let l = load(f0 + vec2<i32>(2, 1));
    let n = load(f0 + vec2<i32>(0, 2));
    let o = load(f0 + vec2<i32>(1, 2));
    let bl = easu_luma(b);
    let cl = easu_luma(c);
    let el = easu_luma(e);
    let fl = easu_luma(f);
    let gl = easu_luma(g);
    let hl = easu_luma(h);
    let il = easu_luma(i);
    let jl = easu_luma(j);
    let kl = easu_luma(k);
    let ll = easu_luma(l);
    let nl = easu_luma(n);
    let ol = easu_luma(o);
    var dir = vec2<f32>(0.0);
    var len = 0.0;
    easu_set(&dir, &len, (1.0 - pp.x) * (1.0 - pp.y), bl, el, fl, gl, jl);
    easu_set(&dir, &len, pp.x * (1.0 - pp.y), cl, fl, gl, hl, kl);
    easu_set(&dir, &len, (1.0 - pp.x) * pp.y, fl, il, jl, kl, nl);
    easu_set(&dir, &len, pp.x * pp.y, gl, jl, kl, ll, ol);
    // The direction normalised (none: along x).
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zero = dir_r < (1.0 / 32768.0);
    dir_r = select(inverseSqrt(dir_r), 1.0, zero);
    dir.x = select(dir.x, 1.0, zero);
    dir = dir * dir_r;
    // How much of an edge: shapes the kernel.
    len = len * 0.5;
    len = len * len;
    let stretch = (dir.x * dir.x + dir.y * dir.y) / max(abs(dir.x), abs(dir.y));
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    let clp = 1.0 / lob;
    var ac = vec3<f32>(0.0);
    var aw = 0.0;
    easu_tap(&ac, &aw, vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, b);
    easu_tap(&ac, &aw, vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, c);
    easu_tap(&ac, &aw, vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, i);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, j);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, f);
    easu_tap(&ac, &aw, vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, e);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, k);
    easu_tap(&ac, &aw, vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, l);
    easu_tap(&ac, &aw, vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, h);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, g);
    easu_tap(&ac, &aw, vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, o);
    easu_tap(&ac, &aw, vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, n);
    // Deringing: within the four nearest texels' range.
    let mn = min(min(f, g), min(j, k));
    let mx = max(max(f, g), max(j, k));
    let pix = min(mx, max(mn, ac / max(aw, 1e-5)));
    return vec4<f32>(pix, 1.0);
}

// ---- FSR 1 RCAS
@fragment
fn fs_rcas(in: V) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.pos.xy));
    //    b
    //  d e f
    //    h
    let b = load(p + vec2<i32>(0, -1));
    let d = load(p + vec2<i32>(-1, 0));
    let e = load(p);
    let f = load(p + vec2<i32>(1, 0));
    let h = load(p + vec2<i32>(0, 1));
    let mn4 = min(min(b, d), min(f, h));
    let mx4 = max(max(b, d), max(f, h));
    let peak = vec2<f32>(1.0, -4.0);
    let hit_min = min(mn4, e) / (4.0 * max(mx4, vec3<f32>(1e-5)));
    let hit_max = (peak.x - max(mx4, e)) / (4.0 * mn4 + peak.y);
    let lobe_rgb = max(-hit_min, hit_max);
    // FSR_RCAS_LIMIT: 0.25 - 1/16.
    let lobe = max(-0.1875, min(max(lobe_rgb.r, max(lobe_rgb.g, lobe_rgb.b)), 0.0)) * u.sharp.x;
    let rcp_l = 1.0 / (4.0 * lobe + 1.0);
    let c = (lobe * (b + d + h + f) + e) * rcp_l;
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

#[cfg(test)]
mod tests {
    #[test]
    fn the_upscale_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(super::SHADER).expect("parses");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("validates");
    }
}
