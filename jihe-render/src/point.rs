use std::borrow::Cow;

use encase::ShaderSize;

use crate::{
    Camera,
    utils::{AsDynamicStorageBytes as _, AsUniformBytes as _},
};

const SHADER: &str = include_str!("point.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("Point Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY: Option<&str> = Some("vs");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub(super) struct Point {
    size_buffer: wgpu::Buffer,
    points_buffer: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,

    instance_count: u32,
}

impl Point {
    pub(super) fn new(
        device: &wgpu::Device,
        points: &[jihe_shared::Point],
        dst_format: wgpu::TextureFormat,
    ) -> Self {
        let size_buffer = create_size_buffer(device);
        let points_buffer = create_points_buffer(
            device,
            PointInstance::SHADER_SIZE.get() * points.len() as u64,
        );

        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(device, &bind_group_layout, &size_buffer);
        let render_pipeline = create_render_pipeline(device, &bind_group_layout, dst_format);

        Self {
            size_buffer,
            points_buffer,

            bind_group,
            render_pipeline,

            instance_count: points.len() as u32,
        }
    }

    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: &[jihe_shared::Point],
        camera: &Camera,
        dst_size: (u32, u32),
    ) {
        let half_size = glam::vec2(dst_size.0 as f32 / 2., dst_size.1 as f32 / 2.);
        queue.write_buffer(&self.size_buffer, 0, &half_size.as_uniform_bytes());
        let points = points
            .iter()
            .map(|p| PointInstance {
                pos: (p.pos - camera.pos) / camera.scale + glam::vec2(0.5, -0.5),
                size: p.size,
                color: p.color,
            })
            .filter(|p| {
                let overflow_x = (-half_size.x - p.size)..(half_size.x + p.size);
                let overflow_y = (-half_size.y - p.size)..(half_size.y + p.size);
                overflow_x.contains(&p.pos.x) && overflow_y.contains(&p.pos.y)
            })
            .collect::<Vec<_>>();
        self.instance_count = points.len() as u32;
        let points_buffer_size = self.instance_count as u64 * PointInstance::SHADER_SIZE.get();
        if self.points_buffer.size() < points_buffer_size {
            self.points_buffer.destroy();
            self.points_buffer = create_points_buffer(device, points_buffer_size);
        }
        queue.write_buffer(&self.points_buffer, 0, &points.as_dynamic_storage_bytes());
    }

    pub(super) fn render(&self, render_pass: &mut super::RenderPass) {
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Point");
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.points_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..self.instance_count);
    }
}

#[inline]
fn create_size_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Points Buffer"),
        size: glam::Vec2::SHADER_SIZE.get(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[inline]
fn create_points_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Points Buffer"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[derive(encase::ShaderType)]
struct PointInstance {
    pos: glam::Vec2,
    size: f32,
    color: glam::Vec4,
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Point Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    };

#[inline]
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    size_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Point Bind Group"),
        layout: bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: size_buffer.as_entire_binding(),
        }],
    })
}

#[inline]
fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    dst_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(SHADER_MODULE_DESCRIPTOR);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Point Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: VERTEX_ENTRY,
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: PointInstance::SHADER_SIZE.get(),
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: glam::Vec2::SHADER_SIZE.get(),
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: glam::Vec4::SHADER_SIZE.get(),
                        shader_location: 2,
                    },
                ],
            }],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: FRAGMENT_ENTRY,
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: dst_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::OVER,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
