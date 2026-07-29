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
        let half_size = (dst_size.0 as i32 / 2, dst_size.1 as i32 / 2);
        let axis_pos_ss = (
            (-camera.pos.x / camera.scale) as i32,
            (-camera.pos.y / camera.scale) as i32,
        );

        let axis_pos_cs = (
            ((axis_pos_ss.0 as f32) / (half_size.0 as f32)).clamp(-0.99, 0.99),
            ((axis_pos_ss.1 as f32) / (half_size.1 as f32)).clamp(-0.99, 0.99),
        );

        if let Some(color) = bg.axis {
            self.axis.prepare(axis_pos_cs, color, queue);
        }
        'grid: {
            let (color, grid_ends_cs) = match (bg.axis, bg.grid) {
                (_, Some(color)) => (color, Bounds::new(-1., 1., -1., 1.)),
                (Some(color), None) => (
                    color,
                    Bounds::new(
                        axis_pos_cs.1,
                        axis_pos_cs.1 + GRADUATION_HEIGHT / (half_size.1 as f32),
                        axis_pos_cs.0,
                        axis_pos_cs.0 + GRADUATION_HEIGHT / (half_size.0 as f32),
                    ),
                ),
                (None, None) => {
                    break 'grid;
                }
            };

            let spacing_range = (bg.spacing as f32 / 2., bg.spacing as f32 * 2.);
            let mut spacing = 1. / camera.scale;
            while spacing < spacing_range.0 {
                spacing *= 2.;
            }
            while spacing > spacing_range.1 {
                spacing /= 2.;
            }
            self.grid.prepare(
                spacing as i32, half_size, axis_pos_ss, grid_ends_cs, color, queue, device,
            );
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

struct Bounds<T> {
    l: T,
    r: T,
    b: T,
    t: T,
}

impl<T> Bounds<T> {
    fn new(l: T, r: T, b: T, t: T) -> Self {
        Self { l, r, b, t }
    }
}
