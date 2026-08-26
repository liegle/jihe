struct Curve {
    thickness: f32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<storage, read> curves: array<Curve>;

@group(0)
@binding(1)
var trace_texture: texture_storage_3d<r32uint, read>;

@group(0)
@binding(2)
var distance_texture: texture_storage_2d_array<r32float, read>;

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
    let pos = vec2<i32>(in.position.xy);
    let ithickness = i32(ceil(curve.thickness));
    var is_around = false;
    for (var i = -ithickness; i <= ithickness; i++) {
        for (var j = -ithickness; j <= ithickness; j++) {
            let word = textureLoad(
                trace_texture,
                vec3<u32>(
                    vec2<u32>(pos + vec2<i32>(i, j)),
                    in.instance_index / 32
                )
            ).x;
            let v = extractBits(word, in.instance_index % 32, 1);
            is_around |= v == 1;
        }
    }
    if !is_around {
        discard;
    }
    let thickness2 = curve.thickness * curve.thickness;
    let dist = textureLoad(distance_texture, vec2<u32>(pos), in.instance_index).x;
    let dist2 = dist * dist;
    let alpha = curve.color.a * saturate(1.5 * (1 - sqrt(dist2 / thickness2)));
    return vec4<f32>(curve.color.rgb * alpha, alpha);
}
