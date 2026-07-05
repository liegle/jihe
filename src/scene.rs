#[derive(Default)]
pub struct Scene {
    pub scale: f32,
    pub pos: glam::Vec2,
    pub curves: Vec<Curve>,
}

pub struct Curve {
    pub thickness: u32,
    pub color: glam::Vec4,
    pub expr: String,
}
