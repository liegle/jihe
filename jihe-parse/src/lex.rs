use std::{iter::Peekable, marker::PhantomData, str::Chars};

pub(super) use crate::lex::{
    error::LexError,
    token::{Token, TokenSet},
};
use crate::{Cursor, Span, Spanned, lex::machine::Machine};

mod automata;
mod error;
mod machine;
#[cfg(test)]
mod test;
mod token;

pub(super) type LexItem<'src> = Result<Spanned<'src, Token>, LexError>;

pub(super) struct Lex<'src> {
    source: &'src str,
    chars: Peekable<Chars<'src>>,
    byte_ptr: usize,
    cursor_ptr: Cursor,
}

impl<'src> Lex<'src> {
    pub(super) fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            byte_ptr: 0,
            cursor_ptr: Default::default(),
        }
    }

    pub(super) fn source(&self) -> &'src str {
        self.source
    }

    fn consume_ignored(&mut self) {
        let mut comment = false;
        while let Some(c) = self.chars.peek().copied() {
            comment = match (c, comment) {
                ('#', _) => true,
                ('\n', true) => false,
                (_, true) => true,
                (' ' | '\n' | '\r' | '\t', _) => comment,
                _ => break,
            };
            self.byte_ptr += c.len_utf8();
            if c == '\n' {
                self.cursor_ptr.line += 1;
                self.cursor_ptr.col = 0;
            } else {
                self.cursor_ptr.col += 1;
            };
            let _ = self.chars.next();
        }
    }
}

impl<'src> Iterator for Lex<'src> {
    type Item = LexItem<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_ignored();

        let byte_begin = self.byte_ptr;
        let cursor_begin = self.cursor_ptr;
        let mut machine = Machine::new();

        while let Some(c) = self.chars.peek().copied() {
            let next_machine = machine.step(c);
            if next_machine.last_matched_char.is_none() {
                if machine.last_matched_char.is_none() {
                    return Some(Err(LexError::UnexpectedBegin {
                        found: c,
                        cursor: self.cursor_ptr,
                    }));
                }
                break;
            } else {
                machine = next_machine;
                self.byte_ptr += c.len_utf8();
                self.cursor_ptr.col += 1;
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
                    cursor: self.cursor_ptr,
                })),
                [token] => Some(Ok((
                    *token,
                    Span {
                        byte_span: byte_begin..self.byte_ptr,
                        cursor_span: cursor_begin..self.cursor_ptr,
                        _phantom: PhantomData,
                    },
                ))),
                _ => Some(Err(LexError::MultipleMatching {
                    matched,
                    range: cursor_begin..self.cursor_ptr,
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
