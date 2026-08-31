@group(0)
@binding(0)
var intersection_texture: texture_storage_2d<rgba8unorm, read>;

@group(0)
@binding(1)
var segment_texture: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let here = textureLoad(intersection_texture, id.xy);
    let l = here.yw;
    let t = here.xz;
    let r = textureLoad(intersection_texture, id.xy + vec2<u32>(1, 0)).yw;
    let b = textureLoad(intersection_texture, id.xy + vec2<u32>(0, 1)).xz;
    let points = array<vec3<f32>, 4>(
        vec3<f32>(vec2<f32>(0, f32(l.x)), l.y),
        vec3<f32>(vec2<f32>(f32(t.x), 0), t.y),
        vec3<f32>(vec2<f32>(1, f32(r.x)), r.y),
        vec3<f32>(vec2<f32>(f32(b.x), 1), b.y),
    );

    var pq = vec4<f32>(0, 0, 0, 0);
    for (var i = 0u; i < 3; i += 1) {
        for (var j = i + 1; j < 4; j += 1) {
            pq = select(
                pq,
                vec4<f32>(points[i].xy, points[j].xy),
                points[i].z == 0 && points[j].z == 0
            );
        }
    }
    textureStore(segment_texture, id.xy, pq);
}
