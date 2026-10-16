use std::{mem, ops::Range};

use crate::{
    Cursor, Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Parse, Prependable, SynError},
};

// TODO: maybe use a continuous data structure like Vec and its indices
// to represent the tree instead of using Box?
#[derive(Clone, Debug)]
pub(crate) enum Node<'src> {
    Primary(Primary<'src>),
    Unary(Unary, Box<Node<'src>>),
    Binary(Binary, Box<Node<'src>>, Box<Node<'src>>),
    Delimit(Delimit, Box<Node<'src>>),
    Function(&'src str, Vec<Node<'src>>),
}

#[derive(Clone, Debug)]
pub(crate) enum Primary<'src> {
    Num(&'src str),
    Var(&'src str),
}

#[derive(Clone, Debug)]
#[repr(u8)]
pub(crate) enum Delimit {
    Paren,
    Floor,
    Ceil,
    Abs,
}

macro_rules! enum_unary {
    ($($u:ident=>$p:literal),+) => {
        #[derive(Clone, Copy, Debug)]
        #[repr(u8)]
        pub(crate) enum Unary {
            $($u),+
        }

        const UNARY_PRIORITY: &[u8] = &[
            $($p),+
        ];
    };
}

enum_unary! {
    Neg => 1
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
    Pow   => 3,
    Mul   => 2,
    Div   => 2,
    Add   => 1,
    Sub   => 1,
    Eq    => 0
}

impl<'src> Parse<'src> for Node<'src> {
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
                (State::Complete(node), Token::BraceR | Token::ParenR | Token::Comma)
                    if stack.is_empty() =>
                {
                    lex.prepend(Ok(spanned));
                    let span = Span {
                        byte_span: start.0..end.0,
                        cursor_span: start.1..end.1,
                        ..Default::default()
                    };
                    return Ok((node, span));
                }
                (state, _) => state,
            };
            end = (spanned.1.byte_span.end, spanned.1.cursor_span.end);

            curr = match curr {
                State::None => begin_expr(lex, spanned)?,
                State::Complete(node) => append_complete(lex, spanned, &mut stack, node)?,
                State::Operate(operate) => append_operate(lex, spanned, &mut stack, operate)?,
                State::Delimit(delimit) => {
                    stack.push(Frame::Delimit(delimit));
                    begin_expr(lex, spanned)?
                }
                State::Function(function) => match (function, spanned) {
                    (Function::Name(..), (Token::ParenR, Span { cursor_span, .. })) => {
                        return Err(SynError::FunctionHasNoArg { cursor: cursor_span.start });
                    }
                    // Trailing comma
                    (Function::Comma(name, args), (Token::ParenR, _)) => {
                        State::Complete(Node::Function(name, args))
                    }
                    (function, spanned) => {
                        stack.push(Frame::Function(function));
                        begin_expr(lex, spanned)?
                    }
                },
            };
            curr = rewind_stack(&mut stack, curr);
        }
    }
}

enum Frame<'src> {
    Operate(Operate<'src>),
    Delimit(Delimit),
    Function(Function<'src>),
}

enum State<'src> {
    None,
    Complete(Node<'src>),
    Operate(Operate<'src>),
    Delimit(Delimit),
    Function(Function<'src>),
}

enum Operate<'src> {
    Unary(Unary),
    Binary(Binary, Node<'src>),
}

enum Function<'src> {
    Name(&'src str),
    Comma(&'src str, Vec<Node<'src>>),
}

fn begin_expr<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
) -> Result<State<'src>, SynError> {
    fn delimit<'src>(delimit: Delimit) -> Result<State<'src>, SynError> {
        Ok(State::Delimit(delimit))
    }
    handle_token! { spanned =>
        (Token::Num, Span { byte_span, .. }) => {
            Ok(State::Complete(Node::Primary(Primary::Num(&lex.inner().source()[byte_span]))))
        }
        (Token::Ident, Span { byte_span, .. }) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                Ok(State::Function(Function::Name(&lex.inner().source()[byte_span])))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(Node::Primary(Primary::Var(&lex.inner().source()[byte_span]))))
            }
        }
        (Token::Sub, _) => { Ok(State::Operate(Operate::Unary(Unary::Neg))) }
        (Token::ParenL, _) => { delimit(Delimit::Paren) }
        (Token::SlashF, _) => { delimit(Delimit::Ceil) }
        (Token::SlashB, _) => { delimit(Delimit::Floor) }
        (Token::Bar, _) => { delimit(Delimit::Abs) }
        _ => UnexpectedToken @ lex
    }
}

