struct LineIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct LineOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> watcher: vec4<f32>;
@group(0) @binding(1) var<uniform> size: vec2<u32>;

@vertex
fn vs_main(in: LineIn) -> LineOut {
    var out: LineOut;
    out.position = vec4<f32>(in.position - (watcher.xy - watcher.zw), 0.0, 1.0);
    out.position.x *= f32(size.y) / f32(size.x);
    out.color = vec4<f32>(in.color, 1.0);
    return out;
}

@fragment
fn fs_main(in: LineOut) -> @location(0) vec4<f32> {
    return in.color;
}
