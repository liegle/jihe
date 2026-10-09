use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    ops::Range,
};

use crate::{
    lex::{Lex, LexError},
    syn::{SynError, syn},
};

mod array;
mod intset;
mod lex;
mod syn;

pub fn parse(source: &str) -> Result<jihe_shared::Content, ParseError> {
    let lex = Lex::new(source);
    let _tree = syn(lex)?;
    Ok(jihe_shared::Content::example())
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("Failed to create lexical token because:{0}")]
    LexError(#[from] LexError),
    #[error("Failed to create syntax tree because:{0}")]
    SynError(#[from] SynError),
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
