use std::{mem, ops::Range};

use crate::{
    Lex, Spanned,
    lex::{Token, TokenSet},
    syn::{Parse, Prependable, SynError},
};

// TODO: maybe use a continuous data structure like Vec and its indices
// to represent the tree instead of using Box?
#[derive(Clone, Debug)]
pub(crate) enum Node {
    Primary(Primary),
    Unary(Unary, Box<Node>),
    Binary(Binary, Box<Node>, Box<Node>),
    Delimit(Delimit, Box<Node>),
    Function(Range<usize>, Vec<Node>),
}

#[derive(Clone, Debug)]
pub(crate) enum Primary {
    Num(Range<usize>),
    Var(Range<usize>),
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

impl Parse<'_> for Node {
    fn parse(lex: &mut Prependable<Lex>) -> Result<Spanned<Self>, SynError> {
        let first = lex.expect_token()?;
        let start = first.1.start;
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
                    return Ok((node, start..end));
                }
                (state, _) => state,
            };
            end = spanned.1.end;

            curr = match curr {
                State::None => begin_expr(lex, spanned)?,
                State::Complete(node) => append_complete(lex, spanned, &mut stack, node)?,
                State::Operate(operate) => append_operate(lex, spanned, &mut stack, operate)?,
                State::Delimit(delimit) => {
                    stack.push(Frame::Delimit(delimit));
                    begin_expr(lex, spanned)?
                }
                State::Function(function) => match (function, spanned) {
                    (Function::Name(name), (Token::ParenR, _)) => {
                        State::Complete(Node::Function(name, Vec::new()))
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

enum Frame {
    Operate(Operate),
    Delimit(Delimit),
    Function(Function),
}

enum State {
    None,
    Complete(Node),
    Operate(Operate),
    Delimit(Delimit),
    Function(Function),
}

enum Operate {
    Unary(Unary),
    Binary(Binary, Node),
}

enum Function {
    Name(Range<usize>),
    Comma(Range<usize>, Vec<Node>),
}

fn begin_expr(lex: &mut Prependable<Lex>, spanned: Spanned<Token>) -> Result<State, SynError> {
    fn delimit(delimit: Delimit) -> Result<State, SynError> {
        Ok(State::Delimit(delimit))
    }
    handle_token! { spanned =>
        (Token::Num, span) => {
            Ok(State::Complete(Node::Primary(Primary::Num(span))))
        }
        (Token::Ident, span) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                Ok(State::Function(Function::Name(span)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(Node::Primary(Primary::Var(span))))
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

fn append_complete(
    lex: &mut Prependable<Lex>,
    spanned: Spanned<Token>,
    stack: &mut Vec<Frame>,
    node: Node,
) -> Result<State, SynError> {
    fn binary(binary: Binary, node: Node) -> Result<State, SynError> {
        Ok(State::Operate(Operate::Binary(binary, node)))
    }
    fn delimit(
        token: Token,
        stack: &mut Vec<Frame>,
        node: Node,
        span: Range<usize>,
    ) -> Result<State, SynError> {
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
                    Err(SynError::DismatchDelimit { expected, found: token, span: span })
                }
            }
            Some(Frame::Function(..)) => Err(SynError::DismatchDelimit {
                expected: Token::ParenR,
                found: token,
                span: span,
            }),
        }
    }
    handle_token! { spanned =>
        (Token::Pow, _) => { binary(Binary::Pow, node) }
        (Token::Mul, _) => { binary(Binary::Mul, node) }
        (Token::Div, _) => { binary(Binary::Div, node) }
        (Token::Add, _) => { binary(Binary::Add, node) }
        (Token::Sub, _) => { binary(Binary::Sub, node) }
        (Token::Eq, _) => { binary(Binary::Eq, node) }
        (Token::Comma, span) => {
            match stack.pop() {
                None => unreachable!("Expr::parse() should have returned but did not"),
                Some(Frame::Operate(..)) => {
                    unreachable!("Frame::Operate should have been poped but was not")
                }
                Some(Frame::Delimit(..)) => {
                    Err(SynError::CommaAsBinary { byte: span.start })
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
        (Token::ParenR, span) => {
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
                    Err(SynError::DismatchDelimit { expected, found: Token::ParenR, span })
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
        (Token::SlashF, span) => {
            delimit(Token::SlashF, stack, node, span)
        }
        (Token::SlashB, span) => {
            delimit(Token::SlashB, stack, node, span)
        }
        (Token::Bar, span) => {
            delimit(Token::Bar, stack, node, span)
        }
        _ => UnexpectedToken @ lex
    }
}

fn append_operate(
    lex: &mut Prependable<Lex>,
    spanned: Spanned<Token>,
    stack: &mut Vec<Frame>,
    operate: Operate,
) -> Result<State, SynError> {
    fn delimit(
        delimit: Delimit,
        stack: &mut Vec<Frame>,
        operate: Operate,
    ) -> Result<State, SynError> {
        stack.push(Frame::Operate(operate));
        Ok(State::Delimit(delimit))
    }
    handle_token! { spanned =>
        (Token::Num, span) => {
            Ok(State::Complete(merge_operate(
                operate,
                Node::Primary(Primary::Num(span)),
            )))
        }
        (Token::Ident, span) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, _) = &next {
                stack.push(Frame::Operate(operate));
                Ok(State::Function(Function::Name(span)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(merge_operate(
                    operate,
                    Node::Primary(Primary::Var(span)),
                )))
            }
        }
        (Token::Sub, span) => {
            return Err(SynError::UnaryInsideExpr { span })
        }
        (Token::ParenL, _) => { delimit(Delimit::Paren, stack, operate) }
        (Token::SlashF, _) => { delimit(Delimit::Ceil, stack, operate) }
        (Token::SlashB, _) => { delimit(Delimit::Floor, stack, operate) }
        (Token::Bar, _) => { delimit(Delimit::Abs, stack, operate) }
        _ => UnexpectedToken @ lex
    }
}

fn merge_operate(operate: Operate, next: Node) -> Node {
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
            const DUMMY: Node = Node::Primary(Primary::Num(0..0));

            let old = mem::replace(curr, DUMMY);
            *curr = Node::Binary(binary, Box::new(old), Box::new(next));

            node
        }
    }
}

fn rewind_stack(stack: &mut Vec<Frame>, mut curr: State) -> State {
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

    use super::*;

    #[test]
    fn test_primary() {
        let mut lex = Prependable::from_iter(Lex::new("10.,"));
        let node = Node::parse(&mut lex);
        assert_matches!(
            node,
            Ok((Node::Primary(Primary::Num(Range { start: 0, end: 3 })), span))
                if &lex.inner().source()[span.clone()] == "10."
                && matches!(span, Range { start: 0, end: 3 })
        );

        let mut lex = Prependable::from_iter(Lex::new("x,"));
        let node = Node::parse(&mut lex);
        assert_matches!(
            node,
            Ok((Node::Primary(Primary::Var(Range { start: 0, end: 1 })), span))
                if &lex.inner().source()[span.clone()] == "x"
                && matches!(span, Range { start: 0, end: 1 })
        );
    }

    #[test]
    fn test_unary() {
        let mut lex = Prependable::from_iter(Lex::new("-0.2,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Unary(Unary::Neg, inner), span)) = node else {
            panic!("test_unary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "-0.2");
        assert_matches!(span, Range { start: 0, end: 4 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary(Primary::Num(Range { start: 1, end: 4 }))
        );
    }

    #[test]
    fn test_binary() {
        let mut lex = Prependable::from_iter(Lex::new("l+  \nr,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Binary(Binary::Add, l, r), span)) = node else {
            panic!("test_binary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "l+  \nr");
        assert_matches!(span, Range { start: 0, end: 6 });
        assert_matches!(
            l.as_ref(),
            Node::Primary(Primary::Var(Range { start: 0, end: 1 }))
        );
        assert_matches!(
            r.as_ref(),
            Node::Primary(Primary::Var(Range { start: 5, end: 6 }))
        );
    }

    #[test]
    fn test_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("(233),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Paren, inner), span)) = node else {
            panic!("test_delimit failed 1, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "(233)");
        assert_matches!(span, Range { start: 0, end: 5 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary(Primary::Num(Range { start: 1, end: 4 }))
        );

        let mut lex = Prependable::from_iter(Lex::new("\\\\233//,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Floor, inner), span)) = node else {
            panic!("test_delimit failed 2, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "\\\\233//");
        assert_matches!(span, Range { start: 0, end: 7 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary(Primary::Num(Range { start: 2, end: 5 }))
        );

        let mut lex = Prependable::from_iter(Lex::new("//233\\\\,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Delimit(Delimit::Ceil, inner), span)) = node else {
            panic!("test_delimit failed 3, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "//233\\\\");
        assert_matches!(span, Range { start: 0, end: 7 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary(Primary::Num(Range { start: 2, end: 5 }))
        );
    }

    #[test]
    fn test_function() {
        let mut lex = Prependable::from_iter(Lex::new("x(233),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Function(Range { start: 0, end: 1 }, inner), span)) = node else {
            panic!("test_function failed 1, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "x(233)");
        assert_matches!(span, Range { start: 0, end: 6 });
        assert_eq!(inner.len(), 1);
        assert_matches!(
            inner[0],
            Node::Primary(Primary::Num(Range { start: 2, end: 5 }))
        );

        let mut lex = Prependable::from_iter(Lex::new("x(233, y),"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Function(Range { start: 0, end: 1 }, inner), span)) = node else {
            panic!("test_function failed 2, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "x(233, y)");
        assert_matches!(span, Range { start: 0, end: 9 });
        assert_eq!(inner.len(), 2);
        assert_matches!(
            inner[0],
            Node::Primary(Primary::Num(Range { start: 2, end: 5 }))
        );
        assert_matches!(
            inner[1],
            Node::Primary(Primary::Var(Range { start: 7, end: 8 }))
        );
    }

    #[test]
    fn test_all() {
        let mut lex =
            Prependable::from_iter(Lex::new("//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8,"));
        let node = Node::parse(&mut lex);
        let Ok((Node::Binary(Binary::Eq, l, r), span)) = node else {
            panic!("test_all failed 1, left: {:?}", node);
        };
        assert_eq!(
            &lex.inner().source()[span.clone()],
            "//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8"
        );
        assert_matches!(span, Range { start: 0, end: 45 });
        assert_matches!(
            r.as_ref(),
            Node::Primary(Primary::Num(Range { start: 44, end: 45 }))
        );
        let Node::Binary(Binary::Add, ll, rr) = l.as_ref() else {
            panic!("test_all failed 2, left: {:?}", l);
        };
        let Node::Delimit(Delimit::Ceil, i) = ll.as_ref() else {
            panic!("test_all failed 3, left: {:?}", ll);
        };
        let Node::Binary(Binary::Sub, lll, rrr) = i.as_ref() else {
            panic!("test_all failed 4, left: {:?}", i);
        };
        assert_matches!(
            lll.as_ref(),
            Node::Primary(Primary::Var(Range { start: 2, end: 3 }))
        );
        assert_matches!(
            rrr.as_ref(),
            Node::Primary(Primary::Num(Range { start: 6, end: 7 }))
        );
        let Node::Binary(Binary::Pow, llll, rrrr) = rr.as_ref() else {
            panic!("test_all failed 5, left: {:?}", rr);
        };
        assert_matches!(
            rrrr.as_ref(),
            Node::Primary(Primary::Var(Range { start: 40, end: 41 }))
        );
        let Node::Delimit(Delimit::Paren, ii) = llll.as_ref() else {
            panic!("test_all failed 6, left: {:?}", llll);
        };
        let Node::Unary(Unary::Neg, iii) = ii.as_ref() else {
            panic!("test_all failed 7, left: {:?}", ii);
        };
        let Node::Binary(Binary::Mul, lllll, rrrrr) = iii.as_ref() else {
            panic!("test_all failed 8, left: {:?}", iii);
        };
        assert_matches!(
            lllll.as_ref(),
            Node::Primary(Primary::Var(Range { start: 14, end: 15 }))
        );
        let Node::Function(Range { start: 18, end: 22 }, iiii) = rrrrr.as_ref() else {
            panic!("test_all failed 9, left: {:?}", rrrrr);
        };
        assert_eq!(iiii.len(), 3);
        let Node::Binary(Binary::Div, llllll, rrrrrr) = &iiii[0] else {
            panic!("test_all failed 10, left: {:?}", iiii);
        };
        assert_matches!(
            llllll.as_ref(),
            Node::Primary(Primary::Var(Range { start: 23, end: 24 }))
        );
        assert_matches!(
            rrrrrr.as_ref(),
            Node::Primary(Primary::Num(Range { start: 27, end: 28 }))
        );
        assert_matches!(
            iiii[1],
            Node::Primary(Primary::Var(Range { start: 30, end: 31 }))
        );
        assert_matches!(
            iiii[2],
            Node::Primary(Primary::Num(Range { start: 33, end: 35 }))
        );
    }

    #[test]
    fn test_unary_inside_expr() {
        let mut lex = Prependable::from_iter(Lex::new("1 + -1,"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::UnaryInsideExpr { span: Range { start: 4, end: 5 } })
        );
    }

    #[test]
    fn test_dismatch_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("//1)"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::DismatchDelimit {
                expected: Token::SlashB,
                found: Token::ParenR,
                span: Range { start: 3, end: 4 }
            })
        );
    }

    #[test]
    fn test_comma_in_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("(1,)"));
        assert_matches!(
            Node::parse(&mut lex),
            Err(SynError::CommaAsBinary { byte: 2 })
        );
    }

    #[test]
    fn test_function_has_no_arg() {
        let mut lex = Prependable::from_iter(Lex::new("f(),"));
        assert_matches!(
            Node::parse(&mut lex),
            Ok((Node::Function(Range { start: 0, end: 1 }, v), _)) if v.is_empty()
        );
    }
}
