struct Vertex {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) noraml: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) noraml: vec4<f32>,
}

@group(0) @binding(0) var<uniform> mv: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;
@group(0) @binding(2) var normal_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var tex_sampler: sampler;

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = mv * proj * in.position;
    out.color = in.color;
    out.noraml = mv * in.noraml;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    textureStore(normal_tex, vec2<i32>(in.position.xy), in.noraml);

    return in.color;
}
