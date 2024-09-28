struct Ray {
    position: vec2<f32>,
    direction: vec2<f32>,
    color: vec3<f32>,
};

struct Line {
    sp: vec2<f32>,
    ep: vec2<f32>,
    light: f32,
    color: vec3<f32>,
    roughness: f32,
    seed: f32,
};

const PI: f32 = 3.141592654;
const MIN_DISTANCE: f32 = 1e-6;

@group(0) @binding(0) var<storage, read_write> texture: array<u32>;
@group(0) @binding(1) var<uniform> size: vec2<u32>;
@group(0) @binding(2) var<storage, read> line_v: array<array<vec4<f32>, 3>>;
@group(0) @binding(3) var<uniform> watcher: vec4<f32>;

@compute @workgroup_size(180, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let angle = PI / 1800.0 * f32(global_id.x);
    var ray: Ray;
    ray.position = watcher.xy;
    ray.direction = vec2<f32>(cos(angle), sin(angle));
    ray.color = vec3<f32>(1.0, 1.0, 1.0);
    let r_ray = ray_trace(ray);
    if r_ray.direction.x != 0.0 {
        return;
    }
    draw_point(r_ray);
}

fn draw_point(ray: Ray) {
    var position = ray.position - watcher.xy + watcher.zw;
    position.x *= f32(size.y) / f32(size.x);
    if abs(position.x) >= 1.0 || abs(position.y) >= 1.0 {
        return;
    }
    let uv: vec2<u32> = pos_to_uv(position, size);
    let o_c = u32_to_c4(texture[uv.y + uv.x * size.y]);
    let cnt = o_c.a * 255.0;
    let n_c = (o_c.rgb * cnt + ray.color) / (cnt + 1.0);
    texture[uv.y + uv.x * size.y] = c4_to_u32(vec4<f32>(n_c, (cnt + 1.0) / 255.0));
}

fn c4_to_u32(c4: vec4<f32>) -> u32 {
    let texel = vec4<u32>(c4 * 255.0);
    return texel.r | (texel.g << 8) | (texel.b << 16) | (texel.a << 24);
}

fn u32_to_c4(u: u32) -> vec4<f32> {
    return vec4<f32>(f32(u & 0xff), f32((u >> 8) & 0xff), f32((u >> 16) & 0xff), f32((u >> 24) & 0xff)) / 255.0;
}

fn pos_to_uv(pos: vec2<f32>, size: vec2<u32>) -> vec2<u32> {
    let uvf: vec2<f32> = (pos + vec2<f32>(1.0, -1.0)) * vec2<f32>(size >> vec2<u32>(1, 1));
    return vec2<u32>(u32(uvf.x), u32(-uvf.y));
}

fn randf(seed: f32) -> f32 {
    return fract(sin(seed)*100000.0);
}

fn ray_trace(ray: Ray) -> Ray {
    let f_ray = ray_trace_first(ray);
    if f_ray.direction.x <= 0.0 {
        return f_ray;
    }
    var n_ray = ray_trace_to_light(f_ray, 15);
    n_ray.position = f_ray.position;
    return n_ray;
}

fn ray_trace_to_light(ray: Ray, life: i32) -> Ray {
    var r_ray: Ray = ray;
    for (var i = 0; i < life; i++) {
        r_ray = ray_trace_first(r_ray);
        if r_ray.direction.x <= 0.0  {
            break;
        }
    }
    return r_ray;
}

fn ray_trace_first(ray: Ray) -> Ray {
    var r_line: Line;
    var r_distance: f32 = -1.0;
    let len = arrayLength(&line_v);
    var line: Line;
    for (var i: u32 = 0; i < len; i++) {
        let item = line_v[i];
        line.sp = item[0].xy;
        line.ep = item[0].zw;
        line.light = item[1].x;
        line.color = item[1].yzw;
        line.roughness = item[2].x;
        line.seed = item[2].y;
        let oa = line.sp - ray.position;
        let ob = line.ep - ray.position;
        let d = ray.direction.x * (oa.y - ob.y) - ray.direction.y * (oa.x - ob.x);
        if d == 0.0 {
            continue;
        }
        let lambda = (ray.direction.y * ob.x - ray.direction.x * ob.y) / d;
        if lambda < 0.0 || lambda > 1.0 {
            continue;
        }
        let oh = lambda * oa + (1.0 - lambda) * ob;
        let n_distance = dot(oh, ray.direction);
        if n_distance <= MIN_DISTANCE {
            continue;
        }
        if r_distance == -1.0 || r_distance > n_distance {
            r_distance = n_distance;
            r_line = line;
        }
    }
    var r_ray: Ray;
    if r_distance < 0.0 {
        r_ray.direction.x = -1.0;
        return r_ray;
    }
    r_ray.position = ray.position + ray.direction * r_distance;
    if r_line.light > 0.0 {
        r_ray.color = r_line.color * ray.color * r_line.light;
        return r_ray;
    }

    r_ray.direction = get_reflection(r_line, ray.direction, r_line.seed * length(r_ray.position - r_line.sp));
    r_ray.color = r_line.color * ray.color;
    return r_ray;
}

fn get_reflection(line: Line, direction: vec2<f32>, seed: f32) -> vec2<f32> {
    let ab = normalize(line.ep - line.sp);
    let reflection = -reflect(direction, ab);

    let angle_ab = atan2(ab.y, ab.x);
    let angle_r = atan2(reflection.y, reflection.x) - angle_ab;
    let diff_angle = PI * (randf(seed) - 0.5) * line.roughness;
    let n_angle = angle_r + diff_angle;
    if sin(angle_r) * sin(n_angle) <= 0.0 {
        return vec2<f32>(-1.0, 0.0);
    }
    return vec2<f32>(cos(angle_ab + n_angle), sin(angle_ab + n_angle));
}
