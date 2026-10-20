use crate::lex::set::{self, IntSet};

use std::fmt::{self, Debug, Display, Formatter};

#[rustfmt::skip]
macro_rules! enum_token {
    ($($token:ident = $disp:literal)+) => {
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
        #[repr(u8)]
        pub enum Token {
            #[default]
            $($token),+
        }

        const TOKENS: &[Token] = &[
            $(Token::$token),+
        ];

        impl Display for Token {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$token => write!(f, "{}", $disp)),+
                }
            }
        }
    };
}

#[rustfmt::skip]
enum_token! {
    Num    = "number"
    Ident  = "identifier"
    Pow    = '^'
    Mul    = '*'
    Div    = '/'
    Add    = '+'
    Sub    = '-'
    Eq     = '='
    Comma  = ','
    Colon  = ':'
    BraceL = '{'
    BraceR = '}'
    ParenL = '('
    ParenR = ')'
    SlashF = "//"
    SlashB = "\\\\"
    Bar    = '|'
}

#[derive(Clone, Copy)]
pub struct TokenSet(IntSet);

impl TokenSet {
    pub(crate) fn with_values<T: IntoIterator<Item = Token>>(values: T) -> Self {
        Self(IntSet::with_values(values.into_iter().map(|k| k as u8)))
    }
}

impl Debug for TokenSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "TokenSet {{ ")?;
        let mut is_begin = true;
        for t in self.into_iter() {
            if is_begin {
                write!(f, "{t:?}")?;
                is_begin = false;
            } else {
                write!(f, ", {t:?}")?;
            }
        }
        write!(f, " }}")
    }
}

impl Display for TokenSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut is_begin = true;
        for t in self.into_iter() {
            if is_begin {
                write!(f, "{t:?}")?;
                is_begin = false;
            } else {
                write!(f, "or {t:?}")?;
            }
        }
        Ok(())
    }
}

pub struct IntoIter(set::IntoIter);

impl IntoIterator for TokenSet {
    type Item = Token;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl Iterator for IntoIter {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.0.next() {
            if let Some(token) = TOKENS.get(index as usize) {
                return Some(*token);
            }
        }
        None
    }
}