fn append_complete<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
    stack: &mut Vec<Frame<'src>>,
    node: Node<'src>,
) -> Result<State<'src>, SynError> {
    fn binary<'src>(binary: Binary, node: Node<'src>) -> Result<State<'src>, SynError> {
        Ok(State::Operate(Operate::Binary(binary, node)))
    }
    fn delimit<'src>(
        token: Token,
        stack: &mut Vec<Frame<'src>>,
        node: Node<'src>,
        cursor_span: Range<Cursor>,
    ) -> Result<State<'src>, SynError> {
        match stack.pop() {
            None => unreachable!("Expr::parse() should have returned but did not"),
            Some(Frame::Operate(..)) => {
                unreachable!("Frame::Operate should have been poped but was not")
            }
            Some(Frame::Delimit(delimit)) => {
                let expected = match delimit {
                    Delimit::Paren => Token::ParenR,
                    Delimit::Floor => Token::SlashF,
                    Delimit::Ceil => Token::SlashB,
                    Delimit::Abs => Token::Bar,
                };
                if token == expected {
                    Ok(State::Complete(Node::Delimit(delimit, Box::new(node))))
                } else {
                    Err(SynError::DismatchDelimit { expected, cursor_span })
                }
            }
            Some(Frame::Function(..)) => {
                Err(SynError::DismatchDelimit { expected: Token::ParenR, cursor_span })
            }
        }
    }
    handle_token! { spanned =>
        (Token::Pow, _) => { binary(Binary::Pow, node) }
        (Token::Mul, _) => { binary(Binary::Mul, node) }
        (Token::Div, _) => { binary(Binary::Div, node) }
        (Token::Add, _) => { binary(Binary::Add, node) }
        (Token::Sub, _) => { binary(Binary::Sub, node) }
        (Token::Eq, _) => { binary(Binary::Eq, node) }
        (Token::Comma, Span { cursor_span, .. }) => {
            match stack.pop() {
                None => unreachable!("Expr::parse() should have returned but did not"),
                Some(Frame::Operate(..)) => {
                    unreachable!("Frame::Operate should have been poped but was not")
                }
                Some(Frame::Delimit(..)) => {
                    Err(SynError::CommaInDelimit { cursor: cursor_span.start })
                }
                Some(Frame::Function(function)) => {
                    match function {
                        Function::Name(name) => {
                            Ok(State::Function(Function::Comma(name, vec![node])))
                        }
                        Function::Comma(name, mut args) => {
                            args.push(node);
                            Ok(State::Function(Function::Comma(name, args)))
                        }
                    }
                }
            }
        }
        (Token::ParenR, Span { cursor_span, .. }) => {
            match stack.pop() {
                None => unreachable!("Expr::parse() should have returned but did not"),
                Some(Frame::Operate(..)) => {
                    unreachable!("Frame::Operate should have been poped but was not")
                }
                Some(Frame::Delimit(delimit)) => {
                    let expected = match delimit {
                        Delimit::Paren => {
                            return Ok(State::Complete(Node::Delimit(delimit, Box::new(node))))
                        }
                        Delimit::Floor => Token::SlashF,
                        Delimit::Ceil => Token::SlashB,
                        Delimit::Abs => Token::Bar,
                    };
                    Err(SynError::DismatchDelimit { expected, cursor_span })
                }
                Some(Frame::Function(function)) => {
                    match function {
                        Function::Name(name) => {
                            Ok(State::Complete(Node::Function(name, vec![node])))
                        }
                        Function::Comma(name, mut args) => {
                            args.push(node);
                            Ok(State::Complete(Node::Function(name, args)))
                        }
                    }
                }
            }
        }
        (Token::SlashF, Span { cursor_span, .. }) => {
            delimit(Token::SlashF, stack, node, cursor_span)
        }
        (Token::SlashB, Span { cursor_span, .. }) => {
            delimit(Token::SlashB, stack, node, cursor_span)
        }
        (Token::Bar, Span { cursor_span, .. }) => {
            delimit(Token::Bar, stack, node, cursor_span)
        }
        _ => UnexpectedToken @ lex
    }
}

