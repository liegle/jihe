// TODO: add logs

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
    Ready {
        scene: Scene,
        window: Arc<winit::window::Window>,
        renderer: Renderer,
    },
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
                log::error!("Can't create window because:\n{e}");
                return;
            }
        };
        let window = Arc::new(window);
        let renderer = match Renderer::new(
            scene.data.clone(),
            window.clone(),
            scene.config.render_per_sec,
            scene.config.resize_per_sec,
        ) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Can't create renderer because:\n{e}");
                return;
            }
        };
        *self = App::Ready {
            scene,
            window,
            renderer,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Ready {
            scene,
            window,
            renderer,
        } = self
        else {
            return;
        };
        use winit::event::WindowEvent;
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                renderer.exit()
            }
            WindowEvent::RedrawRequested => renderer.render(),
            WindowEvent::Resized(size) => renderer.resize(size.into()),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if scene.handle_keyboard_input(&event) {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                if scene.handle_cursor_moved(&position) {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                use winit::window::{Cursor, CursorIcon};
                if scene.handle_mouse_input(&state, &button) {
                    window.set_cursor(Cursor::Icon(CursorIcon::Grabbing));
                } else {
                    window.set_cursor(Cursor::Icon(CursorIcon::Default));
                }
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase,
            } => {
                if scene.handle_mouse_wheel(&delta, &phase) {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready {
            scene: _,
            window: _,
            renderer,
        } = mem::replace(self, App::Uninitialized)
        {
            // TODO: how to handle this?
            if let Err(_) = renderer.join() {
                log::error!("Render thread is found panicked when exiting");
            }
        }
    }
}
