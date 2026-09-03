const BINARY_ITERATION: u32 = 4;
const F32_MAX: f32 = 3.40282347e+38;
const F32_MIN: f32 = -F32_MAX;

struct Camera {
    scale: f32,
    pos: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

@group(0)
@binding(1)
var intersection_texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let half_size = (textureDimensions(intersection_texture) - vec2<u32>(1, 1)) / 2;
    let lrtb = vec4<f32>(
        (f32(id.x) - f32(half_size.x)),
        (f32(id.x) + 1 - f32(half_size.x)),
        -(f32(id.y) - f32(half_size.y)),
        -(f32(id.y) + 1 - f32(half_size.y)),
    ) * camera.scale + camera.pos.xxyy;
    let t = binary(vec2<f32>(lrtb.x, lrtb.z), vec2<f32>(lrtb.y, lrtb.z));
    let l = binary(vec2<f32>(lrtb.x, lrtb.z), vec2<f32>(lrtb.x, lrtb.w));
    textureStore(intersection_texture, id.xy, vec4<f32>(t.x, t.y, l.x, l.y));
}

fn binary(pa: vec2<f32>, pb: vec2<f32>) -> vec2<f32> {
    let fa = sign(f(pa.x, pa.y));
    let fb = sign(f(pb.x, pb.y));
    if fa == 0 { return vec2<f32>(0, 0); }
    if fb == 0 { return vec2<f32>(1, 0); }
    if fa == fb { return vec2<f32>(0, 1); }

    var range = vec4<f32>(0, 1, fa, fb);
    var t: f32;
    var pt: vec2<f32>;
    var ft: f32;

    for (var i = 0u; i < BINARY_ITERATION; i += 1) {
        t = (range.x + range.y) / 2;
        pt = mix(pa, pb, t);
        ft = sign(f(pt.x, pt.y));
        if ft == 0 { return vec2<f32>(t, 0); }
        range = select(
            vec4<f32>(range.x, t, range.z, ft),
            vec4<f32>(t, range.y, ft, range.w),
            ft == range.z
        );
    }

    return vec2<f32>((range.x + range.y) / 2, 0);
}

fn safeLog(x: f32) -> f32 {
    return select(F32_MIN, log(x), x > 0);
}

fn safeLog2(x: f32) -> f32 {
    return select(F32_MIN, log2(x), x > 0);
}

// fn f(x: f32, y: f32) -> f32 { return 0; }
