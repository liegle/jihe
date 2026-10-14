use std::mem;

use crate::{
    Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Parse, Prependable, SynError},
};

#[derive(Clone, Debug)]
pub(crate) enum Expr<'src> {
    Primary(Primary<'src>),
    Delimit(Delimit<'src>, Box<Expr<'src>>),
    Unary(Unary, Box<Expr<'src>>),
    Binary(Binary, Box<Expr<'src>>, Box<Expr<'src>>),
}

#[derive(Clone, Debug)]
pub(crate) enum Primary<'src> {
    Num(&'src str),
    Var(&'src str),
}

#[derive(Clone, Debug)]
pub(crate) enum Delimit<'src> {
    Fn(&'src str),
    Paren,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub(crate) enum Unary {
    Neg,
}

macro_rules! enum_binary {
    ($($b:ident=>$p:literal),+) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(u8)]
        pub(crate) enum Binary {
            $($b),+
        }

        const BINARY_PRIORITY: &[u8] = &[
            $($p),+
        ];
    };
}

enum_binary! {
    Pow   => 4,
    Mul   => 3,
    Div   => 3,
    Add   => 2,
    Sub   => 2,
    Eq    => 1,
    Comma => 0
}

impl<'src> Parse<'src> for Expr<'src> {
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
                State::None => begin_expr(lex, spanned)?,
                State::Complete(expr) => append_complete(lex, spanned, &mut stack, expr)?,
                State::Delimit(delimit) => {
                    stack.push(Frame::Delimit(delimit));
                    begin_expr(lex, spanned)?
                }
                State::Operate(op) => append_operate(lex, spanned, &mut stack, op)?,
            };
            curr = rewind_stack(&mut stack, curr);
        }
    }
}

enum Operate<'src> {
    Unary(Unary),
    Binary(Binary, Expr<'src>),
}

enum Frame<'src> {
    Delimit(Delimit<'src>),
    Operate(Operate<'src>),
}

enum State<'src> {
    None,
    Complete(Expr<'src>),
    Delimit(Delimit<'src>),
    Operate(Operate<'src>),
}

fn begin_expr<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
) -> Result<State<'src>, SynError> {
    handle_token! {
        (lex, spanned) => {
            (Token::Num, Span { byte_span, .. }) => {
                Ok(State::Complete(Expr::Primary(Primary::Num(&lex.inner().source()[byte_span]))))
            }
            (Token::Ident, Span { byte_span, .. }) => {
                let next = lex.expect_token()?;
                if let (Token::ParenL, _) = &next {
                    Ok(State::Delimit(Delimit::Fn(&lex.inner().source()[byte_span])))
                } else {
                    lex.prepend(Ok(next));
                    Ok(State::Complete(Expr::Primary(Primary::Var(&lex.inner().source()[byte_span]))))
                }
            }
            (Token::ParenL, _span) => {
                Ok(State::Delimit(Delimit::Paren))
            }
            (Token::Sub, _span) => {
                Ok(State::Operate(Operate::Unary(Unary::Neg)))
            }
        }
    }
}

fn append_complete<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
    stack: &mut Vec<Frame<'src>>,
    expr: Expr<'src>,
) -> Result<State<'src>, SynError> {
    let binary = handle_token! {
        (lex, spanned) => {
            (Token::ParenR, _span) => {
                match stack.pop() {
                    None => unreachable!("Expr::parse() should have returned but did not"),
                    Some(Frame::Delimit(d)) => {
                        return Ok(State::Complete(Expr::Delimit(d, Box::new(expr))));
                    }
                    Some(Frame::Operate(..)) => {
                        unreachable!("Frame::Operate should have been poped but was not")
                    }
                }
            }
            (Token::Pow, _span) => { Binary::Pow }
            (Token::Mul, _span) => { Binary::Mul }
            (Token::Div, _span) => { Binary::Div }
            (Token::Add, _span) => { Binary::Add }
            (Token::Sub, _span) => { Binary::Sub }
            (Token::Eq, _span) => { Binary::Eq }
            (Token::Comma, _span) => { Binary::Comma }
        }
    };
    Ok(State::Operate(Operate::Binary(binary, expr)))
}

