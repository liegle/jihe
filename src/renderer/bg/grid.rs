use std::borrow::Cow;

use encase::ShaderSize as _;

use crate::renderer::{
    bg::Bounds,
    buffer::{AsDynamicStorageBytes, AsUniformBytes},
};

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
        let vertex_buffer = create_vertex_buffer(device, glam::Vec2::SHADER_SIZE.get() * 2);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: glam::Vec3::SHADER_SIZE.get(),
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
        distance: f32,
        screen_bounds_ws: Bounds,
        grid_ends_cs: Bounds,
        color: glam::Vec3,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        let (w, h) = (screen_bounds_ws.w(), screen_bounds_ws.h());
        let mut vertices = Vec::<glam::Vec2>::new();

        let mut x = (screen_bounds_ws.l / distance).ceil() * distance;
        while x < screen_bounds_ws.r {
            let x_cs = 2. * (x - screen_bounds_ws.l) / w - 1.;
            vertices.extend(&[
                glam::vec2(x_cs, grid_ends_cs.b),
                glam::vec2(x_cs, grid_ends_cs.t),
            ]);
            x += distance;
        }

        let mut y = (screen_bounds_ws.b / distance).ceil() * distance;
        while y < screen_bounds_ws.t {
            let y_cs = 2. * (y - screen_bounds_ws.b) / h - 1.;
            vertices.extend(&[
                glam::vec2(grid_ends_cs.l, y_cs),
                glam::vec2(grid_ends_cs.r, y_cs),
            ]);
            y += distance;
        }

        self.vertex_count = vertices.len() as u32;
        let vertex_buffer_size = vertices.len() as u64 * glam::Vec2::SHADER_SIZE.get();
        if self.vertex_buffer.size() < vertex_buffer_size {
            self.vertex_buffer.destroy();
            self.vertex_buffer = create_vertex_buffer(device, vertex_buffer_size);
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

fn create_vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
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
                array_stride: glam::Vec2::SHADER_SIZE.get(),
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
