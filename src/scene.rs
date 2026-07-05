use std::sync::{Arc, Mutex};

pub struct Scene {
    pub data: Arc<Mutex<SceneData>>,
}

pub struct SceneData {
    pub scale: f32,
    pub pos: glam::Vec2,
    pub curves: Vec<Curve>,
}

pub struct Curve {
    pub thickness: u32,
    pub color: glam::Vec4,
    pub expr: String,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(SceneData {
                scale: 0.01,
                pos: glam::Vec2::ZERO,
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

    pub fn handle_key(&mut self, event: &winit::event::KeyEvent) {
        
    }
}
