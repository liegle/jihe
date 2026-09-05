struct Curve {
    thickness: f32,
    color: vec4<f32>,
}

struct Layer {
    value: u32,
}

@group(0)
@binding(0)
var segment_texture: texture_storage_2d<rgba8unorm, read>;

@group(0)
@binding(1)
var mark_texture: texture_storage_2d<r32uint, read>;

@group(0)
@binding(2)
var curve_texture: texture_storage_3d<rgba8unorm, write>;

@group(0)
@binding(3)
var<storage, read> curves: array<Curve>;

var<immediate> layer: Layer;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let marked = textureLoad(mark_texture, id.xy).x;
    if marked != 1 {
        textureStore(curve_texture, vec3<u32>(id.xy, layer.value), vec4<f32>(0, 0, 0, 0));
        return;
    }

    let curve = curves[layer.value];
    let here = vec2<f32>(id.xy) + vec2<f32>(0.5, 0.5);
    let thickness2 = curve.thickness * curve.thickness;
    let span = u32(ceil(curve.thickness));
    var least_dist2 = thickness2;
    for (var i = id.x - span; i <= id.x + span; i++) {
        for (var j = id.y - span; j <= id.y + span; j++) {
            let corner = vec2<u32>(i, j);
            let pq = textureLoad(segment_texture, corner);
            least_dist2 = min(least_dist2, dist2(here, vec2<f32>(corner), pq));
        }
    }
    if least_dist2 >= thickness2 {
        textureStore(curve_texture, vec3<u32>(id.xy, layer.value), vec4<f32>(0, 0, 0, 0));
        return;
    }
    let alpha = curve.color.a * saturate(1.5 * (1 - sqrt(least_dist2 / thickness2)));
    textureStore(curve_texture, vec3<u32>(id.xy, layer.value), vec4<f32>(curve.color.rgb * alpha, alpha));
}

fn dist2(here: vec2<f32>, corner: vec2<f32>, pq: vec4<f32>) -> f32 {
    let p = corner + pq.xy;
    let q = corner + pq.zw;
    let p_q = p - q;
    let p_a = here - p;
    let q_a = here - q;

    let p_q_2 = dot(p_q, p_q);
    let a_p_q = dot(p_a, -p_q);
    let a_q_p = dot(q_a, p_q);

    let crozz = p_q.x * p_a.y - p_q.y * p_a.x;
    let height2 = crozz * crozz / p_q_2;

    return select(
        select(
            height2,
            dot(q_a, q_a),
            a_q_p < 0,
        ),
        dot(p_a, p_a),
        a_p_q < 0,
    );
}
