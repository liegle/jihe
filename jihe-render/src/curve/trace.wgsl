@group(0)
@binding(0)
var distance_texture: texture_storage_2d_array<r32float, read>;

@group(0)
@binding(1)
var trace_texture: texture_storage_3d<r32uint, read_write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let u = textureLoad(distance_texture, vec2<u32>(id.x, id.y + 1), id.z).x;
    let r = textureLoad(distance_texture, vec2<u32>(id.x + 1, id.y), id.z).x;
    let here = textureLoad(distance_texture, id.xy, id.z).x;
    let v = select(0u, 1u,
        (u < 0. && here >= 0) || (u > 0. && here <= 0.) ||
        (r < 0. && here >= 0) || (r > 0. && here <= 0.)
    );

    let pos = vec3<u32>(id.xy, id.z / 32);
    var word = textureLoad(trace_texture, pos).x;
    word = insertBits(word, v, id.z % 32, 1);
    textureStore(trace_texture, pos, vec4<u32>(word, 0, 0, 0));
}

fn differentSign(l: f32, r: f32) -> bool {
    return (l <= 0. || r <= 0.) && (l >= 0. || r >= 0.);
}
