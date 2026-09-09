use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use notify::Watcher as _;

use crate::schedule::Scheduler;

enum Task {
    Exit,
    Parse,
    Rename(PathBuf),
    Lost,
}

pub(super) struct Parse {
    join_handle: JoinHandle<()>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
}

impl Parse {
    pub(super) fn new(
        path: &Path,
        scene: Arc<Mutex<jihe_render::Scene>>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Option<Self> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let join_handle = {
            let sender = sender.clone();
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    log::error!("Can't create tokio runtime for parse because:{e}");
                    return None;
                }
            };
            let mut watcher = match notify::recommended_watcher(Filter {
                path: path.to_owned(),
                sender,
            }) {
                Ok(watcher) => watcher,
                Err(e) => {
                    log::error!("Can't create watcher because:{e}");
                    return None;
                }
            };
            if let Err(e) = watcher.watch(path, notify::RecursiveMode::NonRecursive) {
                log::error!("Can't watch target file because:{e}");
                return None;
            }
            thread::spawn(move || rt.block_on(run(scene, callback, receiver)))
        };
        Some(Self {
            join_handle,
            sender,
        })
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

struct Filter {
    path: PathBuf,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
}

impl notify::EventHandler for Filter {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        use notify::{
            Event, EventKind,
            event::{DataChange, ModifyKind, RenameMode},
        };

        match event {
            Ok(event) => match event {
                Event {
                    kind: EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                    ..
                } => {
                    self.parse();
                }
                Event {
                    kind: EventKind::Modify(ModifyKind::Name(mode)),
                    paths,
                    ..
                } => match mode {
                    RenameMode::To => {
                        if let Some(path) = paths.first() {
                            self.rename(path.to_owned());
                        }
                    }
                    RenameMode::Both => {
                        if let Some(path) = paths.iter().nth(1) {
                            self.rename(path.to_owned());
                        }
                    }
                    _ => match fs::exists(&self.path) {
                        Ok(false) | Err(_) => self.lost(),
                        _ => {}
                    },
                },
                _ => {}
            },
            Err(e) => {
                log::error!("Fail to handle notify event because:{e}");
            }
        }
    }
}

impl Filter {
    fn parse(&self) {
        self.send(Task::Parse);
    }

    fn rename(&mut self, path: PathBuf) {
        self.path = path.clone();
        self.send(Task::Rename(path));
    }

    fn lost(&self) {
        self.send(Task::Lost);
    }

    fn send(&self, task: Task) {
        if self.sender.send(task).is_err() {
            log::error!("Parse task receiver has been closed");
        }
    }
}

async fn run(
    scene: Arc<Mutex<jihe_render::Scene>>,
    callback: impl Fn(),
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
) {
    let todo = || {};

    let mut scheduler = Scheduler::new(1);

    loop {
        if receiver.is_closed() {
            log::error!("Parse task reveiver has been closed");
            break;
        }
        tokio::select! {
            task = receiver.recv() => {
                match task {
                    None => {
                        log::error!("Parse task channel has been closed");
                        break;
                    }
                    Some(Task::Exit) => {
                        break;
                    }
                    Some(Task::Parse) => {
                        if let Some(_) = scheduler.push_task(()) {
                            // TODO
                        }
                    }
                    Some(Task::Rename(path)) => {

                    }
                    Some(Task::Lost) => {
                        break;
                    }
                }
            }
            Some(_) = scheduler.sleep() => {
                // TODO
            }
        }
    }
}
