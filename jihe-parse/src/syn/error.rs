use std::{
    collections::HashSet,
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::lex::Kind;

#[derive(Debug)]
pub enum SynError {
    UnexpectedToken { expected: HashSet<Kind>, found: String },
    UndefinedStatementKind { found: String },
}

impl Error for SynError {}

impl Display for SynError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken { expected, found } => {
                write!(f, "Expected ")?;
                let mut is_begin = true;
                for e in expected {
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
