use std::{iter::Peekable, str::Chars};

pub(super) use crate::lex::{
    error::LexError,
    token::{Kind, Token},
};
use crate::{
    Cursor,
    lex::{machine::Machine, token::SKIP},
};

mod automata;
mod error;
mod machine;
#[cfg(test)]
mod test;
mod token;

pub(super) struct Lex<'src> {
    source: &'src str,
    chars: Peekable<Chars<'src>>,
    byte_ptr: usize,
    char_ptr: Cursor,
}

impl<'src> Lex<'src> {
    pub(super) fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            byte_ptr: 0,
            char_ptr: Default::default(),
        }
    }

    fn consume_whitespaces(&mut self) {
        while let Some(c) = self.chars.peek() {
            if SKIP.contains(c) {
                self.byte_ptr += c.len_utf8();
                self.char_ptr.step(*c == '\n');
                self.chars.next();
            } else {
                break;
            }
        }
    }
}

impl<'src> Iterator for Lex<'src> {
    type Item = Result<Token<'src>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_whitespaces();

        let byte_begin = self.byte_ptr;
        let char_begin = self.char_ptr;
        let mut machine = Machine::new();

        while let Some(c) = self.chars.peek().copied() {
            if SKIP.contains(&c) {
                break;
            }

            let next_machine = machine.step(c);
            if next_machine.last_matched_char.is_none() {
                if machine.last_matched_char.is_none() {
                    return Some(Err(LexError::UnexpectedBegin {
                        found: c,
                        cursor: self.char_ptr,
                    }));
                }
                break;
            } else {
                machine = next_machine;
                self.byte_ptr += c.len_utf8();
                self.char_ptr.step(false);
                self.chars.next();
            }
        }

        if let Some(c) = machine.last_matched_char {
            // v v v v x x x x ...
            //   curr^ ^peek
            let matched = machine.end();
            match &matched[..] {
                [] => Some(Err(LexError::UnexpectedMid {
                    expected: machine.gather_expected(),
                    found: c,
                    cursor: self.char_ptr,
                })),
                [kind] => Some(Ok(Token {
                    kind: *kind,
                    string: &self.source[byte_begin..self.byte_ptr],
                })),
                _ => Some(Err(LexError::MultipleMatching {
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
