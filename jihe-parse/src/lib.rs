use std::{
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    lex::{Lex, LexError},
    syn::{SynError, syn},
};

mod array;
mod intset;
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
        let _tree = syn(lex)?;
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn step(&mut self, is_line: bool) {
        if is_line {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
    }
}

impl Display for Cursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
