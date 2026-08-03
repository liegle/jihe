use std::{
    io, iter,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

#[cfg(feature = "profile")]
use crate::renderer::profile::Profiler;
use crate::{
    renderer::{bg::Bg, curve::Curve, point::Point, schedule::Scheduler},
    scene::SceneData,
};

mod bg;
mod buffer;
mod curve;
mod point;
#[cfg(feature = "profile")]
mod profile;
mod schedule;

enum Task {
    Exit,
    Draw,
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
            log::info!("Created inner renderer");
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

    pub fn exit(&self) {
        self.send(Task::Exit);
    }

    pub fn draw(&self) {
        self.send(Task::Draw);
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        if size.0 > 0 && size.1 > 0 && size != self.size {
            self.size = size;
            self.send(Task::Resize(size));
        }
    }

    fn send(&self, task: Task) {
        if let Err(_) = self.sender.send(task) {
            log::error!("Render task receiver has been closed")
        }
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
        if receiver.is_closed() {
            log::error!("Render task receiver has been closed");
            break;
        }
        tokio::select! {
            task = receiver.recv() => {
                match task {
                    None => {
                        log::error!("Render task channel has been closed");
                        break;
                    }
                    Some(Task::Exit) => {
                        break;
                    }
                    Some(Task::Draw) => {
                        if let Some(_) = render_scheduler.push_task(()) {
                            renderer.draw();
                        }
                    }
                    Some(Task::Resize(size)) => {
                        if let Some(size) = resize_scheduler.push_task(size) {
                            renderer.resize(size);
                            if let Err(e) = sender.send(Task::Draw) {
                                log::error!("{e}");
                                log::error!("Render task channel has been closed");
                                break;
                            }
                        }
                    }
                }
            }
            Some(_) = render_scheduler.sleep() => {
                renderer.draw();
            },
            Some(size) = resize_scheduler.sleep() => {
                renderer.resize(size);
                if let Err(e) = sender.send(Task::Draw) {
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
        type CommandEncoder<'a> = wgpu_profiler::Scope<'a, wgpu::CommandEncoder>;
    }
    _ => {
        type ComputePass<'a> = wgpu::ComputePass<'a>;
        type RenderPass<'a> = wgpu::RenderPass<'a>;
        type CommandEncoder = wgpu::CommandEncoder;
    }
}

struct Inner {
    scene: Arc<Mutex<SceneData>>,
    window: Arc<winit::window::Window>,

    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    bg: Bg,
    curve: Curve,
    point: Point,

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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        log::info!("Surface config: {config:?}");

        let bg = Bg::new(&device, surface_format);
        let curve = Curve::new(
            &device,
            &scene.lock().unwrap().curves,
            surface_format,
            window.inner_size().into(),
        );
        let point = Point::new(&device, &scene.lock().unwrap().points, surface_format);

        #[cfg(feature = "profile")]
        let profiler = Profiler::new(&device, 180);

        Ok(Self {
            scene,
            window,

            instance,
            surface,
            device,
            queue,
            config,

            bg,
            curve,
            point,

            #[cfg(feature = "profile")]
            profiler,
        })
    }

    fn resize(&mut self, size: (u32, u32)) {
        self.config.width = size.0;
        self.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    fn draw(&mut self) {
        let Some(output) = self.get_surface_texture() else {
            return;
        };

        let view = output.texture.create_view(&SURFACE_VIEW_DESCRIPTOR);
        let dst_size = (view.texture().width(), view.texture().height());

        let mut encoder = self
            .device
            .create_command_encoder(&COMMAND_ENCODER_DECRIPTOR);

        '_lock_scene: {
            let scene = self.scene.lock().unwrap();

            self.bg.prepare(
                &self.device,
                &self.queue,
                &scene.bg,
                &scene.camera,
                dst_size,
            );
            self.curve.prepare(
                &self.device,
                &self.queue,
                &scene.curves,
                &scene.camera,
                dst_size,
            );
            self.point.prepare(
                &self.device,
                &self.queue,
                &scene.points,
                &scene.camera,
                dst_size,
            );
            '_profile_scope: {
                #[cfg(feature = "profile")]
                let mut encoder = self.profiler.scope("Encode", &mut encoder);
                '_compute_pass: {
                    let mut compute_pass = create_compute_pass(&mut encoder);
                    self.curve
                        .compute(&mut compute_pass, dst_size, scene.curves.len() as u32);
                }
                '_render_pass: {
                    let mut render_pass = create_render_pass(&mut encoder, &view, scene.bg.color);
                    self.bg.render(&mut render_pass);
                    self.curve
                        .render(&mut render_pass, scene.curves.len() as u32);
                    self.point.render(&mut render_pass);
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

    #[inline]
    fn get_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                log::warn!("Surface texture is suboptimal");
                drop(tex);
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                log::warn!("Surface texture is timeout or occluded");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                log::warn!("Surface texture is oudated");
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                log::warn!("Surface texture is lost");
                if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
                    self.surface = surface;
                    self.surface.configure(&self.device, &self.config);
                }
                return None;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                unreachable!("Wgpu example says its unreachable so");
            }
        };
        Some(output)
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

const SURFACE_VIEW_DESCRIPTOR: wgpu::TextureViewDescriptor = wgpu::TextureViewDescriptor {
    label: Some("Dst Texture View"),
    format: None,
    dimension: Some(wgpu::TextureViewDimension::D2),
    usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
    aspect: wgpu::TextureAspect::All,
    base_mip_level: 0,
    mip_level_count: None,
    base_array_layer: 0,
    array_layer_count: None,
};

const COMMAND_ENCODER_DECRIPTOR: wgpu::CommandEncoderDescriptor = wgpu::CommandEncoderDescriptor {
    label: Some("Command Encoder"),
};

#[inline]
fn create_compute_pass<'a>(encoder: &'a mut CommandEncoder) -> ComputePass<'a> {
    cfg_select! {
        feature = "profile" => {
            encoder.scoped_compute_pass("Compute Pass")
        }
        _ => {
            encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            })
        }
    }
}

#[inline]
fn create_render_pass<'a>(
    encoder: &'a mut CommandEncoder,
    view: &'a wgpu::TextureView,
    clear: glam::Vec3,
) -> RenderPass<'a> {
    let render_pass_descriptor = wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear.x as f64,
                    g: clear.y as f64,
                    b: clear.z as f64,
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
    cfg_select! {
        feature = "profile" => {
            encoder.scoped_render_pass("Render Pass", render_pass_descriptor)
        }
        _ => {
            encoder.begin_render_pass(&render_pass_descriptor)
        }
    }
}
