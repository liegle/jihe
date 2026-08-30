use std::{borrow::Cow, ops::Range};

use encase::ShaderSize;

use crate::{bg::Bounds, buffer::AsUniformBytes as _};

const SHADER: &str = include_str!("grid.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("Grid Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY_HORI: Option<&str> = Some("vs_hori");
const VERTEX_ENTRY_VERT: Option<&str> = Some("vs_vert");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub(super) struct Grid {
    color_buffer: wgpu::Buffer,

    hori: Lines,
    vert: Lines,
}

struct Lines {
    lines_buffer: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,

    count: u32,
}

impl Grid {
    pub(super) fn new(device: &wgpu::Device, dst_format: wgpu::TextureFormat) -> Self {
        let color_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Color Buffer"),
            size: glam::Vec3::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let hori = Lines::new(
            device,
            VERTEX_ENTRY_HORI,
            &bind_group_layout,
            &color_buffer,
            dst_format,
        );
        let vert = Lines::new(
            device,
            VERTEX_ENTRY_VERT,
            &bind_group_layout,
            &color_buffer,
            dst_format,
        );
        Self {
            color_buffer,
            hori,
            vert,
        }
    }

    pub(super) fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        distance: f32,
        screen_bounds_ws: Bounds,
        grid_ends_cs: Bounds,
        color: glam::Vec3,
        delta: glam::Vec2,
    ) {
        self.hori.prepare(
            queue,
            distance,
            screen_bounds_ws.b..screen_bounds_ws.t,
            glam::vec2(grid_ends_cs.l, grid_ends_cs.r),
            -delta.y,
        );
        self.vert.prepare(
            queue,
            distance,
            screen_bounds_ws.l..screen_bounds_ws.r,
            glam::vec2(grid_ends_cs.b, grid_ends_cs.t),
            delta.x,
        );
        queue.write_buffer(&self.color_buffer, 0, &color.as_uniform_bytes());
    }

    pub(super) fn render(&self, render_pass: &mut wgpu::RenderPass) {
        self.hori.render(render_pass);
        self.vert.render(render_pass);
    }
}

impl Lines {
    fn new(
        device: &wgpu::Device,
        vertex_entry: Option<&str>,
        bind_group_layout: &wgpu::BindGroupLayout,
        color_buffer: &wgpu::Buffer,
        dst_format: wgpu::TextureFormat,
    ) -> Self {
        let lines_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Lines Buffer"),
            size: LinesUniform::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group =
            create_bind_group(device, &bind_group_layout, &lines_buffer, &color_buffer);
        let render_pipeline =
            create_render_pipeline(device, &bind_group_layout, vertex_entry, dst_format);
        Self {
            lines_buffer,
            bind_group,
            render_pipeline,
            count: 0,
        }
    }

    fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        distance: f32,
        range: Range<f32>,
        ends: glam::Vec2,
        delta: f32,
    ) {
        let w = range.end - range.start;
        let begin_ws = (range.start / distance).ceil() * distance;
        self.count = (w / distance).ceil() as u32;
        let spacing = distance / w * 2.;
        let begin = (begin_ws - range.start) / w * 2. - 1. + delta;

        queue.write_buffer(
            &self.lines_buffer,
            0,
            &LinesUniform {
                begin,
                spacing,
                ends,
            }
            .as_uniform_bytes(),
        );
    }

    fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..2, 0..self.count);
    }
}

#[derive(encase::ShaderType)]
struct LinesUniform {
    begin: f32,
    spacing: f32,
    ends: glam::Vec2,
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Grid Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    };

#[inline]
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    lines_buffer: &wgpu::Buffer,
    color_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Grid Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: lines_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: color_buffer.as_entire_binding(),
            },
        ],
    })
}

#[inline]
fn create_render_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    vertex_entry: Option<&str>,
    dst_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(SHADER_MODULE_DESCRIPTOR);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Grid Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Grid Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: vertex_entry,
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
