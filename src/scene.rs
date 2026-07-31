use std::sync::{Arc, Mutex};

pub struct Scene {
    pub config: Config,
    pub data: Arc<Mutex<SceneData>>,
    mouse: MouseState,
}

pub struct Config {
    pub render_per_sec: u64,
    pub resize_per_sec: u64,
    pub move_speed: f32,
    pub zoom_factor: f32,
}

pub struct SceneData {
    pub camera: Camera,
    pub bg: Bg,
    pub curves: Vec<Curve>,
}

enum MouseState {
    Released {
        mouse_current: glam::Vec2,
    },
    Pressed {
        mouse_pressed: glam::Vec2,
        camera_anchor: glam::Vec2,
    },
}

#[derive(encase::ShaderType)]
pub struct Camera {
    /// Coord units per pixel
    pub scale: f32,
    pub pos: glam::Vec2,
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
            config: Config {
                render_per_sec: 60,
                resize_per_sec: 60,
                move_speed: 30.,
                zoom_factor: 0.2,
            },
            data: Arc::new(Mutex::new(SceneData {
                camera: Camera {
                    scale: 0.01,
                    pos: glam::Vec2::ZERO,
                },
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
            mouse: MouseState::Released {
                mouse_current: glam::vec2(0., 0.),
            },
        }
    }

    pub fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) -> bool {
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

    pub fn handle_cursor_moved(&mut self, position: &winit::dpi::PhysicalPosition<f64>) -> bool {
        let position = glam::vec2(-position.x as f32, position.y as f32);
        match &self.mouse {
            MouseState::Released { mouse_current: _ } => {
                self.mouse = MouseState::Released {
                    mouse_current: position,
                };
                false
            }
            MouseState::Pressed {
                mouse_pressed,
                camera_anchor,
            } => {
                let camera = &mut self.data.lock().unwrap().camera;
                camera.pos = (position - mouse_pressed) * camera.scale + camera_anchor;
                true
            }
        }
    }

    pub fn handle_mouse_input(
        &mut self,
        state: &winit::event::ElementState,
        _button: &winit::event::MouseButton,
    ) -> bool {
        use winit::event::ElementState;
        match (&self.mouse, state) {
            (MouseState::Released { mouse_current }, ElementState::Pressed) => {
                self.mouse = MouseState::Pressed {
                    mouse_pressed: *mouse_current,
                    camera_anchor: self.data.lock().unwrap().camera.pos,
                }
            }
            (
                MouseState::Pressed {
                    mouse_pressed,
                    camera_anchor,
                },
                ElementState::Released,
            ) => {
                let camera = &self.data.lock().unwrap().camera;
                let delta = (camera.pos - camera_anchor) / camera.scale;
                self.mouse = MouseState::Released {
                    mouse_current: mouse_pressed + delta,
                }
            }
            _ => {}
        };
        false
    }

    pub fn handle_mouse_wheel(
        &mut self,
        delta: &winit::event::MouseScrollDelta,
        phase: &winit::event::TouchPhase,
    ) -> bool {
        use winit::{
            dpi::PhysicalPosition,
            event::{MouseScrollDelta, TouchPhase},
        };
        if *phase == TouchPhase::Moved {
            let (x, y) = match delta {
                MouseScrollDelta::LineDelta(x, y) => (*x, *y),
                MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (*x as f32, *y as f32),
            };
            let zoom = if x.abs() > y.abs() { x } else { y };
            let data = &mut self.data.lock().unwrap();
            data.camera.scale *= 2f32.powf(-zoom * self.config.zoom_factor);
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
