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

pub struct Renderer {
    join_handle: JoinHandle<()>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    size: (u32, u32),
}

impl Renderer {
    pub fn new(
        scene: Arc<Mutex<jihe_shared::Scene>>,
        window: Arc<winit::window::Window>,
        render_per_sec: u64,
        resize_per_sec: u64,
    ) -> Result<Self, jihe_render::CreateRendererError> {
        let size = window.inner_size().into();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = {
            let sender = sender.clone();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()?;
            let renderer = rt.block_on(jihe_render::Render::new(scene, window))?;
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
    mut renderer: jihe_render::Render,
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
