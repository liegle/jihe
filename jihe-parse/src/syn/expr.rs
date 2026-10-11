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
        let first = lex.expect_token()?;
        let start = (first.1.byte_span.start, first.1.cursor_span.start);
        lex.prepend(Ok(first));
        let mut end = start;

        let mut stack = Vec::new();
        let mut curr = State::None;
        loop {
            let spanned = lex.expect_token()?;
            curr = match (curr, spanned.0) {
                (State::Ok(expr), Token::BraceR | Token::ParenR | Token::Comma)
                    if stack.is_empty() =>
                {
                    lex.prepend(Ok(spanned));
                    let span = Span {
                        byte_span: start.0..end.0,
                        cursor_span: start.1..end.1,
                        ..Default::default()
                    };
                    return Ok((expr, span));
                }
                (state, _) => state,
            };
            end = (spanned.1.byte_span.end, spanned.1.cursor_span.end);

            curr = match curr {
                State::None => begin_expr(spanned, lex)?,
                State::Ok(expr) => append_ok(spanned, lex, &mut stack, expr)?,
                State::Unary(unary) => append_unary(spanned, lex, &mut stack, unary)?,
                State::Binary(binary, expr) => {
                    append_binary(spanned, lex, &mut stack, binary, expr)?
                }
            };
            rewind_stack(&mut curr, &mut stack);
        }
    }
}

enum Frame<'src> {
    Unary(Unary<'src>),
    Binary(Binary, Expr<'src>),
}

enum State<'src> {
    None,
    Ok(Expr<'src>),
    Unary(Unary<'src>),
    Binary(Binary, Expr<'src>),
}

fn begin_expr<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    match token {
        Token::Num => Ok(State::Ok(Expr::Primary(Primary::Num(string)))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                Ok(State::Unary(Unary::Fn(string)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Ok(Expr::Primary(Primary::Var(string))))
            }
        }
        Token::ParenL => Ok(State::Unary(Unary::Paren)),
        Token::Sub => Ok(State::Unary(Unary::Neg)),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL, Token::Sub]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn append_ok<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
    stack: &mut Vec<Frame<'src>>,
    expr: Expr<'src>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    let binary = match token {
        Token::ParenR => match stack.pop().expect("End of expr should have been checked") {
            Frame::Unary(u @ (Unary::Fn(..) | Unary::Paren)) => {
                return Ok(State::Ok(Expr::Unary(u, Box::new(expr))));
            }
            _ => {
                return Err(SynError::DismatchParenR { cursor: cursor_span.start });
            }
        },
        Token::Pow => Binary::Pow,
        Token::Mul => Binary::Mul,
        Token::Div => Binary::Div,
        Token::Add => Binary::Add,
        Token::Sub => Binary::Sub,
        Token::Eq => Binary::Eq,
        Token::Comma => Binary::Comma,
        _ => {
            return Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([
                    Token::Pow,
                    Token::Mul,
                    Token::Div,
                    Token::Add,
                    Token::Sub,
                    Token::Eq,
                    Token::Comma,
                ]),
                found: string.to_owned(),
                cursor_span,
            });
        }
    };
    Ok(State::Binary(binary, expr))
}

fn append_unary<'src>(
    spanned: Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
    stack: &mut Vec<Frame<'src>>,
    unary: Unary<'src>,
) -> Result<State<'src>, SynError> {
    if let f @ (Unary::Fn(..) | Unary::Paren) = unary {
        stack.push(Frame::Unary(f));
        return begin_expr(spanned, lex);
    }
    let (token, Span { byte_span, cursor_span, .. }) = spanned;
    let string = &lex.inner().source()[byte_span];
    match token {
        Token::Num => Ok(State::Ok(Expr::Unary(
            Unary::Neg,
            Box::new(Expr::Primary(Primary::Num(string))),
        ))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Unary(Unary::Neg));
                Ok(State::Unary(Unary::Fn(string)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Ok(Expr::Unary(
                    Unary::Neg,
                    Box::new(Expr::Primary(Primary::Var(string))),
                )))
            }
        }
        Token::ParenL => {
            stack.push(Frame::Unary(Unary::Neg));
            Ok(State::Unary(Unary::Paren))
        }
        Token::Sub => Err(SynError::NegInsideExpr { cursor: cursor_span.start }),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn append_binary<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
    stack: &mut Vec<Frame<'src>>,
    binary: Binary,
    expr: Expr<'src>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    match token {
        Token::Num => Ok(State::Ok(merge_binary(
            binary,
            expr,
            Expr::Primary(Primary::Num(string)),
        ))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Binary(binary, expr));
                Ok(State::Unary(Unary::Fn(string)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Ok(merge_binary(
                    binary,
                    expr,
                    Expr::Primary(Primary::Var(string)),
                )))
            }
        }
        Token::ParenL => {
            stack.push(Frame::Binary(binary, expr));
            Ok(State::Unary(Unary::Paren))
        }
        Token::Sub => Err(SynError::NegInsideExpr { cursor: cursor_span.start }),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn merge_binary<'src>(binary: Binary, l: Expr<'src>, r: Expr<'src>) -> Expr<'src> {
    todo!()
}

fn rewind_stack<'src>(curr: &mut State<'src>, stack: &mut Vec<Frame<'src>>) {
    todo!()
}
