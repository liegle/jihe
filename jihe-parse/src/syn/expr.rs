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

        let mut stack = Vec::<Frame<'src>>::new();
        let mut curr = State::None;
        loop {
            let (token, span) = lex.expect_token()?;
            curr = match (curr, token) {
                (State::Ok(expr), Token::BraceR | Token::ParenR | Token::Comma)
                    if stack.is_empty() =>
                {
                    let span = Span {
                        byte_span: start.0..span.byte_span.end,
                        cursor_span: start.1..span.cursor_span.end,
                        ..Default::default()
                    };
                    return Ok((expr, span));
                }
                (state, _) => state,
            };

            let Span { byte_span, cursor_span, .. } = span;
            let string = &lex.inner().source()[byte_span];
            match curr {
                State::None => curr = from_first_token(token, string, cursor_span, lex)?,
                State::Ok(expr) => {
                    curr = from_after_expr(expr, token, string, cursor_span, &mut stack)?
                }
                State::Frame(f @ Frame::Unary(Unary::Fn(..) | Unary::Paren)) => {
                    stack.push(f);
                    curr = from_first_token(token, string, cursor_span, lex)?;
                }
                State::Frame(Frame::Unary(Unary::Neg)) => {
                    curr = from_after_neg(token, string, cursor_span, &mut stack, lex)?
                }
                State::Frame(Frame::Binary(binary, expr)) => match token {
                    Token::Num => {
                        curr = State::Ok(Expr::Binary(
                            binary,
                            Box::new(expr),
                            Box::new(Expr::Primary(Primary::Num(string))),
                        ));
                    }
                    Token::Ident => {
                        let next = lex.expect_token()?;
                        if let (Token::ParenL, _) = &next {
                            stack.push(Frame::Binary(binary, expr));
                            curr = State::Frame(Frame::Unary(Unary::Fn(string)));
                        } else {
                            lex.prepend(Ok(next));
                            curr = State::Ok(Expr::Binary(
                                binary,
                                Box::new(expr),
                                Box::new(Expr::Primary(Primary::Var(string))),
                            ));
                        }
                    }
                    Token::ParenL => {
                        stack.push(Frame::Binary(binary, expr));
                        curr = State::Frame(Frame::Unary(Unary::Paren));
                    }
                    Token::Sub => {
                        return Err(SynError::NegInsideExpr { cursor: cursor_span.start });
                    }
                    _ => {
                        return Err(SynError::UnexpectedToken {
                            expected: TokenSet::with_values([
                                Token::Num,
                                Token::Ident,
                                Token::ParenL,
                            ]),
                            found: string.to_owned(),
                            cursor_span,
                        });
                    }
                },
            }
            // TODO: rewind stack
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
    Frame(Frame<'src>),
}

fn from_first_token<'src>(
    token: Token,
    string: &'src str,
    cursor_span: Range<Cursor>,
    lex: &mut Prependable<Lex<'src>>,
) -> Result<State<'src>, SynError> {
    match token {
        Token::Num => Ok(State::Ok(Expr::Primary(Primary::Num(string)))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                Ok(State::Frame(Frame::Unary(Unary::Fn(string))))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Ok(Expr::Primary(Primary::Var(string))))
            }
        }
        Token::ParenL => Ok(State::Frame(Frame::Unary(Unary::Paren))),
        Token::Sub => Ok(State::Frame(Frame::Unary(Unary::Neg))),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL, Token::Sub]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn from_after_expr<'src>(
    expr: Expr<'src>,
    token: Token,
    string: &'src str,
    cursor_span: Range<Cursor>,
    stack: &mut Vec<Frame<'src>>,
) -> Result<State<'src>, SynError> {
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
    Ok(State::Frame(Frame::Binary(binary, expr)))
}

fn from_after_neg<'src>(
    token: Token,
    string: &'src str,
    cursor_span: Range<Cursor>,
    stack: &mut Vec<Frame<'src>>,
    lex: &mut Prependable<Lex<'src>>,
) -> Result<State<'src>, SynError> {
    match token {
        Token::Num => Ok(State::Ok(Expr::Unary(
            Unary::Neg,
            Box::new(Expr::Primary(Primary::Num(string))),
        ))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Unary(Unary::Neg));
                Ok(State::Frame(Frame::Unary(Unary::Fn(string))))
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
            Ok(State::Frame(Frame::Unary(Unary::Paren)))
        }
        Token::Sub => Err(SynError::NegInsideExpr { cursor: cursor_span.start }),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn from_after_op<'src>(
    token: Token,
    string: &'src str,
    cursor_span: Range<Cursor>,
    stack: &mut Vec<Frame<'src>>,
    lex: &mut Prependable<Lex<'src>>,
) -> Result<State<'src>, SynError> {
    todo!()
}
