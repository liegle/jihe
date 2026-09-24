use crate::{
    intset::{self, IntSet},
    lex::automata::Automata,
};

use std::sync::LazyLock;

pub(super) const SKIP: &[char] = &[' ', '\t', '\n', '\r'];

#[derive(Debug)]
pub(crate) struct Token<'src> {
    pub(crate) kind: Kind,
    pub(crate) string: &'src str,
}

pub(crate) struct Pattern {
    pub(super) kind: Kind,
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
    ($kind:expr, $prio:literal, $($ch:tt $rpt:tt),*) => {
        {
            Pattern {
                kind: $kind,
                priority: $prio,
                automata: Automata::new(&[$((character!($ch), repeat!($rpt)),)*]),
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
macro_rules! enum_kind {
    ($(($prio:literal)$kind:ident = [$($patt:tt)*])*) => {
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
        #[repr(u8)]
        pub enum Kind {
            #[default]
            $($kind,)*
        }

        pub(super) static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| vec![
            $(pattern!(Kind::$kind, $prio, $($patt)*),)*
        ]);

        pub(super) const PATTERN_COUNT: usize = {
            let count = $({ let _ = $prio; 1 } + )* 0;
            assert!(count < IntSet::CAPACITY, "Count of kind is to large to use intset");
            count as usize
        };
    };
}

#[rustfmt::skip]
enum_kind! {
    (0)Number      = [(num)+, ('.')?, (num)*]
    (0)Identifier  = [(uni)+, ('\'')*]
    (1)VariableX   = [('x')!]
    (1)VariableY   = [('y')!]
    (0)BraceL      = [('{')!]
    (0)BraceR      = [('}')!]
    (0)ParentheseL = [('(')!]
    (0)ParentheseR = [(')')!]
    (0)Power       = [('^')!]
    (0)Multiply    = [('*')!]
    (0)Divide      = [('/')!]
    (0)Plus        = [('+')!]
    (0)Minus       = [('-')!]
    (0)Equal       = [('=')!]
    (0)Comma       = [(',')!]
    (0)Colon       = [(':')!]
}

#[derive(Clone, Copy, Debug)]
pub struct KindSet(IntSet);

impl KindSet {
    pub(crate) fn with_values<T: IntoIterator<Item = Kind>>(values: T) -> Self {
        Self(IntSet::with_values(values.into_iter().map(|k| k as u8)))
    }
}

pub struct IntoIter(intset::IntoIter);

impl IntoIterator for KindSet {
    type Item = Kind;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl Iterator for IntoIter {
    type Item = Kind;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.0.next() {
            if let Some(pattern) = PATTERNS.get(index as usize) {
                return Some(pattern.kind);
            }
        }
        None
    }
}
