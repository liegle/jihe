use std::iter::Peekable;

use crate::{
    Lex,
    syn::{Syn, SynError},
};

pub(crate) enum Expr<'src> {
    Number {
        negative: bool,
        integer: u32,
        decimal: u32,
    },
    Parameter(&'src str),
    VariableX,
    VariableY,
    FunctionCall(&'src str, Vec<Expr<'src>>),
    Parenthese(Box<Expr<'src>>),
    Power(Box<Expr<'src>>, Box<Expr<'src>>),
    Multiply(Box<Expr<'src>>, Box<Expr<'src>>),
    Divide(Box<Expr<'src>>, Box<Expr<'src>>),
    Plus(Box<Expr<'src>>, Box<Expr<'src>>),
    Minus(Box<Expr<'src>>, Box<Expr<'src>>),
    Equal(Box<Expr<'src>>, Box<Expr<'src>>),
}

impl Expr<'static> {
    pub(super) const fn number(value: u32) -> Self {
        Expr::Number {
            negative: false,
            integer: value,
            decimal: 0,
        }
    }
}

impl<'src> Syn<'src> for Expr<'src> {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
        todo!()
    }
}
