use std::borrow::Cow;

const SHADER: &str = include_str!("write.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("Write Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY: Option<&str> = Some("vs");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub(super) struct Write {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl Write {
    pub(super) fn new(
        device: &wgpu::Device,
        curves_buffer: &wgpu::Buffer,
        segment_texture_view: &wgpu::TextureView,
        dst_format: wgpu::TextureFormat,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            curves_buffer,
            segment_texture_view,
        );
        let render_pipeline = create_render_pipeline(device, &bind_group_layout, dst_format);
        Self {
            bind_group_layout,
            bind_group,
            render_pipeline,
        }
    }

    pub(super) fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        curves_buffer: &wgpu::Buffer,
        segment_texture_view: &wgpu::TextureView,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            curves_buffer,
            segment_texture_view,
        );
    }

    pub(super) fn render(&self, render_pass: &mut wgpu::RenderPass, layer_count: u32) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, Some(&self.bind_group), &[]);
        render_pass.draw(0..4, 0..layer_count);
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Write Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D3,
                },
                count: None,
            },
        ],
    };

#[inline]
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    curves_buffer: &wgpu::Buffer,
    segment_texture_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Write Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: curves_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(segment_texture_view),
            },
        ],
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
        label: Some("Write Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Write Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: VERTEX_ENTRY,
            buffers: &[],
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
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
