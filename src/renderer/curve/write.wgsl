struct CurveConfig {
    thickness: i32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<storage, read> curve_configs: array<CurveConfig>;

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
    if textureLoad(trace_texture, vec3<u32>(vec2<u32>(in.position.xy), in.instance_index)).x == 0 {
        discard;
    }
    return curve_configs[in.instance_index].color;
}
