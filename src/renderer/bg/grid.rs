use std::borrow::Cow;

const SHADER: &str = include_str!("grid.wgsl");
const SHADER_MODULE_DESCRIPTOR: wgpu::ShaderModuleDescriptor = wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
};
const VERTEX_ENTRY: Option<&str> = Some("vs");
const FRAGMENT_ENTRY: Option<&str> = Some("fs");

pub struct Grid {
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}
