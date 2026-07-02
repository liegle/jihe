use std::{
    iter,
    thread::{self, JoinHandle},
};

use encase::ShaderType;
use thiserror;

#[cfg(feature = "profile")]
use crate::renderer::profile::Profiler;
use crate::renderer::{buffer::AsUniformBytes, curve::Curve};

mod buffer;
mod curve;
#[cfg(feature = "profile")]
mod profile;

#[derive(encase::ShaderType)]
pub struct Camera {
    pixel_delta: f32,
    pos: glam::Vec2,
}

enum Task {
    Exit,
    Render,
    Resize((u32, u32)),
}

pub struct Renderer {
    join_handle: JoinHandle<()>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    size: (u32, u32),
}

impl Renderer {
    pub fn new<W: Into<wgpu::SurfaceTarget<'static>> + Clone + Send + 'static>(
        window: W,
        size: (u32, u32),
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap()
                .block_on(run(window, size, receiver));
        });
        Self {
            join_handle,
            sender,
            size,
        }
    }

    pub fn join(self) {
        self.join_handle.join().unwrap();
    }

    pub fn exit(&self) {
        self.sender.send(Task::Exit).unwrap();
    }

    pub fn render(&self) {
        self.sender.send(Task::Render).unwrap();
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if size != self.size {
            self.size = size;
            self.sender.send(Task::Resize(size)).unwrap();
        }
    }
}

async fn run<W: Into<wgpu::SurfaceTarget<'static>> + Clone>(
    window: W,
    size: (u32, u32),
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
) {
    let mut renderer = match Inner::new(window, size).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Can't create renderer: {}", e);
            return;
        }
    };

    const REDRAW_INTERVAL: tokio::time::Duration =
        tokio::time::Duration::from_millis((1000. / 60.) as u64);
    const RESIZE_INTERVAL: tokio::time::Duration =
        tokio::time::Duration::from_millis((2000. / 1.) as u64);

    let mut next_render = tokio::time::Instant::now();
    let mut next_resize = next_render;

    let mut render_scheduled = false;
    let mut resize_scheduled = false;
    let mut next_size = (1, 1);

    loop {
        tokio::select! {
            task = receiver.recv() => {
                let now = tokio::time::Instant::now();
                match task.unwrap() {
                    Task::Exit => {
                        break;
                    }
                    Task::Render => {
                        if now > next_render {
                            renderer.render();
                            next_render = now + REDRAW_INTERVAL;
                        } else {
                            render_scheduled = true;
                        }
                    }
                    Task::Resize(size) => {
                        if now > next_render {
                            renderer.resize(size);
                            next_resize = now + RESIZE_INTERVAL;
                        } else {
                            resize_scheduled = true;
                            next_size = size;
                        }
                    }
                }
            }
            _ = tokio::time::sleep_until(next_render),
                if render_scheduled && next_render > tokio::time::Instant::now() => {
                renderer.render();
                next_render = tokio::time::Instant::now() + REDRAW_INTERVAL;
                render_scheduled = false;
            },
            _ = tokio::time::sleep_until(next_resize),
                if resize_scheduled && next_resize > tokio::time::Instant::now() => {
                renderer.resize(next_size);
                next_resize = tokio::time::Instant::now() + RESIZE_INTERVAL;
                resize_scheduled = false;
            },
            else => break,
        }
    }
}

struct Inner<W: Into<wgpu::SurfaceTarget<'static>>> {
    instance: wgpu::Instance,
    window: W,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    camera_buffer: wgpu::Buffer,
    curve: Curve,

    #[cfg(feature = "profile")]
    profiler: Profiler,
}

impl<W: Into<wgpu::SurfaceTarget<'static>> + Clone> Inner<W> {
    async fn new(window: W, size: (u32, u32)) -> Result<Self, CreateRendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        #[cfg(feature = "profile")]
        let required_features =
            adapter.features() & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES;
        #[cfg(not(feature = "profile"))]
        let required_features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: Default::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        log::info!("Surface capabilities: {surface_caps:?}");

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
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        log::info!("Surface config: {surface_config:?}");

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: Camera::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let curve = Curve::new(&device, &camera_buffer, surface_format, size);

        #[cfg(feature = "profile")]
        let profiler = Profiler::new(&device, 180);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            surface_config,

            camera_buffer,
            curve,

            #[cfg(feature = "profile")]
            profiler,
        })
    }

    fn resize(&mut self, size: (u32, u32)) {
        if size.0 > 0
            && size.1 > 0
            && size.0 != self.surface_config.width
            && size.1 != self.surface_config.height
        {
            self.surface_config.width = size.0;
            self.surface_config.height = size.1;
            self.surface.configure(&self.device, &self.surface_config);
            self.curve.dst_resize(&self.device, size);
        }
    }

    fn render(&mut self) {
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

        let view = output.texture.create_view(&Default::default());
        let camera = Camera {
            pixel_delta: 0.01,
            pos: glam::vec2(0., 0.),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, &camera.as_uniform_bytes().unwrap());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        #[cfg(feature = "profile")]
        self.profiler.encode(&mut encoder, &mut |scope| {
            self.curve.render(&self.queue, scope, &view);
        });
        #[cfg(not(feature = "profile"))]
        self.curve.render(&self.queue, &mut encoder, &view);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        #[cfg(feature = "profile")]
        self.profiler.end_frame(&self.queue);
    }
}

#[derive(thiserror::Error, Debug)]
enum CreateRendererError {
    #[error("Failed to create surface, err: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to request adapter, err: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device, err: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}
