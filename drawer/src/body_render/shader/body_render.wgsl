struct Vertex {
    @location(0) position: vec4<f32>,
    // @location(1) color: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) pos: vec4<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.pos = view * in.position;
    out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);

    out.position = proj * out.pos;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    return in.color;
}
