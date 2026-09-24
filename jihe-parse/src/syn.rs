use std::{collections::HashSet, iter::Peekable};

use crate::{
    Lex,
    lex::{Kind, Token},
    syn::tree::{Class, Statement},
};
pub(super) use {error::SynError, tree::Tree};

mod error;
mod expr;
mod tree;

// Why there's no TryFromIterator
pub(super) fn syn<'src>(lex: Lex<'src>) -> Result<Tree<'src>, SynError> {
    let mut lex = lex.peekable();
    Tree::parse(&mut lex)
}

trait Syn<'src>: Sized {
    fn parse(lex: &mut Peekable<Lex<'src>>) -> Result<Self, SynError>;
}

trait ExpectKind<'src>: 'src {
    fn next_kind(&mut self, kind: Kind) -> Result<Token<'src>, SynError>;
}

impl<'src> ExpectKind<'src> for Peekable<Lex<'src>> {
    fn next_kind(&mut self, kind: Kind) -> Result<Token<'src>, SynError> {
        let Some(token) = self.next() else {
            return Err(SynError::UnexpectedEof);
        };
        match token {
            Ok(token) => {
                if token.kind == kind {
                    Ok(token)
                } else {
                    let mut expected = HashSet::new();
                    expected.insert(kind);
                    Err(SynError::UnexpectedToken { expected, found: token.string.to_owned() })
                }
            }
            Err(e) => Err(SynError::LexError(e)),
        }
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
        let name = lex.next_kind(Kind::Identifier)?.string;
        let _ = lex.next_kind(Kind::Colon)?;
        let class = lex.next_kind(Kind::Identifier)?.string;

        let Some(l) = lex.next() else {
            return Err(SynError::UnexpectedEof);
        };
        let (class, r) = match l {
            Ok(Token { kind: Kind::BraceL, .. }) => (Class::named(class, lex)?, Kind::BraceR),
            Ok(Token { kind: Kind::ParentheseL, .. }) => {
                (Class::unnamed(class, lex)?, Kind::ParentheseR)
            }
            Ok(Token { string, .. }) => {
                let mut expected = HashSet::new();
                expected.insert(Kind::BraceL);
                expected.insert(Kind::ParentheseL);
                return Err(SynError::UnexpectedToken { expected, found: string.to_owned() });
            }
            Err(e) => return Err(SynError::LexError(e)),
        };

        // trailing comma or end ) or }
        let Some(token) = lex.next() else {
            return Err(SynError::UnexpectedEof);
        };
        match token {
            Ok(Token { kind: Kind::Comma, .. }) => {
                let _ = lex.next_kind(r)?;
            }
            Ok(Token { kind, .. }) if kind == r => {}
            Ok(Token { string, .. }) => {
                let mut expected = HashSet::new();
                expected.insert(Kind::BraceR);
                expected.insert(Kind::ParentheseR);
                expected.insert(Kind::Comma);
                return Err(SynError::UnexpectedToken { expected, found: string.to_owned() });
            }
            Err(e) => return Err(SynError::LexError(e)),
        }

        Ok(Statement { name, class })
    }
}
