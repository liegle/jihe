use std::borrow::Cow;

const SHADER_BASE: &str = include_str!("evaluate.wgsl");
const FN_START: &str = "\nfn f(x: f32, y: f32) -> f32 { return ";
const FN_END: &str = "; }";
const COMPUTE_ENTRY: Option<&str> = Some("cs");
const COMPUTE_WORKGROUP_SIZE: (u32, u32, u32) = (16, 16, 1);

pub struct Evaluate {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
    pub layer: u32,
}

impl Evaluate {
    pub fn new(
        function: &str,
        layer: u32,
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        residual_texture: &wgpu::Texture,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            camera_buffer,
            residual_texture,
            layer,
        );
        let compute_pipeline = create_compute_pipeline(device, &bind_group_layout, function);
        Self {
            bind_group_layout,
            bind_group,
            compute_pipeline,
            layer,
        }
    }

    pub fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        residual_texture: &wgpu::Texture,
        layer: u32,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            camera_buffer,
            residual_texture,
            layer,
        )
    }

    pub fn render(&self, compute_pass: &mut wgpu::ComputePass, dst_size: (u32, u32)) {
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(
            dst_size.0.div_ceil(COMPUTE_WORKGROUP_SIZE.0),
            dst_size.1.div_ceil(COMPUTE_WORKGROUP_SIZE.1),
            1,
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
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::R32Float,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    };

fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    residual_texture: &wgpu::Texture,
    layer: u32,
) -> wgpu::BindGroup {
    let offset_texture_view = residual_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(residual_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D2),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: layer,
        array_layer_count: Some(1),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: camera_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&offset_texture_view),
            },
        ],
    })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    function: &str,
) -> wgpu::ComputePipeline {
    let mut source = String::from(SHADER_BASE);
    source.push_str(FN_START);
    source.push_str(function);
    source.push_str(FN_END);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
    });
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
