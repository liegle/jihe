use std::iter::Peekable;

use crate::{Lex, SynError, lex::LexItem, syn::expr::Expr};

pub(crate) struct Tree<'src> {
    pub(super) statements: Vec<Statement<'src>>,
}

pub(crate) struct Statement<'src> {
    pub(super) name: &'src str,
    pub(super) class: Class<'src>,
}

pub(crate) enum Class<'src> {
    Param {
        from: Expr<'src>,
        to: Expr<'src>,
    },
    Var {
        expr: Expr<'src>,
    },
    Point {
        x: Expr<'src>,
        y: Expr<'src>,
        thickness: Expr<'src>,
        color: Expr<'src>,
    },
    Curve {
        equation: Expr<'src>,
        size: Expr<'src>,
        color: Expr<'src>,
    },
}

impl<'src> Class<'src> {
    pub(super) fn named(
        class: &'src str,
        lex: &mut Peekable<Lex<'src>>,
    ) -> Result<Self, SynError> {
        todo!()
    }

    pub(super) fn unnamed(
        class: &'src str,
        lex: &mut Peekable<Lex<'src>>,
    ) -> Result<Self, SynError> {
        todo!()
    }
}
