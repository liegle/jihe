
use std::borrow::Cow;

const SHADER: &str = include_str!("measure.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: Some("Measure Shader"),
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const COMPUTE_ENTRY: Option<&str> = Some("cs");
const COMPUTE_WORKGROUP_SIZE: (u32, u32, u32) = (16, 16, 1);

pub(super) struct Measure {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
}

impl Measure {
    pub(super) fn new(
        device: &wgpu::Device,
        segment_texture_view: &wgpu::TextureView,
        mark_texture_view: &wgpu::TextureView,
        curve_texture_view: &wgpu::TextureView,
        curves_buffer: &wgpu::Buffer,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            segment_texture_view,
            mark_texture_view,
            curve_texture_view,
            curves_buffer,
        );
        let compute_pipeline = create_compute_pipeline(device, &bind_group_layout);
        Self {
            bind_group_layout,
            bind_group,
            compute_pipeline,
        }
    }

    pub(super) fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        segment_texture_view: &wgpu::TextureView,
        mark_texture_view: &wgpu::TextureView,
        curve_texture_view: &wgpu::TextureView,
        curves_buffer: &wgpu::Buffer,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            segment_texture_view,
            mark_texture_view,
            curve_texture_view,
            curves_buffer,
        );
    }

    pub(super) fn compute(
        &self,
        compute_pass: &mut wgpu::ComputePass,
        dst_size: (u32, u32),
        index: usize,
    ) {
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.set_immediates(0, &(index as u32).to_ne_bytes());
        compute_pass.dispatch_workgroups(
            dst_size.0.div_ceil(COMPUTE_WORKGROUP_SIZE.0),
            dst_size.1.div_ceil(COMPUTE_WORKGROUP_SIZE.1),
            1,
        );
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Measure Bind Group Layout"),
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
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    format: wgpu::TextureFormat::R32Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
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
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
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
    segment_texture_view: &wgpu::TextureView,
    mark_texture_view: &wgpu::TextureView,
    curve_texture_view: &wgpu::TextureView,
    curves_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Measure Bind Group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(segment_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(mark_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(curve_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: curves_buffer.as_entire_binding(),
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
        label: Some("Measure Pipeline Layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 4,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Measure Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: COMPUTE_ENTRY,
        compilation_options: Default::default(),
        cache: None,
    })
}
