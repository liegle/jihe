// 2-1 3
// |/ /|
// 0 4-5
const XS = array(-1., 1, -1, 1, -1, 1);
const YS = array(-1., 1, 1, 1, -1, -1);

struct Curve {
    thickness: i32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<uniform> curve: Curve;

@group(0)
@binding(1)
var tex: texture_storage_2d<r32sint, read>;

@vertex
fn vs(@builtin(vertex_index) in: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(XS[in], YS[in], 0, 1);
}

@fragment
fn fs(@builtin(position) in: vec4<f32>) -> @location(0) vec4<f32> {
    let id = vec2<i32>(in.xy);
    var negative_count = 0;
    var positive_count = 0;
    for (var i = -curve.thickness; i <= curve.thickness; i++) {
        for (var j = -curve.thickness; j <= curve.thickness; j++) {
            if i * i + j * j < curve.thickness * curve.thickness {
                let v = textureLoad(tex, id + vec2<i32>(i, j));
                if v.x == -1 {
                    negative_count++;
                } else if v.x == 1 {
                    positive_count++;
                }
            }
        }
    }
    if negative_count == 0 || positive_count == 0 {
        discard;
    }
    return curve.color * count_to_alpha(negative_count, positive_count);
}

fn count_to_alpha(negative_count: i32, positive_count: i32) -> f32 {
    let t = 1. - f32(abs(negative_count - positive_count)) / f32(negative_count + positive_count);
    return saturate(t * t * 4.);
}
