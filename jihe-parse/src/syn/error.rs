use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::{LexError, lex::KindSet};

#[derive(Debug)]
pub enum SynError {
    LexError(LexError),
    UnexpectedEof,
    UnexpectedToken { expected: KindSet, found: String },
    UndefinedStatementKind { found: String },
}

impl Error for SynError {}

impl Display for SynError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LexError(e) => e.fmt(f),
            Self::UnexpectedEof => {
                write!(f, "Source file ended")
            }
            Self::UnexpectedToken { expected, found } => {
                write!(f, "Expected ")?;
                let mut is_begin = true;
                for e in expected.into_iter() {
                    if is_begin {
                        write!(f, "{e:?}")?;
                        is_begin = false;
                    } else {
                        write!(f, "or {e:?}")?;
                    }
                }
                write!(f, ", found '{found}'") // TODO: send cursor to here to print
            }
            Self::UndefinedStatementKind { found } => {
                write!(f, "Specified kind not defined: {found}")
            }
        }
    }
}
