struct Vertex {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> mv: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;
@group(0) @binding(2) var normal_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var depth_tex: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(4) var tex_sampler: sampler;

fn f_2_f4(f: f32) -> vec4<f32> {
    let bit_shift = vec4<f32>(1.0, 256.0, 256.0 * 256.0, 256.0 * 256.0 * 256.0);
    let bit_mask = vec4<f32>(1.0/256.0, 1.0/256.0, 1.0/256.0, 0.0);

    var f4 = fract(f * bit_shift);

    f4 -= f4.gbaa * bit_mask;

    return f4;
}

fn f4_2_f(rgbaDepth: vec4<f32>) -> f32 {
    let bitShift = vec4<f32>(1.0, 1.0/256.0, 1.0/(256.0*256.0), 1.0/(256.0*256.0*256.0));
    return dot(rgbaDepth, bitShift);
}

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.pos = mv * in.position;
    out.color = in.color;
    out.normal = mv * in.normal;

    out.position = proj * out.pos;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let crd = vec2<u32>(in.position.xy);

    let o_depth = 1.0 - f4_2_f(textureLoad(depth_tex, crd));
    let c_depth = - in.pos.z / 500.0;

    if c_depth >= o_depth {
        discard;
    }

    textureStore(normal_tex, crd, (normalize(in.normal) + 1.0) * 0.5);
    textureStore(depth_tex, crd, f_2_f4(1.0 - c_depth));

    return in.color;
}
