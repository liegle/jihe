use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Range,
};

use crate::{Cursor, LexError, lex::KindSet};

#[derive(Debug)]
pub enum SynError {
    LexError(LexError),
    UnexpectedEof,
    UnexpectedToken {
        expected: KindSet,
        found: String,
        range: Range<Cursor>,
    },
    UndefinedStatementKind {
        found: String,
        range: Range<Cursor>,
    },
    DuplicatedStatementField {
        name: String,
        range: Range<Cursor>,
    },
    StatementFieldLost {
        name: &'static str,
    },
}

impl Error for SynError {}

impl Display for SynError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::LexError(e) => e.fmt(f),
            Self::UnexpectedEof => {
                write!(f, "Source file ended")
            }
            Self::UnexpectedToken { expected, found, range } => {
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
                write!(f, ", found \"{found}\" at {}..{}", range.start, range.end)
            }
            Self::UndefinedStatementKind { found, range } => {
                write!(
                    f,
                    "Specified kind not defined: {found} at {}..{}",
                    range.start, range.end
                )
            }
            Self::DuplicatedStatementField { name, range } => {
                write!(
                    f,
                    "Field {name} has been defined twice at {}..{}",
                    range.start, range.end
                )
            }
            Self::StatementFieldLost { name } => {
                write!(f, "Field {name} is not defined")
            }
        }
    }
}
