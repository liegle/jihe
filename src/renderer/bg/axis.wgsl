@group(0)
@binding(0)
var<uniform> color: vec3<f32>;

@vertex
fn vs(@location(0) in: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(in, 0, 1);
}

@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(color, 1.);
}
