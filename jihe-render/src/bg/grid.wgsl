struct Lines {
    begin: f32,
    spacing: f32,
    ends: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> lines: Lines;

@group(0)
@binding(1)
var<uniform> color: vec3<f32>;

@vertex
fn vs_hori(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        lines.ends[vertex_index],
        lines.begin + lines.spacing * f32(instance_index),
        0 ,1
    );
}

@vertex
fn vs_vert(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        lines.begin + lines.spacing * f32(instance_index),
        lines.ends[vertex_index],
        0 ,1
    );
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1);
}
