use std::{
    iter,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use encase::ShaderType as _;

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
        size: (u32, u32),
    ) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = {
            let sender = sender.clone();
            thread::spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .build()
                    .unwrap()
                    .block_on(async {
                        match Inner::new(scene, window, size).await {
                            Ok(renderer) => run(renderer, sender, receiver).await,
                            Err(err) => {
                                log::error!("Can't create renderer: {}", err);
                            }
                        };
                    });
            })
        };
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

async fn run(
    mut renderer: Inner,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
) {
    // TODO: from config
    const REDRAW_INTERVAL: tokio::time::Duration =
        tokio::time::Duration::from_millis((1000. / 60.) as u64);
    const RESIZE_INTERVAL: tokio::time::Duration =
        tokio::time::Duration::from_millis((1000. / 10.) as u64);

    let mut render_scheduler = Scheduler::new(REDRAW_INTERVAL);
    let mut resize_scheduler = Scheduler::new(RESIZE_INTERVAL);

    loop {
        tokio::select! {
            task = receiver.recv() => {
                match task.unwrap() {
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
                            sender.send(Task::Render).unwrap();
                        }
                    }
                }
            }
            Some(_) = render_scheduler.sleep() => {
                renderer.render();
            },
            Some(size) = resize_scheduler.sleep() => {
                renderer.resize(size);
                sender.send(Task::Render).unwrap();
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
        size: (u32, u32),
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
                label: None,
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
            size: scene::Camera::min_size().get(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = Bg::new(&device, surface_format);
        let curve = {
            let scene = &scene.lock().unwrap();
            Curve::new(&scene.curves, &device, &camera_buffer, surface_format, size)
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
            self.curve
                .dst_resize(&self.device, size, &self.camera_buffer);
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
        let dst_size = (view.texture().width(), view.texture().height());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        '_lock_scene: {
            let scene = self.scene.lock().unwrap();

            self.queue
                .write_buffer(&self.camera_buffer, 0, &scene.camera.as_uniform_bytes());
            self.bg.prepare(&scene.bg, &scene.camera, &self.queue, dst_size);
            self.curve.prepare(&scene.curves, &self.queue);
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
                                load: wgpu::LoadOp::Clear(scene.bg.color),
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
                    self.curve.render(scene.curves.len() as u32, &mut render_pass);
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
enum CreateRendererError {
    #[error("Failed to create surface, err: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to request adapter, err: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device, err: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}
