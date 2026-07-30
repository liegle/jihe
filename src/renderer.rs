use std::{
    io, iter,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use encase::ShaderSize as _;

#[cfg(feature = "profile")]
use crate::renderer::profile::Profiler;
use crate::{
    renderer::{bg::Bg, buffer::AsUniformBytes, curve::Curve, schedule::Scheduler},
    scene::{self, SceneData},
};

mod bg;
mod buffer;
mod curve;
#[cfg(feature = "profile")]
mod profile;
mod schedule;

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
    pub fn new(
        scene: Arc<Mutex<SceneData>>,
        window: Arc<winit::window::Window>,
        render_per_sec: u64,
        resize_per_sec: u64,
    ) -> Result<Self, CreateRendererError> {
        let size = window.inner_size().into();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = {
            let sender = sender.clone();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()?;
            let renderer = rt.block_on(Inner::new(scene, window))?;
            thread::spawn(move || {
                rt.block_on(run(
                    renderer,
                    sender,
                    receiver,
                    render_per_sec,
                    resize_per_sec,
                ))
            })
        };
        Ok(Self {
            join_handle,
            sender,
            size,
        })
    }

    pub fn join(self) -> thread::Result<()> {
        self.join_handle.join()
    }

    pub fn exit(&self) -> Result<(), SendError> {
        self.sender.send(Task::Exit)?;
        Ok(())
    }

    pub fn render(&self) -> Result<(), SendError> {
        self.sender.send(Task::Render)?;
        Ok(())
    }

    pub fn resize(&mut self, size: (u32, u32)) -> Result<(), SendError> {
        if size != self.size {
            self.size = size;
            self.sender.send(Task::Resize(size))?;
        }
        Ok(())
    }
}

async fn run(
    mut renderer: Inner,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
    render_per_sec: u64,
    resize_per_sec: u64,
) {
    let mut render_scheduler = Scheduler::new(render_per_sec);
    let mut resize_scheduler = Scheduler::new(resize_per_sec);

    loop {
        tokio::select! {
            task = receiver.recv() => {
                let Some(task) = task else {
                    log::error!("Render task channel has been closed");
                    break;
                };
                match task {
                    Task::Exit => {
                        break;
                    }
                    Task::Render => {
                        if let Some(_) = render_scheduler.push_task(()) {
                            renderer.render();
                        }
                    }
                    Task::Resize(size) => {
                        if let Some(size) = resize_scheduler.push_task(size) {
                            renderer.resize(size);
                            if let Err(e) = sender.send(Task::Render) {
                                log::error!("{e}");
                                log::error!("Render task channel has been closed");
                                break;
                            }
                        }
                    }
                }
            }
            Some(_) = render_scheduler.sleep() => {
                renderer.render();
            },
            Some(size) = resize_scheduler.sleep() => {
                renderer.resize(size);
                if let Err(e) = sender.send(Task::Render) {
                    log::error!("{e}");
                    log::error!("Render task channel has been closed");
                    break;
                }
            },
            else => break,
        }
    }
}

cfg_select! {
    feature = "profile" => {
        type ComputePass<'a> = wgpu_profiler::OwningScope<'a, wgpu::ComputePass<'a>>;
        type RenderPass<'a> = wgpu_profiler::OwningScope<'a, wgpu::RenderPass<'a>>;
    }
    _ => {
        type ComputePass<'a> = wgpu::ComputePass<'a>;
        type RenderPass<'a> = wgpu::RenderPass<'a>;
    }
}

struct Inner {
    scene: Arc<Mutex<SceneData>>,

    instance: wgpu::Instance,
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,

    camera_buffer: wgpu::Buffer,
    bg: Bg,
    curve: Curve,

    #[cfg(feature = "profile")]
    profiler: Profiler,
}

impl Inner {
    async fn new(
        scene: Arc<Mutex<SceneData>>,
        window: Arc<winit::window::Window>,
    ) -> Result<Self, CreateRendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let required_features = cfg_select! {
             feature = "profile" => {
                 adapter.features() & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES
             }
             _ => {
                 wgpu::Features::empty()
             }
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features,
                required_limits: Default::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
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
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);
        log::info!("Surface config: {surface_config:?}");

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Buffer"),
            size: scene::Camera::SHADER_SIZE.get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = Bg::new(&device, surface_format);
        let curve = {
            let scene = &scene.lock().unwrap();
            Curve::new(
                &scene.curves,
                &device,
                &camera_buffer,
                surface_format,
                window.inner_size().into(),
            )
        };

        #[cfg(feature = "profile")]
        let profiler = Profiler::new(&device, 180);

        Ok(Self {
            scene,
            instance,
            window,
            surface,
            device,
            queue,
            surface_config,

            camera_buffer,
            bg,
            curve,

            #[cfg(feature = "profile")]
            profiler,
        })
    }

    fn resize(&mut self, size: (u32, u32)) {
        if size.0 > 0
            && size.1 > 0
            && (size.0 != self.surface_config.width || size.1 != self.surface_config.height)
        {
            self.surface_config.width = size.0;
            self.surface_config.height = size.1;
            self.surface.configure(&self.device, &self.surface_config);
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

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Dst Texture View"),
            format: Some(output.texture.format()),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });
        let dst_size = (view.texture().width(), view.texture().height());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        '_lock_scene: {
            let scene = self.scene.lock().unwrap();

            self.queue
                .write_buffer(&self.camera_buffer, 0, &scene.camera.as_uniform_bytes());
            self.bg.prepare(
                &scene.bg,
                &scene.camera,
                &self.device,
                &self.queue,
                dst_size,
            );
            self.curve.prepare(
                &scene.curves,
                &self.device,
                &self.queue,
                (self.surface_config.width, self.surface_config.height),
                &self.camera_buffer,
            );
            '_profile_scope: {
                #[cfg(feature = "profile")]
                let mut encoder = self.profiler.scope("Encode", &mut encoder);
                '_compute_pass: {
                    let mut compute_pass = cfg_select! {
                        feature = "profile" => {
                            encoder.scoped_compute_pass("Compute Pass")
                        }
                        _ => {
                            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("Compute Pass"),
                                timestamp_writes: None,
                            })
                        }
                    };
                    self.curve
                        .compute(scene.curves.len() as u32, &mut compute_pass, dst_size);
                }
                '_render_pass: {
                    let render_pass_descriptor = wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: scene.bg.color.x as f64,
                                    g: scene.bg.color.y as f64,
                                    b: scene.bg.color.z as f64,
                                    a: 0.,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    };
                    let mut render_pass = cfg_select! {
                        feature = "profile" => {
                            encoder.scoped_render_pass("Render Pass", render_pass_descriptor)
                        }
                        _ => {
                            encoder.begin_render_pass(&render_pass_descriptor)
                        }
                    };
                    self.bg.render(&mut render_pass);
                    self.curve
                        .render(scene.curves.len() as u32, &mut render_pass);
                }
            }
        }
        #[cfg(feature = "profile")]
        self.profiler.resolve_queries(&mut encoder);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();
        #[cfg(feature = "profile")]
        self.profiler.end_frame(&self.queue);
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CreateRendererError {
    #[error("Failed to create tokio runtime because:\n{0}")]
    CreateTokioRuntime(#[from] io::Error),
    #[error("Failed to create surface because:\n{0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to request adapter because:\n{0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device because:\n{0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

#[derive(thiserror::Error, Debug)]
#[error("Render task receiver has closed")]
pub struct SendError;

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SendError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self
    }
}