fn append_operate<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
    stack: &mut Vec<Frame<'src>>,
    op: Operate<'src>,
) -> Result<State<'src>, SynError> {
    handle_token! {
        (lex, spanned) => {
            (Token::Num, Span { byte_span, .. }) => {
                Ok(State::Complete(merge_operate(
                    op,
                    Expr::Primary(Primary::Num(&lex.inner().source()[byte_span])),
                )))
            }
            (Token::Ident, Span { byte_span, .. }) => {
                let next = lex.expect_token()?;
                if let (Token::ParenL, _) = &next {
                    stack.push(Frame::Operate(op));
                    Ok(State::Delimit(Delimit::Fn(&lex.inner().source()[byte_span])))
                } else {
                    lex.prepend(Ok(next));
                    Ok(State::Complete(merge_operate(
                        op,
                        Expr::Primary(Primary::Var(&lex.inner().source()[byte_span])),
                    )))
                }
            }
            (Token::ParenL, Span { .. }) => {
                stack.push(Frame::Operate(op));
                Ok(State::Delimit(Delimit::Paren))
            }
            (Token::Sub, Span { cursor_span, .. }) => {
                Err(SynError::UnaryInsideExpr { cursor: cursor_span.start })
            }
        }
    }
}

fn merge_operate<'src>(op: Operate<'src>, next: Expr<'src>) -> Expr<'src> {
    match op {
        Operate::Unary(unary) => Expr::Unary(unary, Box::new(next)),
        Operate::Binary(binary, mut expr) => {
            let prio = BINARY_PRIORITY[binary as usize];

            let mut curr = &mut expr;
            while matches!(
                curr,
                Expr::Binary(b, _, r) if BINARY_PRIORITY[*b as usize] < prio
            ) {
                // it's been checked twice but I don't know how to do it in once😭
                if let Expr::Binary(.., r) = curr {
                    curr = r.as_mut();
                }
            }
            const DUMMY: Expr<'static> = Expr::Primary(Primary::Num("0"));

            let old = mem::replace(curr, DUMMY);
            *curr = Expr::Binary(binary, Box::new(old), Box::new(next));

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

#[cfg(test)]
mod test {
    use std::assert_matches;

    use crate::Cursor;

    use super::*;

    #[test]
    fn test_primary() {
        let mut lex = Prependable::from_iter(Lex::new("10.,"));
        let expr = Expr::parse(&mut lex);
        assert_matches!(
            expr,
            Ok((Expr::Primary(Primary::Num("10.")), Span { byte_span, cursor_span, .. }))
                if &lex.inner().source()[byte_span.clone()] == "10."
                && matches!(cursor_span.start, Cursor { line: 0, col: 0 })
                && matches!(cursor_span.end, Cursor { line: 0, col: 3 })
        );

        let mut lex = Prependable::from_iter(Lex::new("x,"));
        let expr = Expr::parse(&mut lex);
        assert_matches!(
            expr,
            Ok((Expr::Primary(Primary::Var("x")), Span { byte_span, cursor_span, .. }))
                if &lex.inner().source()[byte_span.clone()] == "x"
                && matches!(cursor_span.start, Cursor { line: 0, col: 0 })
                && matches!(cursor_span.end, Cursor { line: 0, col: 1 })
        );
    }

    #[test]
    fn test_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("x(233),"));
        let expr = Expr::parse(&mut lex);
        let Ok((Expr::Delimit(Delimit::Fn("x"), inner), Span { byte_span, cursor_span, .. })) =
            expr
        else {
            panic!("test_delimit failed 1, left: {:?}", expr);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "x(233)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 6 });
        assert_matches!(inner.as_ref(), Expr::Primary(Primary::Num("233")));

        let mut lex = Prependable::from_iter(Lex::new("(233),"));
        let expr = Expr::parse(&mut lex);
        let Ok((Expr::Delimit(Delimit::Paren, inner), Span { byte_span, cursor_span, .. })) = expr
        else {
            panic!("test_delimit failed 2, left: {:?}", expr);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "(233)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 5 });
        assert_matches!(inner.as_ref(), Expr::Primary(Primary::Num("233")));
    }

    #[test]
    fn test_unary() {
        let mut lex = Prependable::from_iter(Lex::new("-0.2,"));
        let expr = Expr::parse(&mut lex);
        let Ok((Expr::Unary(Unary::Neg, inner), Span { byte_span, cursor_span, .. })) = expr else {
            panic!("test_unary failed, left: {:?}", expr);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "-0.2");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 4 });
        assert_matches!(inner.as_ref(), Expr::Primary(Primary::Num("0.2")));
    }

    #[test]
    fn test_binary() {
        let mut lex = Prependable::from_iter(Lex::new("l+  \nr,"));
        let expr = Expr::parse(&mut lex);
        let Ok((Expr::Binary(Binary::Add, l, r), Span { byte_span, cursor_span, .. })) = expr
        else {
            panic!("test_binary failed, left: {:?}", expr);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "l+  \nr");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 1, col: 1 });
        assert_matches!(l.as_ref(), Expr::Primary(Primary::Var("l")));
        assert_matches!(r.as_ref(), Expr::Primary(Primary::Var("r")));
    }

    #[test]
    fn test_all() {
        let mut lex =
            Prependable::from_iter(Lex::new("abs(x - 3) + (-y * func(x / 2, y, 10)) ^ x = 8,"));
        let expr = Expr::parse(&mut lex);
        let Ok((Expr::Binary(Binary::Eq, l, r), Span { byte_span, cursor_span, .. })) = expr else {
            panic!("test_all failed 1, left: {:?}", expr);
        };
        assert_eq!(
            &lex.inner().source()[byte_span.clone()],
            "abs(x - 3) + (-y * func(x / 2, y, 10)) ^ x = 8"
        );
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 46 });
        assert_matches!(r.as_ref(), Expr::Primary(Primary::Num("8")));
        let Expr::Binary(Binary::Add, ll, rr) = l.as_ref() else {
            panic!("test_all failed 2, left: {:?}", l);
        };
        let Expr::Delimit(Delimit::Fn("abs"), i) = ll.as_ref() else {
            panic!("test_all failed 3, left: {:?}", ll);
        };
        let Expr::Binary(Binary::Sub, lll, rrr) = i.as_ref() else {
            panic!("test_all failed 4, left: {:?}", i);
        };
        assert_matches!(lll.as_ref(), Expr::Primary(Primary::Var("x")));
        assert_matches!(rrr.as_ref(), Expr::Primary(Primary::Num("3")));
        let Expr::Binary(Binary::Pow, llll, rrrr) = rr.as_ref() else {
            panic!("test_all failed 5, left: {:?}", rr);
        };
        assert_matches!(rrrr.as_ref(), Expr::Primary(Primary::Var("x")));
        let Expr::Delimit(Delimit::Paren, ii) = llll.as_ref() else {
            panic!("test_all failed 6, left: {:?}", llll);
        };
        let Expr::Binary(Binary::Mul, lllll, rrrrr) = ii.as_ref() else {
            panic!("test_all failed 7, left: {:?}", ii);
        };
        let Expr::Unary(Unary::Neg, iii) = lllll.as_ref() else {
            panic!("test_all failed 8, left: {:?}", lllll);
        };
        assert_matches!(iii.as_ref(), Expr::Primary(Primary::Var("y")));
        let Expr::Delimit(Delimit::Fn("func"), iiii) = rrrrr.as_ref() else {
            panic!("test_all failed 9, left: {:?}", rrrrr);
        };
        let Expr::Binary(Binary::Comma, llllll, rrrrrr) = iiii.as_ref() else {
            panic!("test_all failed 10, left: {:?}", iiii);
        };
        assert_matches!(rrrrrr.as_ref(), Expr::Primary(Primary::Num("10")));
        let Expr::Binary(Binary::Comma, lllllll, rrrrrrr) = llllll.as_ref() else {
            panic!("test_all failed 11, left: {:?}", llllll);
        };
        assert_matches!(rrrrrrr.as_ref(), Expr::Primary(Primary::Var("y")));
        let Expr::Binary(Binary::Div, llllllll, rrrrrrrr) = lllllll.as_ref() else {
            panic!("test_all failed 12, left: {:?}", lllllll);
        };
        assert_matches!(llllllll.as_ref(), Expr::Primary(Primary::Var("x")));
        assert_matches!(rrrrrrrr.as_ref(), Expr::Primary(Primary::Num("2")));
    }

    #[test]
    fn test_unary_inside_expr() {
        let mut lex = Prependable::from_iter(Lex::new("1 + -1,"));
        assert_matches!(
            Expr::parse(&mut lex),
            Err(SynError::UnaryInsideExpr { cursor: Cursor { line: 0, col: 4 } })
        );
    }
}
