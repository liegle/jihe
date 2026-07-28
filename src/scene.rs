use std::sync::{Arc, Mutex};

pub struct Scene {
    pub config: Config,
    pub data: Arc<Mutex<SceneData>>,
}

pub struct Config {
    pub move_speed: f32,
}

pub struct SceneData {
    pub camera: Camera,
    pub bg: Bg,
    pub curves: Vec<Curve>,
}

#[derive(encase::ShaderType)]
pub struct Camera {
    pub scale: f32,
    pub pos: glam::Vec2,
}

pub struct Bg {
    pub color: glam::Vec3,
    pub axis: Option<glam::Vec4>,
    pub grid: Option<glam::Vec4>,
    pub spacing: u32,
}

pub struct Curve {
    pub config: CurveConfig,
    // TODO: When parsing, prevent pow(minus, xxx);
    // replace log/log2 with safeLog/safeLog2
    pub expr: String,
}

#[derive(encase::ShaderType, Clone, Copy)]
pub struct CurveConfig {
    pub thickness: u32,
    pub color: glam::Vec4,
}

enum Direction {
    Down,
    Left,
    Right,
    Up,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            config: Config { move_speed: 30. },
            data: Arc::new(Mutex::new(SceneData {
                camera: Camera {
                    scale: 0.01,
                    pos: glam::Vec2::ZERO,
                },
                bg: Bg {
                    color: glam::vec3(0.8, 0.8, 0.8),
                    axis: Some(glam::vec4(0.1, 0.1, 0.1, 1.)),
                    grid: Some(glam::vec4(0.4, 0.4, 0.4, 1.)),
                    spacing: 100,
                },
                curves: vec![
                    Curve {
                        config: CurveConfig {
                            thickness: 0,
                            color: glam::vec4(1., 0., 0., 1.),
                        },
                        expr: "pow(x, x) + pow(2, y) - 10".to_string(),
                    },
                    Curve {
                        config: CurveConfig {
                            thickness: 0,
                            color: glam::vec4(0., 0., 1., 1.),
                        },
                        expr: "y - 3".to_string(),
                    },
                    Curve {
                        config: CurveConfig {
                            thickness: 0,
                            color: glam::vec4(0., 1., 0., 1.),
                        },
                        expr: "pow(x, 3) + safeLog(y) - 10".to_string(),
                    },
                ],
            })),
        }
    }

    pub fn handle_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        if event.state == winit::event::ElementState::Pressed {
            use winit::keyboard::{Key, NamedKey};
            match event.logical_key.as_ref() {
                Key::Named(NamedKey::ArrowDown) | Key::Character("j") => {
                    self.scene_move(Direction::Down)
                }
                Key::Named(NamedKey::ArrowLeft) | Key::Character("h") => {
                    self.scene_move(Direction::Left)
                }
                Key::Named(NamedKey::ArrowRight) | Key::Character("l") => {
                    self.scene_move(Direction::Right)
                }
                Key::Named(NamedKey::ArrowUp) | Key::Character("k") => {
                    self.scene_move(Direction::Up)
                }
                _ => {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn scene_move(&mut self, dir: Direction) {
        let data = &mut self.data.lock().unwrap();
        let delta = match dir {
            Direction::Down => glam::vec2(0., -1.),
            Direction::Left => glam::vec2(-1., 0.),
            Direction::Right => glam::vec2(1., 0.),
            Direction::Up => glam::vec2(0., 1.),
        } * self.config.move_speed
            * data.camera.scale;
        data.camera.pos += delta;
        log::info!("Current pos: {}", data.camera.pos);
    }
}
