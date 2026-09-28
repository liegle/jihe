use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    iter::Peekable,
    sync::LazyLock,
};

use crate::{
    Cursor, Lex, SynError,
    lex::{Kind, KindSet, Token},
    syn::{ExpectKind, Syn, expr::Expr},
};

pub(crate) struct Tree<'src> {
    pub(super) statements: Vec<Statement<'src>>,
}

pub(crate) struct Statement<'src> {
    pub(super) name: &'src str,
    pub(super) class: Class<'src>,
}

struct Field<'src> {
    slot: Slot<'src>,
    check: Check,
}

enum Slot<'src> {
    None,
    Parsed(Expr<'src>),
    Default(Expr<'src>),
}

impl<'src> Slot<'src> {
    fn get(self, name: &'static str, cursor: Cursor) -> Result<Expr<'src>, SynError> {
        match self {
            Self::None => Err(SynError::StatementFieldLost { name, cursor }),
            Self::Parsed(f) | Self::Default(f) => Ok(f),
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

struct Constructor {
    named: for<'src> fn(&mut Peekable<Lex<'src>>) -> Result<Class<'src>, SynError>,
    unnamed: for<'src> fn(&mut Peekable<Lex<'src>>) -> Result<Class<'src>, SynError>,
}

macro_rules! enum_class {
    ($($class:ident {$($field:ident:$check:ident$(=$slot:expr)?),+}),+) => {
        pub(crate) enum Class<'src> {
            $($class{$($field: Expr<'src>),+}),+
        }

        static CONSTRUCTORS: LazyLock<HashMap<&'static str, Constructor>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            paste::paste! {$(
                map.insert(
                    stringify!($class),
                    Constructor {
                        named: [<$class:snake:lower _named>],
                        unnamed: [<$class:snake:lower _unnamed>],
                    }
                );
            )+}
            map
        });

        paste::paste! {$(
            fn [<$class:snake:lower _named>]<'src>(
                lex: &mut Peekable<Lex<'src>>
            ) -> Result<Class<'src>, SynError> {
                let mut fields = HashMap::new();
                $(
                    fields.insert(
                        stringify!($field),
                        Field {
                            check:check!($check),
                            slot: default_slot!($($slot)?)
                        }
                    );
                )+
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
                                return Err(SynError:: UnexpectedStatementFieldKind {
                                    expected: field.check,
                                    cursor: range.start,
                                });
                            }
                            Slot::Parsed(expr)
                        },
                        Slot::Parsed(_) => {
                            return Err(SynError::DuplicatedStatementField {
                                name: string.to_owned(),
                                range,
                            });
                        }
                    }
                }
                lex.maybe_kind(Kind::Comma);
                let end = lex.next_kind(Kind::BraceR)?.range.end;
                $(let $field =
                    fields.remove(stringify!($field)).unwrap().slot.get(stringify!($field), end)?;)+
                Ok(Class::$class{ $($field),+ })
            }

            fn [<$class:snake:lower _unnamed>]<'src>(
                lex: &mut Peekable<Lex<'src>>
            ) -> Result<Class<'src>, SynError> {
                let mut fields = vec![$(
                    Field {
                        check:check!($check),
                        slot: default_slot!($($slot)?)
                    }
                ),+];
                let mut is_first = true;
                for field in &mut fields {
                    if is_first {
                        is_first = false;
                    } else {
                        let _ = lex.next_kind(Kind::Comma)?;
                    }
                    match lex.peek() {
                        Some(Ok(Token { kind: Kind::ParenR, .. })) => break,
                        Some(Ok(Token { range, .. })) => field.slot = {
                            let start = range.start;
                            let expr = Expr::parse(lex)?;
                            if !field.check.check(&expr) {
                                return Err(SynError::UnexpectedStatementFieldKind {
                                    expected: field.check,
                                    cursor: start,
                                });
                            }
                            Slot::Parsed(expr)
                        },
                        Some(Err(_)) => return Err(SynError::LexError(lex.next().unwrap().unwrap_err())),
                        None => return Err(SynError::UnexpectedEof),
                    }
                }
                lex.maybe_kind(Kind::Comma);
                let end = lex.next_kind(Kind::ParenR)?.range.end;
                $(let $field = fields.remove(0).slot.get(stringify!($field), end)?;)+
                Ok(Class::$class{ $($field),+ })
            }
        )+}
    };
}

impl<'src> Syn<'src> for Class<'src> {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
        let Token { string, range, .. } = lex.next_kind(Kind::Ident)?;
        let Some(Constructor { named, unnamed }) = CONSTRUCTORS.get(string) else {
            return Err(SynError::UndefinedStatementKind { found: string.to_owned(), range });
        };
        let Some(l) = lex.next() else {
            return Err(SynError::UnexpectedEof);
        };
        match l {
            Ok(Token { kind: Kind::BraceL, .. }) => Ok(named(lex)?),
            Ok(Token { kind: Kind::ParenL, .. }) => Ok(unnamed(lex)?),
            Ok(Token { string, range, .. }) => Err(SynError::UnexpectedToken {
                expected: KindSet::with_values([Kind::BraceL, Kind::ParenL]),
                found: string.to_owned(),
                range,
            }),
            Err(e) => Err(SynError::LexError(e)),
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
