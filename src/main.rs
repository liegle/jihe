use std::{
    mem,
    sync::Arc,
    thread::{self, JoinHandle},
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
    Ready {
        join_handle: JoinHandle<()>,
        sender: tokio::sync::mpsc::UnboundedSender<Task>,
    },
}

enum Task {
    Exit,
    Redraw,
    Resize((u32, u32)),
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
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let window = Arc::new(window);
        let join_handle = thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .unwrap()
                .block_on(async move {
                    let mut renderer =
                        match Renderer::new(window.clone(), window.inner_size().into()).await {
                            Ok(r) => r,
                            Err(e) => {
                                log::error!("Can't create renderer: {}", e);
                                return;
                            }
                        };

                    const REDRAW_INTERVAL: tokio::time::Duration =
                        tokio::time::Duration::from_millis((1000. / 60.) as u64);
                    const RESIZE_INTERVAL: tokio::time::Duration =
                        tokio::time::Duration::from_millis((1000. / 5.) as u64);

                    let mut next_redraw = tokio::time::Instant::now();
                    let mut next_resize = next_redraw;
                    let mut next_size = (1, 1);

                    loop {
                        let now = tokio::time::Instant::now();
                        tokio::select! {
                            task = receiver.recv() => {
                                let now = tokio::time::Instant::now();
                                match task.unwrap() {
                                    Task::Exit => {
                                        break;
                                    }
                                    Task::Redraw => {
                                        if now > next_redraw {
                                            renderer.render();
                                            next_redraw = now + REDRAW_INTERVAL;
                                        }
                                    }
                                    Task::Resize(size) => {
                                        if now > next_redraw {
                                            renderer.resize(size);
                                            next_resize = now + RESIZE_INTERVAL;
                                        } else {
                                            next_size = size;
                                        }
                                    }
                                }
                            }
                            _ = tokio::time::sleep_until(next_redraw), if next_redraw > now => {
                                renderer.render();
                                next_redraw = now + REDRAW_INTERVAL;
                            },
                            _ = tokio::time::sleep_until(next_resize), if next_resize > now => {
                                renderer.resize(next_size);
                                next_resize = now + RESIZE_INTERVAL;
                            },
                            else => break,
                        }
                    }
                });
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
                sender.send(Task::Redraw).unwrap();
                // renderer.render();
            }
            winit::event::WindowEvent::Resized(size) => {
                sender.send(Task::Resize(size.into())).unwrap();
                // renderer.resize(size.into());
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
