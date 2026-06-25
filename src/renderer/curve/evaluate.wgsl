struct Camera {
    pixel_delta: f32,
    pos: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> camera: Camera;

@group(0)
@binding(1)
var residual_texture: texture_storage_2d<r32sint, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let dst_size = textureDimensions(residual_texture);
    let offset = f(
        f32(i32(id.x) - i32(dst_size.x) / 2) * camera.pixel_delta + camera.pos.x,
        f32(i32(dst_size.y) / 2 - i32(id.y)) * camera.pixel_delta + camera.pos.y,
    );
    var v: i32;
    if offset < -0. {
        v = -1;
    } else if offset > 0. {
        v = 1;
    } else {
        v = 0;
    }
    textureStore(residual_texture, id.xy, vec4<i32>(v, 0, 0, 0));
}
