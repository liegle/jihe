use encase::ShaderType;

use crate::{
    renderer::{
        buffer::AsDynamicStorageBytes,
        curve::{evaluate::Evaluate, trace::Trace, write::Write},
    },
    scene,
};

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
        // TODO: Maybe residual can be stored in f16 storage buffers
        let residual_texture = create_residual_texture(&device, dst_size, curves.len() as u32);
        let residual_texture_view = create_residual_texture_view(&residual_texture);
        let trace_texture = create_trace_texture(&device, dst_size, curves.len() as u32);
        let trace_texture_view = create_trace_texture_view(&trace_texture);

        let curves_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: scene::CurveConfig::min_size().get() * curves.len().max(1) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let evaluates = curves
            .iter()
            .enumerate()
            .map(|(i, c)| {
                Evaluate::new(&c.expr, i as u32, device, camera_buffer, &residual_texture)
            })
            .collect();
        let trace = Trace::new(&device, &residual_texture_view, &trace_texture_view);
        // TODO: Current write can only write to dst out of order
        // To do it in order, maybe we should fold dst and color tex to another tex,
        // and then write it back to dst
        let write = Write::new(&device, &curves_buffer, &trace_texture_view, dst_format);

        Self {
            evaluates,
            trace,
            write,
            residual_texture,
            trace_texture,
            curves_buffer,
        }
    }

    pub fn dst_resize(
        &mut self,
        device: &wgpu::Device,
        dst_size: (u32, u32),
        camera_buffer: &wgpu::Buffer,
    ) {
        self.residual_texture.destroy();
        self.residual_texture =
            create_residual_texture(&device, dst_size, self.evaluates.len() as u32);
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

    pub fn prepare(&self, curves: &Vec<scene::Curve>, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.curves_buffer,
            0,
            &curves
                .iter()
                .map(|c| c.config)
                .collect::<Vec<_>>()
                .as_dynamic_storage_bytes(),
        );
    }

    pub fn compute(
        &self,
        layers: u32,
        compute_pass: &mut super::ComputePass,
        dst_size: (u32, u32),
    ) {
        for evaluate in &self.evaluates {
            #[cfg(feature = "profile")]
            let _ = compute_pass.scope(format!("Curve evalute {}", evaluate.layer));
            evaluate.render(compute_pass, dst_size);
        }
        {
            #[cfg(feature = "profile")]
            let _ = compute_pass.scope("Curve trace");
            self.trace.render(compute_pass, dst_size, layers);
        }
    }

    pub fn render(&self, layers: u32, render_pass: &mut super::RenderPass) {
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Curve write");
        self.write.render(render_pass, layers);
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
            depth_or_array_layers: layer_count / 32 + 1,
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
