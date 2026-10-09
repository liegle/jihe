use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    sync::LazyLock,
};

use crate::{
    Lex, Span, Spanned, SynError,
    lex::{Token, TokenSet},
    syn::{
        ExpectToken, Prependable, Syn,
        expr::{Binary, Expr, Primary, Unary},
    },
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
            Self::Fn => matches!(expr, Expr::Unary(Unary::Fn(..), ..)),
            Self::Eq => matches!(expr, Expr::Binary(Binary::Eq, _, _)),
            Self::Def => {
                matches!(
                    expr,
                    Expr::Binary(Binary::Eq, l, _) if matches!(**l, Expr::Unary(Unary::Fn(..), ..))
                )
            }
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
        start: &Span<'src>,
        fields: &HashMap<&'static str, Field<'static>>,
        insert: fn(HashMap<&'static str, Field<'src>>) -> Result<Class<'src>, &'static str>,
    ) -> Result<Spanned<'src, Class<'src>>, SynError> {
        let mut fields = fields.clone();
        let mut is_first = true;
        for _ in 0..fields.len() {
            if is_first {
                is_first = false;
            } else {
                let _ = lex.expect_token_is(Token::Comma)?;
            }
            let Span { byte_span, cursor_span, .. } = lex.expect_token_is(Token::Ident)?;
            let name = &lex.inner().source()[byte_span];
            let Some(field) = fields.get_mut(name) else {
                return Err(SynError::UndefinedStatementClass {
                    found: name.to_owned(),
                    cursor_span,
                });
            };
            let _ = lex.expect_token_is(Token::Colon)?;
            field.slot = match field.slot {
                Slot::None | Slot::Default(_) => {
                    let (expr, span) = Expr::parse(lex)?;
                    if !field.check.check(&expr) {
                        return Err(SynError::UnexpectedStatementFieldType {
                            expected: field.check,
                            cursor_span: span.cursor_span,
                        });
                    }
                    Slot::Parsed(expr)
                }
                Slot::Parsed(_) => {
                    return Err(SynError::DuplicatedStatementField {
                        name: name.to_owned(),
                        cursor_span,
                    });
                }
            }
        }
        lex.next_if_token_is(Token::Comma);
        let end = lex.expect_token_is(Token::BraceR)?;
        insert(fields)
            .map_err(|name| SynError::StatementFieldLost {
                name,
                cursor_span: end.cursor_span.clone(),
            })
            .map(|class| (class, Span::join(start, &end)))
    }

    fn parse_unnamed(
        lex: &mut Prependable<Lex<'src>>,
        start: &Span<'src>,
        fields: &Vec<Field<'static>>,
        insert: fn(Vec<Field<'src>>) -> Result<Class<'src>, &'static str>,
    ) -> Result<Spanned<'src, Class<'src>>, SynError> {
        let mut fields = fields.clone();
        let mut is_first = true;
        for field in &mut fields {
            if is_first {
                is_first = false;
            } else {
                let _ = lex.expect_token_is(Token::Comma)?;
            }
            match lex.expect_token()? {
                (Token::ParenR, ..) => break,
                spanned => {
                    lex.prepend(Ok(spanned));
                    let (expr, span) = Expr::parse(lex)?;
                    if !field.check.check(&expr) {
                        return Err(SynError::UnexpectedStatementFieldType {
                            expected: field.check,
                            cursor_span: span.cursor_span,
                        });
                    }
                    field.slot = Slot::Parsed(expr)
                }
            }
        }
        lex.next_if_token_is(Token::Comma);
        let end = lex.expect_token_is(Token::ParenR)?;
        insert(fields)
            .map_err(|name| SynError::StatementFieldLost {
                name,
                cursor_span: end.cursor_span.clone(),
            })
            .map(|class| (class, Span::join(start, &end)))
    }
}

impl<'src> Syn<'src> for Class<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        let start = lex.expect_token_is(Token::Ident)?;
        let class = &lex.inner().source()[start.byte_span.clone()];
        let Some(Pair { named, unnamed }) = CONSTRUCTORS.get(class) else {
            return Err(SynError::UndefinedStatementClass {
                found: class.to_owned(),
                cursor_span: start.cursor_span,
            });
        };
        match lex.expect_token()? {
            (Token::BraceL, ..) => {
                let (fields, insert) = named;
                Ok(Self::parse_named(lex, &start, fields, *insert)?)
            }
            (Token::ParenL, ..) => {
                let (fields, insert) = unnamed;
                Ok(Self::parse_unnamed(lex, &start, fields, *insert)?)
            }
            (_, Span { byte_span, cursor_span, .. }) => Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([Token::BraceL, Token::ParenL]),
                found: lex.inner().source()[byte_span].to_owned(),
                cursor_span,
            }),
        }
    }
}

macro_rules! num {
    ($i:literal) => {
        Expr::Primary(Primary::Num(stringify!($i)))
    };
}

macro_rules! color {
    ($r:literal, $g:literal, $b:literal) => {
        Expr::Unary(
            Unary::Fn("color"),
            Box::new(Expr::Binary(
                Binary::Comma,
                Box::new(num!($r)),
                Box::new(Expr::Binary(
                    Binary::Comma,
                    Box::new(num!($g)),
                    Box::new(num!($b)),
                )),
            )),
        )
    };
}

enum_class! {
    Slide {
        from: expr,
        to: expr
    },
    Bind {
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
