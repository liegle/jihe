use std::{
    mem,
    sync::Arc,
};

use crate::renderer::Renderer;

mod renderer;

fn main() {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = App::Uninitialized;
    event_loop.run_app(&mut app).unwrap();
}

enum App {
    Uninitialized,
    Ready { renderer: Renderer },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { .. } = self {
            return;
        }
        let window = match event_loop.create_window(Default::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Can't create window: {}", e);
                return;
            }
        };
        let window = Arc::new(window);
        let size = window.inner_size().into();
        let renderer = Renderer::new(window, size);
        *self = App::Ready { renderer };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Ready { renderer } = self else {
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
        if let App::Ready { renderer } = mem::replace(self, App::Uninitialized) {
            renderer.join();
        }
    }
}
