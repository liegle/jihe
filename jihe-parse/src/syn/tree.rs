use std::{collections::HashMap, iter::Peekable, sync::LazyLock};

use crate::{
    Lex, SynError,
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

enum Field<'src> {
    None,
    Parsed(Expr<'src>),
    Default(Expr<'src>),
}

impl<'src> Field<'src> {
    fn unwrap(self, name: &'static str) -> Result<Expr<'src>, SynError> {
        match self {
            Field::None => Err(SynError::StatementFieldLost { name }),
            Self::Parsed(f) | Self::Default(f) => Ok(f),
        }
    }
}

#[rustfmt::skip]
macro_rules! default_field {
    () => { Field::None };
    ($default:expr) => { Field::Default($default) };
}

struct Constructor {
    named: for<'src> fn(&mut Peekable<Lex<'src>>) -> Result<Class<'src>, SynError>,
    unnamed: for<'src> fn(&mut Peekable<Lex<'src>>) -> Result<Class<'src>, SynError>,
}

macro_rules! enum_class {
    ($($class:ident={$($field:ident$(=$default:expr)?),+})+) => {
        pub(crate) enum Class<'src> {
            $($class{$($field: Expr<'src>),+}),+
        }

        static CONSTRUCTORS: LazyLock<HashMap<&'static str, Constructor>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            paste::paste! {$(
                map.insert(
                    stringify!($class),
                    Constructor {
                        named: [<$class:lower _named>],
                        unnamed: [<$class:lower _unnamed>],
                    }
                );
            )+}
            map
        });

        impl<'src> Syn<'src> for Class<'src> {
            fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
                let Token { string, range, .. } = lex.next_kind(Kind::Identifier)?;
                let Some(Constructor { named, unnamed }) = CONSTRUCTORS.get(string) else {
                    return Err(SynError::UndefinedStatementKind {
                        found: string.to_owned(),
                        range,
                    });
                };
                let Some(l) = lex.next() else {
                    return Err(SynError::UnexpectedEof);
                };
                match l {
                    Ok(Token { kind: Kind::BraceL, .. }) => Ok(named(lex)?),
                    Ok(Token { kind: Kind::ParentheseL, .. }) => Ok(unnamed(lex)?),
                    Ok(Token { string, range, .. }) => Err(SynError::UnexpectedToken {
                        expected: KindSet::with_values([Kind::BraceL, Kind::ParentheseL]),
                        found: string.to_owned(),
                        range,
                    }),
                    Err(e) => Err(SynError::LexError(e)),
                }
            }
        }

        paste::paste! {$(
            fn [<$class:snake:lower _named>]<'src>(
                lex: &mut Peekable<Lex<'src>>
            ) -> Result<Class<'src>, SynError> {
                let mut fields = HashMap::new();
                $(fields.insert(stringify!($field), default_field!($($default)?));)+
                let mut is_first = true;
                for _ in 0..fields.len() {
                    if is_first {
                        is_first = false;
                    } else {
                        let _ = lex.next_kind(Kind::Comma)?;
                    }
                    let Token { string, range, .. } = lex.next_kind(Kind::Identifier)?;
                    let Some(field) = fields.get_mut(string) else {
                        return Err(SynError::UndefinedStatementKind { found: string.to_owned(), range });
                    };
                    let _ = lex.next_kind(Kind::Colon)?;
                    *field = match field {
                        Field::None | Field::Default(_) => Field::Parsed(Expr::parse(lex)?),
                        Field::Parsed(_) => {
                            return Err(SynError::DuplicatedStatementField {
                                name: string.to_owned(),
                                range,
                            });
                        }
                    }
                }
                lex.maybe_kind(Kind::Comma);
                let _ = lex.next_kind(Kind::BraceR)?;
                $(let $field = fields.remove(stringify!($field)).unwrap().unwrap(stringify!($field))?;)+
                Ok(Class::$class{ $($field),+ })
            }

            fn [<$class:snake:lower _unnamed>]<'src>(
                lex: &mut Peekable<Lex<'src>>
            ) -> Result<Class<'src>, SynError> {
                let mut fields = vec![$(default_field!($($default)?)),+];
                let mut is_first = true;
                for field in &mut fields {
                    if is_first {
                        is_first = false;
                    } else {
                        let _ = lex.next_kind(Kind::Comma)?;
                    }
                    match lex.peek() {
                        Some(Ok(Token { kind: Kind::BraceR, .. })) => break,
                        Some(_) => *field = Field::Parsed(Expr::parse(lex)?), // let expr handle lex error
                        None => return Err(SynError::UnexpectedEof),
                    }
                }
                lex.maybe_kind(Kind::Comma);
                let _ = lex.next_kind(Kind::ParentheseR)?;
                $(
                    let $field = fields.remove(0).unwrap(stringify!($field))?;
                )+
                Ok(Class::$class{ $($field),+ })
            }
        )+}
    };
}

macro_rules! number {
    ($i:literal) => {
        Expr::Number { negative: false, integer: $i, decimal: 0 }
    };
}

macro_rules! color {
    ($r:literal, $g:literal, $b:literal) => {
        Expr::FunctionCall("color", vec![number!($r), number!($g), number!($b)])
    };
}

enum_class! {
    Param = {
        from,
        to
    }
    Var = {
        expr
    }
    Point = {
        x,
        y,
        size = number!(3),
        color = color!(0, 0, 0)
    }
    Curve = {
        equation,
        thickness = number!(3),
        color = color!(0, 0, 1)
    }
}
