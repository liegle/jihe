use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::debounce::Debounce;

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
                    log::error!("Can't create tokio runtime for render because:{e}");
                    return None;
                }
            };
            let render = match rt.block_on(jihe_render::Render::new(scene, window, size)) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Can't create render because:{e}");
                    return None;
                }
            };
            thread::spawn(move || {
                rt.block_on(run(
                    render,
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
        if self.sender.send(task).is_err() {
            log::error!("Render task receiver has been closed");
        }
    }
}

async fn run(
    mut render: jihe_render::Render<winit::window::Window>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
    render_per_sec: u64,
    resize_per_sec: u64,
) {
    let mut render_debounce = Debounce::new(render_per_sec);
    let mut resize_debounce = Debounce::new(resize_per_sec);

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
                        if let Some(()) = render_debounce.push_task(()) {
                            render.draw();
                        }
                    }
                    Some(Task::Resize(size)) => {
                        if let Some(size) = resize_debounce.push_task(size) {
                            render.resize(size);
                            if let Err(e) = sender.send(Task::Draw) {
                                log::error!("{e}");
                                log::error!("Render task channel has been closed");
                                break;
                            }
                        }
                    }
                }
            }
            Some(_) = render_debounce.sleep() => {
                render.draw();
            }
            Some(size) = resize_debounce.sleep() => {
                render.resize(size);
                if let Err(e) = sender.send(Task::Draw) {
                    log::error!("{e}");
                    log::error!("Render task channel has been closed");
                    break;
                }
            }
            else => break,
        }
    }
}
