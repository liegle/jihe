use std::sync::{Arc, Mutex};

use crate::config::Config;

pub(super) struct Memory {
    pub config: Config,
    pub scene: Arc<Mutex<jihe_shared::Scene>>,
    drag: DragState,
}

enum DragState {
    Released { mouse: glam::Vec2 },
    DraggingFrom { mouse: glam::Vec2, cam: glam::Vec2 },
}

impl Memory {
    pub(super) fn new() -> Self {
        Self {
            config: Config::new(),
            scene: Arc::new(Mutex::new(jihe_shared::Scene::new())),
            drag: DragState::Released {
                mouse: glam::vec2(0., 0.),
            },
        }
    }

    pub(super) fn handle_keyboard_input(&mut self, event: &winit::event::KeyEvent) -> bool {
        enum Direction {
            Down,
            Left,
            Right,
            Up,
        }

        fn scene_move(this: &mut Memory, dir: Direction) {
            let data = &mut this.scene.lock().unwrap();
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

    pub(super) fn handle_cursor_moved(
        &mut self,
        position: &winit::dpi::PhysicalPosition<f64>,
    ) -> bool {
        let position = glam::vec2(-position.x as f32, position.y as f32);
        match &self.drag {
            DragState::Released { mouse: _ } => {
                self.drag = DragState::Released { mouse: position };
                false
            }
            DragState::DraggingFrom { mouse, cam } => {
                let camera = &mut self.scene.lock().unwrap().camera;
                camera.pos = (position - mouse) * camera.scale + cam;
                true
            }
        }
    }

    pub(super) fn handle_mouse_input(
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
                    cam: self.scene.lock().unwrap().camera.pos,
                };
                true
            }
            (DragState::DraggingFrom { mouse, cam }, ElementState::Released) => {
                let camera = &self.scene.lock().unwrap().camera;
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

    pub(super) fn handle_mouse_wheel(
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
            let data = &mut self.scene.lock().unwrap();
            data.camera.scale *= 2f32.powf(-zoom * self.config.zoom_factor);
            true
        } else {
            false
        }
    }
}
