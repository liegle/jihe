use std::{
    mem,
    sync::{Arc, Mutex},
};

use crate::{renderer::Renderer, scene::{Curve, Scene}};

mod renderer;
mod scene;

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = App::Uninitialized;
    event_loop.run_app(&mut app).unwrap();
}

enum App {
    Uninitialized,
    Ready {
        scene: Arc<Mutex<Scene>>,
        renderer: Renderer,
    },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { .. } = self {
            return;
        }

        let scene = Arc::new(Mutex::new(Scene {
            scale: 0.01,
            pos: glam::Vec2::ZERO,
            curves: vec![
                Curve {
                    thickness: 2,
                    color: glam::vec4(1., 0., 0., 1.),
                    expr: "pow(x, x) + pow(2, y) - 10".to_string(),
                },
                Curve {
                    thickness: 2,
                    color: glam::vec4(0., 0., 1., 1.),
                    expr: "y - 3".to_string(),
                },
                Curve {
                    thickness: 2,
                    color: glam::vec4(1., 1., 1., 1.),
                    expr: "pow(x, 3) + log(y) - 10".to_string(),
                }
            ]
        }));
        let window = match event_loop.create_window(Default::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Can't create window: {}", e);
                return;
            }
        };
        let window = Arc::new(window);
        let size = window.inner_size().into();
        let renderer = Renderer::new(scene.clone(), window, size);
        *self = App::Ready { scene, renderer };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Ready { scene, renderer } = self else {
            return;
        };
        match event {
            winit::event::WindowEvent::CloseRequested => {
                renderer.exit();
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                renderer.render();
            }
            winit::event::WindowEvent::Resized(size) => {
                renderer.resize(size.into());
            }
            _ => (),
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { scene: _, renderer } = mem::replace(self, App::Uninitialized) {
            renderer.join();
        }
    }
}
