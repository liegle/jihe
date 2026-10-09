use std::ops::Range;

use crate::{
    Cursor, Lex, Span, Spanned,
    lex::{Token, TokenSet},
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
            match lex.expect_token()? {
                (Token::BraceR | Token::ParenR | Token::Comma, ..) => return Ok(accu),
                token => lex.prepend(Ok(token)),
            }
            let mid = lex.expect_token()?;
            todo!()
        }
    }
}

impl<'src> Expr<'src> {
    fn unit(lex: &mut Prependable<Lex<'src>>, is_start: bool) -> Result<Self, SynError> {
        match lex.expect_token()? {
            (Token::Num, Span { byte_span: byte, cursor_span: cursor, .. }) => {
                let (integer, decimal) = num(&lex.inner().source()[byte], cursor)?;
                Ok(Self::Num { integer, decimal })
            }
            (Token::Ident, Span { byte_span: byte, .. }) => {
                let next = lex.expect_token()?;
                let ident = &lex.inner().source()[byte];
                if next.0 == Token::ParenL {
                    let args = args(lex)?;
                    let _ = lex.expect_token_is(Token::ParenR)?;
                    Ok(Self::Fn(ident, args))
                } else {
                    lex.prepend(Ok(next));
                    Ok(Self::Param(ident))
                }
            }
            (Token::VarX, ..) => Ok(Self::VarX),
            (Token::VarY, ..) => Ok(Self::VarY),
            (Token::ParenL, ..) => {
                let inner = Self::parse(lex)?;
                let _ = lex.expect_token_is(Token::ParenR)?;
                Ok(Self::Paren(Box::new(inner)))
            }
            (Token::Sub, ..) if is_start => {
                let inner = Self::parse(lex)?;
                Ok(Self::Neg(Box::new(inner)))
            }
            (Token::Sub, Span { cursor_span, .. }) => {
                Err(SynError::NegInsideExpr { cursor: cursor_span.start })
            }
            (_, Span { byte_span, cursor_span, .. }) => Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([
                    Token::Num,
                    Token::Ident,
                    Token::VarX,
                    Token::VarY,
                    Token::ParenL,
                    Token::Sub,
                ]),
                found: lex.inner().source()[byte_span].to_owned(),
                cursor_span,
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
