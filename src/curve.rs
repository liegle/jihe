use encase::ShaderType;

use crate::curve::{curve_count::CurveCount, curve_eval::CurveEval, curve_write::CurveWrite};
use crate::renderer::AsStorageBytes;

mod curve_count;
mod curve_eval;
mod curve_write;

const CURVES: &[(CurveConfig, &str)] = &[
    (
        CurveConfig {
            thickness: 3,
            color: glam::vec4(1., 0., 0., 1.),
        },
        "pow(x, 5) - y",
    ),
    (
        CurveConfig {
            thickness: 3,
            color: glam::vec4(0., 0., 1., 1.),
        },
        "pow(x, 2) + pow(y, 2) - 4",
    ),
];

#[derive(encase::ShaderType, Clone)]
struct CurveConfig {
    thickness: i32,
    color: glam::Vec4,
}

pub struct Curve {
    evals: Vec<CurveEval>,
    count: CurveCount,
    write: CurveWrite,

    curve_buffer: wgpu::Buffer,
}

impl Curve {
    pub fn new(
        device: &wgpu::Device,
        global_buffer: &wgpu::Buffer,
        target_format: wgpu::TextureFormat,
        target_size: (u32, u32),
    ) -> Self {
        let offset_texture = offset_texture(&device, target_size, CURVES.len() as u32);
        let offset_texture_view = offset_texture_view(&offset_texture);
        let color_texture = color_texture(&device, target_size, CURVES.len() as u32);
        let color_texture_view = color_texture_view(&color_texture);

        let curve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: CurveConfig::min_size().get() * CURVES.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let evals = CURVES
            .iter()
            .enumerate()
            .map(|(i, c)| CurveEval::new(c.1, i as u32, device, global_buffer, &offset_texture))
            .collect();
        let count = CurveCount::new(
            &device,
            &curve_buffer,
            &offset_texture_view,
            &color_texture_view,
            CURVES.len() as u32,
        );
        let write = CurveWrite::new(&device, &color_texture_view, target_format);

        Self {
            evals,
            count,
            write,
            curve_buffer,
        }
    }

    pub fn target_resize(&mut self, device: &wgpu::Device, target_size: (u32, u32)) {
        let offset_texture = offset_texture(&device, target_size, CURVES.len() as u32);
        let offset_texture_view = offset_texture_view(&offset_texture);
        let color_texture = color_texture(&device, target_size, CURVES.len() as u32);
        let color_texture_view = color_texture_view(&color_texture);

        for (slice, eval) in &mut self.evals.iter_mut().enumerate() {
            eval.remake_bind_group(&device, &offset_texture, slice as u32);
        }
        self.count.remake_bind_group(
            &device,
            &self.curve_buffer,
            &offset_texture_view,
            &color_texture_view,
        );
        self.write.remake_bind_group(&device, &color_texture_view);
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let size = (view.texture().width(), view.texture().height());
        queue.write_buffer(
            &self.curve_buffer,
            0,
            &CURVES
                .iter()
                .map(|c| c.0.clone())
                .collect::<Vec<CurveConfig>>()
                .as_storage_bytes()
                .unwrap(),
        );

        '_compute: {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ComputePass"),
                timestamp_writes: None,
            });
            for eval in &self.evals {
                eval.render(&mut compute_pass, size);
            }
            self.count.render(&mut compute_pass, size);
        }
        '_render: {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.write.render(&mut render_pass, CURVES.len() as u32);
        }
    }
}

fn offset_texture(device: &wgpu::Device, target_size: (u32, u32), slices: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: target_size.0,
            height: target_size.1,
            depth_or_array_layers: slices,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Sint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

fn offset_texture_view(offset_texture: &wgpu::Texture) -> wgpu::TextureView {
    offset_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(offset_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

fn color_texture(device: &wgpu::Device, target_size: (u32, u32), slices: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: target_size.0,
            height: target_size.1,
            depth_or_array_layers: slices,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

fn color_texture_view(color_texture: &wgpu::Texture) -> wgpu::TextureView {
    color_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(color_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D3),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}
