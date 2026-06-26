@group(0)
@binding(0)
var residual_texture: texture_storage_2d_array<r32float, read>;

@group(0)
@binding(1)
var trace_texture: texture_storage_3d<r32uint, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let u = textureLoad(residual_texture, vec2<u32>(id.x, id.y + 1), id.z).x;
    let d = textureLoad(residual_texture, vec2<u32>(id.x, id.y - 1), id.z).x;
    let r = textureLoad(residual_texture, vec2<u32>(id.x + 1, id.y), id.z).x;
    let l = textureLoad(residual_texture, vec2<u32>(id.x - 1, id.y), id.z).x;
    let here = textureLoad(residual_texture, id.xy, id.z).x;
    let abs_here = abs(here);
    if any(vec4<bool>(
        all(vec2<bool>(u * here <= 0, abs_here <= abs(u))),
        all(vec2<bool>(d * here <= 0, abs_here <= abs(d))),
        all(vec2<bool>(r * here <= 0, abs_here <= abs(r))),
        all(vec2<bool>(l * here <= 0, abs_here <= abs(l))),
    )) {
        textureStore(trace_texture, id, vec4<u32>(1, 0, 0, 0));
    } else {
        textureStore(trace_texture, id, vec4<u32>(0, 0, 0, 0));
    }
}
