use std::{borrow::Cow, iter};

use encase::ShaderType;
use thiserror;

#[derive(encase::ShaderType)]
struct GlobalConfig {
    pixel_delta: f32,
    pos: glam::Vec2,
    half_size: glam::Vec2,
}

#[derive(encase::ShaderType)]
struct CurveConfig {
    thickness: i32,
    color: glam::Vec4,
}

pub(crate) struct Renderer<W: Into<wgpu::SurfaceTarget<'static>>> {
    instance: wgpu::Instance,
    window: W,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,

    global_config_uniform_buffer: wgpu::Buffer,
    curve_render_pipeline: wgpu::RenderPipeline,
    curve_config_uniform_buffer: wgpu::Buffer,
    curve_bind_group: wgpu::BindGroup,
}

impl<W: Into<wgpu::SurfaceTarget<'static>> + Clone> Renderer<W> {
    pub(crate) async fn new(window: W) -> Result<Self, CreateRendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: Default::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 1,
            height: 1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };

        let global_config_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: GlobalConfig::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let curve_config_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: CurveConfig::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let curve_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
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
            });
        let curve_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &curve_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &global_config_uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &curve_config_uniform_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&curve_bind_group_layout)],
            immediate_size: 0,
        });
        let curve_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(surface_format.into())],
                }),
                multiview_mask: None,
                cache: None,
            });
        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            surface_config,
            is_surface_configured: false,

            global_config_uniform_buffer,
            curve_render_pipeline,
            curve_config_uniform_buffer,
            curve_bind_group,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub(crate) fn render(&mut self) {
        if !self.is_surface_configured {
            return;
        }

        let output = self.surface.get_current_texture();
        let output = match output {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                drop(tex);
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
                    self.surface = surface;
                    self.surface.configure(&self.device, &self.surface_config);
                }
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("Wgpu example says its unreachable so");
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let global_config = GlobalConfig {
            pixel_delta: 0.01,
            pos: glam::vec2(0., 0.),
            half_size: glam::vec2(
                output.texture.width() as f32 / 2.,
                output.texture.height() as f32 / 2.,
            ),
        };
        let curve_config = CurveConfig {
            thickness: 3,
            color: glam::vec4(1., 0., 0., 1.),
        };
        self.queue.write_buffer(
            &self.global_config_uniform_buffer,
            0,
            &global_config.as_wgsl_bytes().unwrap(),
        );
        self.queue.write_buffer(
            &self.curve_config_uniform_buffer,
            0,
            &curve_config.as_wgsl_bytes().unwrap(),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        '_encode: {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render_pass.set_pipeline(&self.curve_render_pipeline);
            render_pass.set_bind_group(0, Some(&self.curve_bind_group), &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CreateRendererError {
    #[error("Failed to create surface, err: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to request adapter, err: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device, err: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

trait AsWgslBytes {
    fn as_wgsl_bytes(&self) -> encase::internal::Result<Vec<u8>>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsWgslBytes for T {
    fn as_wgsl_bytes(&self) -> encase::internal::Result<Vec<u8>> {
        let mut buffer = encase::UniformBuffer::new(vec![]);
        buffer.write(self)?;
        Ok(buffer.into_inner())
    }
}
