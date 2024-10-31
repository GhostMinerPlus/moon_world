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
@group(0) @binding(4) var depth_tex: texture_depth_2d;
@group(0) @binding(5) var light_tex: texture_2d<f32>;

fn f_2_f4(f: f32) -> vec4<f32> {
    let bit_shift = vec4<f32>(1.0, 10.0, 10.0 * 10.0, 10.0 * 10.0 * 10.0);
    let bit_mask = vec4<f32>(1.0 / 10.0, 1.0 / 10.0, 1.0 / 10.0, 0.0);

    var f4 = fract(f * bit_shift);

    f4 -= f4.gbaa * bit_mask;

    return f4 / 0.9;
}

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = in.position;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let crd = vec2<u32>(in.position.xy);
    let i_light = normalize(vec4<f32>(0.0, -1.0, -1.0, 0.0));
    let income = normalize(-vec4<f32>(in.position.x / 1024.0 * 2.0 - 1.0, 1.0 - 2.0 * in.position.y / 1024.0, -0.1, 0.0));

    let view = textureLoad(view_tex, crd, 0);
    let depth = textureLoad(depth_tex, crd, 0);

    let color = f_2_f4(view.w);
    let normal = vec4<f32>(view.xyz, 0.0);

    let r_light = normalize(reflect(i_light, normal));

    let lightness = dot(r_light, income);

    return vec4<f32>(color.rgb * lightness, color.a);
}
