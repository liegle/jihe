use std::borrow::Cow;

const SHADER_BASE: &str = include_str!("curve_eval.wgsl");
const FN_START: &str = "\nfn f(x: f32, y: f32) -> f32 { return ";
const FN_END: &str = "; }";

fn source(function: &str) -> String {
    let mut source = String::from(SHADER_BASE);
    source.push_str(FN_START);
    source.push_str(function);
    source.push_str(FN_END);
    source
}

pub struct CurveEval {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    slice: u32,
    global_buffer: wgpu::Buffer,
}

impl CurveEval {
    pub fn new(
        function: &str,
        slice: u32,
        device: &wgpu::Device,
        global_buffer: &wgpu::Buffer,
        offset_texture: &wgpu::Texture,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        format: wgpu::TextureFormat::R32Sint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = bind_group(
            &device,
            &bind_group_layout,
            &offset_texture,
            slice,
            &global_buffer,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&source(function))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            slice,
            global_buffer: global_buffer.clone(),
        }
    }

    pub fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        offset_texture: &wgpu::Texture,
        slice: u32,
    ) {
        self.bind_group = bind_group(
            &device,
            &self.bind_group_layout,
            &offset_texture,
            slice,
            &self.global_buffer,
        )
    }

    pub fn render(&self, compute_pass: &mut wgpu::ComputePass, size: (u32, u32)) {
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(size.0.div_ceil(16), size.1.div_ceil(16), 1);
    }
}

fn bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    offset_texture: &wgpu::Texture,
    slice: u32,
    global_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let offset_texture_view = offset_texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(offset_texture.format()),
        dimension: Some(wgpu::TextureViewDimension::D2),
        usage: Some(wgpu::TextureUsages::STORAGE_BINDING),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: slice,
        array_layer_count: Some(1),
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &global_buffer,
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
