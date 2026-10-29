use std::str::FromStr;

use crate::{
    sem::{Dep, Id, Table, error::SemError},
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
    pub(super) fn new<T: Table>(
        mut node: Node,
        source: &str,
        table: &mut T,
    ) -> Result<Self, SemError> {
        enum Next {
            Op(Op),
            Node(Node),
        }

        let mut ops = Vec::new();
        let mut rest = Vec::new();
        loop {
            node = match node {
                Node::Primary(p) => {
                    match p {
                        syn::Primary::Num(span) => ops.push(Op::Const(
                            f64::from_str(&source[span]).expect("Num was checked by lex"),
                        )),
                        syn::Primary::Var(span) => {
                            let name = &source[span.clone()];
                            let id = match table.depend(name, Dep::Var) {
                                Ok(id) => id,
                                Err((prev_span, prev_dep)) => {
                                    return Err(SemError::WrongDependenceType {
                                        name: name.to_owned(),
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
                        match rest.pop() {
                            Some(Next::Node(node)) => break node,
                            Some(Next::Op(op)) => ops.push(op),
                            None => return Ok(Expr::Tree(ops)),
                        }
                    }
                }
                Node::Unary(syn::Unary::Neg, i) => {
                    rest.push(Next::Op(Op::Unary(Unary::Neg)));
                    *i
                }
                Node::Binary(b, l, r) => {
                    let binary = match b {
                        syn::Binary::Pow => Binary::Pow,
                        syn::Binary::Mul => Binary::Mul,
                        syn::Binary::Div => Binary::Div,
                        syn::Binary::Add => Binary::Add,
                        syn::Binary::Sub => Binary::Sub,
                        syn::Binary::Eq => todo!(),
                    };
                    rest.push(Next::Op(Op::Binary(binary)));
                    rest.push(Next::Node(*r));
                    *l
                }
                Node::Delimit(d, i) => {
                    'unary: {
                        let unary = match d {
                            syn::Delimit::Paren => break 'unary,
                            syn::Delimit::Floor => Unary::Floor,
                            syn::Delimit::Ceil => Unary::Ceil,
                            syn::Delimit::Abs => Unary::Abs,
                        };
                        rest.push(Next::Op(Op::Unary(unary)));
                    }
                    *i
                }
                Node::Function(span, mut args) => {
                    // TODO: check "fn"
                    let name = &source[span.clone()];
                    let id = match table.depend(name, Dep::Fn) {
                        Ok(id) => id,
                        Err((prev_span, prev_dep)) => {
                            return Err(SemError::WrongDependenceType {
                                name: name.to_owned(),
                                prev_span,
                                prev_dep,
                                span,
                                dep: Dep::Fn,
                            });
                        }
                    };
                    match args.len() {
                        0 => {
                            ops.push(Op::Fn(id, 0));
                            loop {
                                match rest.pop() {
                                    Some(Next::Node(node)) => break node,
                                    Some(Next::Op(op)) => ops.push(op),
                                    None => return Ok(Expr::Tree(ops)),
                                }
                            }
                        }
                        1 => {
                            rest.push(Next::Op(Op::Fn(id, 1)));
                            args.remove(0)
                        }
                        n => {
                            rest.push(Next::Op(Op::Fn(id, n as u8)));
                            let mut args = args.into_iter();
                            let first = args.next().unwrap();
                            for arg in args.rev() {
                                rest.push(Next::Node(arg));
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
