use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    lex::{Lex, LexError},
    syn::{Syn, SynError},
};

mod array;
mod cursor;
mod lex;
mod syn;

pub struct Parse {
    path: PathBuf,
}

impl Parse {
    pub fn new(path: &Path) -> Self {
        Self { path: path.to_owned() }
    }

    pub fn parse(&self) -> Result<jihe_shared::Content, ParseError> {
        if let Ok(false) | Err(_) = fs::exists(&self.path) {
            return Err(ParseError::FileLost);
        }
        let source = fs::read_to_string(&self.path)?;
        let lex = Lex::new(&source);
        let mut syn = Syn::new();
        for token in lex {
            syn.input(token?)?;
        }
        let _ = syn.output()?;
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
    LexError(#[from] LexError),
    #[error("Failed to create syntax because:{0}")]
    SynError(#[from] SynError),
}
