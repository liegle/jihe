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
    Primary {
        primary: Primary,
    },
    Unary {
        unary: Unary,
        pos: Range<usize>,
        inner: Box<Node>,
    },
    Binary {
        binary: Binary,
        pos: Range<usize>,
        left: Box<Node>,
        right: Box<Node>,
    },
    Delimit {
        delimit: Delimit,
        left: Range<usize>,
        right: Range<usize>,
        inner: Box<Node>,
    },
    Function {
        name: Range<usize>,
        left: usize,
        right: usize,
        args: Vec<Node>,
    },
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
    // TODO: maybe let Eq only appear at top level
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
                State::Delimit(delimit, left) => {
                    stack.push(Frame::Delimit(delimit, left));
                    begin_expr(lex, spanned)?
                }
                State::Function(function) => match (function, spanned) {
                    (Function::Name(name, left), (Token::ParenR, span)) => {
                        State::Complete(Node::Function {
                            name,
                            left,
                            right: span.start,
                            args: Vec::new(),
                        })
                    }
                    // Trailing comma
                    (Function::Comma(name, left, args), (Token::ParenR, span)) => {
                        State::Complete(Node::Function { name, left, right: span.start, args })
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
    Delimit(Delimit, Range<usize>),
    Function(Function),
}

enum State {
    None,
    Complete(Node),
    Operate(Operate),
    Delimit(Delimit, Range<usize>),
    Function(Function),
}

enum Operate {
    Unary(Unary, Range<usize>),
    Binary(Binary, Range<usize>, Node),
}

enum Function {
    Name(Range<usize>, usize),
    Comma(Range<usize>, usize, Vec<Node>),
}

fn begin_expr(lex: &mut Prependable<Lex>, spanned: Spanned<Token>) -> Result<State, SynError> {
    handle_token! { spanned =>
        (Token::Num, span) => {
            Ok(State::Complete(Node::Primary { primary: Primary::Num(span) }))
        }
        (Token::Ident, span) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, left) = next {
                Ok(State::Function(Function::Name(span, left.start)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(Node::Primary { primary: Primary::Var(span) }))
            }
        }
        (Token::Sub, span) => { Ok(State::Operate(Operate::Unary(Unary::Neg, span))) }
        (Token::ParenL, span) => { Ok(State::Delimit(Delimit::Paren, span)) }
        (Token::SlashF, span) => { Ok(State::Delimit(Delimit::Ceil, span)) }
        (Token::SlashB, span) => { Ok(State::Delimit(Delimit::Floor, span)) }
        (Token::Bar, span) => { Ok(State::Delimit(Delimit::Abs, span)) }
        _ => UnexpectedToken @ lex
    }
}

