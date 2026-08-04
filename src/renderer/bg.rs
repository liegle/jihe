use std::sync::LazyLock;

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
        queue: &wgpu::Queue,
        bg: &scene::Bg,
        camera: &scene::Camera,
        dst_size: (u32, u32),
    ) {
        let half_size = glam::vec2(dst_size.0 as f32 / 2., dst_size.1 as f32 / 2.);

        const AXIS_MARGIN: f32 = 15.;
        let axis_area = half_size - AXIS_MARGIN;
        let axis_pos_cs = glam::vec2(
            (-camera.pos.x / camera.scale).clamp(-axis_area.x, axis_area.x) / half_size.x,
            (-camera.pos.y / camera.scale).clamp(-axis_area.y, axis_area.y) / half_size.y,
        );

        if let Some(scene::Axis {
            color,
            grad_height: _,
        }) = bg.axis
        {
            self.axis.prepare(queue, axis_pos_cs, color);
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

            const ZOOM_UNIT: f32 = 10.;
            static SQRT_ZOOM_UNIT: LazyLock<f32> = LazyLock::new(|| ZOOM_UNIT.sqrt());
            let spacing_range =
                bg.spacing as f32 / *SQRT_ZOOM_UNIT..bg.spacing as f32 * *SQRT_ZOOM_UNIT;
            let mut spacing = 1. / camera.scale;
            while spacing < spacing_range.start {
                spacing *= ZOOM_UNIT;
            }
            while spacing > spacing_range.end {
                spacing /= ZOOM_UNIT;
            }

            let half_size_ws = half_size * camera.scale;
            let screen_bounds_ws = Bounds::with_center_and_extend(camera.pos, half_size_ws);
            self.grid.prepare(
                queue,
                spacing * camera.scale,
                screen_bounds_ws,
                grid_ends_cs,
                color,
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
}
