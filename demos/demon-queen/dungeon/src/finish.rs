//! Restrained optical glow, isolated from the HUD. Quarter-resolution separable
//! blur: three small passes and one composite, no full-resolution blur kernel.
//! All textures/bind groups are retained and recreated only on resize.
pub struct Finish {
    format: wgpu::TextureFormat,
    size: (u32, u32),
    pub scene: wgpu::TextureView,
    glow: [wgpu::TextureView; 3],
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bindings: [wgpu::BindGroup; 4],
    pipelines: [wgpu::RenderPipeline; 4],
}
const SHADER: &str = r#"
struct V { @builtin(position) p:vec4<f32>, @location(0) uv:vec2<f32> };
@group(0) @binding(0) var source:texture_2d<f32>;
@group(0) @binding(1) var linear_sampler:sampler;
@group(0) @binding(2) var glow:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) index:u32)->V {
    var positions=array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));
    var o:V;
    o.p=vec4<f32>(positions[index],0.0,1.0);
    o.uv=positions[index]*vec2<f32>(0.5,-0.5)+vec2<f32>(0.5);
    return o;
}
fn highlight(uv:vec2<f32>)->vec3<f32> {
    let c=textureSample(source,linear_sampler,uv).rgb;
    let luminance=dot(c,vec3<f32>(0.2126,0.7152,0.0722));
    return c*smoothstep(0.55,0.95,luminance);
}
@fragment fn extract(v:V)->@location(0) vec4<f32> {
    let d=1.3/vec2<f32>(textureDimensions(source));
    let c=(highlight(v.uv+vec2<f32>(-d.x,-d.y))+highlight(v.uv+vec2<f32>(d.x,-d.y))
        +highlight(v.uv+vec2<f32>(-d.x,d.y))+highlight(v.uv+d))*0.25;
    return vec4<f32>(c,1.0);
}
fn blur(uv:vec2<f32>,axis:vec2<f32>)->vec4<f32> {
    let d=axis/vec2<f32>(textureDimensions(source));
    var c=textureSample(source,linear_sampler,uv).rgb*0.227027;
    c+=(textureSample(source,linear_sampler,uv+d*1.384615).rgb
       +textureSample(source,linear_sampler,uv-d*1.384615).rgb)*0.316216;
    c+=(textureSample(source,linear_sampler,uv+d*3.230769).rgb
       +textureSample(source,linear_sampler,uv-d*3.230769).rgb)*0.070270;
    return vec4<f32>(c,1.0);
}
@fragment fn horizontal(v:V)->@location(0) vec4<f32> { return blur(v.uv,vec2<f32>(1.9,0.0)); }
@fragment fn vertical(v:V)->@location(0) vec4<f32> { return blur(v.uv,vec2<f32>(0.0,1.9)); }
@fragment fn composite(v:V)->@location(0) vec4<f32> {
    let base=textureSample(source,linear_sampler,v.uv).rgb;
    let halo=textureSample(glow,linear_sampler,v.uv).rgb;
    let border=length((v.uv-0.5)*vec2<f32>(1.0,0.72));
    let vignette=1.0-0.2*smoothstep(0.3,0.72,border);
    var c=base+halo*0.42;
    // Painterly grade: richer saturation, gentle S-curve, warm highlights.
    let luma=dot(c,vec3<f32>(0.2126,0.7152,0.0722));
    c=max(mix(vec3<f32>(luma),c,1.14),vec3<f32>(0.0));
    c=clamp(c,vec3<f32>(0.0),vec3<f32>(1.0));
    c=mix(c,c*c*(3.0-2.0*c),0.28);
    c=c*vec3<f32>(1.04,1.0,0.95);
    return vec4<f32>(c*vignette,1.0);
}
"#;
fn canvas(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
fn bind(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    source: &wgpu::TextureView,
    glow: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("dungeon-finish-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(glow),
            },
        ],
    })
}
impl Finish {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dungeon-finish-layout"),
            entries: &[
                texture(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture(2),
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("dungeon-glow-linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dungeon-finish"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipelines = ["extract", "horizontal", "vertical", "composite"].map(|entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: if entry == "composite" {
                            format
                        } else {
                            wgpu::TextureFormat::Rgba16Float
                        },
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        });
        let size = (1920, 1080);
        let scene = canvas(device, size, format, "dungeon-scene");
        let glow = std::array::from_fn(|_| {
            canvas(
                device,
                (480, 270),
                wgpu::TextureFormat::Rgba16Float,
                "dungeon-glow-quarter",
            )
        });
        let bindings = [
            bind(device, &layout, &sampler, &scene, &scene),
            bind(device, &layout, &sampler, &glow[0], &scene),
            bind(device, &layout, &sampler, &glow[1], &scene),
            bind(device, &layout, &sampler, &scene, &glow[2]),
        ];
        Self {
            format,
            size,
            scene,
            glow,
            layout,
            sampler,
            bindings,
            pipelines,
        }
    }
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size {
            return;
        }
        self.size = size;
        self.scene = canvas(device, size, self.format, "dungeon-scene");
        self.glow = std::array::from_fn(|_| {
            canvas(
                device,
                ((size.0 / 4).max(1), (size.1 / 4).max(1)),
                wgpu::TextureFormat::Rgba16Float,
                "dungeon-glow-quarter",
            )
        });
        self.bindings = [
            bind(
                device,
                &self.layout,
                &self.sampler,
                &self.scene,
                &self.scene,
            ),
            bind(
                device,
                &self.layout,
                &self.sampler,
                &self.glow[0],
                &self.scene,
            ),
            bind(
                device,
                &self.layout,
                &self.sampler,
                &self.glow[1],
                &self.scene,
            ),
            bind(
                device,
                &self.layout,
                &self.sampler,
                &self.scene,
                &self.glow[2],
            ),
        ];
    }
    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("dungeon-optical-finish"),
        });
        for (i, view) in [&self.glow[0], &self.glow[1], &self.glow[2], target]
            .into_iter()
            .enumerate()
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dungeon-finish-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipelines[i]);
            pass.set_bind_group(0, &self.bindings[i], &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
    }
}
