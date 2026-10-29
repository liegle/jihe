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

pub fn compile(path: &Path) -> Result<jihe_shared::Content, CompileError> {
    let source = fs::read_to_string(path)?;
    let lex = Lex::new(&source);
    let _syn = Syn::new(lex);
    // TODO: Report here
    Ok(jihe_shared::Content::example())
}

#[derive(thiserror::Error, Debug)]
pub enum CompileError {
    #[error("Can't read source file because:{0}")]
    FileReadFail(#[from] io::Error),
    #[error("Compile error")]
    CompileError,
}

trait Report {
    fn report(self) -> ariadne::Report<'static>;
}

type Spanned<T> = (T, Range<usize>);
