use std::ops::Range;

use crate::{
    Cursor, Lex,
    lex::{Kind, KindSet, Token},
    syn::{ExpectToken, Prependable, Syn, SynError},
};

#[derive(Clone)]
pub(crate) enum Expr<'src> {
    Num { integer: u32, decimal: u32 },
    Param(&'src str),
    Fn(&'src str, Vec<Expr<'src>>),
    VarX,
    VarY,
    Paren(Box<Expr<'src>>),
    Neg(Box<Expr<'src>>),
    Pow(Box<Expr<'src>>, Box<Expr<'src>>),
    Mul(Box<Expr<'src>>, Box<Expr<'src>>),
    Div(Box<Expr<'src>>, Box<Expr<'src>>),
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
    Sub(Box<Expr<'src>>, Box<Expr<'src>>),
    Eq(Box<Expr<'src>>, Box<Expr<'src>>),
}

impl<'src> Syn<'src> for Expr<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError> {
        let accu = Self::unit(lex, true)?;
        loop {
            match lex.next_token()? {
                Token {
                    kind: Kind::BraceR | Kind::ParenR | Kind::Comma,
                    ..
                } => return Ok(accu),
                token => lex.prepend(Ok(token)),
            }
            let mid = lex.next_token()?;
            todo!()
        }
    }
}

impl<'src> Expr<'src> {
    fn unit(lex: &mut Prependable<Lex<'src>>, is_start: bool) -> Result<Self, SynError> {
        match lex.next_token()? {
            Token { kind: Kind::Num, string, range } => {
                let (integer, decimal) = num(string, range)?;
                Ok(Self::Num { integer, decimal })
            }
            Token { kind: Kind::Ident, string, .. } => {
                let next = lex.next_token()?;
                if next.kind == Kind::ParenL {
                    let args = args(lex)?;
                    let _ = lex.next_kind(Kind::ParenR)?;
                    Ok(Self::Fn(string, args))
                } else {
                    lex.prepend(Ok(next));
                    Ok(Self::Param(string))
                }
            }
            Token { kind: Kind::VarX, .. } => Ok(Self::VarX),
            Token { kind: Kind::VarY, .. } => Ok(Self::VarY),
            Token { kind: Kind::ParenL, .. } => {
                let inner = Self::parse(lex)?;
                let _ = lex.next_kind(Kind::ParenR)?;
                Ok(Self::Paren(Box::new(inner)))
            }
            Token { kind: Kind::Sub, .. } if is_start => {
                let inner = Self::parse(lex)?;
                Ok(Self::Neg(Box::new(inner)))
            }
            Token { kind: Kind::Sub, range, .. } => {
                Err(SynError::NegInsideExpr { cursor: range.start })
            }
            Token { string, range, .. } => Err(SynError::UnexpectedToken {
                expected: KindSet::with_values([
                    Kind::Num,
                    Kind::Ident,
                    Kind::VarX,
                    Kind::VarY,
                    Kind::ParenL,
                    Kind::Sub,
                ]),
                found: string.to_owned(),
                range: range,
            }),
        }
    }
}

fn num<'src>(string: &'src str, range: Range<Cursor>) -> Result<(u32, u32), SynError> {
    let mut integer = 0;
    let mut decimal = 0;
    let mut curr = &mut integer;
    for c in string.chars() {
        match c {
            '.' => curr = &mut decimal,
            c @ '0'..='9' => {
                *curr = match u32::checked_mul(10, *curr) {
                    None => return Err(SynError::NumOverflow { range }),
                    Some(n) => match u32::checked_add(n, c as u32 - '0' as u32) {
                        None => return Err(SynError::NumOverflow { range }),
                        Some(next) => next,
                    },
                };
            }
            _ => return Err(SynError::NumInvalid { range }),
        }
    }
    Ok((integer, decimal))
}

fn args<'src>(lex: &mut Prependable<Lex<'src>>) -> Result<Vec<Expr<'src>>, SynError> {
    todo!()
}
