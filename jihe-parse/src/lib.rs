use std::path::{Path, PathBuf};

pub struct Parse {
    path: PathBuf,
}

impl Parse {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned()
        }
    }

    pub fn parse() -> jihe_shared::Content {
        jihe_shared::Content::example()
    }
}