fn append_complete(
    lex: &mut Prependable<Lex>,
    spanned: Spanned<Token>,
    stack: &mut Vec<Frame>,
    node: Node,
) -> Result<State, SynError> {
    fn delimit(
        (token, span): Spanned<Token>,
        stack: &mut Vec<Frame>,
        node: Node,
    ) -> Result<State, SynError> {
        match stack.pop() {
            None => unreachable!("Expr::parse() should have returned but did not"),
            Some(Frame::Operate(..)) => {
                unreachable!("Frame::Operate should have been poped but was not")
            }
            Some(Frame::Delimit(delimit, left)) => {
                let expected = match delimit {
                    Delimit::Paren => Token::ParenR,
                    Delimit::Floor => Token::SlashF,
                    Delimit::Ceil => Token::SlashB,
                    Delimit::Abs => Token::Bar,
                };
                if token == expected {
                    Ok(State::Complete(Node::Delimit {
                        delimit,
                        left,
                        right: span,
                        inner: Box::new(node),
                    }))
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
        (Token::Pow, span) => { Ok(State::Operate(Operate::Binary(Binary::Pow, span, node))) }
        (Token::Mul, span) => { Ok(State::Operate(Operate::Binary(Binary::Mul, span, node))) }
        (Token::Div, span) => { Ok(State::Operate(Operate::Binary(Binary::Div, span, node))) }
        (Token::Add, span) => { Ok(State::Operate(Operate::Binary(Binary::Add, span, node))) }
        (Token::Sub, span) => { Ok(State::Operate(Operate::Binary(Binary::Sub, span, node))) }
        (Token::Eq, span) => { Ok(State::Operate(Operate::Binary(Binary::Eq, span, node))) }
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
                        Function::Name(name, left) => {
                            Ok(State::Function(Function::Comma(name, left, vec![node])))
                        }
                        Function::Comma(name, left, mut args) => {
                            args.push(node);
                            Ok(State::Function(Function::Comma(name, left, args)))
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
                Some(Frame::Delimit(delimit, left)) => {
                    let expected = match delimit {
                        Delimit::Paren => return Ok(State::Complete(Node::Delimit {
                            delimit,
                            left,
                            right: span,
                            inner: Box::new(node)
                        })),
                        Delimit::Floor => Token::SlashF,
                        Delimit::Ceil => Token::SlashB,
                        Delimit::Abs => Token::Bar,
                    };
                    Err(SynError::DismatchDelimit { expected, found: Token::ParenR, span })
                }
                Some(Frame::Function(function)) => {
                    match function {
                        Function::Name(name, left) => {
                            Ok(State::Complete(Node::Function {
                                name,
                                left,
                                right: span.start,
                                args: vec![node]
                            }))
                        }
                        Function::Comma(name, left, mut args) => {
                            args.push(node);
                            Ok(State::Complete(Node::Function {
                                name,
                                left,
                                right: span.start,
                                args
                            }))
                        }
                    }
                }
            }
        }
        (Token::SlashF, span) => { delimit((Token::SlashF, span), stack, node) }
        (Token::SlashB, span) => { delimit((Token::SlashB, span), stack, node) }
        (Token::Bar, span) => { delimit((Token::Bar, span), stack, node) }
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
        left: Range<usize>,
        stack: &mut Vec<Frame>,
        operate: Operate,
    ) -> Result<State, SynError> {
        stack.push(Frame::Operate(operate));
        Ok(State::Delimit(delimit, left))
    }
    handle_token! { spanned =>
        (Token::Num, span) => {
            Ok(State::Complete(merge_operate(
                operate,
                Node::Primary { primary: Primary::Num(span) },
            )))
        }
        (Token::Ident, span) => {
            let next = lex.expect_token()?;
            if let (Token::ParenL, left) = next {
                stack.push(Frame::Operate(operate));
                Ok(State::Function(Function::Name(span, left.start)))
            } else {
                lex.prepend(Ok(next));
                Ok(State::Complete(merge_operate(
                    operate,
                    Node::Primary { primary: Primary::Var(span) },
                )))
            }
        }
        (Token::Sub, span) => {
            return Err(SynError::UnaryInsideExpr { span })
        }
        (Token::ParenL, span) => { delimit(Delimit::Paren, span, stack, operate) }
        (Token::SlashF, span) => { delimit(Delimit::Ceil, span, stack, operate) }
        (Token::SlashB, span) => { delimit(Delimit::Floor, span, stack, operate) }
        (Token::Bar, span) => { delimit(Delimit::Abs, span, stack, operate) }
        _ => UnexpectedToken @ lex
    }
}

fn merge_operate(operate: Operate, next: Node) -> Node {
    match operate {
        Operate::Unary(unary, pos) => Node::Unary { unary, pos, inner: Box::new(next) },
        Operate::Binary(binary, pos, mut node) => {
            let prio = BINARY_PRIORITY[binary as usize];

            let mut curr = &mut node;
            loop {
                match curr {
                    Node::Unary { unary: u, .. } if UNARY_PRIORITY[*u as usize] < prio => {}
                    Node::Binary { binary: b, .. } if BINARY_PRIORITY[*b as usize] < prio => {}
                    _ => break,
                }
                // TODO: it's been checked twice but I don't know how to do it in once😭
                curr = match curr {
                    Node::Unary { inner: i, .. } => i.as_mut(),
                    Node::Binary { right: r, .. } => r.as_mut(),
                    _ => unreachable!(),
                };
            }
            const DUMMY: Node = Node::Primary { primary: Primary::Num(0..0) };

            let old = mem::replace(curr, DUMMY);
            *curr = Node::Binary {
                binary,
                pos,
                left: Box::new(old),
                right: Box::new(next),
            };

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
            Ok((Node::Primary { primary: Primary::Num(Range { start: 0, end: 3 }) }, span))
                if &lex.inner().source()[span.clone()] == "10."
                && matches!(span, Range { start: 0, end: 3 })
        );

        let mut lex = Prependable::from_iter(Lex::new("x,"));
        let node = Node::parse(&mut lex);
        assert_matches!(
            node,
            Ok((Node::Primary { primary: Primary::Var(Range { start: 0, end: 1 }) }, span))
                if &lex.inner().source()[span.clone()] == "x"
                && matches!(span, Range { start: 0, end: 1 })
        );
    }

    #[test]
    fn test_unary() {
        let mut lex = Prependable::from_iter(Lex::new("-0.2,"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Unary {
                unary: Unary::Neg,
                pos: Range { start: 0, end: 1 },
                inner,
            },
            span,
        )) = node
        else {
            panic!("test_unary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "-0.2");
        assert_matches!(span, Range { start: 0, end: 4 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 1, end: 4 })
            }
        );
    }

    #[test]
    fn test_binary() {
        let mut lex = Prependable::from_iter(Lex::new("l+  \nr,"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Binary {
                binary: Binary::Add,
                pos: Range { start: 1, end: 2 },
                left,
                right,
            },
            span,
        )) = node
        else {
            panic!("test_binary failed, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "l+  \nr");
        assert_matches!(span, Range { start: 0, end: 6 });
        assert_matches!(
            left.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 0, end: 1 })
            }
        );
        assert_matches!(
            right.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 5, end: 6 })
            }
        );
    }

    #[test]
    fn test_delimit() {
        let mut lex = Prependable::from_iter(Lex::new("(233),"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Delimit {
                delimit: Delimit::Paren,
                left: Range { start: 0, end: 1 },
                right: Range { start: 4, end: 5 },
                inner,
            },
            span,
        )) = node
        else {
            panic!("test_delimit failed 1, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "(233)");
        assert_matches!(span, Range { start: 0, end: 5 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 1, end: 4 })
            }
        );

        let mut lex = Prependable::from_iter(Lex::new("\\\\233//,"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Delimit {
                delimit: Delimit::Floor,
                left: Range { start: 0, end: 2 },
                right: Range { start: 5, end: 7 },
                inner,
            },
            span,
        )) = node
        else {
            panic!("test_delimit failed 2, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "\\\\233//");
        assert_matches!(span, Range { start: 0, end: 7 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 2, end: 5 })
            }
        );

        let mut lex = Prependable::from_iter(Lex::new("//233\\\\,"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Delimit {
                delimit: Delimit::Ceil,
                left: Range { start: 0, end: 2 },
                right: Range { start: 5, end: 7 },
                inner,
            },
            span,
        )) = node
        else {
            panic!("test_delimit failed 3, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "//233\\\\");
        assert_matches!(span, Range { start: 0, end: 7 });
        assert_matches!(
            inner.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 2, end: 5 })
            }
        );
    }

    #[test]
    fn test_function() {
        let mut lex = Prependable::from_iter(Lex::new("x(233),"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Function {
                name: Range { start: 0, end: 1 },
                left: 1,
                right: 5,
                args,
            },
            span,
        )) = node
        else {
            panic!("test_function failed 1, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "x(233)");
        assert_matches!(span, Range { start: 0, end: 6 });
        assert_eq!(args.len(), 1);
        assert_matches!(
            args[0],
            Node::Primary {
                primary: Primary::Num(Range { start: 2, end: 5 })
            }
        );

        let mut lex = Prependable::from_iter(Lex::new("x(233, y),"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Function {
                name: Range { start: 0, end: 1 },
                left: 1,
                right: 8,
                args,
            },
            span,
        )) = node
        else {
            panic!("test_function failed 2, left: {:?}", node);
        };
        assert_eq!(&lex.inner().source()[span.clone()], "x(233, y)");
        assert_matches!(span, Range { start: 0, end: 9 });
        assert_eq!(args.len(), 2);
        assert_matches!(
            args[0],
            Node::Primary {
                primary: Primary::Num(Range { start: 2, end: 5 })
            }
        );
        assert_matches!(
            args[1],
            Node::Primary {
                primary: Primary::Var(Range { start: 7, end: 8 })
            }
        );
    }

    #[test]
    fn test_all() {
        let mut lex =
            Prependable::from_iter(Lex::new("//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8,"));
        let node = Node::parse(&mut lex);
        let Ok((
            Node::Binary {
                binary: Binary::Eq,
                pos: Range { start: 42, end: 43 },
                left: l,
                right: r,
            },
            span,
        )) = node
        else {
            panic!("test_all failed 1, left: {:?}", node);
        };
        assert_eq!(
            &lex.inner().source()[span.clone()],
            "//x - 3\\\\ + (-y * func(x / 2, y, 10)) ^ x = 8"
        );
        assert_matches!(span, Range { start: 0, end: 45 });
        assert_matches!(
            r.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 44, end: 45 })
            }
        );
        let Node::Binary {
            binary: Binary::Add,
            pos: Range { start: 10, end: 11 },
            left: ll,
            right: rr,
        } = l.as_ref()
        else {
            panic!("test_all failed 2, left: {:?}", l);
        };
        let Node::Delimit {
            delimit: Delimit::Ceil,
            left: Range { start: 0, end: 2 },
            right: Range { start: 7, end: 9 },
            inner: i,
        } = ll.as_ref()
        else {
            panic!("test_all failed 3, left: {:?}", ll);
        };
        let Node::Binary {
            binary: Binary::Sub,
            pos: Range { start: 4, end: 5 },
            left: lll,
            right: rrr,
        } = i.as_ref()
        else {
            panic!("test_all failed 4, left: {:?}", i);
        };
        assert_matches!(
            lll.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 2, end: 3 })
            }
        );
        assert_matches!(
            rrr.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 6, end: 7 })
            }
        );
        let Node::Binary {
            binary: Binary::Pow,
            pos: Range { start: 38, end: 39 },
            left: llll,
            right: rrrr,
        } = rr.as_ref()
        else {
            panic!("test_all failed 5, left: {:?}", rr);
        };
        assert_matches!(
            rrrr.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 40, end: 41 })
            }
        );
        let Node::Delimit {
            delimit: Delimit::Paren,
            left: Range { start: 12, end: 13 },
            right: Range { start: 36, end: 37 },
            inner: ii,
        } = llll.as_ref()
        else {
            panic!("test_all failed 6, left: {:?}", llll);
        };
        let Node::Unary {
            unary: Unary::Neg,
            pos: Range { start: 13, end: 14 },
            inner: iii,
        } = ii.as_ref()
        else {
            panic!("test_all failed 7, left: {:?}", ii);
        };
        let Node::Binary {
            binary: Binary::Mul,
            pos: Range { start: 16, end: 17 },
            left: lllll,
            right: rrrrr,
        } = iii.as_ref()
        else {
            panic!("test_all failed 8, left: {:?}", iii);
        };
        assert_matches!(
            lllll.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 14, end: 15 })
            }
        );
        let Node::Function {
            name: Range { start: 18, end: 22 },
            left: 22,
            right: 35,
            args: iiii,
        } = rrrrr.as_ref()
        else {
            panic!("test_all failed 9, left: {:?}", rrrrr);
        };
        assert_eq!(iiii.len(), 3);
        let Node::Binary {
            binary: Binary::Div,
            pos: Range { start: 25, end: 26 },
            left: llllll,
            right: rrrrrr,
        } = &iiii[0]
        else {
            panic!("test_all failed 10, left: {:?}", iiii);
        };
        assert_matches!(
            llllll.as_ref(),
            Node::Primary {
                primary: Primary::Var(Range { start: 23, end: 24 })
            }
        );
        assert_matches!(
            rrrrrr.as_ref(),
            Node::Primary {
                primary: Primary::Num(Range { start: 27, end: 28 })
            }
        );
        assert_matches!(
            iiii[1],
            Node::Primary {
                primary: Primary::Var(Range { start: 30, end: 31 })
            }
        );
        assert_matches!(
            iiii[2],
            Node::Primary {
                primary: Primary::Num(Range { start: 33, end: 35 })
            }
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
            Ok((
                Node::Function {
                    name: Range { start: 0, end: 1 },
                    left: 1,
                    right: 2,
                    args
                },
                _
            )) if args.is_empty()
        );
    }
}
