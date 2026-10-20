use std::{
    fmt::{self, Display, Formatter},
    fs, io,
    marker::PhantomData,
    ops::Range,
    path::{Path, PathBuf},
};

use crate::{
    lex::{Lex, LexError},
    syn::{Syn, SynError},
};

mod lex;
mod syn;

pub fn parse(path: &Path) -> Result<jihe_shared::Content, ParseError> {
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    line: usize,
    col: usize,
}

impl Display for Cursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Default)]
struct Span<'src> {
    byte_span: Range<usize>,
    cursor_span: Range<Cursor>,
    // TODO: is this required?
    _phantom: PhantomData<&'src ()>,
}

impl Span<'_> {
    fn join(start: &Self, end: &Self) -> Self {
        Self {
            byte_span: start.byte_span.start..end.byte_span.end,
            cursor_span: start.cursor_span.start..end.cursor_span.end,
            ..Default::default()
        }
    }
}

type Spanned<'src, T> = (T, Span<'src>);
