const PI: f32 = 3.141592654;

struct Vertex {
    @location(0) position: vec4<f32>,
}

struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uniform_pos: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: mat4x4<f32>;
@group(0) @binding(1) var<uniform> proj: mat4x4<f32>;
@group(0) @binding(2) var<uniform> light_v: mat4x4<f32>;
// pos + color
@group(0) @binding(3) var view_tex: texture_2d<f32>;
// normal + color
@group(0) @binding(4) var light_color_tex: texture_2d<f32>;
@group(0) @binding(5) var light_depth_tex: texture_depth_2d;
@group(0) @binding(6) var<uniform> light_p: mat4x4<f32>;
@group(0) @binding(7) var<uniform> ratio: f32;

fn f_2_f4(f: f32) -> vec4<f32> {
    let bit_shift = vec4<f32>(1.0, 10.0, 10.0 * 10.0, 10.0 * 10.0 * 10.0);
    let bit_mask = vec4<f32>(1.0 / 10.0, 1.0 / 10.0, 1.0 / 10.0, 0.0);

    var f4 = fract(f * bit_shift);

    f4 -= f4.gbaa * bit_mask;

    return f4 / 0.9;
}

fn reverse_pt_from_mat(pt: vec4<f32>, m: mat4x4<f32>) -> vec4<f32> {
    let v = pt - m * vec4<f32>(0.0, 0.0, 0.0, 1.0);

    let ox = m * vec4<f32>(1.0, 0.0, 0.0, 0.0);
    let oy = m * vec4<f32>(0.0, 1.0, 0.0, 0.0);
    let oz = m * vec4<f32>(0.0, 0.0, 1.0, 0.0);

    return vec4<f32>(dot(v, ox), dot(v, oy), dot(v, oz), 1.0);
}

fn reverse_vec_from_mat(v: vec4<f32>, m: mat4x4<f32>) -> vec4<f32> {
    let ox = m * vec4<f32>(1.0, 0.0, 0.0, 0.0);
    let oy = m * vec4<f32>(0.0, 1.0, 0.0, 0.0);
    let oz = m * vec4<f32>(0.0, 0.0, 1.0, 0.0);

    return vec4<f32>(dot(v, ox), dot(v, oy), dot(v, oz), 0.0);
}

fn calc_light_income(normal: vec4<f32>, income: vec4<f32>, light: vec4<f32>) -> f32 {
    let o_light = normalize(reflect(light, normal));

    return (1.0 - acos(dot(o_light, income)) / PI) * 0.9;
}

fn calc_normal_income(normal: vec4<f32>, income: vec4<f32>) -> f32 {
    let ratio = (1.0 - acos(abs(dot(normal, income))) / PI);

    return ratio * ratio * 0.2;
}

@vertex
fn vs_main(in: Vertex) -> Fragment {
    var out: Fragment;

    out.position = in.position;
    out.uniform_pos = in.position.xy;

    return out;
}

@fragment
fn fs_main(in: Fragment) -> @location(0) vec4<f32> {
    let sz = 1024.0;

    let half_sz = sz * 0.5;

    var f_crd = in.uniform_pos;

    if (ratio > 1.0) {
        f_crd = vec2<f32>(in.uniform_pos.x, in.uniform_pos.y / ratio);
    } else if (ratio < 1.0) {
        f_crd = vec2<f32>(in.uniform_pos.x * ratio, in.uniform_pos.y);
    }

    let crd = vec2<i32>(i32(f_crd.x * half_sz + half_sz), i32(-f_crd.y * half_sz + half_sz));

    let i_light_in_view = normalize(view * reverse_vec_from_mat(vec4<f32>(0.0, 0.0, -1.0, 0.0), light_v));
    var lightness = 0.08;

    let pos_vc = textureLoad(view_tex, crd, 0);

    let cur_pos = vec4<f32>(pos_vc.xyz, 1.0);
    let color_in_view = f_2_f4(pos_vc.w);

    var cur_pos_in_light_proj = light_p * light_v * cur_pos;

    cur_pos_in_light_proj /= cur_pos_in_light_proj.w;

    let crd_in_light = vec2<u32>((vec2<f32>(cur_pos_in_light_proj.x, -cur_pos_in_light_proj.y) * 0.5 + 0.5) * sz);
    let cur_depth_in_light_proj = cur_pos_in_light_proj.z;

    let std_depth_in_light_proj = textureLoad(light_depth_tex, crd_in_light, 0);
    let nml_lc = textureLoad(light_color_tex, crd_in_light, 0);

    let normal = vec4<f32>(nml_lc.xyz, 0.0);

    let normal_in_view = normalize(view * normal);

    let cur_pos_in_view = view * cur_pos;

    let income_in_view = normalize(-vec4<f32>(cur_pos_in_view.xyz, 0.0));

    if (abs(cur_depth_in_light_proj - std_depth_in_light_proj) < 0.0035) {
        // let color_in_light = f_2_f4(nml_lc.w);

        lightness += calc_light_income(normal_in_view, income_in_view, i_light_in_view);
    }

    lightness += calc_normal_income(normal_in_view, income_in_view);

    return vec4<f32>(color_in_view.rgb * lightness, color_in_view.a);
}
