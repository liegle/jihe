pub(super) const SKIP: &[char] = &[' ', '\t', '\n', '\r'];

#[derive(Debug)]
pub(super) struct Token<'src> {
    pub(super) kind: Kind,
    pub(super) string: &'src str,
}

pub(super) struct Pattern {
    pub(super) kind: Kind,
    pub(super) priority: u8,
    pub(super) expression: &'static [(Character, Repeat)],
    pub(super) endable_index: usize,
}

const fn calc_endable_index(expression: &[(Character, Repeat)]) -> usize {
    {
        assert!(!expression.is_empty(), "No expression should be empty");
        let mut i = 0;
        let mut all_zeroable = true;
        while i < expression.len() {
            if !expression[i].1.accepts(0) {
                all_zeroable = false;
                break;
            }
            i += 1;
        }
        assert!(!all_zeroable, "No expression should be all zeroable");
    }

    let mut index = expression.len() - 1;
    while index > 0 {
        if !expression[index].1.accepts(0) {
            return index;
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
    ((num))         => { Character::Number };
    ((uni))         => { Character::Unicode };
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
        #[derive(Clone, Copy, Debug, Default)]
        pub enum Kind {
            #[default]
            $($kind,)*
        }

        pub(super) const PATTERNS: &[Pattern] = &[
            $(pattern!(Kind::$kind, $prio, $($patt)*),)*
        ];

        pub(super) const PATTERN_COUNT: usize = PATTERNS.len();
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

#[cfg(test)]
#[test]
fn test_calc_endable_index() {
    let p = pattern!(Kind::Number, 0, ('x')!, ('x')!, ('x')?).expression;
    assert_eq!(calc_endable_index(p), 1);

    let p = pattern!(Kind::Number, 0, ('x')!, ('x')*, ('x')!).expression;
    assert_eq!(calc_endable_index(p), 2);

    let p = pattern!(Kind::Number, 0, ('x')+, ('x')*, ('x')!, ('x')*).expression;
    assert_eq!(calc_endable_index(p), 2);
}
