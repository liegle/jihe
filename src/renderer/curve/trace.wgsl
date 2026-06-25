struct CurveConfig {
    thickness: i32,
    color: vec4<f32>,
}

@group(0)
@binding(0)
var<storage, read> curve_configs: array<CurveConfig>;

@group(0)
@binding(1)
var residual_texture: texture_storage_2d_array<r32sint, read>;

@group(0)
@binding(2)
var color_texture: texture_storage_3d<rgba8unorm, write>;

@compute
@workgroup_size(16, 16, 1)
fn cs(@builtin(global_invocation_id) id: vec3<u32>) {
    let curve_config = curve_configs[id.z];
    var negative_count = 0;
    var positive_count = 0;
    for (var i = -curve_config.thickness; i <= curve_config.thickness; i++) {
        for (var j = -curve_config.thickness; j <= curve_config.thickness; j++) {
            if i * i + j * j < curve_config.thickness * curve_config.thickness {
                let v = textureLoad(residual_texture, vec2<i32>(id.xy) + vec2<i32>(i, j), id.z);
                if v.x == -1 {
                    negative_count++;
                } else if v.x == 1 {
                    positive_count++;
                }
            }
        }
    }
    if negative_count == 0 || positive_count == 0 {
        textureStore(color_texture, id, vec4<f32>(0, 0, 0, 0));
    } else {
        textureStore(color_texture, id, curve_config.color * count_to_alpha(negative_count, positive_count));
    }
}

fn count_to_alpha(negative_count: i32, positive_count: i32) -> f32 {
    let t = 1. - f32(abs(negative_count - positive_count)) / f32(negative_count + positive_count);
    return saturate(t * t * 4.);
}
