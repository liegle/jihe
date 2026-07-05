use encase::ShaderType;

use crate::{renderer::{
    buffer::{self, AsDynamicStorageBytes},
    curve::{evaluate::Evaluate, trace::Trace, write::Write},
}, scene};

mod evaluate;
mod trace;
mod write;

pub struct Curve {
    evaluates: Vec<Evaluate>,
    trace: Trace,
    write: Write,

    residual_texture: wgpu::Texture,
    trace_texture: wgpu::Texture,
    curves_buffer: wgpu::Buffer,
}

impl Curve {
    pub fn new(
        curves: &Vec<scene::Curve>,
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        dst_format: wgpu::TextureFormat,
        dst_size: (u32, u32),
    ) -> Self {
        // TODO: Store 1 residual in 1 bit, to store 32 curves in 1 texture layer
        let residual_texture = create_residual_texture(&device, dst_size, curves.len() as u32);
        let residual_texture_view = create_residual_texture_view(&residual_texture);
        let trace_texture = create_trace_texture(&device, dst_size, curves.len() as u32);
        let trace_texture_view = create_trace_texture_view(&trace_texture);

        let curves_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer::Curve::min_size().get() * curves.len().max(1) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let evaluates = curves
            .iter()
            .enumerate()
            .map(|(i, c)| Evaluate::new(&c.expr, i as u32, device, camera_buffer, &residual_texture))
            .collect();
        let trace = Trace::new(&device, &residual_texture_view, &trace_texture_view);
        // TODO: Current write can only write to dst out of order
        // To do it in order, maybe we should fold dst and color tex to another tex,
        // and then write it back to dst
        let write = Write::new(
            &device,
            &curves_buffer,
            &trace_texture_view,
            dst_format,
        );

        Self {
            evaluates,
            trace,
            write,
            residual_texture,
            trace_texture,
            curves_buffer,
        }
    }

    pub fn dst_resize(&mut self, device: &wgpu::Device, dst_size: (u32, u32), camera_buffer: &wgpu::Buffer) {
        self.residual_texture.destroy();
        self.residual_texture = create_residual_texture(&device, dst_size, self.evaluates.len() as u32);
        let residual_texture_view = create_residual_texture_view(&self.residual_texture);
        self.trace_texture.destroy();
        self.trace_texture = create_trace_texture(&device, dst_size, self.evaluates.len() as u32);
        let trace_texture_view = create_trace_texture_view(&self.trace_texture);

        for (layer, evaluate) in &mut self.evaluates.iter_mut().enumerate() {
            evaluate.remake_bind_group(
                &device,
                camera_buffer,
                &self.residual_texture,
                layer as u32,
            );
        }
        self.trace
            .remake_bind_group(&device, &residual_texture_view, &trace_texture_view);
        self.write
            .remake_bind_group(&device, &self.curves_buffer, &trace_texture_view);
    }

    pub fn render(
        &self,
        curves: &Vec<scene::Curve>,
        queue: &wgpu::Queue,
        #[cfg(feature = "profile")] encoder: &mut wgpu_profiler::Scope<'_, wgpu::CommandEncoder>,
        #[cfg(not(feature = "profile"))] encoder: &mut wgpu::CommandEncoder,
        dst_texture_view: &wgpu::TextureView,
    ) {
        #[cfg(feature = "profile")]
        let mut curve_encoder = encoder.scope("Curve");
        let dst_size = (
            dst_texture_view.texture().width(),
            dst_texture_view.texture().height(),
        );
        queue.write_buffer(
            &self.curves_buffer,
            0,
            &curves
                .iter()
                .map(|c| buffer::Curve {
                    thickness: c.thickness,
                    color: c.color
                })
                .collect::<Vec<_>>()
                .as_dynamic_storage_bytes()
                .unwrap(),
        );

        '_compute: {
            #[cfg(feature = "profile")]
            let mut compute_pass = curve_encoder.scoped_compute_pass("ComputePass");
            #[cfg(not(feature = "profile"))]
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ComputePass"),
                timestamp_writes: None,
            });
            for evaluate in &self.evaluates {
                #[cfg(feature = "profile")]
                let _ = compute_pass.scope(format!("Curve evalute {}", evaluate.layer));
                evaluate.render(&mut compute_pass, dst_size);
            }
            {
                #[cfg(feature = "profile")]
                let _ = compute_pass.scope("Curve trace");
                self.trace
                    .render(&mut compute_pass, dst_size, curves.len() as u32);
            }
        }
        '_render: {
            let render_pass_descriptor = wgpu::RenderPassDescriptor {
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
            };
            #[cfg(feature = "profile")]
            let mut render_pass =
                curve_encoder.scoped_render_pass("RenderPass", render_pass_descriptor);
            #[cfg(not(feature = "profile"))]
            let mut render_pass = encoder.begin_render_pass(&render_pass_descriptor);
            {
                #[cfg(feature = "profile")]
                let _ = render_pass.scope("Curve write");
                self.write.render(&mut render_pass, curves.len() as u32);
            }
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
            depth_or_array_layers: layer_count.max(1),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
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

fn create_trace_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layer_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layer_count.max(1),
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

fn create_trace_texture_view(trace_texture: &wgpu::Texture) -> wgpu::TextureView {
    trace_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(trace_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D3),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}
