use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use notify::Watcher as _;

enum Task {
    Exit,
    Res(notify::Result<notify::Event>),
}

pub(super) struct Parse {
    join_handle: JoinHandle<()>,
    sender: mpsc::Sender<Task>,
    scene: Arc<Mutex<jihe_render::Scene>>,
}

impl Parse {
    pub(super) fn new(
        path: String,
        scene: Arc<Mutex<jihe_render::Scene>>,
        callback: impl Fn(),
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Task>();
        let join_handle = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut watcher = notify::recommended_watcher(move |res| {
                    sender.send(Task::Res(res)).unwrap();
                })
                .unwrap();
                watcher
                    .watch(path.as_ref(), notify::RecursiveMode::NonRecursive)
                    .unwrap();
                for res in receiver {
                    match res {
                        Task::Exit => break,
                        Task::Res(Ok(event)) => {
                            // TODO
                            log::info!("111 {event:?}");
                        }
                        Task::Res(Err(e)) => {
                            log::error!("File watch error: {e}");
                        }
                    }
                }
            })
        };
        Self {
            join_handle,
            sender,
            scene,
        }
    }

    pub(super) fn join(self) -> thread::Result<()> {
        self.join_handle.join()
    }

    pub(super) fn exit(&self) {
        if let Err(_) = self.sender.send(Task::Exit) {
            log::error!("Parse task receiver has been closed");
        }
    }
}
