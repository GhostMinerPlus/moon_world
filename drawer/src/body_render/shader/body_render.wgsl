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
@group(0) @binding(5) var light_tex: texture_2d<f32>;
@group(0) @binding(6) var tex_sampler: sampler;

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = in.position;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let color = textureSample(view_tex, tex_sampler, in.position.xy / 1024.0);

    return color;
}
