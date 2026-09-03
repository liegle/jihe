use std::borrow::Cow;

const SHADER: &str = include_str!("connect.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("Connect Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const COMPUTE_ENTRY: Option<&str> = Some("cs");
const COMPUTE_WORKGROUP_SIZE: (u32, u32, u32) = (16, 16, 1);

pub(super) struct Connect {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
}

impl Connect {
    pub(super) fn new(
        device: &wgpu::Device,
        intersection_texture_view: &wgpu::TextureView,
        segment_texture_view: &wgpu::TextureView,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            intersection_texture_view,
            segment_texture_view,
        );
        let compute_pipeline = create_compute_pipeline(&device, &bind_group_layout);
        Self {
            bind_group_layout,
            bind_group,
            compute_pipeline,
        }
    }

    pub(super) fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        intersection_texture_view: &wgpu::TextureView,
        segment_texture_view: &wgpu::TextureView,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            intersection_texture_view,
            segment_texture_view,
        );
    }

    pub(super) fn compute(
        &self,
        compute_pass: &mut wgpu::ComputePass,
        dst_size: (u32, u32),
        layer: u32,
    ) {
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.set_immediates(0, &layer.to_ne_bytes());
        compute_pass.dispatch_workgroups(
            dst_size.0.div_ceil(COMPUTE_WORKGROUP_SIZE.0),
            dst_size.1.div_ceil(COMPUTE_WORKGROUP_SIZE.1),
            1,
        );
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Connect Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
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

#[inline]
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    intersection_texture_view: &wgpu::TextureView,
    segment_texture_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Connect Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(intersection_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(segment_texture_view),
            },
        ],
    })
}

#[inline]
fn create_compute_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(SHADER_MODULE_DESCRIPTOR);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Connect Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 4,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Connect Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: COMPUTE_ENTRY,
        compilation_options: Default::default(),
        cache: None,
    })
}
