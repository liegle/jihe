use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Range,
};

use crate::{Cursor, LexError, lex::TokenSet};

#[derive(Debug)]
pub enum SynError {
    LexError(LexError),
    UnexpectedEof,
    UnexpectedToken {
        expected: TokenSet,
        found: String,
        cursor_span: Range<Cursor>,
    },
    DuplicatedStatementField {
        name: String,
        cursor_span: Range<Cursor>,
    },
    UnaryInsideExpr {
        cursor: Cursor,
    },
}

impl Error for SynError {}

impl Display for SynError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LexError(e) => e.fmt(f),
            Self::UnexpectedEof => write!(f, "Source file ended"),
            Self::UnexpectedToken { expected, found, cursor_span } => {
                write!(f, "Expected ")?;
                let mut is_begin = true;
                for e in expected.into_iter() {
                    if is_begin {
                        write!(f, "{e}")?;
                        is_begin = false;
                    } else {
                        write!(f, "or {e}")?;
                    }
                }
                write!(
                    f,
                    ", found \"{found}\" at {}..{}",
                    cursor_span.start, cursor_span.end
                )
            }
            Self::DuplicatedStatementField { name, cursor_span } => {
                write!(
                    f,
                    "Field {name} has been defined twice at {}..{}",
                    cursor_span.start, cursor_span.end
                )
            }
            Self::UnaryInsideExpr { cursor } => {
                write!(
                    f,
                    "Unary operator at {cursor} inside expression should be surrounded by '(' & ')'"
                )
            }
        }
    }
}
