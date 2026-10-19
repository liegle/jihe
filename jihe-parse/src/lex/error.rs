use std::{
    collections::HashSet,
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::Cursor;

#[derive(Debug)]
pub enum LexError {
    UnexpectedBegin {
        found: char,
        cursor: Cursor,
    },
    UnexpectedMid {
        expected: HashSet<char>,
        found: char,
        cursor: Cursor,
    },
    UnexpectedEof {
        cursor: Cursor,
    },
    UnderscoreAtEnd {
        cursor: Cursor,
    },
}

impl Error for LexError {}

impl Display for LexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedBegin { found, cursor } => {
                write!(
                    f,
                    "Char '{found}' at {cursor} is not a beginning of any known token kind",
                )
            }
            Self::UnexpectedMid { expected, found, cursor } => {
                write!(f, "Expected ")?;
                let mut is_begin = true;
                for e in expected {
                    if is_begin {
                        write!(f, "{e}")?;
                        is_begin = false;
                    } else {
                        write!(f, "or {e}")?;
                    }
                }
                write!(f, ", found '{found}' at {cursor}")
            }
            Self::UnexpectedEof { cursor } => {
                write!(f, "Unexpected end of file at {cursor}")
            }
            Self::UnderscoreAtEnd { cursor } => {
                write!(
                    f,
                    "Underscore should not appear at end of identifier {cursor}"
                )
            }
        }
    }
}
