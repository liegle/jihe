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
    @builtin(position) position: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) size: f32,
    @location(2) color: vec4<f32>,
}

// 2 2-3
// |\ \|
// 0-1 1
@vertex
fn vs(in: Point) -> VertexOut {
    var out: VertexOut;
    let relative = vec2<f32>(
        f32(i32((in.vertex_index & 1u) << 1) - 1),
        f32(i32(in.vertex_index & 2u) - 1),
    );
    let size = ceil(in.size) + 1;
    out.position = vec4<f32>((in.pos + relative * size) / half_size, 0, 1);
    out.pos = vec2<f32>(in.pos.x + 0.5, -in.pos.y - 0.5) + half_size;
    out.size = in.size;
    out.color = in.color;
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    let alpha = in.color.a * saturate(3 * (1 - distance(in.pos, in.position.xy) / in.size));
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
