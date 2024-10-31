struct Vertex {
    @location(0) position: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;
@group(0) @binding(2) var<uniform> light: mat4x4<f32>;
@group(0) @binding(3) var view_tex: texture_2d<f32>;
@group(0) @binding(4) var view_depth_tex: texture_2d<f32>;
@group(0) @binding(5) var view_normal_tex: texture_2d<f32>;
@group(0) @binding(6) var light_tex: texture_2d<f32>;
@group(0) @binding(7) var tex_sampler: sampler;

fn f4_2_f(rgbaDepth: vec4<f32>) -> f32 {
    let bitShift = vec4<f32>(1.0, 1.0/256.0, 1.0/(256.0*256.0), 1.0/(256.0*256.0*256.0));
    return dot(rgbaDepth, bitShift);
}

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = in.position;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let crd = in.position.xy / 1024.0;
    let i_light = normalize(vec4<f32>(0.0, -1.0, -1.0, 0.0));

    let income = normalize(-vec4<f32>(crd.x * 2.0 - 1.0, 1.0 - 2.0 * crd.y, -0.1, 0.0));

    let color = textureSample(view_tex, tex_sampler, crd);
    // let depth = (1.0 - f4_2_f(textureSample(view_depth_tex, tex_sampler, crd))) * 500.0;
    let normal = textureLoad(view_normal_tex, vec2<u32>(in.position.xy), 0) * 2.0 - 1.0;

    // let pos = vec4<f32>(0.0, 0.0, 0.0, 1.0) - depth * income;
    let r_light = normalize(reflect(i_light, normal));

    let lightness = dot(r_light, income);

    return vec4<f32>(color.rgb * lightness, color.a);
}
