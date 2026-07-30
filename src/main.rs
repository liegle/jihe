// TODO: change pubs to pub(crate)s and pub(super)s

use std::{mem, sync::Arc};

use crate::{renderer::Renderer, scene::Scene};

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
    Ready { scene: Scene, renderer: Renderer },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { .. } = self {
            return;
        }

        let scene = Scene::new();
        let window = match event_loop.create_window(Default::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Can't create window: {}", e);
                return;
            }
        };
        let window = Arc::new(window);
        let size = window.inner_size().into();
        let renderer = Renderer::new(scene.data.clone(), window, size);
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
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if scene.handle_keyboard_input(&event) {
                    renderer.render();
                }
            }
            winit::event::WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase,
            } => {
                if scene.handle_mouse_wheel(&delta, &phase) {
                    renderer.render();
                }
            }
            // TODO: mouse control
            _ => (),
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { scene: _, renderer } = mem::replace(self, App::Uninitialized) {
            renderer.join();
        }
    }
}
