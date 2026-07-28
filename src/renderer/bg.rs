use crate::{renderer::bg::line::Line, scene};

mod line;

const GRADUATION_HEIGHT: f32 = 5.;

pub struct Bg {
    axis: Line,
    grid: Line,
}

impl Bg {
    pub fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let axis = Line::new(device, dst_format);
        let grid = Line::new(device, dst_format);
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
        let h_begin = ((-half_size.0 - axis_pos.0) / spacing) * spacing + axis_pos.0;
        let h_count = (half_size.0 - h_begin) / spacing + 1;
        let v_begin = ((-half_size.1 - axis_pos.1) / spacing) * spacing + axis_pos.1;
        let v_count = (half_size.1 - v_begin) / spacing + 1;

        let clamped_axis_pos = (
            ((axis_pos.0 as f32) / (half_size.0 as f32)).clamp(-0.99, 0.99),
            ((axis_pos.1 as f32) / (half_size.1 as f32)).clamp(-0.99, 0.99),
        );
        if let Some(color) = bg.axis {
            let mut vertices = Vec::<glam::Vec2>::new();
            vertices.extend(&[
                // y axis
                glam::vec2(clamped_axis_pos.0, -1.),
                glam::vec2(clamped_axis_pos.0, 1.),
                // x axis
                glam::vec2(-1., clamped_axis_pos.1),
                glam::vec2(1., clamped_axis_pos.1),
            ]);
            let h_end = clamped_axis_pos.1 + GRADUATION_HEIGHT / (half_size.1 as f32);
            let v_end = clamped_axis_pos.0 + GRADUATION_HEIGHT / (half_size.0 as f32);
            for i in 0..h_count {
                let x = ((h_begin + spacing * i) as f32) / (half_size.0 as f32);
                vertices.extend(&[glam::vec2(x, clamped_axis_pos.1), glam::vec2(x, h_end)])
            }
            for i in 0..v_count {
                let y = ((v_begin + spacing * i) as f32) / (half_size.1 as f32);
                vertices.extend(&[glam::vec2(clamped_axis_pos.0, y), glam::vec2(v_end, y)])
            }
            self.axis.prepare(&vertices, color, queue, device);
        }
        if let Some(color) = bg.grid {
            let mut vertices = Vec::<glam::Vec2>::new();
            for i in 0..h_count {
                let x = ((h_begin + spacing * i) as f32) / (half_size.0 as f32);
                vertices.extend(&[glam::vec2(x, -1.), glam::vec2(x, 1.)])
            }
            for i in 0..v_count {
                let y = ((v_begin + spacing * i) as f32) / (half_size.1 as f32);
                vertices.extend(&[glam::vec2(-1., y), glam::vec2(1., y)])
            }
            self.grid.prepare(&vertices, color, queue, device);
        }
    }

    pub fn render(&self, render_pass: &mut super::RenderPass) {
        {
            #[cfg(feature = "profile")]
            let _ = render_pass.scope("Grid");
            self.grid.render(render_pass);
        }
        {
            #[cfg(feature = "profile")]
            let _ = render_pass.scope("Axis");
            self.axis.render(render_pass);
        }
    }
}
