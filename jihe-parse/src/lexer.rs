use std::{iter::Peekable, str::Chars};

use crate::{
    cursor::Cursor,
    lexer::{error::LexerError, stage::Match},
    token::{SKIP, Token},
};

pub(super) mod error;
mod stage;
#[cfg(test)]
mod test;

pub(super) struct Lexer<'source> {
    source: Peekable<Chars<'source>>,
    byte_ptr: usize,
    char_ptr: Cursor,
}

impl<'source> Lexer<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self {
            source: source.chars().peekable(),
            byte_ptr: 0,
            char_ptr: Default::default(),
        }
    }

    fn consume_whitespaces(&mut self) {
        while let Some(c) = self.source.peek() {
            if SKIP.contains(c) {
                self.byte_ptr += c.len_utf8();
                self.char_ptr.step(*c == '\n');
                self.source.next();
            } else {
                break;
            }
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_whitespaces();

        let byte_begin = self.byte_ptr;
        let char_begin = self.char_ptr;
        let mut prev_match = Match::new();
        let mut curr_match = Match::new();

        while let Some(c) = self.source.peek() {
            if SKIP.contains(c) {
                break;
            }

            curr_match.step(*c);
            if !curr_match.any() {
                if !prev_match.any() {
                    return Some(Err(LexerError::UnexpectedBegin {
                        found: *c,
                        cursor: self.char_ptr,
                    }));
                }
                break;
            } else {
                prev_match = curr_match.clone();
                self.byte_ptr += c.len_utf8();
                self.char_ptr.step(false);
                self.source.next();
            }
        }

        if prev_match.any() {
            let matched = prev_match.cmp_priority();
            match &matched[..] {
                [] => Some(Err(LexerError::UnexpectedMid {
                    expected: prev_match.gather_expected(),
                    found: *self
                        .source
                        .peek()
                        .expect("Technically the source is not end"),
                    cursor: self.char_ptr,
                })),
                [kind] => Some(Ok(Token {
                    kind: *kind,
                    bytes: byte_begin..self.byte_ptr,
                })),
                _ => Some(Err(LexerError::MultipleMatching {
                    matched,
                    range: char_begin..self.char_ptr,
                })),
            }
        } else {
            None
        }
    }
}
