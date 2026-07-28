use std::borrow::Cow;

use encase::ShaderType;

use crate::renderer::buffer::{AsDynamicStorageBytes, AsUniformBytes};

const SHADER: &str = include_str!("line.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY: Option<&str> = Some("vs");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub struct Grid {
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl Grid {
    pub fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec2::min_size().get() * 2,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec3::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(device, &bind_group_layout, &uniform_buffer);
        let render_pipeline = create_render_pipeline(device, &bind_group_layout, dst_format);
        Self {
            bind_group,
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            vertex_count: 2,
        }
    }

    pub fn prepare(
        &mut self,
        spacing: i32,
        half_size: (i32, i32),
        axis_pos: (i32, i32),
        h_ends: (f32, f32),
        v_ends: (f32, f32),
        color: glam::Vec3,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let h_begin = ((-half_size.0 - axis_pos.0) / spacing) * spacing + axis_pos.0;
        let h_count = (half_size.0 - h_begin) / spacing + 1;
        let v_begin = ((-half_size.1 - axis_pos.1) / spacing) * spacing + axis_pos.1;
        let v_count = (half_size.1 - v_begin) / spacing + 1;
        let mut vertices = Vec::<glam::Vec2>::new();
        for i in 0..h_count {
            let x = ((h_begin + spacing * i) as f32) / (half_size.0 as f32);
            vertices.extend(&[glam::vec2(x, h_ends.0), glam::vec2(x, h_ends.1)])
        }
        for i in 0..v_count {
            let y = ((v_begin + spacing * i) as f32) / (half_size.1 as f32);
            vertices.extend(&[glam::vec2(v_ends.0, y), glam::vec2(v_ends.1, y)])
        }
        self.vertex_count = vertices.len() as u32;
        let vertex_buffer_size = vertices.len() as u64 * glam::Vec2::min_size().get();
        if self.vertex_buffer.size() < vertex_buffer_size {
            self.vertex_buffer.destroy();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: vertex_buffer_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, &vertices.as_dynamic_storage_bytes());
        queue.write_buffer(&self.uniform_buffer, 0, &color.as_uniform_bytes());
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..self.vertex_count, 0..1);
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
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                        dst_factor: wgpu::BlendFactor::Zero,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
