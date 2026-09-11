use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::lexer::{BadChar, Lexer};

mod lexer;
mod token;

pub struct Parse {
    path: PathBuf,
}

impl Parse {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
        }
    }

    pub fn parse(&self) -> Result<jihe_shared::Content, ParseError> {
        if let Ok(false) | Err(_) = fs::exists(&self.path) {
            return Err(ParseError::FileLost);
        }
        let source = fs::read_to_string(&self.path)?;
        let lexer = Lexer::new(&source);
        for token in lexer {
            let _ = token?;
            // TODO
        }
        Ok(jihe_shared::Content::example())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("File lost")]
    FileLost,
    #[error("Failed to read jihe because:{0}")]
    ReadFail(#[from] io::Error),
    #[error("Failed to create token because:{0}")]
    LexError(#[from] BadChar),
}
