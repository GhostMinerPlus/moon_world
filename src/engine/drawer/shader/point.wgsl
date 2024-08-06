struct PointInput {
    @location(0) position: vec2<f32>,
}

@vertex
fn vs_main(in: PointInput) -> @builtin(position) vec4<f32> {
    return vec4<f32>(in.position, 0.0, 1.0);
}

@group(0) @binding(0) var<storage, read_write> texture: array<u32>;
@group(0) @binding(1) var<uniform> size: vec2<u32>;

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<i32>(position.xy);
    let c1 = get_c3(uv + vec2<i32>(-1, 1));
    let c2 = get_c3(uv + vec2<i32>(-1, 0));
    let c3 = get_c3(uv + vec2<i32>(-1, -1));
    let c4 = get_c3(uv + vec2<i32>(0, -1));
    let c5 = get_c3(uv + vec2<i32>(1, -1));
    let c6 = get_c3(uv + vec2<i32>(1, 0));
    let c7 = get_c3(uv + vec2<i32>(1, 1));
    let c8 = get_c3(uv + vec2<i32>(0, 1));
    let c9 = get_c3(uv);
    let c = (c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8) / 8.0 * 0.6 + c9 * 0.4;
    return vec4<f32>(c, 1.0);
}

fn get_c3(uv: vec2<i32>) -> vec3<f32> {
    let c32: u32 = texture[uv.y + uv.x * i32(size.y)];
    return vec3<f32>(f32(c32 & 0xff), f32((c32 >> 8) & 0xff), f32((c32 >> 16) & 0xff)) / 255.0;
}
