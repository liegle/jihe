use std::iter;

use encase::ShaderType;
use thiserror;

use crate::curve::Curve;

#[derive(encase::ShaderType)]
pub struct GlobalConfig {
    pixel_delta: f32,
    pos: glam::Vec2,
    half_size: glam::Vec2,
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

    curve: Curve,
}

impl<W: Into<wgpu::SurfaceTarget<'static>> + Clone> Renderer<W> {
    pub(crate) async fn new(window: W, size: (u32, u32)) -> Result<Self, CreateRendererError> {
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
            width: size.0,
            height: size.1,
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

        let curve = Curve::new(&device, &global_config_uniform_buffer, surface_format, size);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            surface_config,
            is_surface_configured: false,

            global_config_uniform_buffer,
            curve,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.is_surface_configured = true;
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.curve.dst_resize(&self.device, (width, height));
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
        self.queue.write_buffer(
            &self.global_config_uniform_buffer,
            0,
            &global_config.as_uniform_bytes().unwrap(),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.curve.render(&self.queue, &mut encoder, &view);

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

pub trait AsUniformBytes {
    fn as_uniform_bytes(&self) -> encase::internal::Result<Vec<u8>>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsUniformBytes for T {
    fn as_uniform_bytes(&self) -> encase::internal::Result<Vec<u8>> {
        let mut buffer = encase::UniformBuffer::new(vec![]);
        buffer.write(self)?;
        Ok(buffer.into_inner())
    }
}

pub trait AsStorageBytes {
    fn as_storage_bytes(&self) -> encase::internal::Result<Vec<u8>>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsStorageBytes for T {
    fn as_storage_bytes(&self) -> encase::internal::Result<Vec<u8>> {
        let mut buffer = encase::DynamicStorageBuffer::new(vec![]);
        buffer.write(self)?;
        Ok(buffer.into_inner())
    }
}
