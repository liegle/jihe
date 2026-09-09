use std::{fs, path::{Path, PathBuf}};

pub struct Parse {
    path: PathBuf,
}

impl Parse {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned()
        }
    }

    pub fn parse(&self) -> Result<jihe_shared::Content, ParseError> {
        log::info!("Parse"); // TEMP
        if let Ok(false) | Err(_) = fs::exists(&self.path) {
            return Err(ParseError::FileLost);
        }
        Ok(jihe_shared::Content::example())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("File lost")]
    FileLost,
}
