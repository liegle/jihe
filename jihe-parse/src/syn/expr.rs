use std::ops::Range;

use crate::{
    Cursor, Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Prependable, Syn, SynError},
};

#[derive(Clone)]
pub(crate) enum Expr<'src> {
    Num(&'src str),
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
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        let accu = Self::unit(lex, true)?;
        loop {
            let spanned = lex.expect_token()?;
            let token = spanned.0;
            match token {
                Token::BraceR | Token::ParenR | Token::Comma => {
                    lex.prepend(Ok(spanned));
                    return Ok(accu);
                }
                // TODO: maybe no recursive, but statemachine
                token => todo!("token as mid, use it to prepend next unit to accu"),
            }
        }
    }
}

impl<'src> Expr<'src> {
    fn unit(
        lex: &mut Prependable<Lex<'src>>,
        is_start: bool,
    ) -> Result<Spanned<'src, Self>, SynError> {
        match lex.expect_token()? {
            (Token::Num, span) => Ok((
                Self::Num(&lex.inner().source()[span.byte_span.clone()]),
                span,
            )),
            (Token::Ident, start) => {
                let ident = &lex.inner().source()[start.byte_span.clone()];
                let next = lex.expect_token()?;
                if next.0 == Token::ParenL {
                    let args = args(lex)?;
                    let end = lex.expect_token_is(Token::ParenR)?;
                    Ok((Self::Fn(ident, args), Span::join(&start, &end)))
                } else {
                    lex.prepend(Ok(next));
                    Ok((Self::Param(ident), start))
                }
            }
            (Token::VarX, span) => Ok((Self::VarX, span)),
            (Token::VarY, span) => Ok((Self::VarY, span)),
            (Token::ParenL, start) => {
                let (inner, _) = Self::parse(lex)?;
                let end = lex.expect_token_is(Token::ParenR)?;
                Ok((Self::Paren(Box::new(inner)), Span::join(&start, &end)))
            }
            (Token::Sub, start) if is_start => {
                let (inner, end) = Self::parse(lex)?;
                Ok((Self::Neg(Box::new(inner)), Span::join(&start, &end)))
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

fn args<'src>(lex: &mut Prependable<Lex<'src>>) -> Result<Vec<Expr<'src>>, SynError> {
    todo!()
}
