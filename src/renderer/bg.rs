use crate::{
    renderer::bg::{axis::Axis, grid::Grid},
    scene,
};

mod axis;
mod grid;

const GRADUATION_HEIGHT: f32 = 5.;

pub struct Bg {
    axis: Axis,
    grid: Grid,
}

impl Bg {
    pub fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let axis = Axis::new(device, dst_format);
        let grid = Grid::new(device, dst_format);
        Self { axis, grid }
    }

    pub fn prepare(
        &mut self,
        bg: &scene::Bg,
        camera: &scene::Camera,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dst_size: (u32, u32),
    ) {
        let size = (dst_size.0 as i32, dst_size.1 as i32);
        let spacing = bg.spacing as i32;
        let half_size = (size.0 / 2, size.1 / 2);
        let axis_pos = (
            (-camera.pos.x / camera.scale) as i32 + 1,
            (-camera.pos.y / camera.scale) as i32 - 1,
        );

        let clamped_axis_pos = (
            ((axis_pos.0 as f32) / (half_size.0 as f32)).clamp(-0.99, 0.99),
            ((axis_pos.1 as f32) / (half_size.1 as f32)).clamp(-0.99, 0.99),
        );

        if let Some(color) = bg.axis {
            self.axis.prepare(clamped_axis_pos, color, queue);
        }
        match (bg.axis, bg.grid) {
            (_, Some(color)) => {
                self.grid.prepare(
                    spacing,
                    half_size,
                    axis_pos,
                    (-1., 1.),
                    (-1., 1.),
                    color,
                    queue,
                    device,
                );
            }
            (Some(color), None) => {
                self.grid.prepare(
                    spacing,
                    half_size,
                    axis_pos,
                    (
                        clamped_axis_pos.1,
                        clamped_axis_pos.1 + GRADUATION_HEIGHT / (half_size.1 as f32),
                    ),
                    (
                        clamped_axis_pos.0,
                        clamped_axis_pos.0 + GRADUATION_HEIGHT / (half_size.0 as f32),
                    ),
                    color,
                    queue,
                    device,
                );
            }
            (None, None) => {}
        }
    }

    pub fn render(&self, render_pass: &mut super::RenderPass) {
        {
            #[cfg(feature = "profile")]
            let _ = render_pass.scope("Axis");
            self.axis.render(render_pass);
        }
        {
            #[cfg(feature = "profile")]
            let _ = render_pass.scope("Grid");
            self.grid.render(render_pass);
        }
    }
}
