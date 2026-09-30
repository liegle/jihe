use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    sync::LazyLock,
};

use crate::{
    Lex, SynError,
    lex::{Kind, KindSet, Token},
    syn::{ExpectToken, Prependable, Syn, expr::Expr},
};

pub(crate) struct Tree<'src> {
    pub(super) statements: Vec<Statement<'src>>,
}

pub(crate) struct Statement<'src> {
    pub(super) name: &'src str,
    pub(super) class: Class<'src>,
}

#[derive(Clone)]
enum Slot<'src> {
    None,
    Parsed(Expr<'src>),
    Default(Expr<'src>),
}

impl<'src> Slot<'src> {
    fn get(self) -> Option<Expr<'src>> {
        match self {
            Self::None => None,
            Self::Parsed(f) | Self::Default(f) => Some(f),
        }
    }
}

#[rustfmt::skip]
macro_rules! default_slot {
    () => { Slot::None };
    ($default:expr) => { Slot::Default($default) };
}

#[derive(Clone, Copy, Debug)]
pub enum Check {
    None,
    Fn,
    Eq,
    Def,
}

impl Display for Check {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let example = match self {
            Self::None => "",
            Self::Fn => "_(_)",
            Self::Eq => "_ = _",
            Self::Def => "_(_) = _",
        };
        write!(f, "{}", example)
    }
}

impl Check {
    fn check(&self, expr: &Expr) -> bool {
        match self {
            Self::None => true,
            Self::Fn => matches!(expr, Expr::Fn(..)),
            Self::Eq => matches!(expr, Expr::Eq(..)),
            Self::Def => matches!(expr, Expr::Eq(l, _) if matches!(**l, Expr::Fn(..))),
        }
    }
}

#[rustfmt::skip]
macro_rules! check {
    (expr) => { Check::None };
    (fn) => { Check::Fn };
    (eq) => { Check::Eq };
    (def) => { Check::Def };
}

#[derive(Clone)]
struct Field<'src> {
    slot: Slot<'src>,
    check: Check,
}

struct Pair {
    named: (
        HashMap<&'static str, Field<'static>>,
        for<'src> fn(HashMap<&'static str, Field<'src>>) -> Result<Class<'src>, &'static str>,
    ),
    unnamed: (
        Vec<Field<'static>>,
        for<'src> fn(Vec<Field<'src>>) -> Result<Class<'src>, &'static str>,
    ),
}

macro_rules! enum_class {
    ($($class:ident {$($field:ident:$check:ident$(=$slot:expr)?),+}),+) => {
        pub(crate) enum Class<'src> {
            $($class {
                $($field: Expr<'src>),+
            }),+
        }

        static CONSTRUCTORS: LazyLock<HashMap<&'static str, Pair>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            paste::paste! {$(
                // TODO: hash_map_macro #144032
                let mut named_fields = HashMap::new();
                $(
                    named_fields.insert(
                        stringify!($field),
                        Field { slot: default_slot!($($slot)?), check: check!($check) }
                    );
                )+
                let unnamed_fields = vec![$(
                    Field { slot: default_slot!($($slot)?), check: check!($check) }
                ),+];
                map.insert(
                    stringify!($class),
                    Pair {
                        named: (named_fields, [<$class:snake:lower _named>]),
                        unnamed: (unnamed_fields, [<$class:snake:lower _unnamed>]),
                    }
                );
            )+}
            map
        });

        paste::paste! {$(
            fn [<$class:snake:lower _named>]<'src>(
                mut fields: HashMap<&'static str, Field<'src>>
            ) -> Result<Class<'src>, &'static str> {
                $(
                    let Some($field) = fields.remove(stringify!($field)) else {
                        return Err(stringify!($field));
                    };
                    let Some($field) = $field.slot.get() else {
                        return Err(stringify!($field));
                    };
                )+
                Ok(Class::$class { $($field),+ })
            }

            fn [<$class:snake:lower _unnamed>]<'src>(
                mut fields: Vec<Field<'src>>
            ) -> Result<Class<'src>, &'static str> {
                fields.reverse();
                $(
                    // TODO: vec_try_remove #146954
                    let Some($field) = fields.pop() else {
                        return Err(stringify!($field));
                    };
                    let Some($field) = $field.slot.get() else {
                        return Err(stringify!($field));
                    };
                )+
                Ok(Class::$class { $($field),+ })
            }
        )+}
    };
}