fn append_operate<'src>(
    lex: &mut Prependable<Lex<'src>>,
    spanned: Spanned<'src, Token>,
    stack: &mut Vec<Frame<'src>>,
    operate: Operate<'src>,
) -> Result<State<'src>, SynError> {
    fn delimit<'src>(
        delimit: Delimit,
        stack: &mut Vec<Frame<'src>>,
        operate: Operate<'src>,
    ) -> Result<State<'src>, SynError> {
        stack.push(Frame::Operate(operate));
        Ok(State::Delimit(delimit))
    }
    handle_token! { spanned =>
        (Token::Num, Span { byte_span, .. }) => {
            Ok(State::Complete(merge_operate(
                operate,
                Node::Primary(Primary::Num(&lex.inner().source()[byte_span])),
            )))
        }
        (Token::Ident, Span { byte_span, .. }) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Operate(operate));
                Ok(State::Function(Function::Name(&lex.inner().source()[byte_span])))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(merge_operate(
                    operate,
                    Node::Primary(Primary::Var(&lex.inner().source()[byte_span])),
                )))
            }
        }
        (Token::Sub, Span { cursor_span, .. }) => {
            return Err(SynError::UnaryInsideExpr { cursor: cursor_span.start })
        }
        (Token::ParenL, _) => { delimit(Delimit::Paren, stack, operate) }
        (Token::SlashF, _) => { delimit(Delimit::Ceil, stack, operate) }
        (Token::SlashB, _) => { delimit(Delimit::Floor, stack, operate) }
        (Token::Bar, _) => { delimit(Delimit::Abs, stack, operate) }
        _ => UnexpectedToken @ lex
    }
}

fn merge_operate<'src>(operate: Operate<'src>, next: Node<'src>) -> Node<'src> {
    match operate {
        Operate::Unary(unary) => Node::Unary(unary, Box::new(next)),
        Operate::Binary(binary, mut node) => {
            let prio = BINARY_PRIORITY[binary as usize];

            let mut curr = &mut node;
            loop {
                match curr {
                    Node::Unary(u, ..) if UNARY_PRIORITY[*u as usize] < prio => {}
                    Node::Binary(b, ..) if BINARY_PRIORITY[*b as usize] < prio => {}
                    _ => break,
                }
                // TODO: it's been checked twice but I don't know how to do it in once😭
                curr = match curr {
                    Node::Unary(.., i) => i.as_mut(),
                    Node::Binary(.., r) => r.as_mut(),
                    _ => unreachable!(),
                };
            }
            const DUMMY: Node<'static> = Node::Primary(Primary::Num("0"));

            let old = mem::replace(curr, DUMMY);
            *curr = Node::Binary(binary, Box::new(old), Box::new(next));

            node
        }
    }
}

