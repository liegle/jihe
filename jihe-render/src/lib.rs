use std::{
    iter,
    sync::{Arc, Mutex},
};

#[cfg(feature = "profile")]
use crate::profile::Profiler;
use crate::{bg::Bg, curve::Curve, point::Point};

pub use scene::{Camera, Scene};

mod bg;
mod buffer;
mod curve;
mod point;
mod scene;

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

pub struct Render<W>
where
    Arc<W>: Into<wgpu::SurfaceTarget<'static>>,
{
    scene: Arc<Mutex<Scene>>,
    window: Arc<W>,

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

impl<W> Render<W>
where
    Arc<W>: Into<wgpu::SurfaceTarget<'static>>,
{
    pub async fn new(
        scene: Arc<Mutex<Scene>>,
        window: Arc<W>,
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
        } | wgpu::Features::IMMEDIATES;
        let required_limits = wgpu::Limits {
            max_immediate_size: 4,
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features,
                required_limits,
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
            width: size.0,
            height: size.1,
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
            &scene.lock().unwrap().content.curves,
            surface_format,
            size,
        );
        let point = Point::new(
            &device,
            &scene.lock().unwrap().content.points,
            surface_format,
        );

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

    pub fn resize(&mut self, size: (u32, u32)) {
        self.config.width = size.0;
        self.config.height = size.1;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn draw(&mut self) {
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

            self.bg
                .prepare(&self.queue, &scene.content.bg, &scene.camera, dst_size);
            self.curve.prepare(
                &self.device,
                &self.queue,
                &scene.content.curves,
                &scene.camera,
                dst_size,
            );
            self.point.prepare(
                &self.device,
                &self.queue,
                &scene.content.points,
                &scene.camera,
                dst_size,
            );
            '_profile_scope: {
                #[cfg(feature = "profile")]
                let mut encoder = self.profiler.scope("Encode", &mut encoder);
                '_compute_pass: {
                    let mut compute_pass = create_compute_pass(&mut encoder);
                    self.curve.compute(&mut compute_pass, dst_size);
                }
                '_render_pass: {
                    let mut render_pass =
                        create_render_pass(&mut encoder, &view, scene.content.bg.color);
                    self.bg.render(&mut render_pass);
                    self.curve.render(&mut render_pass);
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

#[derive(thiserror::Error, Debug)]
pub enum CreateRendererError {
    #[error("Failed to create surface because:\n{0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("Failed to request adapter because:\n{0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("Failed to request device because:\n{0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}
