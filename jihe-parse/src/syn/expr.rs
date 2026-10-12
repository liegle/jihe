use crate::{
    Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Prependable, Syn, SynError},
};

#[derive(Clone)]
pub(crate) enum Expr<'src> {
    Primary(Primary<'src>),
    Delimit(Delimit<'src>, Box<Expr<'src>>),
    Operate(Operate<'src>),
}

#[derive(Clone)]
pub(crate) enum Primary<'src> {
    Num(&'src str),
    Var(&'src str),
}

#[derive(Clone)]
pub(crate) enum Delimit<'src> {
    Fn(&'src str),
    Paren,
}

#[derive(Clone)]
pub(crate) enum Operate<'src> {
    Unary(Unary, Box<Expr<'src>>),
    Binary(Binary, Box<Expr<'src>>, Box<Expr<'src>>),
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Unary {
    Neg,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Binary {
    Pow,
    Mul,
    Div,
    Add,
    Sub,
    Eq,
    Comma,
}

// TODO: to proc macro
const BINARY_PRIORITY: &[u8] = &[
    4, // Pow
    3, // Mul
    3, // Div
    2, // Add
    2, // Sub
    1, // Eq
    0, // Comma
];

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
                (State::Complete(expr), Token::BraceR | Token::ParenR | Token::Comma)
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
                State::Complete(expr) => append_complete(spanned, lex, &mut stack, expr)?,
                State::Delimit(delimit) => {
                    stack.push(Frame::Delimit(delimit));
                    begin_expr(spanned, lex)?
                }
                State::Operate(op) => append_operate(spanned, lex, &mut stack, op)?,
            };
            curr = rewind_stack(&mut stack, curr);
        }
    }
}

enum Op<'src> {
    Unary(Unary),
    Binary(Binary, Expr<'src>),
}

enum Frame<'src> {
    Delimit(Delimit<'src>),
    Operate(Op<'src>),
}

enum State<'src> {
    None,
    Complete(Expr<'src>),
    Delimit(Delimit<'src>),
    Operate(Op<'src>),
}

fn begin_expr<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    match token {
        Token::Num => Ok(State::Complete(Expr::Primary(Primary::Num(string)))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                Ok(State::Delimit(Delimit::Fn(string)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(Expr::Primary(Primary::Var(string))))
            }
        }
        Token::ParenL => Ok(State::Delimit(Delimit::Paren)),
        Token::Sub => Ok(State::Operate(Op::Unary(Unary::Neg))),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL, Token::Sub]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn append_complete<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
    stack: &mut Vec<Frame<'src>>,
    expr: Expr<'src>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    let binary = match token {
        Token::ParenR => match stack.pop() {
            Some(Frame::Delimit(d)) => {
                return Ok(State::Complete(Expr::Delimit(d, Box::new(expr))));
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
    Ok(State::Operate(Op::Binary(binary, expr)))
}

fn append_operate<'src>(
    (token, Span { byte_span, cursor_span, .. }): Spanned<'src, Token>,
    lex: &mut Prependable<Lex<'src>>,
    stack: &mut Vec<Frame<'src>>,
    op: Op<'src>,
) -> Result<State<'src>, SynError> {
    let string = &lex.inner().source()[byte_span];
    match token {
        Token::Num => Ok(State::Complete(merge_operate(
            op,
            Expr::Primary(Primary::Num(string)),
        ))),
        Token::Ident => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Operate(op));
                Ok(State::Delimit(Delimit::Fn(string)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(merge_operate(
                    op,
                    Expr::Primary(Primary::Var(string)),
                )))
            }
        }
        Token::ParenL => {
            stack.push(Frame::Operate(op));
            Ok(State::Delimit(Delimit::Paren))
        }
        Token::Sub => Err(SynError::UnaryInsideExpr { cursor: cursor_span.start }),
        _ => Err(SynError::UnexpectedToken {
            expected: TokenSet::with_values([Token::Num, Token::Ident, Token::ParenL]),
            found: string.to_owned(),
            cursor_span,
        }),
    }
}

fn merge_operate<'src>(op: Op<'src>, next: Expr<'src>) -> Expr<'src> {
    match op {
        Op::Unary(unary) => Expr::Operate(Operate::Unary(unary, Box::new(next))),
        Op::Binary(binary, mut expr) => {
            let prio = BINARY_PRIORITY[binary as usize];

            let mut curr = &mut expr;
            while matches!(
                curr,
                Expr::Operate(Operate::Binary(b, _, r)) if BINARY_PRIORITY[*b as usize] < prio
            ) {
                // it's been checked twice but I don't know how to do it in once😭
                if let Expr::Operate(Operate::Binary(.., r)) = curr {
                    curr = r.as_mut();
                }
            }
            *curr = Expr::Operate(Operate::Binary(
                binary,
                Box::new(curr.clone()),
                Box::new(next),
            ));
            expr
        }
    }
}

fn rewind_stack<'src>(stack: &mut Vec<Frame<'src>>, mut curr: State<'src>) -> State<'src> {
    while let State::Complete(expr) = curr {
        match stack.pop() {
            None => return State::Complete(expr),
            Some(Frame::Operate(operate)) => curr = State::Complete(merge_operate(operate, expr)),
            Some(f) => {
                stack.push(f);
                return State::Complete(expr);
            }
        }
    }
    curr
}
