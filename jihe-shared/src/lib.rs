pub struct Content {
    pub bg: Bg,
    pub curves: Vec<Curve>,
    pub points: Vec<Point>,
}

pub struct Bg {
    pub color: glam::Vec3,
    pub axis: Option<Axis>,
    pub grid: Option<Grid>,
    pub spacing: u32,
}

pub struct Axis {
    pub color: glam::Vec3,
    pub grad_height: u32,
}

pub struct Grid {
    pub color: glam::Vec3,
}

pub struct Curve {
    // TODO: When parsing, prevent pow(minus, xxx);
    // replace log/log2 with safeLog/safeLog2
    pub expr: String,
    pub thickness: f32,
    pub color: glam::Vec4,
}

pub struct Point {
    pub pos: glam::Vec2,
    pub size: f32,
    pub color: glam::Vec4,
}

impl Content {
    pub fn example() -> Self {
        Self {
            bg: Bg {
                color: glam::vec3(0.8, 0.8, 0.8),
                axis: Some(Axis {
                    color: glam::vec3(0.1, 0.1, 0.1),
                    grad_height: 5,
                }),
                grid: Some(Grid {
                    color: glam::vec3(0.5, 0.5, 0.5),
                }),
                spacing: 100,
            },
            curves: vec![
                // Curve {
                //     expr: "y - 1".to_string(),
                //     thickness: 1.5,
                //     color: glam::vec4(0., 0., 1., 1.),
                // },
                // Curve {
                //     expr: "x - 1".to_string(),
                //     thickness: 1.5,
                //     color: glam::vec4(0., 0., 1., 1.),
                // },
                // Curve {
                //     expr: "pow(y, 3) + safeLog(x) - 10".to_string(),
                //     thickness: 1.5,
                //     color: glam::vec4(1., 0., 0., 1.),
                // },
                // Curve {
                //     expr: "pow(x, 3) + safeLog(y) - 10".to_string(),
                //     thickness: 1.5,
                //     color: glam::vec4(0., 1., 0., 1.),
                // },
            ],
            points: vec![
                Point {
                    pos: glam::vec2(1., 1.),
                    size: 3.,
                    color: glam::vec4(1., 0., 0., 1.),
                },
                Point {
                    pos: glam::vec2(2., 1.),
                    size: 3.5,
                    color: glam::vec4(1., 0., 0., 1.),
                },
                Point {
                    pos: glam::vec2(3., 1.),
                    size: 4.,
                    color: glam::vec4(1., 0., 0., 1.),
                },
            ],
        }
    }
}
