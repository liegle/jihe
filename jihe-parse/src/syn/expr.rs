use std::iter::Peekable;

use crate::{
    Lex,
    syn::{Syn, SynError},
};

pub(crate) enum Expr<'src> {
    Num {
        integer: u32,
        decimal: u32,
    },
    Param(&'src str),
    VarX,
    VarY,
    Fn(&'src str, Vec<Expr<'src>>),
    Paren(Box<Expr<'src>>),
    Neg(Box<Expr<'src>>),
    Pow(Box<Expr<'src>>, Box<Expr<'src>>),
    Mul(Box<Expr<'src>>, Box<Expr<'src>>),
    Div(Box<Expr<'src>>, Box<Expr<'src>>),
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
    Sub(Box<Expr<'src>>, Box<Expr<'src>>),
    Eq(Box<Expr<'src>>, Box<Expr<'src>>),
}

impl<'src> Syn<'src> for Expr<'src> {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
        todo!()
    }
}
