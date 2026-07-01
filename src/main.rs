use std::{
    mem,
    sync::Arc,
    thread::{self, JoinHandle},
};

use crate::renderer::Task;

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
    Ready {
        join_handle: JoinHandle<()>,
        sender: tokio::sync::mpsc::UnboundedSender<Task>,
    },
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
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let window = Arc::new(window);
        let join_handle = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap()
                .block_on(renderer::run(
                    window.clone(),
                    window.inner_size().into(),
                    receiver,
                ));
        });
        *self = App::Ready {
            join_handle,
            sender,
        };
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Ready { sender, .. } = self else {
            return;
        };
        match event {
            winit::event::WindowEvent::CloseRequested => {
                sender.send(Task::Exit).unwrap();
                event_loop.exit();
            }
            winit::event::WindowEvent::RedrawRequested => {
                sender.send(Task::Render).unwrap();
            }
            winit::event::WindowEvent::Resized(size) => {
                sender.send(Task::Resize(size.into())).unwrap();
            }
            _ => (),
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Ready { join_handle, .. } = mem::replace(self, App::Uninitialized) {
            join_handle.join().unwrap();
        }
    }
}
