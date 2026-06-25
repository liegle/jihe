use encase::ShaderType;

use crate::renderer::{
    buffer::AsDynamicStorageBytes,
    curve::{evaluate::Evaluate, trace::Trace, write::Write},
};

mod evaluate;
mod trace;
mod write;

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
    camera_buffer: wgpu::Buffer,

    evaluates: Vec<Evaluate>,
    trace: Trace,
    write: Write,

    curve_configs_buffer: wgpu::Buffer,
}

impl Curve {
    pub fn new(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        dst_format: wgpu::TextureFormat,
        dst_size: (u32, u32),
    ) -> Self {
        // TODO: Store 1 residual in 1 bit, to store 32 curves in 1 texture layer
        let residual_texture = create_residual_texture(&device, dst_size, CURVES.len() as u32);
        let residual_texture_view = create_residual_texture_view(&residual_texture);
        let color_texture = create_color_texture(&device, dst_size, CURVES.len() as u32);
        let color_texture_view = create_color_texture_view(&color_texture);

        let curve_configs_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: CurveConfig::min_size().get() * CURVES.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let evaluates = CURVES
            .iter()
            .enumerate()
            .map(|(i, c)| Evaluate::new(c.1, i as u32, device, camera_buffer, &residual_texture))
            .collect();
        let trace = Trace::new(
            &device,
            &curve_configs_buffer,
            &residual_texture_view,
            &color_texture_view,
        );
        // TODO: Current write can only write to dst out of order
        // To do it in order, maybe we should fold dst and color tex to another tex,
        // and then write it back to dst
        let write = Write::new(&device, &color_texture_view, dst_format);

        Self {
            camera_buffer: camera_buffer.clone(),
            evaluates,
            trace,
            write,
            curve_configs_buffer,
        }
    }

    pub fn dst_resize(&mut self, device: &wgpu::Device, dst_size: (u32, u32)) {
        let residual_texture = create_residual_texture(&device, dst_size, CURVES.len() as u32);
        let residual_texture_view = create_residual_texture_view(&residual_texture);
        let color_texture = create_color_texture(&device, dst_size, CURVES.len() as u32);
        let color_texture_view = create_color_texture_view(&color_texture);

        for (layer, evaluate) in &mut self.evaluates.iter_mut().enumerate() {
            evaluate.remake_bind_group(
                &device,
                &self.camera_buffer,
                &residual_texture,
                layer as u32,
            );
        }
        self.trace.remake_bind_group(
            &device,
            &self.curve_configs_buffer,
            &residual_texture_view,
            &color_texture_view,
        );
        self.write.remake_bind_group(&device, &color_texture_view);
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        dst_texture_view: &wgpu::TextureView,
    ) {
        let dst_size = (
            dst_texture_view.texture().width(),
            dst_texture_view.texture().height(),
        );
        queue.write_buffer(
            &self.curve_configs_buffer,
            0,
            &CURVES
                .iter()
                .map(|c| c.0.clone())
                .collect::<Vec<CurveConfig>>()
                .as_dynamic_storage_bytes()
                .unwrap(),
        );

        '_compute: {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ComputePass"),
                timestamp_writes: None,
            });
            for evaluate in &self.evaluates {
                evaluate.render(&mut compute_pass, dst_size);
            }
            self.trace
                .render(&mut compute_pass, dst_size, CURVES.len() as u32);
        }
        '_render: {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst_texture_view,
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

fn create_residual_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layer_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Sint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

fn create_residual_texture_view(residual_texture: &wgpu::Texture) -> wgpu::TextureView {
    residual_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(residual_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

fn create_color_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layer_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layer_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

fn create_color_texture_view(color_texture: &wgpu::Texture) -> wgpu::TextureView {
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
