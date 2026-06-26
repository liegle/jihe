// 2-1 3
// |/ /|
// 0 4-5
const XS = array(-1., 1, -1, 1, -1, 1);
const YS = array(-1., 1, 1, 1, -1, -1);

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

@vertex
fn vs(@builtin(vertex_index) in: u32, @builtin(instance_index) instance_index: u32) -> VertexOut {
    return VertexOut(vec4<f32>(XS[in], YS[in], 0, 1), instance_index);
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    if textureLoad(trace_texture, vec3<u32>(vec2<u32>(in.position.xy), in.instance_index)).x == 0 {
        discard;
    }
    return curve_configs[in.instance_index].color;
}
