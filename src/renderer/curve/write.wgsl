struct Curve {
    thickness: u32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<storage, read> curves: array<Curve>;

@group(0)
@binding(1)
var trace_texture: texture_storage_3d<r32uint, read>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
}

// 2 2-3
// |\ \|
// 0-1 1
@vertex
fn vs(@builtin(vertex_index) vertex_index: u32, @builtin(instance_index) instance_index: u32) -> VertexOut {
    return VertexOut(vec4<f32>(
        f32(i32((vertex_index & 1u) << 1) - 1),
        f32(i32(vertex_index & 2u) - 1),
        0, 1
    ), instance_index);
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let thickness = i32(curves[in.instance_index].thickness);
    let thickness2 = thickness * thickness;
    let pos = vec2<i32>(in.position.xy);

    var least_dist2 = thickness2 + 1;
    // TODO: Still looks strange
    for (var i = -thickness; i <= thickness; i++) {
        for (var j = -thickness; j <= thickness; j++) {
            let word = textureLoad(trace_texture,
                vec3<u32>(vec2<u32>(pos + vec2<i32>(i, j)), in.instance_index / 32)).x;
            let v = extractBits(word, in.instance_index % 32, 1);
            let dist2 = i * i + j * j;
            if dist2 <= thickness2 && dist2 < least_dist2 && v == 1 {
                least_dist2 = dist2;
            }
        }
    }
    if least_dist2 > thickness2 {
        discard;
    }
    return curves[in.instance_index].color;
}