impl<'src> Class<'src> {
    fn parse_named(
        lex: &mut Prependable<Lex<'src>>,
        fields: &HashMap<&'static str, Field<'static>>,
        insert: fn(HashMap<&'static str, Field<'src>>) -> Result<Class<'src>, &'static str>,
    ) -> Result<Class<'src>, SynError> {
        let mut fields = fields.clone();
        let mut is_first = true;
        for _ in 0..fields.len() {
            if is_first {
                is_first = false;
            } else {
                let _ = lex.next_kind(Kind::Comma)?;
            }
            let Token { string, range, .. } = lex.next_kind(Kind::Ident)?;
            let Some(field) = fields.get_mut(string) else {
                return Err(SynError::UndefinedStatementKind { found: string.to_owned(), range });
            };
            let _ = lex.next_kind(Kind::Colon)?;
            field.slot = match field.slot {
                Slot::None | Slot::Default(_) => {
                    let expr = Expr::parse(lex)?;
                    if !field.check.check(&expr) {
                        return Err(SynError::UnexpectedStatementFieldKind {
                            expected: field.check,
                            cursor: range.start,
                        });
                    }
                    Slot::Parsed(expr)
                }
                Slot::Parsed(_) => {
                    return Err(SynError::DuplicatedStatementField {
                        name: string.to_owned(),
                        range,
                    });
                }
            }
        }
        lex.next_if_kind(Kind::Comma);
        let end = lex.next_kind(Kind::BraceR)?.range.end;
        insert(fields).map_err(|name| SynError::StatementFieldLost { name, end })
    }

    fn parse_unnamed(
        lex: &mut Prependable<Lex<'src>>,
        fields: &Vec<Field<'static>>,
        insert: fn(Vec<Field<'src>>) -> Result<Class<'src>, &'static str>,
    ) -> Result<Class<'src>, SynError> {
        let mut fields = fields.clone();
        let mut is_first = true;
        for field in &mut fields {
            if is_first {
                is_first = false;
            } else {
                let _ = lex.next_kind(Kind::Comma)?;
            }
            match lex.next_token()? {
                Token { kind: Kind::ParenR, .. } => break,
                token => {
                    let start = token.range.start;
                    lex.prepend(Ok(token));
                    let expr = Expr::parse(lex)?;
                    if !field.check.check(&expr) {
                        return Err(SynError::UnexpectedStatementFieldKind {
                            expected: field.check,
                            cursor: start,
                        });
                    }
                    field.slot = Slot::Parsed(expr)
                }
            }
        }
        lex.next_if_kind(Kind::Comma);
        let end = lex.next_kind(Kind::ParenR)?.range.end;
        insert(fields).map_err(|name| SynError::StatementFieldLost { name, end })
    }
}

impl<'src> Syn<'src> for Class<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError> {
        let Token { string, range, .. } = lex.next_kind(Kind::Ident)?;
        let Some(Pair { named, unnamed }) = CONSTRUCTORS.get(string) else {
            return Err(SynError::UndefinedStatementKind { found: string.to_owned(), range });
        };
        match lex.next_token()? {
            Token { kind: Kind::BraceL, .. } => {
                let (fields, insert) = named;
                Ok(Self::parse_named(lex, fields, *insert)?)
            }
            Token { kind: Kind::ParenL, .. } => {
                let (fields, insert) = unnamed;
                Ok(Self::parse_unnamed(lex, fields, *insert)?)
            }
            Token { string, range, .. } => Err(SynError::UnexpectedToken {
                expected: KindSet::with_values([Kind::BraceL, Kind::ParenL]),
                found: string.to_owned(),
                range,
            }),
        }
    }
}

macro_rules! num {
    ($i:literal) => {
        Expr::Num { integer: $i, decimal: 0 }
    };
}

macro_rules! color {
    ($r:literal, $g:literal, $b:literal) => {
        Expr::Fn("color", vec![num!($r), num!($g), num!($b)])
    };
}

enum_class! {
    Param {
        from: expr,
        to: expr
    },
    Var {
        expr: expr
    },
    Fn {
        def: def
    },
    Point {
        x: expr,
        y: expr,
        size: expr = num!(3),
        color: fn = color!(0, 0, 0)
    },
    Curve {
        equation: eq,
        thickness: expr = num!(3),
        color: fn = color!(0, 0, 1)
    }
}
