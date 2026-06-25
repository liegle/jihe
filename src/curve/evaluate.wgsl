struct Config {
    pixel_delta: f32,
    pos: vec2<f32>,
    half_tex_size: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> config: Config;

@group(0)
@binding(1)
var residual_texture: texture_storage_2d<r32sint, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let offset = f(
        (f32(id.x) - config.half_tex_size.x) * config.pixel_delta + config.pos.x,
        (config.half_tex_size.y - f32(id.y)) * config.pixel_delta + config.pos.y,
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
