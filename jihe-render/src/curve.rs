use encase::ShaderSize as _;

use crate::{
    Camera,
    curve::{binary::Binary, connect::Connect, measure::Measure, write::Write},
    utils::{AsDynamicStorageBytes as _, AsUniformBytes as _},
};

mod binary;
mod connect;
mod measure;
mod write;

pub(super) struct Curve {
    // Curve intersection with left and top line segment of each pixel
    // (normalized on the border, x for top or y=1 if no intersection,
    // z for left or w=1 if no intersection)
    intersection_texture_view: wgpu::TextureView,
    // Line segments in each pixel connected from intersections
    // (line ends in normalized pixel inner space)
    segment_texture_view: wgpu::TextureView,
    // 1 if in thickness * thicknesss area of line pixels
    mark_texture_view: wgpu::TextureView,
    // Final color output of every curve
    curve_texture_view: wgpu::TextureView,

    camera_buffer: wgpu::Buffer,
    curves_buffer: wgpu::Buffer,

    binary: Binary,
    connect: Connect,
    measure: Measure,
    write: Write,

    dst_size: (u32, u32),
    len: usize,
}

impl Curve {
    pub(super) fn new(
        device: &wgpu::Device,
        curves: &[jihe_shared::Curve],
        dst_format: wgpu::TextureFormat,
        dst_size: (u32, u32),
    ) -> Self {
        let len = curves.len();

        let intersection_texture_view = create_intersection_texture_view(device, dst_size);
        let segment_texture_view = create_segment_texture_view(device, dst_size);
        let mark_texture_view = create_mark_texture_view(device, dst_size);
        let curve_texture_view = create_curve_texture_view(device, dst_size, len);

        let camera_buffer = create_camera_buffer(device);
        let curves_buffer = create_curves_buffer(device, len);

        let binary = Binary::new(device, &camera_buffer, &intersection_texture_view, curves);
        let connect = Connect::new(
            device,
            &intersection_texture_view,
            &segment_texture_view,
            &mark_texture_view,
            &curves_buffer,
        );
        let measure = Measure::new(
            device,
            &segment_texture_view,
            &mark_texture_view,
            &curve_texture_view,
            &curves_buffer,
        );
        let write = Write::new(device, &curve_texture_view, dst_format);

        Self {
            intersection_texture_view,
            segment_texture_view,
            mark_texture_view,
            curve_texture_view,

            camera_buffer,
            curves_buffer,

            binary,
            connect,
            measure,
            write,

            dst_size,
            len,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        curves: &[jihe_shared::Curve],
        camera: &Camera,
        dst_size: (u32, u32),
    ) {
        //\\ Check
        let dst_resized = self.dst_size != dst_size;
        self.dst_size = dst_size;
        let len_changed = self.len != curves.len();
        self.len = curves.len();

        //\\ Remake texture
        if dst_resized {
            self.intersection_texture_view.texture().destroy();
            self.intersection_texture_view = create_intersection_texture_view(device, dst_size);
            self.segment_texture_view.texture().destroy();
            self.segment_texture_view = create_segment_texture_view(device, dst_size);
            self.mark_texture_view.texture().destroy();
            self.mark_texture_view = create_mark_texture_view(device, dst_size);
        }

        if dst_resized || len_changed {
            self.curve_texture_view.texture().destroy();
            self.curve_texture_view = create_curve_texture_view(device, dst_size, self.len);
        }

        //\\ Remake buffer
        if len_changed {
            self.curves_buffer.destroy();
            self.curves_buffer = create_curves_buffer(device, self.len);
        }

        //\\ Remake bind group
        self.binary.prepare(device, curves);
        if dst_resized {
            self.binary.remake_bind_group(
                device,
                &self.camera_buffer,
                &self.intersection_texture_view,
            );
        }

        if dst_resized || len_changed {
            self.connect.remake_bind_group(
                device,
                &self.intersection_texture_view,
                &self.segment_texture_view,
                &self.mark_texture_view,
                &self.curves_buffer,
            );
            self.measure.remake_bind_group(
                device,
                &self.segment_texture_view,
                &self.mark_texture_view,
                &self.curve_texture_view,
                &self.curves_buffer,
            );
            self.write
                .remake_bind_group(device, &self.curve_texture_view);
        }

        //\\ Write buffer
        queue.write_buffer(
            &self.camera_buffer,
            0,
            &CameraUniform {
                scale: camera.scale,
                pos: camera.pos,
            }
            .as_uniform_bytes(),
        );
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

    pub(super) fn compute(&self, compute_pass: &mut super::ComputePass) {
        for index in 0..self.len {
            '_binary: {
                #[cfg(feature = "profile")]
                let _ = compute_pass.scope(format!("Curve binary {}", index));
                self.binary.compute(compute_pass, self.dst_size, index);
            }
            '_connect: {
                #[cfg(feature = "profile")]
                let _ = compute_pass.scope(format!("Curve connect {}", index));
                self.connect.compute(compute_pass, self.dst_size, index);
            }
            '_measure: {
                #[cfg(feature = "profile")]
                let _ = compute_pass.scope(format!("Curve measure {}", index));
                self.measure.compute(compute_pass, self.dst_size, index);
            }
        }
    }

    pub(super) fn render(&self, render_pass: &mut super::RenderPass) {
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Curve write");
        self.write.render(render_pass, self.len);
    }
}

#[inline]
fn create_intersection_texture_view(
    device: &wgpu::Device,
    dst_size: (u32, u32),
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Intersection Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0 + 1,
            height: dst_size.1 + 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Intersection Texture View"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

#[inline]
fn create_segment_texture_view(device: &wgpu::Device, dst_size: (u32, u32)) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Segment Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Segment Texture View"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

#[inline]
fn create_mark_texture_view(device: &wgpu::Device, dst_size: (u32, u32)) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Mark Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Mark Texture View"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D2),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

#[inline]
fn create_curve_texture_view(
    device: &wgpu::Device,
    dst_size: (u32, u32),
    len: usize,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Curve Texture"),
        size: wgpu::Extent3d {
            width: dst_size.0,
            height: dst_size.1,
            depth_or_array_layers: len.max(1) as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Curve Texture View"),
        format: None,
        dimension: Some(wgpu::TextureViewDimension::D3),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    })
}

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
