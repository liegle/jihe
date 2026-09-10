use std::{
    fs,
    path::{self, PathBuf},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use notify::Watcher as _;

use crate::debounce::Debounce;

enum Task {
    Exit,
    Parse,
}

pub(super) struct Parse {
    join_handle: JoinHandle<()>,
    sender: tokio::sync::mpsc::UnboundedSender<Task>,
}

impl Parse {
    pub(super) fn new(
        path: PathBuf,
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

            let abs = path::absolute(&path).unwrap();
            let dir = abs.parent().unwrap().to_owned();
            let mut watcher = match notify::recommended_watcher(Filter {
                path: path.clone(),
                sender,
            }) {
                Ok(watcher) => watcher,
                Err(e) => {
                    log::error!("Can't create watcher because:{e}");
                    return None;
                }
            };

            if let Err(e) = watcher.watch(&dir, notify::RecursiveMode::NonRecursive) {
                log::error!("Can't watch target file because:{e}");
                return None;
            }
            let parse = jihe_parse::Parse::new(&path);
            thread::spawn(move || {
                rt.block_on(run(parse, scene, callback, receiver));
                let _ = watcher.unwatch(&dir); // Keep watcher alive
            })
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
        if self.sender.send(Task::Exit).is_err() {
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
        use notify::{Event, EventKind, event::ModifyKind};

        log::error!("{event:?}"); // TEMP
        match event {
            Ok(event) => {
                if let Event {
                    kind: EventKind::Modify(ModifyKind::Data(_)),
                    paths,
                    ..
                } = event
                {
                    let file_name = self.path.file_name();
                    if paths.iter().any(|path| path.file_name() == file_name)
                        && matches!(fs::exists(&self.path), Ok(true) | Err(_))
                        && self.sender.send(Task::Parse).is_err()
                    {
                        log::error!("Parse task receiver has been closed");
                    }
                }
            }
            Err(e) => {
                log::error!("Fail to handle notify event because:{e}");
            }
        };
    }
}

async fn run(
    parse: jihe_parse::Parse,
    scene: Arc<Mutex<jihe_render::Scene>>,
    callback: impl Fn(),
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<Task>,
) {
    let mut debounce = Debounce::new(1);

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
                        if let Some(()) = debounce.push_task(()) {
                            match parse.parse() {
                                Ok(content) => {
                                    scene.lock().unwrap().content = content;
                                    callback();
                                }
                                Err(e) => log::error!("Failed to parse jihe because:{e}")
                            }
                        }
                    }
                }
            }
            Some(_) = debounce.sleep() => {
                match parse.parse() {
                    Ok(content) => {
                        scene.lock().unwrap().content = content;
                        callback();
                    }
                    Err(e) => log::error!("Failed to parse jihe because:{e}")
                }
            }
        }
    }
}
