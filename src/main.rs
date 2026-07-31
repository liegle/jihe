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
                log::error!("Can't create window because:\n{e}");
                return;
            }
        };
        let window = Arc::new(window);
        let renderer = match Renderer::new(
            scene.data.clone(),
            window,
            scene.config.render_per_sec,
            scene.config.resize_per_sec,
        ) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Can't create renderer because:\n{e}");
                return;
            }
        };
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
        if let Err(e) = match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
                renderer.exit()
            }
            winit::event::WindowEvent::RedrawRequested => renderer.render(),
            winit::event::WindowEvent::Resized(size) => renderer.resize(size.into()),
            winit::event::WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                // TODO: move send out of renderer to reduce duplicated code
                if scene.handle_keyboard_input(&event) {
                    renderer.render()
                } else {
                    Ok(())
                }
            }
            winit::event::WindowEvent::CursorMoved { device_id: _, position } => {
                if scene.handle_cursor_moved(&position) {
                    renderer.render()
                } else {
                    Ok(())
                }
            }
            winit::event::WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if scene.handle_mouse_input(&state, &button) {
                    renderer.render()
                } else {
                    Ok(())
                }
            }
            winit::event::WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase,
            } => {
                if scene.handle_mouse_wheel(&delta, &phase) {
                    renderer.render()
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        } {
            log::error!("{e}")
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { scene: _, renderer } = mem::replace(self, App::Uninitialized) {
            // TODO: how to handle this?
            if let Err(_) = renderer.join() {
                log::error!("Render thread is found panicked when exiting");
            }
        }
    }
}
