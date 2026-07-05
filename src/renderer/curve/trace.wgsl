@group(0)
@binding(0)
var residual_texture: texture_storage_2d_array<r32float, read>;

@group(0)
@binding(1)
var trace_texture: texture_storage_3d<r32uint, read_write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let u = textureLoad(residual_texture, vec2<u32>(id.x, id.y + 1), id.z).x;
    let d = textureLoad(residual_texture, vec2<u32>(id.x, id.y - 1), id.z).x;
    let r = textureLoad(residual_texture, vec2<u32>(id.x + 1, id.y), id.z).x;
    let l = textureLoad(residual_texture, vec2<u32>(id.x - 1, id.y), id.z).x;
    let here = textureLoad(residual_texture, id.xy, id.z).x;
    let abs_here = abs(here);
    let v = select(0u, 1u,
        (differentSign(u, here) && abs_here <= abs(u)) ||
        (differentSign(d, here) && abs_here <= abs(d)) ||
        (differentSign(r, here) && abs_here <= abs(r)) ||
        (differentSign(l, here) && abs_here <= abs(l))
    );

    let pos = vec3<u32>(id.xy, id.z / 32);
    var word = textureLoad(trace_texture, pos).x;
    word = insertBits(word, v, id.z % 32, 1);
    textureStore(trace_texture, pos, vec4<u32>(word, 0, 0, 0));
}

fn differentSign(l: f32, r: f32) -> bool {
    return (l <= 0. || r <= 0.) && (l >= 0. || r >= 0.);
}
