use std::sync::{Arc, Mutex};

pub struct Scene {
    pub camera: Camera,
    pub content: jihe_shared::Content,
}

pub struct Camera {
    /// Coord units per pixel
    pub scale: f32,
    pub pos: glam::Vec2,
}

impl Scene {
    pub fn new(content: jihe_shared::Content) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            camera: Camera {
                scale: 0.01,
                pos: glam::Vec2::ZERO,
            },
            content,
        }))
    }
}
