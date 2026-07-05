use std::sync::{Arc, Mutex};

pub struct Scene {
    pub data: Arc<Mutex<SceneData>>,
}

pub struct SceneData {
    pub camera: Camera,
    pub config: Config,
    pub curves: Vec<Curve>,
}

pub struct Camera {
    pub scale: f32,
    pub pos: glam::Vec2,
}

pub struct Config {
    pub move_speed: f32,
}

pub struct Curve {
    pub thickness: u32,
    pub color: glam::Vec4,
    pub expr: String,
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
            data: Arc::new(Mutex::new(SceneData {
                camera: Camera {
                    scale: 0.01,
                    pos: glam::Vec2::ZERO,
                },
                config: Config { move_speed: 0.3 },
                curves: vec![
                    Curve {
                        thickness: 2,
                        color: glam::vec4(1., 0., 0., 1.),
                        expr: "pow(x, x) + pow(2, y) - 10".to_string(),
                    },
                    Curve {
                        thickness: 2,
                        color: glam::vec4(0., 0., 1., 1.),
                        expr: "y - 3".to_string(),
                    },
                    Curve {
                        thickness: 2,
                        color: glam::vec4(1., 1., 1., 1.),
                        expr: "pow(x, 3) + log(y) - 10".to_string(),
                    },
                ],
            })),
        }
    }

    pub fn handle_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        if event.state == winit::event::ElementState::Pressed {
            match event.logical_key.as_ref() {
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown)
                | winit::keyboard::Key::Character("j") => self.scene_move(Direction::Down),
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft)
                | winit::keyboard::Key::Character("h") => self.scene_move(Direction::Left),
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight)
                | winit::keyboard::Key::Character("l") => self.scene_move(Direction::Right),
                winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp)
                | winit::keyboard::Key::Character("k") => self.scene_move(Direction::Up),
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
        let delta = data.config.move_speed
            * match dir {
                Direction::Down => glam::vec2(0., -1.),
                Direction::Left => glam::vec2(-1., 0.),
                Direction::Right => glam::vec2(1., 0.),
                Direction::Up => glam::vec2(0., 1.),
            };
        data.camera.pos += delta;
    }
}
