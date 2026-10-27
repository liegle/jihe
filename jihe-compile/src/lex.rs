use std::{iter::Peekable, ops::Range, str::Chars};

use crate::Spanned;

pub(super) use crate::lex::{
    error::LexError,
    token::{Token, TokenSet},
};

mod error;
mod set;
#[cfg(test)]
mod test;
mod token;

pub(super) struct Lex<'src> {
    source: &'src str,
    chars: Peekable<Chars<'src>>,
    span: Range<usize>,
}

impl<'src> Lex<'src> {
    pub(super) fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.chars().peekable(),
            span: 0..0,
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
            self.span.end += c.len_utf8();
            let _ = self.chars.next();
        }
    }

    fn num(&mut self) -> <Self as Iterator>::Item {
        let mut dot = false;
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
            self.span.end += c.len_utf8();
        }
        Ok((Token::Num, self.span.clone()))
    }

    fn ident(&mut self, mut prev: char) -> <Self as Iterator>::Item {
        let mut prev_byte = self.span.end;
        while let Some(c) = self.chars.next_if(|c| match c {
            'a'..='z' | 'A'..='Z' | '_' => true,
            c if !c.is_ascii() => true,
            _ => false,
        }) {
            prev = c;
            prev_byte = self.span.end;
            self.span.end += c.len_utf8();
        }
        if prev == '_' {
            return Err(LexError::UnderscoreAtEnd { span: prev_byte..self.span.end });
        }
        while let Some(_) = self.chars.next_if_eq(&'\'') {
            self.span.end += '\''.len_utf8();
        }
        Ok((Token::Ident, self.span.clone()))
    }
}

impl<'src> Iterator for Lex<'src> {
    type Item = Result<Spanned<Token>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_ignored();
        self.span.start = self.span.end;

        let Some(first) = self.chars.next() else {
            return None;
        };
        self.span.end += first.len_utf8();

        match first {
            '0'..='9' => Some(self.num()),
            c @ ('a'..='z' | 'A'..='Z') => Some(self.ident(c)),
            c if !c.is_ascii() => Some(self.ident(c)),
            '^' => Some(Ok((Token::Pow, self.span.clone()))),
            '*' => Some(Ok((Token::Mul, self.span.clone()))),
            '/' => match self.chars.next_if_eq(&'/') {
                Some(c) => {
                    self.span.end += c.len_utf8();
                    Some(Ok((Token::SlashF, self.span.clone())))
                }
                _ => Some(Ok((Token::Div, self.span.clone()))),
            },
            '+' => Some(Ok((Token::Add, self.span.clone()))),
            '-' => Some(Ok((Token::Sub, self.span.clone()))),
            '=' => Some(Ok((Token::Eq, self.span.clone()))),
            ',' => Some(Ok((Token::Comma, self.span.clone()))),
            ':' => Some(Ok((Token::Colon, self.span.clone()))),
            '{' => Some(Ok((Token::BraceL, self.span.clone()))),
            '}' => Some(Ok((Token::BraceR, self.span.clone()))),
            '(' => Some(Ok((Token::ParenL, self.span.clone()))),
            ')' => Some(Ok((Token::ParenR, self.span.clone()))),
            '|' => Some(Ok((Token::Bar, self.span.clone()))),
            '\\' => match self.chars.next() {
                Some('\\') => {
                    self.span.end += '\\'.len_utf8();
                    Some(Ok((Token::SlashB, self.span.clone())))
                }
                Some(found) => {
                    let ptr = self.span.end;
                    self.span.end += found.len_utf8();
                    Some(Err(LexError::UnexpectedMid {
                        expected: '\\',
                        found,
                        span: ptr..self.span.end,
                    }))
                }
                None => Some(Err(LexError::UnexpectedEof {
                    expected: '\\',
                    byte: self.source.len(),
                })),
            },
            c => Some(Err(LexError::UnexpectedBegin {
                found: c,
                span: self.span.clone(),
            })),
        }
    }
}
