// The cards of a stage: a quad per card, its picture premultiplied, lit by a few point lights and hazed by distance
// in the world; flat, in logical pixels, on the screen.

struct Globals {
    view_proj: mat4x4<f32>,
    screen_proj: mat4x4<f32>,
    eye: vec4<f32>,
    ambient: vec4<f32>,
    fog_color: vec4<f32>,
    // start, end, on (1), light count
    fog: vec4<f32>,
    // xyz, range
    light_position: array<vec4<f32>, 16>,
    // rgb × intensity
    light_color: array<vec4<f32>, 16>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var filtering: sampler;

struct Instance {
    @builtin(vertex_index) index: u32,
    @location(0) m0: vec4<f32>,
    @location(1) m1: vec4<f32>,
    @location(2) m2: vec4<f32>,
    @location(3) m3: vec4<f32>,
    // width, height, anchor x, anchor y
    @location(4) shape: vec4<f32>,
    // u0, v0, u1, v1
    @location(5) uv: vec4<f32>,
    @location(6) tint: vec4<f32>,
    // lit, on the screen, -, -
    @location(7) flags: vec4<f32>,
};

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) flags: vec4<f32>,
};

@vertex
fn vs_main(v: Instance) -> Fragment {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let c = corners[v.index];
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var out: Fragment;
    if (v.flags.y > 0.5) {
        // On the screen: y goes down.
        let p = model * vec4<f32>((c.x - v.shape.z) * v.shape.x, (c.y - v.shape.w) * v.shape.y, 0.0, 1.0);
        out.position = g.screen_proj * p;
        out.world = p.xyz;
        out.normal = vec3<f32>(0.0, 0.0, 1.0);
    } else {
        let p = model * vec4<f32>((c.x - v.shape.z) * v.shape.x, (v.shape.w - c.y) * v.shape.y, 0.0, 1.0);
        out.position = g.view_proj * p;
        out.world = p.xyz;
        out.normal = (model * vec4<f32>(0.0, 0.0, 1.0, 0.0)).xyz;
    }
    out.uv = mix(v.uv.xy, v.uv.zw, c);
    out.tint = v.tint;
    out.flags = v.flags;
    return out;
}

@fragment
fn fs_main(f: Fragment) -> @location(0) vec4<f32> {
    let texel = textureSample(picture, filtering, f.uv);
    var color = vec4<f32>(texel.rgb * f.tint.rgb, texel.a) * f.tint.a;
    if (color.a < 0.004) {
        discard;
    }
    if (f.flags.x > 0.5) {
        let normal = normalize(f.normal);
        var light = g.ambient.rgb;
        let count = i32(g.fog.w);
        for (var i = 0; i < count; i = i + 1) {
            let lp = g.light_position[i];
            let to_light = lp.xyz - f.world;
            let dist = length(to_light);
            let reach = clamp(1.0 - dist / lp.w, 0.0, 1.0);
            // Paper takes light from both sides, and never goes fully dark by the angle.
            let facing = abs(dot(normal, to_light / max(dist, 0.0001)));
            light = light + g.light_color[i].rgb * reach * reach * (0.4 + 0.6 * facing);
        }
        color = vec4<f32>(color.rgb * light, color.a);
        if (g.fog.z > 0.5) {
            let away = distance(g.eye.xyz, f.world);
            let haze = clamp((away - g.fog.x) / max(g.fog.y - g.fog.x, 0.0001), 0.0, 1.0);
            color = vec4<f32>(mix(color.rgb, g.fog_color.rgb * color.a, haze), color.a);
        }
    }
    return color;
}
