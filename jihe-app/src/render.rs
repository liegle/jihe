use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::schedule::Scheduler;

enum Task {
    Exit,
    Draw,
    Resize((u32, u32)),
}

pub(super) struct Render {
    join_handle: JoinHandle<()>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    size: (u32, u32),
}

impl Render {
    pub(super) fn new(
        scene: Arc<Mutex<jihe_render::Scene>>,
        window: Arc<winit::window::Window>,
        render_per_sec: u64,
        resize_per_sec: u64,
    ) -> Option<Self> {
        let size = window.inner_size().into();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = {
            let sender = sender.clone();
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("Can't create tokio runtime because:\n{e}");
                    return None;
                }
            };
            let renderer = match rt.block_on(jihe_render::Render::new(scene, window, size)) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Can't create render because:\n{e}");
                    return None;
                }
            };
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
        Some(Self {
            join_handle,
            sender,
            size,
        })
    }

    pub(super) fn join(self) -> thread::Result<()> {
        self.join_handle.join()
    }

    pub(super) fn exit(&self) {
        self.send(Task::Exit);
    }

    pub(super) fn draw(&self) {
        self.send(Task::Draw);
    }

    pub(super) fn resize(&mut self, size: (u32, u32)) {
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
    mut renderer: jihe_render::Render<winit::window::Window>,
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
