use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    range::Range,
};

use crate::token::{Kind, Token};

pub(super) struct Lexer<'source> {
    source: &'source [char],
    ptr: usize,
}

impl<'source> Lexer<'source> {
    pub(super) fn new(source: &'source [char]) -> Self {
        Self { source, ptr: 0 }
    }
}

#[derive(Debug)]
pub struct BadChar {
    expected: &'static str,
    found: char,
    pos: (usize, usize),
}

impl Error for BadChar {}

impl Display for BadChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Expected {}, found {} at {}:{}",
            self.expected, self.found, self.pos.0, self.pos.1
        )
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token, BadChar>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO
        None
    }
}
