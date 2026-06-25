// 2-1 3
// |/ /|
// 0 4-5
const XS = array(-1., 1, -1, 1, -1, 1);
const YS = array(-1., 1, 1, 1, -1, -1);

@group(0)
@binding(0)
var color_texture: texture_storage_3d<rgba8unorm, read>;

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
    return textureLoad(color_texture, vec3<u32>(vec2<u32>(in.position.xy), in.instance_index));
}
