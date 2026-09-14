use std::ops::Range;

pub(super) const SKIP: &[char] = &[' ', '\t', '\n', '\r'];

pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) bytes: Range<usize>,
}

pub(super) struct Pattern(pub(super) Kind, &'static [(Character, Repeat)]);

#[derive(Clone, Copy)]
enum Character {
    Single(char),
    Number,
    Unicode,
}

impl Character {
    fn contains(&self, c: char) -> bool {
        match self {
            Self::Single(ch) => *ch == c,
            Self::Number => matches!(c, '0'..='9'),
            Self::Unicode => matches!(c, 'a'..='z' | 'A'..='Z') || !c.is_ascii(),
        }
    }
}

#[derive(Clone, Copy)]
enum Repeat {
    Any,
    NoneOrOnce,
    Once,
    OnceOrMultiple,
}

impl Repeat {
    fn accepts(&self, count: usize) -> bool {
        matches!(
            (*self, count),
            (Repeat::Any, _)
                | (Repeat::NoneOrOnce, 0 | 1)
                | (Repeat::Once, 1)
                | (Repeat::OnceOrMultiple, 1..)
        )
    }
}

#[derive(Clone, Copy)]
pub(super) enum Stage {
    Matching { index: usize, count: usize },
    Out,
}

impl Stage {
    pub(super) fn step_stage(&self, Pattern(_, patterns): &Pattern, c: char) -> Stage {
        if let Stage::Matching { index, count } = *self {
            // Try to consume as many chars in one sub pattern as possible
            if let Some((character, repeat)) = patterns.get(index)
                && character.contains(c)
                && repeat.accepts(count + 1)
            {
                return Stage::Matching {
                    index: index,
                    count: count + 1,
                };
            }

            let mut windows = patterns[index..].windows(2).enumerate();
            let mut prev_count = count;
            while let Some((index_add, [(_, prev_repeat), (curr_character, _)])) = windows.next() {
                if !prev_repeat.accepts(prev_count) {
                    return Stage::Out;
                }
                prev_count = 0;
                // 1 must be accepted
                if curr_character.contains(c) {
                    return Stage::Matching {
                        index: index + index_add + 1,
                        count: 1,
                    };
                }
            }
        }
        Stage::Out
    }
}

#[rustfmt::skip]
macro_rules! pattern {
    ($k:expr, $($c:tt $r:tt),*) => { Pattern($k, &[$((character!($c), repeat!($r)),)*]) };
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
macro_rules! enum_kind {
    ($($k:ident = [$($p:tt)*])*) => {
        #[derive(Clone, Copy)]
        pub(super) enum Kind {
            $($k,)*
        }

        pub(super) const PATTERNS: &[Pattern] = &[
            $(pattern!(Kind::$k, $($p)*),)*
        ];
    };
}

#[rustfmt::skip]
enum_kind! {
    Integer     = [(number)+]
    Fraction    = [(number)+, ('.')!, (number)+]
    Identifier  = [(unicode)+, ('\'')*]
    VariableX   = [('x')!]
    VariableY   = [('y')!]
    BraceL      = [('{')!]
    BraceR      = [('}')!]
    ParentheseL = [('(')!]
    ParentheseR = [(')')!]
    Power       = [('^')!]
    Multiply    = [('*')!]
    Divide      = [('/')!]
    Plus        = [('+')!]
    Minus       = [('-')!]
    Equal       = [('=')!]
    Comma       = [(',')!]
}
