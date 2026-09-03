use std::{borrow::Cow, mem};

const SHADER_BASE: &str = include_str!("binary.wgsl");
const FN_START: &str = "\nfn f(x: f32, y: f32) -> f32 { return ";
const FN_END: &str = "; }";
const COMPUTE_ENTRY: Option<&str> = Some("cs");
const COMPUTE_WORKGROUP_SIZE: (u32, u32, u32) = (16, 16, 1);

pub(super) struct Binary {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    compute_pipelines: Vec<Pipeline>,
}

struct Pipeline {
    expr: String,
    compute_pipeline: wgpu::ComputePipeline,
}

impl Binary {
    pub(super) fn new(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        intersection_texture_view: &wgpu::TextureView,
        curves: &[jihe_shared::Curve],
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BIND_GROUP_LAYOUT_DESCRIPTOR);
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            camera_buffer,
            intersection_texture_view,
        );
        let compute_pipelines = curves
            .iter()
            .map(|curve| {
                let expr = curve.expr.to_owned();
                let compute_pipeline = create_compute_pipeline(device, &bind_group_layout, &expr);
                Pipeline {
                    expr,
                    compute_pipeline,
                }
            })
            .collect();
        Self {
            bind_group_layout,
            bind_group,
            compute_pipelines,
        }
    }

    pub(super) fn prepare(&mut self, device: &wgpu::Device, curves: &[jihe_shared::Curve]) {
        // TODO: need test in the future when dynamic scene is implemented
        let mut previous = mem::replace(
            &mut self.compute_pipelines,
            Vec::with_capacity(curves.len()),
        );
        for curve in curves.iter() {
            self.compute_pipelines
                .push(match previous.iter().position(|p| p.expr == curve.expr) {
                    Some(index) => previous.remove(index),
                    None => {
                        let expr = curve.expr.to_owned();
                        let compute_pipeline =
                            create_compute_pipeline(device, &self.bind_group_layout, &expr);
                        Pipeline {
                            expr,
                            compute_pipeline,
                        }
                    }
                });
        }
    }

    pub(super) fn remake_bind_group(
        &mut self,
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        intersection_texture_view: &wgpu::TextureView,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            camera_buffer,
            intersection_texture_view,
        )
    }

    pub(super) fn compute(
        &self,
        compute_pass: &mut wgpu::ComputePass,
        dst_size: (u32, u32),
        index: usize,
    ) {
        compute_pass.set_pipeline(&self.compute_pipelines[index].compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.dispatch_workgroups(
            (dst_size.0 + 1).div_ceil(COMPUTE_WORKGROUP_SIZE.0),
            (dst_size.1 + 1).div_ceil(COMPUTE_WORKGROUP_SIZE.1),
            1,
        );
    }
}

const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Binary Bind Group Layout"),
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    };

#[inline]
fn create_bind_group(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    intersection_texture_view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("Binary Bind Group")),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(intersection_texture_view),
            },
        ],
    })
}

#[inline]
fn create_compute_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    expr: &str,
) -> wgpu::ComputePipeline {
    let source = String::from_iter([SHADER_BASE, FN_START, expr, FN_END]);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("Binary \"{expr}\" Shader")),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("Binary \"{expr}\" Pipeline Layout")),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&format!("Binary \"{expr}\" Compute Pipeline")),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: COMPUTE_ENTRY,
        compilation_options: Default::default(),
        cache: None,
    })
}
