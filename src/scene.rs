use std::sync::{Arc, Mutex};

pub struct Scene {
    pub config: Config,
    pub data: Arc<Mutex<SceneData>>,
    drag: DragState,
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
    pub points: Vec<Point>,
}

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
    // TODO: When parsing, prevent pow(minus, xxx);
    // replace log/log2 with safeLog/safeLog2
    pub expr: String,
    // TODO: implement thickness in the future
    // pub thickness: f32,
    pub color: glam::Vec4,
}

pub struct Point {
    pub pos: glam::Vec2,
    pub size: f32,
    pub color: glam::Vec4,
}

enum DragState {
    Released { mouse: glam::Vec2 },
    DraggingFrom { mouse: glam::Vec2, cam: glam::Vec2 },
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
                        expr: "y - 1".to_string(),
                        color: glam::vec4(0., 0., 1., 1.),
                    },
                    Curve {
                        expr: "x - 1".to_string(),
                        color: glam::vec4(0., 0., 1., 1.),
                    },
                    Curve {
                        expr: "pow(y, 3) + safeLog(x) - 10".to_string(),
                        color: glam::vec4(1., 0., 0., 1.),
                    },
                    Curve {
                        expr: "pow(x, 3) + safeLog(y) - 10".to_string(),
                        color: glam::vec4(0., 1., 0., 1.),
                    },
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
            })),
            drag: DragState::Released {
                mouse: glam::vec2(0., 0.),
            },
        }
    }

    pub fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) -> bool {
        enum Direction {
            Down,
            Left,
            Right,
            Up,
        }

        fn scene_move(this: &mut Scene, dir: Direction) {
            let data = &mut this.data.lock().unwrap();
            let delta = match dir {
                Direction::Down => glam::vec2(0., -1.),
                Direction::Left => glam::vec2(-1., 0.),
                Direction::Right => glam::vec2(1., 0.),
                Direction::Up => glam::vec2(0., 1.),
            } * this.config.move_speed
                * data.camera.scale;
            data.camera.pos += delta;
            log::info!("Current pos: {}", data.camera.pos);
        }

        if event.state == winit::event::ElementState::Pressed {
            use winit::keyboard::{Key, NamedKey};
            match event.logical_key.as_ref() {
                Key::Named(NamedKey::ArrowDown) | Key::Character("j") => {
                    scene_move(self, Direction::Down)
                }
                Key::Named(NamedKey::ArrowLeft) | Key::Character("h") => {
                    scene_move(self, Direction::Left)
                }
                Key::Named(NamedKey::ArrowRight) | Key::Character("l") => {
                    scene_move(self, Direction::Right)
                }
                Key::Named(NamedKey::ArrowUp) | Key::Character("k") => {
                    scene_move(self, Direction::Up)
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
        match &self.drag {
            DragState::Released { mouse: _ } => {
                self.drag = DragState::Released { mouse: position };
                false
            }
            DragState::DraggingFrom { mouse, cam } => {
                let camera = &mut self.data.lock().unwrap().camera;
                camera.pos = (position - mouse) * camera.scale + cam;
                true
            }
        }
    }

    pub fn handle_mouse_input(
        &mut self,
        state: &winit::event::ElementState,
        button: &winit::event::MouseButton,
    ) -> bool {
        use winit::event::{ElementState, MouseButton};
        let MouseButton::Left = button else {
            return matches!(self.drag, DragState::DraggingFrom { .. });
        };
        match (&self.drag, state) {
            (DragState::Released { mouse }, ElementState::Pressed) => {
                self.drag = DragState::DraggingFrom {
                    mouse: *mouse,
                    cam: self.data.lock().unwrap().camera.pos,
                };
                true
            }
            (DragState::DraggingFrom { mouse, cam }, ElementState::Released) => {
                let camera = &self.data.lock().unwrap().camera;
                let delta = (camera.pos - cam) / camera.scale;
                self.drag = DragState::Released {
                    mouse: mouse + delta,
                };
                false
            }
            _ => {
                log::warn!("Mouse state not changed");
                matches!(self.drag, DragState::DraggingFrom { .. })
            }
        }
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
}
