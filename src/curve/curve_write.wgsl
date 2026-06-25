// 2-1 3
// |/ /|
// 0 4-5
const XS = array(-1., 1, -1, 1, -1, 1);
const YS = array(-1., 1, 1, 1, -1, -1);

@group(0)
@binding(0)
var tex: texture_storage_3d<rgba8unorm, read>;

struct V2f {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
}

@vertex
fn vs(@builtin(vertex_index) in: u32, @builtin(instance_index) instance_index: u32) -> V2f {
    return V2f(vec4<f32>(XS[in], YS[in], 0, 1), instance_index);
}

@fragment
fn fs(in: V2f) -> @location(0) vec4<f32> {
    return textureLoad(tex, vec3<u32>(vec2<u32>(in.position.xy), in.instance_index));
}
