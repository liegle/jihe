use crate::{
    renderer::bg::{axis::Axis, grid::Grid},
    scene,
};

mod axis;
mod grid;

pub(super) struct Bg {
    axis: Axis,
    grid: Grid,
}

impl Bg {
    pub(super) fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let axis = Axis::new(device, dst_format);
        let grid = Grid::new(device, dst_format);
        Self { axis, grid }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bg: &scene::Bg,
        camera: &scene::Camera,
        dst_size: (u32, u32),
    ) {
        let half_size = glam::vec2(dst_size.0 as f32 / 2., dst_size.1 as f32 / 2.);
        let axis_pos_cs = glam::vec2(
            (-camera.pos.x / camera.scale / half_size.x).clamp(-0.99, 0.99),
            (-camera.pos.y / camera.scale / half_size.y).clamp(-0.99, 0.99),
        );

        if let Some(scene::Axis {
            color,
            grad_height: _,
        }) = bg.axis
        {
            self.axis.prepare(axis_pos_cs, color, queue);
        }
        'grid: {
            let (color, grid_ends_cs) = match (&bg.axis, &bg.grid) {
                (_, Some(scene::Grid { color })) => (*color, Bounds::new(-1., 1., -1., 1.)),
                (Some(scene::Axis { color, grad_height }), None) => (
                    *color,
                    Bounds::with_pos_and_size(
                        axis_pos_cs,
                        glam::vec2(
                            (*grad_height as f32 / half_size.x).copysign(camera.pos.x),
                            (*grad_height as f32 / half_size.y).copysign(camera.pos.y),
                        ),
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

            let half_size_ws = glam::vec2(half_size.x * camera.scale, half_size.y * camera.scale);
            let screen_bounds_ws = Bounds::with_center_and_extend(camera.pos, half_size_ws);
            self.grid.prepare(
                spacing * camera.scale,
                screen_bounds_ws,
                grid_ends_cs,
                color,
                queue,
                device,
            );
        }
    }

    pub(super) fn render(&self, render_pass: &mut super::RenderPass) {
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

struct Bounds {
    l: f32,
    r: f32,
    b: f32,
    t: f32,
}

impl Bounds {
    fn new(l: f32, r: f32, b: f32, t: f32) -> Self {
        Self { l, r, b, t }
    }

    fn with_pos_and_size(pos: glam::Vec2, size: glam::Vec2) -> Self {
        let (l, r) = (pos.x, pos.x + size.x);
        let (l, r) = if size.x < 0. { (r, l) } else { (l, r) };
        let (b, t) = (pos.y, pos.y + size.y);
        let (b, t) = if size.y < 0. { (t, b) } else { (b, t) };
        Self { l, r, b, t }
    }

    fn with_center_and_extend(center: glam::Vec2, extend: glam::Vec2) -> Self {
        let extend = extend.abs();
        Self {
            l: center.x - extend.x,
            r: center.x + extend.x,
            b: center.y - extend.y,
            t: center.y + extend.y,
        }
    }

    fn w(&self) -> f32 {
        self.r - self.l
    }

    fn h(&self) -> f32 {
        self.t - self.b
    }
}
