struct Vertex {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> vm: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;

fn f4_2_f(f4: vec4<f32>) -> f32 {
    let bit_shift = vec4<f32>(1.0, 1.0 / 10.0, 1.0 / (10.0 * 10.0), 1.0 / (10.0 * 10.0 * 10.0)) * 0.9;

    return dot(f4, bit_shift);
}

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = proj * vm * in.position;
    out.pos = in.position;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(in.pos.xyz, f4_2_f(in.color));
}
