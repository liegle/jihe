use std::borrow::Cow;

const SHADER: &str = include_str!("trace.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const COMPUTE_ENTRY: Option<&str> = Some("cs");
const COMPUTE_WORKGROUP_SIZE: (u32, u32, u32) = (16, 16, 1);

pub struct Trace {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
}

impl Trace {
    pub fn new(
        device: &wgpu::Device,
        curves_buffer: &wgpu::Buffer,
        residual_texture_view: &wgpu::TextureView,
        color_texture_view: &wgpu::TextureView,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            curves_buffer,
            residual_texture_view,
            color_texture_view,
        );
        let compute_pipeline = create_compute_pipeline(&device, &bind_group_layout);
        Self {
            bind_group_layout,
            bind_group,
            compute_pipeline,
        }
    }

    pub fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        curves_buffer: &wgpu::Buffer,
        residual_texture_view: &wgpu::TextureView,
        color_texture_view: &wgpu::TextureView,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            curves_buffer,
            residual_texture_view,
            color_texture_view,
        );
    }

    pub fn render(
        &self,
        compute_pass: &mut wgpu::ComputePass,
        dst_size: (u32, u32),
        layer_count: u32,
    ) {
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(
            dst_size.0.div_ceil(COMPUTE_WORKGROUP_SIZE.0),
            dst_size.1.div_ceil(COMPUTE_WORKGROUP_SIZE.1),
            layer_count,
        );
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    format: wgpu::TextureFormat::R32Sint,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D3,
                },
                count: None,
            },
        ],
    };

fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    curves_buffer: &wgpu::Buffer,
    residual_texture_view: &wgpu::TextureView,
    color_texture_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: curves_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(residual_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(color_texture_view),
            },
        ],
    })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(SHADER_MODULE_DESCRIPTOR);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: COMPUTE_ENTRY,
        compilation_options: Default::default(),
        cache: None,
    })
}
