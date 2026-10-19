use std::{collections::HashSet, iter::Peekable, str::Chars};

pub(super) use crate::lex::{
    error::LexError,
    token::{Token, TokenSet},
};
use crate::{Cursor, Span, Spanned};

mod error;
mod set;
#[cfg(test)]
mod test;
mod token;

pub(super) struct Lex<'src> {
    source: &'src str,
    chars: Peekable<Chars<'src>>,
    byte_begin: usize,
    cursor_begin: Cursor,
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
            byte_begin: 0,
            cursor_begin: Default::default(),
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

    fn span(&mut self) -> Span<'src> {
        Span {
            byte_span: self.byte_begin..self.byte_ptr,
            cursor_span: self.cursor_begin..self.cursor_ptr,
            ..Default::default()
        }
    }

    fn num(&mut self, mut dot: bool) -> Result<Spanned<'src, Token>, LexError> {
        while let Some(c) = self.chars.next_if(|c| match c {
            '0'..='9' => true,
            '.' => {
                if dot {
                    false
                } else {
                    dot = true;
                    true
                }
            }
            _ => false,
        }) {
            self.byte_ptr += c.len_utf8();
            self.cursor_ptr.col += 1;
        }
        Ok((Token::Num, self.span()))
    }

    fn ident(&mut self, mut prev: char) -> Result<Spanned<'src, Token>, LexError> {
        let mut prev_cursor = self.cursor_ptr;
        while let Some(c) = self.chars.next_if(|c| match c {
            'a'..='z' | 'A'..='Z' | '_' => true,
            c if !c.is_ascii() => true,
            _ => false,
        }) {
            prev = c;
            prev_cursor = self.cursor_ptr;
            self.byte_ptr += c.len_utf8();
            self.cursor_ptr.col += 1;
        }
        if prev == '_' {
            return Err(LexError::UnderscoreAtEnd { cursor: prev_cursor });
        }
        while let Some(_) = self.chars.next_if_eq(&'\'') {
            self.byte_ptr += '\''.len_utf8();
            self.cursor_ptr.col += 1;
        }
        Ok((Token::Ident, self.span()))
    }
}

impl<'src> Iterator for Lex<'src> {
    type Item = Result<Spanned<'src, Token>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_ignored();
        self.byte_begin = self.byte_ptr;
        self.cursor_begin = self.cursor_ptr;

        let Some(first) = self.chars.next() else {
            return None;
        };
        self.byte_ptr += first.len_utf8();
        self.cursor_ptr.col += 1;

        match first {
            '0'..='9' => Some(self.num(false)),
            '.' => Some(self.num(true)),
            c @ ('a'..='z' | 'A'..='Z') => Some(self.ident(c)),
            c if !c.is_ascii() => Some(self.ident(c)),
            '^' => Some(Ok((Token::Pow, self.span()))),
            '*' => Some(Ok((Token::Mul, self.span()))),
            '/' => match self.chars.next_if_eq(&'/') {
                Some(c) => {
                    self.byte_ptr += c.len_utf8();
                    self.cursor_ptr.col += 1;
                    Some(Ok((Token::SlashF, self.span())))
                }
                _ => Some(Ok((Token::Div, self.span()))),
            },
            '+' => Some(Ok((Token::Add, self.span()))),
            '-' => Some(Ok((Token::Sub, self.span()))),
            '=' => Some(Ok((Token::Eq, self.span()))),
            ',' => Some(Ok((Token::Comma, self.span()))),
            ':' => Some(Ok((Token::Colon, self.span()))),
            '{' => Some(Ok((Token::BraceL, self.span()))),
            '}' => Some(Ok((Token::BraceR, self.span()))),
            '(' => Some(Ok((Token::ParenL, self.span()))),
            ')' => Some(Ok((Token::ParenR, self.span()))),
            '|' => Some(Ok((Token::Bar, self.span()))),
            '\\' => match self.chars.next() {
                Some('\\') => {
                    self.byte_ptr += '\\'.len_utf8();
                    self.cursor_ptr.col += 1;
                    Some(Ok((Token::SlashB, self.span())))
                }
                Some(found) => {
                    let cursor = self.cursor_ptr;
                    self.byte_ptr += found.len_utf8();
                    self.cursor_ptr.col += 1;
                    Some(Err(LexError::UnexpectedMid {
                        expected: {
                            let mut set = HashSet::new();
                            set.insert('\\');
                            set
                        },
                        found,
                        cursor,
                    }))
                }
                None => Some(Err(LexError::UnexpectedEof { cursor: self.cursor_ptr })),
            },
            c => Some(Err(LexError::UnexpectedBegin {
                found: c,
                cursor: self.cursor_begin,
            })),
        }
    }
}
