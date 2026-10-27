use std::{
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
};

use crate::{
    lex::{Lex, LexError},
    syn::{Syn, SynError},
};

mod lex;
mod sem;
mod syn;

pub fn compile(path: &Path) -> Result<jihe_shared::Content, ParseError> {
    if let Ok(false) | Err(_) = fs::exists(path) {
        return Err(ParseError::FileNotExist(path.to_path_buf()));
    }
    let source = fs::read_to_string(path)?;
    let lex = Lex::new(&source);
    let _syn = Syn::new(lex);
    // TODO: Report here
    Ok(jihe_shared::Content::example())
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("Source file {0} lost")]
    FileNotExist(PathBuf),
    #[error("Can't read source file because:{0}")]
    FileReadFail(#[from] io::Error),
    #[error("Parse error")]
    ParseError,
}

trait Report {
    fn report(self) -> ariadne::Report<'static>;
}

type Spanned<T> = (T, Range<usize>);
