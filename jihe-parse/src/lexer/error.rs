use std::{
    collections::HashSet,
    error,
    fmt::{self, Display, Formatter},
    ops::Range,
};

use crate::{
    cursor::Cursor,
    token::{Character, Kind},
};

#[derive(Debug)]
pub enum LexerError {
    UnexpectedBegin {
        found: char,
        cursor: Cursor,
    },
    UnexpectedMid {
        expected: HashSet<Character>,
        found: char,
        cursor: Cursor,
    },
    MultipleMatching {
        matched: Vec<Kind>,
        range: Range<Cursor>,
    },
}

impl error::Error for LexerError {}

impl Display for LexerError {
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
                        write!(f, "{e:?}")?;
                        is_begin = false;
                    } else {
                        write!(f, "or {e:?}")?;
                    }
                }
                write!(f, ", found '{found}' at {cursor}")
            }
            Self::MultipleMatching { matched, range } => {
                write!(
                    f,
                    "String from {} to {} can match more than one token kind: [",
                    range.start, range.end,
                )?;
                let mut is_begin = true;
                for k in matched {
                    if is_begin {
                        write!(f, "{k:?}")?;
                        is_begin = false;
                    } else {
                        write!(f, ", {k:?}")?;
                    }
                }
                write!(f, "], this usually means a wrong design in token priority")
            }
        }
    }
}
