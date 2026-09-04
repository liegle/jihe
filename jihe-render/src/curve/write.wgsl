struct Curve {
    thickness: f32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<storage, read> curves: array<Curve>;

@group(0)
@binding(1)
var segment_texture: texture_storage_3d<rgba8unorm, read>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
}

// 2 2-3
// |\ \|
// 0-1 1
@vertex
fn vs(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOut {
    return VertexOut(vec4<f32>(
        f32(i32((vertex_index & 1u) << 1) - 1),
        f32(i32(vertex_index & 2u) - 1),
        0, 1
    ), instance_index);
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let curve = curves[in.instance_index];

    let thickness2 = curve.thickness * curve.thickness;
    let pos = vec2<i32>(in.position.xy);

    let ithickness = i32(ceil(curve.thickness));
    var least_dist2 = thickness2;
    for (var i = -ithickness; i <= ithickness; i++) {
        for (var j = -ithickness; j <= ithickness; j++) {
            let corner = vec2<u32>(pos + vec2<i32>(i, j));
            let pq = textureLoad(
                segment_texture,
                vec3<u32>(corner, in.instance_index)
            );
            least_dist2 = min(
                least_dist2,
                dist2(vec2<f32>(pos), vec2<f32>(corner), pq)
            );
        }
    }
    if least_dist2 >= thickness2 {
        discard;
    }
    let alpha = curve.color.a * saturate(1.5 * (1 - sqrt(least_dist2 / thickness2)));
    return vec4<f32>(curve.color.rgb * alpha, alpha);
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
