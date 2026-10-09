use std::ops::Range;

use crate::{
    Cursor, Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Prependable, Syn, SynError},
};

#[derive(Clone)]
pub(crate) enum Expr<'src> {
    Primary(Primary<'src>),
    Unary(Unary<'src>, Box<Expr<'src>>),
    Binary(Binary, Box<Expr<'src>>, Box<Expr<'src>>),
}

#[derive(Clone)]
pub(crate) enum Primary<'src> {
    Num(&'src str),
    Var(&'src str),
}

#[derive(Clone)]
pub(crate) enum Unary<'src> {
    Fn(&'src str),
    Paren,
    Neg,
}

#[derive(Clone)]
pub(crate) enum Binary {
    Pow,
    Mul,
    Div,
    Add,
    Sub,
    Eq,
    Comma,
}

impl<'src> Syn<'src> for Expr<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        let mut start = None;
        let mut stack = vec![State::None];
        while let Some(state) = stack.pop() {
            let (token, span) = lex.expect_token()?;
            if let None = start {
                start = Some((span.byte_span.start, span.cursor_span.start));
            }
            match (state, token, &span) {
                (State::Ok(expr), Token::BraceR | Token::ParenR | Token::Comma, _) => {
                    let start = start.unwrap();
                    let span = Span {
                        byte_span: start.0..span.byte_span.end,
                        cursor_span: start.1..span.cursor_span.end,
                        ..Default::default()
                    };
                    return Ok((expr, span));
                }
                _ => todo!(),
            }
        }
        Err(SynError::UnexpectedEof)
    }
}

enum State<'src> {
    None,
    Ok(Expr<'src>),
    Unary(Unary<'src>),
    Binary(Expr<'src>, Binary),
}
