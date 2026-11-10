use std::range::Range;

use crate::{
    sem::{Dep, Id, error::SemError},
    syn::{self, Node},
};

// TODO: maybe name it Context or Dependents
pub struct Todo {}

impl Todo {
    // TODO: return Option
    fn variable(&self, id: Id) -> f64 {
        todo!()
    }

    // TODO: return Option / Fn return Result
    fn function<'a>(&'a self, id: Id) -> &'a dyn Fn(&[f64]) -> f64 {
        todo!()
    }
}

pub enum Expr {
    Literal(f64),
    // TODO: JIT?
    Tree(Vec<Op>),
}

#[derive(Clone)]
pub enum Op {
    Const(f64),
    Var(Id),
    Fn(Id, u8),
    Unary(Unary),
    Binary(Binary),
}

#[derive(Clone)]
pub enum Unary {
    Floor,
    Ceil,
    Abs,
    Neg,
}

#[derive(Clone)]
pub enum Binary {
    Pow,
    Mul,
    Div,
    Add,
    Sub,
}

impl Expr {
    pub(super) fn new(
        mut node: Node,
        mut depend: impl FnMut(Range<usize>, Dep) -> Result<Id, (Range<usize>, Dep)>,
    ) -> Result<Self, SemError> {
        enum Frame {
            Op(Op),
            Node(Node),
        }

        let mut ops = Vec::new();
        let mut stack = Vec::new();
        loop {
            node = match node {
                Node::Primary { primary } => {
                    match primary {
                        syn::Primary::Num(span, val) => ops.push(Op::Const(val)),
                        syn::Primary::Var(span) => {
                            let id = match depend(span, Dep::Var) {
                                Ok(id) => id,
                                Err((prev_span, prev_dep)) => {
                                    return Err(SemError::WrongDependenceType {
                                        prev_span,
                                        prev_dep,
                                        span,
                                        dep: Dep::Var,
                                    });
                                }
                            };
                            ops.push(Op::Var(id))
                        }
                    };
                    loop {
                        match stack.pop() {
                            Some(Frame::Node(node)) => break node,
                            Some(Frame::Op(op)) => ops.push(op),
                            None => return Ok(Expr::Tree(ops)),
                        }
                    }
                }
                Node::Unary { unary: syn::Unary::Neg, pos, inner } => {
                    stack.push(Frame::Op(Op::Unary(Unary::Neg)));
                    *inner
                }
                Node::Binary { binary, pos, left, right } => {
                    let binary = match binary {
                        syn::Binary::Pow => Binary::Pow,
                        syn::Binary::Mul => Binary::Mul,
                        syn::Binary::Div => Binary::Div,
                        syn::Binary::Add => Binary::Add,
                        syn::Binary::Sub => Binary::Sub,
                        syn::Binary::Eq => {
                            return Err(SemError::EqInSingleExpr { byte: pos.start });
                        }
                    };
                    stack.push(Frame::Op(Op::Binary(binary)));
                    stack.push(Frame::Node(*right));
                    *left
                }
                Node::Delimit { delimit, left, right, inner } => {
                    'unary: {
                        let unary = match delimit {
                            syn::Delimit::Paren => break 'unary,
                            syn::Delimit::Floor => Unary::Floor,
                            syn::Delimit::Ceil => Unary::Ceil,
                            syn::Delimit::Abs => Unary::Abs,
                        };
                        stack.push(Frame::Op(Op::Unary(unary)));
                    }
                    *inner
                }
                Node::Function { name, left, right, mut args } => {
                    // TODO: check "fn"
                    let id = match depend(name, Dep::Fn) {
                        Ok(id) => id,
                        Err((prev_span, prev_dep)) => {
                            return Err(SemError::WrongDependenceType {
                                prev_span,
                                prev_dep,
                                span: name,
                                dep: Dep::Fn,
                            });
                        }
                    };
                    match args.len() {
                        0 => {
                            ops.push(Op::Fn(id, 0));
                            loop {
                                match stack.pop() {
                                    Some(Frame::Node(node)) => break node,
                                    Some(Frame::Op(op)) => ops.push(op),
                                    None => return Ok(Expr::Tree(ops)),
                                }
                            }
                        }
                        1 => {
                            stack.push(Frame::Op(Op::Fn(id, 1)));
                            args.remove(0)
                        }
                        n => {
                            stack.push(Frame::Op(Op::Fn(id, n as u8)));
                            let mut args = args.into_iter();
                            let first = args.next().unwrap();
                            for arg in args.rev() {
                                stack.push(Frame::Node(arg));
                            }
                            first
                        }
                    }
                }
            }
        }
    }

    // TODO: return Result
    pub fn eval(&self, todo: &Todo) -> f64 {
        let ops = match self {
            Self::Literal(value) => return *value,
            Self::Tree(ops) => ops,
        };
        let mut stack = Vec::new();
        for op in ops {
            match op.clone() {
                Op::Const(num) => stack.push(num),
                Op::Var(id) => stack.push(todo.variable(id)),
                Op::Fn(id, count) => {
                    let mut args = Vec::with_capacity(count as usize);
                    for _ in 0..count {
                        args.push(stack.pop().expect("todo"));
                    }
                    args.reverse();
                    stack.push(todo.function(id)(&args));
                }
                Op::Unary(u) => {
                    let top = stack.last_mut().expect("todo");
                    *top = match u {
                        Unary::Floor => (*top).floor(),
                        Unary::Ceil => (*top).ceil(),
                        Unary::Abs => (*top).abs(),
                        Unary::Neg => -*top,
                    };
                }
                Op::Binary(b) => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = match b {
                        Binary::Pow => (*top).powf(second),
                        Binary::Mul => *top * second,
                        Binary::Div => *top / second,
                        Binary::Add => *top + second,
                        Binary::Sub => *top - second,
                    };
                }
            }
        }
        stack.pop().expect("todo")
    }

    pub fn translate(&self) -> String {
        todo!()
    }
}
