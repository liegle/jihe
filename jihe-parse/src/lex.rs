use std::{iter::Peekable, str::Chars};

use crate::{
    cursor::Cursor,
    lex::{error::LexerError, machine::Machine},
    token::{SKIP, Token},
};

pub(super) mod error;
mod machine;
#[cfg(test)]
mod test;

pub(super) struct Lex<'src> {
    source: Peekable<Chars<'src>>,
    byte_ptr: usize,
    char_ptr: Cursor,
}

impl<'src> Lex<'src> {
    pub(super) fn new(source: &'src str) -> Self {
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

impl<'source> Iterator for Lex<'source> {
    type Item = Result<Token, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_whitespaces();

        let byte_begin = self.byte_ptr;
        let char_begin = self.char_ptr;
        let mut machine = Machine::new();

        while let Some(c) = self.source.peek().copied() {
            if SKIP.contains(&c) {
                break;
            }

            let next_machine = machine.step(c);
            if next_machine.last_matched_char.is_none() {
                if machine.last_matched_char.is_none() {
                    return Some(Err(LexerError::UnexpectedBegin {
                        found: c,
                        cursor: self.char_ptr,
                    }));
                }
                break;
            } else {
                machine = next_machine.clone();
                self.byte_ptr += c.len_utf8();
                self.char_ptr.step(false);
                self.source.next();
            }
        }

        if let Some(c) = machine.last_matched_char {
            // v v v v x x x x ...
            //   curr^ ^peek
            let matched = machine.end();
            match &matched[..] {
                [] => Some(Err(LexerError::UnexpectedMid {
                    expected: machine.gather_expected(),
                    found: c,
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
            //     o last
            // v v v _ _
            //   curr^ ^peek
            None
        }
    }
}
