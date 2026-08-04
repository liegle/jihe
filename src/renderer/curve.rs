use std::mem;

use encase::ShaderSize as _;

use crate::{
    renderer::{
        buffer::{AsDynamicStorageBytes as _, AsUniformBytes as _},
        curve::{evaluate::Evaluate, trace::Trace, write::Write},
    },
    scene,
};

mod evaluate;
mod trace;
mod write;

pub(super) struct Curve {
    residual_texture: wgpu::Texture,
    trace_texture: wgpu::Texture,
    camera_buffer: wgpu::Buffer,
    curves_buffer: wgpu::Buffer,

    evaluates: Vec<Evaluate>,
    trace: Trace,
    write: Write,

    dst_size: (u32, u32),
}

impl Curve {
    pub(super) fn new(
        device: &wgpu::Device,
        curves: &Vec<scene::Curve>,
        dst_format: wgpu::TextureFormat,
        dst_size: (u32, u32),
    ) -> Self {
        // TODO: Maybe residual can be stored in f16 storage buffers
        let residual_texture = create_residual_texture(&device, dst_size, curves.len());
        let residual_texture_view = residual_texture.create_view(&RESIDUAL_TEXTURE_VIEW_DESCRIPTOR);
        let trace_texture = create_trace_texture(&device, dst_size, curves.len());
        let trace_texture_view = trace_texture.create_view(&TRACE_TEXTURE_VIEW_DESCRIPTOR);
        let camera_buffer = create_camera_buffer(device);
        let curves_buffer = create_curves_buffer(device, curves.len());

        let evaluates = curves
            .iter()
            .enumerate()
            .map(|(i, c)| {
                Evaluate::new(device, &camera_buffer, &residual_texture, &c.expr, i as u32)
            })
            .collect();
        let trace = Trace::new(&device, &residual_texture_view, &trace_texture_view);
        // TODO: Current write can only write to dst out of order
        // To do it in order, maybe we should fold dst and color tex to another tex,
        // and then write it back to dst
        let write = Write::new(&device, &curves_buffer, &trace_texture_view, dst_format);

        Self {
            residual_texture,
            trace_texture,
            camera_buffer,
            curves_buffer,

            evaluates,
            trace,
            write,

            dst_size,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        curves: &Vec<scene::Curve>,
        camera: &scene::Camera,
        dst_size: (u32, u32),
    ) {
        queue.write_buffer(
            &self.camera_buffer,
            0,
            &CameraUniform {
                scale: camera.scale,
                pos: camera.pos,
            }
            .as_uniform_bytes(),
        );

        if self.evaluates.len() != curves.len() {
            self.curves_buffer.destroy();
            self.curves_buffer = create_curves_buffer(device, curves.len());
        }

        let resized = self.dst_size != dst_size;
        if resized {
            self.dst_size = dst_size;

            self.residual_texture.destroy();
            self.residual_texture =
                create_residual_texture(&device, dst_size, self.evaluates.len());
            let residual_texture_view = self
                .residual_texture
                .create_view(&RESIDUAL_TEXTURE_VIEW_DESCRIPTOR);
            self.trace_texture.destroy();
            self.trace_texture = create_trace_texture(&device, dst_size, self.evaluates.len());
            let trace_texture_view = self
                .trace_texture
                .create_view(&TRACE_TEXTURE_VIEW_DESCRIPTOR);

            self.trace
                .remake_bind_group(&device, &residual_texture_view, &trace_texture_view);
            self.write
                .remake_bind_group(&device, &self.curves_buffer, &trace_texture_view);
        }

        // TODO: need test in the future when dynamic scene is implemented
        let mut previous = mem::replace(&mut self.evaluates, Vec::with_capacity(curves.len()));
        for (layer, curve) in curves.iter().enumerate() {
            match previous.iter().position(|e| e.expr() == curve.expr) {
                Some(index) => {
                    let evaluate = self.evaluates.push_mut(previous.remove(index));
                    if index != layer || resized {
                        evaluate.remake_bind_group(
                            device,
                            &self.camera_buffer,
                            &self.residual_texture,
                            layer as u32,
                        );
                    }
                }
                None => {
                    self.evaluates.push(Evaluate::new(
                        device,
                        &self.camera_buffer,
                        &self.residual_texture,
                        &curve.expr,
                        layer as u32,
                    ));
                }
            }
        }

        queue.write_buffer(
            &self.curves_buffer,
            0,
            &curves
                .iter()
                .map(|c| CurveUniform {
                    // thickness: c.thickness,
                    color: c.color,
                })
                .collect::<Vec<_>>()
                .as_dynamic_storage_bytes(),
        );
    }

    pub(super) fn compute(&self, compute_pass: &mut super::ComputePass, dst_size: (u32, u32)) {
        let layers = self.evaluates.len() as u32;
        #[cfg(not(feature = "profile"))]
        for evaluate in &self.evaluates {
            evaluate.compute(compute_pass, dst_size);
        }
        #[cfg(feature = "profile")]
        for (layer, evaluate) in self.evaluates.iter().enumerate() {
            let _ = compute_pass.scope(format!("Curve evalute {}", layer));
            evaluate.compute(compute_pass, dst_size);
        }

        {
            #[cfg(feature = "profile")]
            let _ = compute_pass.scope("Curve trace");
            self.trace.compute(compute_pass, dst_size, layers);
        }
    }

    pub(super) fn render(&self, render_pass: &mut super::RenderPass) {
        let layers = self.evaluates.len() as u32;
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Curve write");
        self.write.render(render_pass, layers);
    }
}

#[inline]
fn create_residual_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layers: usize,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Residual Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layers.max(1) as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

const RESIDUAL_TEXTURE_VIEW_DESCRIPTOR: wgpu::TextureViewDescriptor = wgpu::TextureViewDescriptor {
    label: Some("Residual Texture View"),
    format: None,
    dimension: Some(wgpu::TextureViewDimension::D2Array),
    usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
    aspect: wgpu::TextureAspect::All,
    base_mip_level: 0,
    mip_level_count: None,
    base_array_layer: 0,
    array_layer_count: None,
};

#[inline]
fn create_trace_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layers: usize,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Trace Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layers as u32 / 32 + 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

const TRACE_TEXTURE_VIEW_DESCRIPTOR: wgpu::TextureViewDescriptor = wgpu::TextureViewDescriptor {
    label: Some("Trace Texture View"),
    format: None,
    dimension: Some(wgpu::TextureViewDimension::D3),
    usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
    aspect: wgpu::TextureAspect::All,
    base_mip_level: 0,
    mip_level_count: None,
    base_array_layer: 0,
    array_layer_count: None,
};

#[inline]
fn create_camera_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Camera Buffer"),
        size: CameraUniform::SHADER_SIZE.get(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[inline]
fn create_curves_buffer(device: &wgpu::Device, len: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Curves Buffer"),
        size: CurveUniform::SHADER_SIZE.get() * len.max(1) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[derive(encase::ShaderType)]
pub struct CameraUniform {
    scale: f32,
    pos: glam::Vec2,
}

#[derive(encase::ShaderType)]
struct CurveUniform {
    // thickness: f32,
    color: glam::Vec4,
}
