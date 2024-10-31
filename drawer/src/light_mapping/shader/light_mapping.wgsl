struct Vertex {
    @location(0) position: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> light: mat4x4<f32>;

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = light * in.position;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
