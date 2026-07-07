use crate::{renderer::bg::{axis::Axis, grid::Grid}, scene};

mod axis;
mod grid;

pub struct Bg {
    axis: Axis,
    grid: Grid,
}

impl Bg {
    // pub fn new() -> Self {
    //     Self {

    //     }
    // }

    pub fn render(&self, bg: &scene::Bg) {}
}