fn rewind_stack<'src>(stack: &mut Vec<Frame<'src>>, mut curr: State<'src>) -> State<'src> {
    while let State::Complete(node) = curr {
        match stack.pop() {
            None => return State::Complete(node),
            Some(Frame::Operate(operate)) => curr = State::Complete(merge_operate(operate, node)),
            Some(f @ (Frame::Delimit(..) | Frame::Function(..))) => {
                stack.push(f);
                return State::Complete(node);
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
        let node = Node::parse(&mut lex);
        assert_matches!(
            node,
            Ok((Node::Primary(Primary::Num("10.")), Span { byte_span, cursor_span, .. }))
                if &lex.inner().source()[byte_span.clone()] == "10."
                && matches!(cursor_span.start, Cursor { line: 0, col: 0 })
                && matches!(cursor_span.end, Cursor { line: 0, col: 3 })
        );

        let mut lex = Prependable::from_iter(Lex::new("x,"));
        let node = Node::parse(&mut lex);
        assert_matches!(
            node,
            Ok((Node::Primary(Primary::Var("x")), Span { byte_span, cursor_span, .. }))
                if &lex.inner().source()[byte_span.clone()] == "x"
                && matches!(cursor_span.start, Cursor { line: 0, col: 0 })
                && matches!(cursor_span.end, Cursor { line: 0, col: 1 })
        );
    }

    #[test]
    fn test_unary() {
        let mut lex = Prependable::from_iter(Lex::new("-0.2,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Unary(Unary::Neg, inner), Span { byte_span, cursor_span, .. })) = node else {
            panic!("test_unary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "-0.2");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 4 });
        assert_matches!(inner.as_ref(), Node::Primary(Primary::Num("0.2")));
    }

    #[test]
    fn test_binary() {
        let mut lex = Prependable::from_iter(Lex::new("l+  \nr,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Binary(Binary::Add, l, r), Span { byte_span, cursor_span, .. })) = node
        else {
            panic!("test_binary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "l+  \nr");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 1, col: 1 });
        assert_matches!(l.as_ref(), Node::Primary(Primary::Var("l")));
        assert_matches!(r.as_ref(), Node::Primary(Primary::Var("r")));
    }

    #[test]
    fn test_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("(233),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Paren, inner), Span { byte_span, cursor_span, .. })) = node
        else {
            panic!("test_delimit failed 1, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "(233)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 5 });
        assert_matches!(inner.as_ref(), Node::Primary(Primary::Num("233")));

        let mut lex = Prependable::from_iter(Lex::new("\\\\233//,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Floor, inner), Span { byte_span, cursor_span, .. })) = node
        else {
            panic!("test_delimit failed 2, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "\\\\233//");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 7 });
        assert_matches!(inner.as_ref(), Node::Primary(Primary::Num("233")));

        let mut lex = Prependable::from_iter(Lex::new("//233\\\\,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Ceil, inner), Span { byte_span, cursor_span, .. })) = node
        else {
            panic!("test_delimit failed 3, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "//233\\\\");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 7 });
        assert_matches!(inner.as_ref(), Node::Primary(Primary::Num("233")));
    }

    #[test]
    fn test_function() {
        let mut lex = Prependable::from_iter(Lex::new("x(233),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Function("x", inner), Span { byte_span, cursor_span, .. })) = node else {
            panic!("test_function failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "x(233)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 6 });
        assert_eq!(inner.len(), 1);
        assert_matches!(inner[0], Node::Primary(Primary::Num("233")));

        let mut lex = Prependable::from_iter(Lex::new("x(233, y),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Function("x", inner), Span { byte_span, cursor_span, .. })) = node else {
            panic!("test_function failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[byte_span.clone()], "x(233, y)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 9 });
        assert_eq!(inner.len(), 2);
        assert_matches!(inner[0], Node::Primary(Primary::Num("233")));
        assert_matches!(inner[1], Node::Primary(Primary::Var("y")));
    }

    #[test]
    fn test_all() {
        let mut lex =
            Prependable::from_iter(Lex::new("//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Binary(Binary::Eq, l, r), Span { byte_span, cursor_span, .. })) = node else {
            panic!("test_all failed 1, left: {:?}", node);
        };
        assert_eq!(
            &lex.inner().source()[byte_span.clone()],
            "//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8"
        );
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 45 });
        assert_matches!(r.as_ref(), Node::Primary(Primary::Num("8")));
        let Node::Binary(Binary::Add, ll, rr) = l.as_ref() else {
            panic!("test_all failed 2, left: {:?}", l);
        };
        let Node::Delimit(Delimit::Ceil, i) = ll.as_ref() else {
            panic!("test_all failed 3, left: {:?}", ll);
        };
        let Node::Binary(Binary::Sub, lll, rrr) = i.as_ref() else {
            panic!("test_all failed 4, left: {:?}", i);
        };
        assert_matches!(lll.as_ref(), Node::Primary(Primary::Var("x")));
        assert_matches!(rrr.as_ref(), Node::Primary(Primary::Num("3")));
        let Node::Binary(Binary::Pow, llll, rrrr) = rr.as_ref() else {
            panic!("test_all failed 5, left: {:?}", rr);
        };
        assert_matches!(rrrr.as_ref(), Node::Primary(Primary::Var("x")));
        let Node::Delimit(Delimit::Paren, ii) = llll.as_ref() else {
            panic!("test_all failed 6, left: {:?}", llll);
        };
        let Node::Unary(Unary::Neg, iii) = ii.as_ref() else {
            panic!("test_all failed 7, left: {:?}", ii);
        };
        let Node::Binary(Binary::Mul, lllll, rrrrr) = iii.as_ref() else {
            panic!("test_all failed 8, left: {:?}", iii);
        };
        assert_matches!(lllll.as_ref(), Node::Primary(Primary::Var("y")));
        let Node::Function("func", iiii) = rrrrr.as_ref() else {
            panic!("test_all failed 9, left: {:?}", rrrrr);
        };
        assert_eq!(iiii.len(), 3);
        let Node::Binary(Binary::Div, llllll, rrrrrr) = &iiii[0] else {
            panic!("test_all failed 10, left: {:?}", iiii);
        };
        assert_matches!(llllll.as_ref(), Node::Primary(Primary::Var("x")));
        assert_matches!(rrrrrr.as_ref(), Node::Primary(Primary::Num("2")));
        assert_matches!(iiii[1], Node::Primary(Primary::Var("y")));
        assert_matches!(iiii[2], Node::Primary(Primary::Num("10")));
    }

    #[test]
    fn test_unary_inside_expr() {
        let mut lex = Prependable::from_iter(Lex::new("1 + -1,"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::UnaryInsideExpr { cursor: Cursor { line: 0, col: 4 } })
        );
    }

    #[test]
    fn test_dismatch_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("//1)"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::DismatchDelimit {
                expected: Token::SlashB,
                cursor_span: Range {
                    start: Cursor { line: 0, col: 3 },
                    end: Cursor { line: 0, col: 4 }
                }
            })
        );
    }

    #[test]
    fn test_comma_in_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("(1,)"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::CommaInDelimit { cursor: Cursor { line: 0, col: 2 } })
        );
    }

    #[test]
    fn test_function_has_no_arg() {
        let mut lex = Prependable::from_iter(Lex::new("f()"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::FunctionHasNoArg { cursor: Cursor { line: 0, col: 2 } })
        );
    }
}
