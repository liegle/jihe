use std::ops::Range;

pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) bytes: Range<usize>,
}

#[derive(Clone, Copy)]
pub(super) enum Kind {
    // Number
    Integer,
    Fraction,
    // Name
    Identifier,
    VariableX,
    VariableY,
    // Punctuation
    BraceL,
    BraceR,
    ParentheseL,
    ParentheseR,
    Power,
    Multiply,
    Divide,
    Plus,
    Minus,
    Equal,
    Comma,
}

pub(super) struct Pattern(pub(super) Kind, pub(super) &'static [(Character, Repeat)]);

pub(super) enum Character {
    Single(char),
    Number,
    Unicode,
}

pub(super) enum Repeat {
    Any,
    NoneOrOnce,
    Once,
    OnceOrMultiple,
}

#[rustfmt::skip]
macro_rules! pattern {
    ($k:expr, $($c:tt $r:tt),*) => { Pattern($k, &[$((character!($c), repeat!($r)),)*]) }
}

#[rustfmt::skip]
macro_rules! character {
    (($c:literal)) => { Character::Single($c) };
    ((number))     => { Character::Number };
    ((unicode))    => { Character::Unicode };
}

#[rustfmt::skip]
macro_rules! repeat {
    (*) => { Repeat::Any };
    (?) => { Repeat::NoneOrOnce };
    (!) => { Repeat::Once };
    (+) => { Repeat::OnceOrMultiple };
}

#[rustfmt::skip]
pub(super) const PATTERNS: &[Pattern] = &[
    pattern!(Kind::Integer,     (number)+),
    pattern!(Kind::Fraction,    (number)+, ('.')!, (number)+),
    pattern!(Kind::Identifier,  (unicode)+, ('\'')*),
    pattern!(Kind::VariableX,   ('x')!),
    pattern!(Kind::VariableY,   ('y')!),
    pattern!(Kind::BraceL,      ('{')!),
    pattern!(Kind::BraceR,      ('}')!),
    pattern!(Kind::ParentheseL, ('(')!),
    pattern!(Kind::ParentheseR, (')')!),
    pattern!(Kind::Power,       ('^')!),
    pattern!(Kind::Multiply,    ('*')!),
    pattern!(Kind::Divide,      ('/')!),
    pattern!(Kind::Plus,        ('+')!),
    pattern!(Kind::Minus,       ('-')!),
    pattern!(Kind::Equal,       ('=')!),
    pattern!(Kind::Comma,       (',')!),
];

pub(super) const SKIP: &[char] = &[' ', '\t', '\n', '\r'];
