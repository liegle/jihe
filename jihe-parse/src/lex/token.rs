use crate::{
    intset::{self, IntSet},
    lex::automata::Automata,
};

use std::sync::LazyLock;

pub(crate) struct Pattern {
    pub(super) token: Token,
    pub(super) priority: u8,
    pub(super) automata: Automata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Character {
    Single(char),
    Number,
    Unicode,
}

impl From<char> for Character {
    fn from(value: char) -> Self {
        match value {
            '0'..='9' => Character::Number,
            'a'..='z' | 'A'..='Z' => Character::Unicode,
            c if !c.is_ascii() => Character::Unicode,
            c => Character::Single(c),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Repeat {
    ZeroOrOne,
    One,
    OneOrMore,
    ZeroOrMore,
}

#[rustfmt::skip]
macro_rules! pattern {
    ($token:expr, $prio:literal, $($ch:tt $rpt:tt),+) => {
        {
            Pattern {
                token: $token,
                priority: $prio,
                automata: Automata::new(&[$((character!($ch), repeat!($rpt))),+]),
            }
        }
    };
}

#[rustfmt::skip]
macro_rules! character {
    (($ch:literal)) => { Character::Single($ch) };
    ((num))         => { Character::Number };
    ((uni))         => { Character::Unicode };
}

#[rustfmt::skip]
macro_rules! repeat {
    (?) => { Repeat::ZeroOrOne };
    (!) => { Repeat::One };
    (+) => { Repeat::OneOrMore };
    (*) => { Repeat::ZeroOrMore };
}

#[rustfmt::skip]
macro_rules! enum_token {
    ($(($prio:literal)$token:ident = [$($patt:tt)+])+) => {
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
        #[repr(u8)]
        pub enum Token {
            #[default]
            $($token),+
        }

        pub(super) static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| vec![
            $(pattern!(Token::$token, $prio, $($patt)+)),+
        ]);

        pub(super) const PATTERN_COUNT: usize = {
            let count = $({ let _ = $prio; 1 } + )+ 0;
            assert!(count < IntSet::CAPACITY, "Count of token is to large to use intset");
            count as usize
        };
    };
}

#[rustfmt::skip]
enum_token! {
    (0)Num    = [(num)+, ('.')?, (num)*]
    (0)Ident  = [(uni)+, ('\'')*]
    (0)BraceL = [('{')!]
    (0)BraceR = [('}')!]
    (0)ParenL = [('(')!]
    (0)ParenR = [(')')!]
    (0)Pow    = [('^')!]
    (0)Mul    = [('*')!]
    (0)Div    = [('/')!]
    (0)Add    = [('+')!]
    (0)Sub    = [('-')!]
    (0)Eq     = [('=')!]
    (0)Comma  = [(',')!]
    (0)Colon  = [(':')!]
}

#[derive(Clone, Copy, Debug)]
pub struct TokenSet(IntSet);

impl TokenSet {
    pub(crate) fn with_values<T: IntoIterator<Item = Token>>(values: T) -> Self {
        Self(IntSet::with_values(values.into_iter().map(|k| k as u8)))
    }
}

pub struct IntoIter(intset::IntoIter);

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
            if let Some(pattern) = PATTERNS.get(index as usize) {
                return Some(pattern.token);
            }
        }
        None
    }
}
