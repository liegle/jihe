use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    str::Chars,
};

use crate::token::{Kind, PATTERNS, Pattern, SKIP, Token};

pub(super) struct Lexer<'source> {
    stages: [Stage; PATTERNS.len()],
    source: Chars<'source>,
    byte_ptr: usize,
    char_ptr: usize,
}

impl<'source> Lexer<'source> {
    pub(super) fn new(source: &'source str) -> Self {
        Self {
            stages: [Stage::Matching(0); _],
            source: source.chars(),
            byte_ptr: 0,
            char_ptr: 0,
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Result<Token, BadChar>;

    fn next(&mut self) -> Option<Self::Item> {
        let byte_begin = self.byte_ptr;
        let char_begin = self.char_ptr; // TODO: is it really usefull?
        self.stages = [Stage::Matching(0); _];
        while let Some(c) = self.source.next() {
            self.byte_ptr += c.len_utf8();
            self.char_ptr += 1;
            if SKIP.contains(&c) {
                continue;
            }

            let mut matched = Matched::None;
            for (i, p) in PATTERNS.iter().enumerate() {
                let stage = &mut self.stages[i];
                stage.test_pattern(p, c);
                matched = match (stage, matched) {
                    (Stage::Out, _) => matched,
                    (_, Matched::None) => Matched::Single(i),
                    (_, Matched::Single(_)) | (_, Matched::Multi) => Matched::Multi,
                }
            }
            match matched {
                Matched::None => {
                    return Some(Err(BadChar {
                        expected: "TODO",
                        found: c,
                        pos: (0, 0), // TODO
                    }));
                }
                Matched::Single(index) => {
                    return Some(Ok(Token {
                        kind: PATTERNS[index].0,
                        bytes: byte_begin..self.byte_ptr,
                    }));
                }
                Matched::Multi => {
                    // TODO: consider priority logic such as VariableX > Identifier
                }
            }
        }
        // TODO: let source go back one char or consider using peek
        // or do we really need to?
        None
    }
}

#[derive(Clone, Copy)]
enum Stage {
    Matching(usize),
    Out,
    End,
}

impl Stage {
    fn test_pattern(&mut self, pattern: &Pattern, c: char) {
        *self = match self {
            Stage::Matching(pos) => {
                // TODO
                Stage::Out
            }
            Stage::Out => Stage::Out,
            Stage::End => {
                // TODO
                Stage::Out
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Matched {
    None,
    Single(usize),
    Multi,
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
