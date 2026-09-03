use std::mem;

use encase::ShaderSize as _;

use crate::{
    Camera,
    buffer::{AsDynamicStorageBytes as _, AsUniformBytes as _},
    curve::{binary::Binary, connect::Connect, write::Write},
};

mod binary;
mod connect;
mod write;

pub(super) struct Curve {
    // Curve intersection with left and top line segment of each pixel
    // (normalized on the border, x for top or z=1 if no intersection,
    // y for left or w=1 if no intersection)
    intersection_texture: wgpu::Texture,
    // Line segments in each pixel connected from intersections
    // (line ends in normalized pixel inner space)
    segment_texture: wgpu::Texture,
    camera_buffer: wgpu::Buffer,
    curves_buffer: wgpu::Buffer,

    binaries: Vec<Binary>,
    connect: Connect,
    write: Write,

    dst_size: (u32, u32),
}

impl Curve {
    pub(super) fn new(
        device: &wgpu::Device,
        curves: &Vec<jihe_shared::Curve>,
        dst_format: wgpu::TextureFormat,
        dst_size: (u32, u32),
    ) -> Self {
        let intersection_texture = create_intersection_texture(&device, dst_size, curves.len());
        let intersection_texture_view = intersection_texture.create_view(&INTERSECTION_TEXTURE_VIEW_DESCRIPTOR);
        let segment_texture = create_segment_texture(&device, dst_size, curves.len());
        let segment_texture_view = segment_texture.create_view(&SEGMENT_TEXTURE_VIEW_DESCRIPTOR);
        let camera_buffer = create_camera_buffer(device);
        let curves_buffer = create_curves_buffer(device, curves.len());

        let binaries = curves
            .iter()
            .enumerate()
            .map(|(i, c)| Binary::new(device, &camera_buffer, &intersection_texture, &c.expr, i as u32))
            .collect();
        let connect = Connect::new(&device, &intersection_texture_view, &segment_texture_view);
        let write = Write::new(&device, &curves_buffer, &segment_texture_view, dst_format);

        Self {
            intersection_texture,
            segment_texture,
            camera_buffer,
            curves_buffer,

            binaries,
            connect,
            write,

            dst_size,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        curves: &Vec<jihe_shared::Curve>,
        camera: &Camera,
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

        if self.binaries.len() != curves.len() {
            self.curves_buffer.destroy();
            self.curves_buffer = create_curves_buffer(device, curves.len());
        }

        let resized = self.dst_size != dst_size;
        if resized {
            self.dst_size = dst_size;

            self.intersection_texture.destroy();
            self.intersection_texture = create_intersection_texture(&device, dst_size, self.binaries.len());
            let intersection_texture_view = self
                .intersection_texture
                .create_view(&INTERSECTION_TEXTURE_VIEW_DESCRIPTOR);
            self.segment_texture.destroy();
            self.segment_texture = create_segment_texture(&device, dst_size, self.binaries.len());
            let segment_texture_view = self
                .segment_texture
                .create_view(&SEGMENT_TEXTURE_VIEW_DESCRIPTOR);

            self.connect
                .remake_bind_group(&device, &intersection_texture_view, &segment_texture_view);
            self.write
                .remake_bind_group(&device, &self.curves_buffer, &segment_texture_view);
        }

        // TODO: need test in the future when dynamic scene is implemented
        let mut previous = mem::replace(&mut self.binaries, Vec::with_capacity(curves.len()));
        for (layer, curve) in curves.iter().enumerate() {
            match previous.iter().position(|e| e.expr() == curve.expr) {
                Some(index) => {
                    let binary = self.binaries.push_mut(previous.remove(index));
                    if index != layer || resized {
                        binary.remake_bind_group(
                            device,
                            &self.camera_buffer,
                            &self.intersection_texture,
                            layer as u32,
                        );
                    }
                }
                None => {
                    self.binaries.push(Binary::new(
                        device,
                        &self.camera_buffer,
                        &self.intersection_texture,
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
                    thickness: c.thickness,
                    color: c.color,
                })
                .collect::<Vec<_>>()
                .as_dynamic_storage_bytes(),
        );
    }

    pub(super) fn compute(&self, compute_pass: &mut super::ComputePass, dst_size: (u32, u32)) {
        let layers = self.binaries.len() as u32;
        #[cfg(not(feature = "profile"))]
        for binary in &self.binaries {
            binary.compute(compute_pass, dst_size);
        }
        #[cfg(feature = "profile")]
        for (layer, binary) in self.binaries.iter().enumerate() {
            let _ = compute_pass.scope(format!("Curve binary {}", layer));
            binary.compute(compute_pass, dst_size);
        }

        {
            #[cfg(feature = "profile")]
            let _ = compute_pass.scope("Curve segment");
            self.connect.compute(compute_pass, dst_size, layers);
        }
    }

    pub(super) fn render(&self, render_pass: &mut super::RenderPass) {
        let layers = self.binaries.len() as u32;
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Curve write");
        self.write.render(render_pass, layers);
    }
}

#[inline]
fn create_intersection_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layers: usize,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Intersection Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0 + 1,
            height: dst_size.1 + 1,
            depth_or_array_layers: layers.max(1) as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

const INTERSECTION_TEXTURE_VIEW_DESCRIPTOR: wgpu::TextureViewDescriptor = wgpu::TextureViewDescriptor {
    label: Some("Intersection Texture View"),
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
fn create_segment_texture(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    layers: usize,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Segment Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: layers.max(1) as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

const SEGMENT_TEXTURE_VIEW_DESCRIPTOR: wgpu::TextureViewDescriptor = wgpu::TextureViewDescriptor {
    label: Some("Segment Texture View"),
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
struct CameraUniform {
    scale: f32,
    pos: glam::Vec2,
}

#[derive(encase::ShaderType)]
struct CurveUniform {
    thickness: f32,
    color: glam::Vec4,
}
