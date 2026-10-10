use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Range,
};

use crate::{Cursor, LexError, lex::TokenSet, syn::tree::Check};

#[derive(Debug)]
pub enum SynError {
    LexError(LexError),
    UnexpectedEof,
    UnexpectedToken {
        expected: TokenSet,
        found: String,
        cursor_span: Range<Cursor>,
    },
    UndefinedStatementClass {
        found: String,
        cursor_span: Range<Cursor>,
    },
    DuplicatedStatementField {
        name: String,
        cursor_span: Range<Cursor>,
    },
    StatementFieldLost {
        name: &'static str,
        cursor_span: Range<Cursor>,
    },
    UnexpectedStatementFieldType {
        expected: Check,
        cursor_span: Range<Cursor>,
    },
    DismatchParenR {
        cursor: Cursor,
    },
    NegInsideExpr {
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
                        write!(f, "{e:?}")?;
                        is_begin = false;
                    } else {
                        write!(f, "or {e:?}")?;
                    }
                }
                write!(
                    f,
                    ", found \"{found}\" at {}..{}",
                    cursor_span.start, cursor_span.end
                )
            }
            Self::UndefinedStatementClass { found, cursor_span } => {
                write!(
                    f,
                    "Specified kind not defined: {found} at {}..{}",
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
            Self::StatementFieldLost { name, cursor_span } => {
                write!(
                    f,
                    "Field {name} is not defined before  {}..{}",
                    cursor_span.start, cursor_span.end
                )
            }
            Self::UnexpectedStatementFieldType { expected, cursor_span } => {
                write!(
                    f,
                    "Field at {}..{} should be {expected}",
                    cursor_span.start, cursor_span.end
                )
            }
            Self::DismatchParenR { cursor } => {
                write!(f, "Right parenthesis at {cursor} has no matching left one")
            }
            Self::NegInsideExpr { cursor } => {
                write!(
                    f,
                    "Negative at {cursor} inside expression should be surrounded by '(' & ')'"
                )
            }
        }
    }
}
