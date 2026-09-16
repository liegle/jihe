use std::{
    cmp::Ordering,
    collections::HashSet,
    error,
    fmt::{self, Display, Formatter},
    iter::Peekable,
    ops::Range,
    str::Chars,
};

use crate::{
    cursor::Cursor,
    token::{Character, Kind, PATTERNS, Pattern, SKIP, Token},
};

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
            if curr_match.matching_count == 0 {
                if prev_match.matching_count == 0 {
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

        if prev_match.matching_count != 0 {
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

#[derive(Clone, Copy, Debug)]
enum Stage {
    Matching { index: usize, count: usize },
    Out,
}

#[derive(Clone)]
struct Match {
    stages: [Stage; PATTERNS.len()],
    matching_count: usize,
}

impl Match {
    fn new() -> Self {
        Self {
            stages: [Stage::Matching { index: 0, count: 0 }; _],
            matching_count: 0,
        }
    }

    fn step(&mut self, c: char) {
        self.matching_count = 0;
        for (stage, Pattern { expression, .. }) in self.stages.iter_mut().zip(PATTERNS) {
            *stage = if let Stage::Matching { index, count } = *stage {
                // Try to consume as many chars in one sub pattern as possible
                if let Some((character, repeat)) = expression.get(index)
                    && character.contains(c)
                    && repeat.accepts(count + 1)
                {
                    Stage::Matching { index, count: count + 1 }
                } else {
                    let mut windows = expression[index..].windows(2).enumerate();
                    let mut prev_count = count;
                    loop {
                        if let Some((index_add, [(_, prev_repeat), (curr_character, _)])) =
                            windows.next()
                        {
                            if !prev_repeat.accepts(prev_count) {
                                break Stage::Out;
                            }
                            prev_count = 0;
                            // 1 must be accepted
                            if curr_character.contains(c) {
                                break Stage::Matching {
                                    index: index + index_add + 1,
                                    count: 1,
                                };
                            }
                        } else {
                            break Stage::Out;
                        }
                    }
                }
            } else {
                Stage::Out
            };
            if let Stage::Matching { .. } = stage {
                self.matching_count += 1;
            }
        }
    }

    fn cmp_priority(&self) -> Vec<Kind> {
        let mut greatest_priority = 0;
        let mut matched = Vec::new();
        for (
            stage,
            Pattern {
                kind, priority, endable_index, ..
            },
        ) in self.stages.iter().zip(PATTERNS)
        {
            if !matches!(stage, Stage::Matching { index, .. } if *index >= *endable_index) {
                continue;
            }
            match priority.cmp(&greatest_priority) {
                Ordering::Greater => {
                    greatest_priority = *priority;
                    matched.clear();
                    matched.push(*kind);
                }
                Ordering::Equal => {
                    matched.push(*kind);
                }
                _ => {}
            }
        }
        matched
    }

    fn gather_expected(&self) -> HashSet<Character> {
        let mut expected = HashSet::new();
        for (stage, Pattern { expression, .. }) in self.stages.iter().zip(PATTERNS) {
            let Stage::Matching { index, count } = *stage else {
                continue;
            };
            if let Some((character, repeat)) = expression.get(index)
                && repeat.accepts(count + 1)
            {
                expected.insert(*character);
            }
            let mut windows = expression[index..].windows(2);
            let mut prev_count = count;
            while let Some([(_, prev_repeat), (curr_character, _)]) = windows.next() {
                if prev_repeat.accepts(prev_count) {
                    expected.insert(*curr_character);
                }
                prev_count = 0;
            }
        }
        expected
    }
}

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

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_none() {
        let mut lexer = Lexer::new("");
        assert_matches!(lexer.next(), None);
    }

    #[test]
    fn test_whitespaces() {
        let mut lexer = Lexer::new("   \n\t\r   ");
        assert_matches!(lexer.next(), None);
    }

    #[test]
    fn test_all() {
        let mut lexer = Lexer::new("325 \t 0.6 \n 🍎 xx yy x y { () } \r ^*/ + - = ,:");
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Integer, bytes })) if bytes == (0..3));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Fraction, bytes })) if bytes == (6..9));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (12..16));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (17..19));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (20..22));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::VariableX, bytes })) if bytes == (23..24));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::VariableY, bytes })) if bytes == (25..26));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::BraceL, bytes })) if bytes == (27..28));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::ParentheseL, bytes })) if bytes == (29..30));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::ParentheseR, bytes })) if bytes == (30..31));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::BraceR, bytes })) if bytes == (32..33));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Power, bytes })) if bytes == (36..37));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Multiply, bytes })) if bytes == (37..38));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Divide, bytes })) if bytes == (38..39));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Plus, bytes })) if bytes == (40..41));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Minus, bytes })) if bytes == (42..43));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Equal, bytes })) if bytes == (44..45));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Comma, bytes })) if bytes == (46..47));
        assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Colon, bytes })) if bytes == (47..48));
        assert_matches!(lexer.next(), None);
    }

    #[test]
    fn test_unexcepted_begin() {
        let mut lexer = Lexer::new("  \n \t @");
        assert_matches!(
            lexer.next(),
            Some(Err(LexerError::UnexpectedBegin {
                found: '@',
                cursor: Cursor { line: 1, col: 3 }
            }))
        );
    }
}
