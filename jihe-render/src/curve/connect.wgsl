struct Layer {
    value: u32,
}

struct Curve {
    thickness: f32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var intersection_texture: texture_storage_2d<rgba8unorm, read>;

@group(0)
@binding(1)
var segment_texture: texture_storage_2d<rgba8unorm, write>;

@group(0)
@binding(2)
var mark_texture: texture_storage_2d<r32uint, write>;

@group(0)
@binding(3)
var<storage, read> curves: array<Curve>;

var<immediate> layer: Layer;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let here = textureLoad(intersection_texture, id.xy);
    let t = here.xy;
    let l = here.zw;
    let b = textureLoad(intersection_texture, id.xy + vec2<u32>(0, 1)).xy;
    let r = textureLoad(intersection_texture, id.xy + vec2<u32>(1, 0)).zw;

    var pq = vec4<f32>(0.5, 0.5, 0.5, 0.5);
    if t.y + l.y + b.y + r.y > 2 {
        textureStore(segment_texture, id.xy, pq);
        return;
    }

    let points = array<vec3<f32>, 4>(
        vec3<f32>(0, f32(l.x), l.y),
        vec3<f32>(f32(t.x), 0, t.y),
        vec3<f32>(1, f32(r.x), r.y),
        vec3<f32>(f32(b.x), 1, b.y),
    );

    var len = 0.;
    for (var i = 0u; i < 3; i++) {
        for (var j = i + 1; j < 4; j++) {
            let p = points[i];
            let q = points[j];
            if p.z == 0 && q.z == 0 {
                let cur_pq = vec4<f32>(p.xy, q.xy);
                let p_q = cur_pq.xy - cur_pq.zw;
                let cur_len = dot(p_q, p_q);
                if cur_len > len {
                    len = cur_len;
                    pq = cur_pq;
                }
            }
        }
    }
    textureStore(segment_texture, id.xy, pq);

    if all(pq == vec4<f32>(0.5, 0.5, 0.5, 0.5)) {
        return;
    }

    let span = u32(ceil(curves[layer.value].thickness));
    for (var i = id.x - span; i <= id.x + span; i++) {
        for (var j = id.y - span; j <= id.y + span; j++) {
            textureStore(mark_texture, vec2<u32>(i, j), vec4<u32>(1, 0, 0, 0));
        }
    }
}
