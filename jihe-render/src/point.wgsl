@group(0)
@binding(0)
var<uniform> half_size: vec2<f32>;

struct Point {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) size: f32,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) relative: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// 2 2-3
// |\ \|
// 0-1 1
@vertex
fn vs(in: Point) -> VertexOut {
    var out: VertexOut;
    out.relative = vec2<f32>(
        f32(i32((in.vertex_index & 1u) << 1) - 1),
        f32(i32(in.vertex_index & 2u) - 1),
    );
    out.color = in.color;
    let size = ceil(in.size);
    out.pos = vec4<f32>(in.pos + out.relative * size / half_size, 0, 1);
    out.relative *= size / in.size;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let alpha = in.color.a * saturate(4 * (1 - length(in.relative)));
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
