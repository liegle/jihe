use std::iter::Peekable;

use crate::{
    Lex,
    lex::{Kind, KindSet, Token},
    syn::tree::{Class, Statement},
};
pub(super) use {error::SynError, tree::Tree};

mod error;
mod expr;
mod tree;

pub(super) fn syn<'src>(lex: Lex<'src>) -> Result<Tree<'src>, SynError> {
    let mut lex = lex.peekable();
    Tree::parse(&mut lex)
}

trait Syn<'src>: Sized {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError>;
}

trait ExpectToken<'src>: 'src {
    fn next_token(&mut self) -> Result<Token<'src>, SynError>;
    fn next_kind(&mut self, kind: Kind) -> Result<Token<'src>, SynError>;
    fn next_if_kind(&mut self, kind: Kind);
}

impl<'src> ExpectToken<'src> for Peekable<Lex<'src>> {
    fn next_token(&mut self) -> Result<Token<'src>, SynError> {
        match self.next() {
            Some(token) => token.map_err(SynError::LexError),
            None => Err(SynError::UnexpectedEof),
        }
    }

    fn next_kind(&mut self, kind: Kind) -> Result<Token<'src>, SynError> {
        let token = self.next_token()?;
        if token.kind == kind {
            Ok(token)
        } else {
            Err(SynError::UnexpectedToken {
                expected: KindSet::with_values([kind]),
                found: token.string.to_owned(),
                range: token.range,
            })
        }
    }

    fn next_if_kind(&mut self, kind: Kind) {
        self.next_if(|next| matches!(next, Ok(Token { kind: k, .. }) if kind == *k));
    }
}

impl<'src> Syn<'src> for Tree<'src> {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
        let mut statements = Vec::new();
        while lex.peek().is_some() {
            statements.push(Statement::parse(lex)?);
        }

        Ok(Tree { statements })
    }
}

impl<'src> Syn<'src> for Statement<'src> {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError> {
        let name = lex.next_kind(Kind::Ident)?.string;
        let _ = lex.next_kind(Kind::Colon)?;
        let class = Class::parse(lex)?;
        Ok(Statement { name, class })
    }
}
