use std::ops::Range;

pub(super) const SKIP: &[char] = &[' ', '\t', '\n', '\r'];

#[derive(Debug)]
pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) bytes: Range<usize>,
}

pub(super) struct Pattern {
    pub(super) kind: Kind,
    pub(super) priority: u8,
    pub(super) expression: &'static [(Character, Repeat)],
    pub(super) endable_index: usize,
}

const fn calc_endable_index(expression: &[(Character, Repeat)]) -> usize {
    assert!(
        !expression.is_empty(),
        "Technically no expression should be empty"
    );
    let mut index = expression.len() - 1;
    while index > 0 {
        if !expression[index].1.accepts(0) {
            return index - 1;
        }
        index -= 1;
    }
    0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Character {
    Single(char),
    Number,
    Unicode,
}

impl Character {
    pub(super) fn contains(&self, c: char) -> bool {
        match self {
            Self::Single(ch) => *ch == c,
            Self::Number => c.is_ascii_digit(),
            Self::Unicode => c.is_ascii_alphabetic() || !c.is_ascii(),
        }
    }
}

#[allow(dead_code)] // TODO: add a token kind that uses NoneOrOnce
#[derive(Clone, Copy, Debug)]
pub(super) enum Repeat {
    Any,
    NoneOrOnce,
    Once,
    OnceOrMultiple,
}

impl Repeat {
    pub(super) const fn accepts(&self, count: usize) -> bool {
        matches!(
            (*self, count),
            (Repeat::Any, _)
                | (Repeat::NoneOrOnce, 0 | 1)
                | (Repeat::Once, 1)
                | (Repeat::OnceOrMultiple, 1..)
        )
    }
}

#[rustfmt::skip]
macro_rules! pattern {
    ($kind:expr, $prio:literal, $($ch:tt $rpt:tt),*) => {
        {
            let expression = &[$((character!($ch), repeat!($rpt)),)*];
            Pattern {
                kind: $kind,
                priority: $prio,
                expression,
                endable_index: calc_endable_index(expression),
            }
        }
    };
}

#[rustfmt::skip]
macro_rules! character {
    (($ch:literal)) => { Character::Single($ch) };
    ((number))      => { Character::Number };
    ((unicode))     => { Character::Unicode };
}

#[rustfmt::skip]
macro_rules! repeat {
    (*) => { Repeat::Any };
    (?) => { Repeat::NoneOrOnce };
    (!) => { Repeat::Once };
    (+) => { Repeat::OnceOrMultiple };
}

#[rustfmt::skip]
macro_rules! enum_kind {
    ($(($prio:literal)$kind:ident = [$($patt:tt)*])*) => {
        #[derive(Clone, Copy, Debug)]
        pub enum Kind {
            $($kind,)*
        }

        pub(super) const PATTERNS: &[Pattern] = &[
            $(pattern!(Kind::$kind, $prio, $($patt)*),)*
        ];
    };
}

#[rustfmt::skip]
enum_kind! {
    (0)Integer     = [(number)+]
    (0)Fraction    = [(number)+, ('.')!, (number)+]
    (0)Identifier  = [(unicode)+, ('\'')*]
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
