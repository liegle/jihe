use std::borrow::Cow;

use encase::ShaderType;

use crate::{
    renderer::buffer::{AsDynamicStorageBytes, AsUniformBytes},
    scene,
};

const SHADER: &str = include_str!("color.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY: Option<&str> = Some("vs");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub struct Bg {
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    axis_buffer: wgpu::Buffer,
    grid_buffer: wgpu::Buffer,
}

impl Bg {
    pub fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec2::min_size().get() * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let axis_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec4::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec4::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(device, &bind_group_layout, &axis_buffer);
        let render_pipeline = create_render_pipeline(device, &bind_group_layout, dst_format);
        Self {
            bind_group,
            render_pipeline,
            vertex_buffer,
            axis_buffer,
            grid_buffer,
        }
    }

    pub fn prepare(
        &self,
        bg: &scene::Bg,
        camera: &scene::Camera,
        queue: &wgpu::Queue,
        dst_size: (u32, u32),
    ) {
        let size = (dst_size.0 as i32, dst_size.1 as i32);
        let half_size = (size.0 / 2, size.1 / 2);
        let axis_pos = (
            (-camera.pos.x / camera.scale) as i32,
            (-camera.pos.y / camera.scale) as i32,
        );

        let clamped_axis_pos = (
            ((axis_pos.0 as f32) / (half_size.0 as f32)).clamp(-1., 1.),
            ((axis_pos.1 as f32) / (half_size.1 as f32)).clamp(-1., 1.),
        );
        let vertices = vec![
            // y axis
            glam::vec2(clamped_axis_pos.0, -1.),
            glam::vec2(clamped_axis_pos.0, 1.),
            // x axis
            glam::vec2(-1., clamped_axis_pos.1),
            glam::vec2(1., clamped_axis_pos.1),
        ];
        queue.write_buffer(&self.vertex_buffer, 0, &vertices.as_dynamic_storage_bytes());
        if let Some(color) = bg.axis {
            queue.write_buffer(&self.axis_buffer, 0, &color.as_uniform_bytes());
        }
        if let Some(color) = bg.grid {
            queue.write_buffer(&self.grid_buffer, 0, &color.as_uniform_bytes());
        }
    }

    pub fn render(&self, render_pass: &mut super::RenderPass) {
        #[cfg(feature = "profile")]
        let _ = render_pass.scope("Axis");
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    };

fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    dst_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(SHADER_MODULE_DESCRIPTOR);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: VERTEX_ENTRY,
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: glam::Vec2::min_size().get(),
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
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
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
