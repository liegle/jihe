use crate::{sem::Id, syn::Node};

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
    Tree(Tree),
}

// TODO: JIT?
struct Tree {
    ops: Vec<Op>,
}

impl Expr {
    pub(super) fn new(node: Node) -> Self {
        todo!()
    }

    // TODO: return Result
    pub fn eval(&self, todo: &Todo) -> f64 {
        let ops = match self {
            Self::Literal(value) => return *value,
            Self::Tree(inner) => &inner.ops,
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
                Op::Floor => {
                    let top = stack.last_mut().expect("todo");
                    *top = (*top).floor();
                }
                Op::Ceil => {
                    let top = stack.last_mut().expect("todo");
                    *top = (*top).ceil();
                }
                Op::Abs => {
                    let top = stack.last_mut().expect("todo");
                    *top = (*top).abs();
                }
                Op::Neg => {
                    let top = stack.last_mut().expect("todo");
                    *top = -*top;
                }
                Op::Pow => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = (*top).powf(second);
                }
                Op::Mul => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = *top * second;
                }
                Op::Div => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = *top / second;
                }
                Op::Add => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = *top + second;
                }
                Op::Sub => {
                    let second = stack.pop().expect("todo");
                    let top = stack.last_mut().expect("todo");
                    *top = *top - second;
                }
            }
        }
        stack.pop().expect("todo")
    }
}

#[derive(Clone)]
pub enum Op {
    Const(f64),
    Var(Id),
    Fn(Id, u8),
    Floor,
    Ceil,
    Abs,
    Neg,
    Pow,
    Mul,
    Div,
    Add,
    Sub,
}
