struct Vertex {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> mvp: mat4x4<f32>;

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = mvp * in.position;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    // return in.color;
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
