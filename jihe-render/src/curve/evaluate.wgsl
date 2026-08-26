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
var distance_texture: texture_storage_2d<r32float, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let dst_size = textureDimensions(distance_texture);
    let pos = vec2<f32>(
        f32(i32(id.x) - i32(dst_size.x) / 2) * camera.scale + camera.pos.x,
        f32(i32(dst_size.y) / 2 - i32(id.y)) * camera.scale + camera.pos.y,
    );
    let residual = f(pos.x, pos.y);
    let derivative_x = dfx(pos.x);
    let dx2 = derivative_x * derivative_x;
    let derivative_y = dfy(pos.y);
    let dy2 = derivative_y * derivative_y;
    let scale2 = camera.scale * camera.scale;
    textureStore(
        distance_texture,
        id.xy,
        vec4<f32>(residual * abs(residual) / (dx2 + dy2) / scale2, 0, 0, 0)
    );
}

fn safeLog(x: f32) -> f32 {
    return select(F32_MIN, log(x), x > 0);
}

fn safeLog2(x: f32) -> f32 {
    return select(F32_MIN, log2(x), x > 0);
}
