// 2-1 3
// |/ /|
// 0 4-5
const XS = array(-1., 1, -1, 1, -1, 1);
const YS = array(-1., 1, 1, 1, -1, -1);

struct Global {
    pixel_delta: f32,
    pos: vec2<f32>,
    half_tex_size: vec2<f32>,
}

@group(0)
@binding(0)
var<uniform> global: Global;

struct Curve {
    thickness: i32,
    color: vec4<f32>,
}

@group(0)
@binding(1)
var<uniform> curve: Curve;

@vertex
fn vs_main(@builtin(vertex_index) in: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(XS[in], YS[in], 0, 1);
}

@fragment
fn fs_main(@builtin(position) in: vec4<f32>) -> @location(0) vec4<f32> {
    let here = vec2<f32>(
        (in.x - global.half_tex_size.x) * global.pixel_delta + global.pos.x,
        (global.half_tex_size.y - in.y) * global.pixel_delta + global.pos.y,
    );

    var negative = false;
    var positive = false;
    for (var i = -curve.thickness; i <= curve.thickness; i++) {
        for (var j = -curve.thickness; j <= curve.thickness; j++) {
            if i * i + j * j < curve.thickness * curve.thickness {
                let there = here + vec2<f32>(f32(i) * global.pixel_delta, f32(j) * global.pixel_delta);
                let val = f(there.x, there.y);
                if val < -0. {
                    negative = true;
                } else if val > 0. {
                    positive = true;
                }
            }
        }
    }

    if negative && positive {
        return curve.color;
    } else {
        return vec4(0., 0, 0, 0);
    }
}

fn f(x: f32, y: f32) -> f32 {
    return pow(x, 5) - y;
}
