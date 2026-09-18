pub(super) use crate::syn::error::SynError;
use crate::{lex::Token, syn::tree::Tree};

mod error;
mod tree;

pub(super) struct Syn<'src> {
    tree: Tree<'src>,
}

impl<'src> Syn<'src> {
    pub(super) fn new() -> Self {
        Self { tree: Tree { statements: Vec::new() } }
    }

    pub(super) fn input(&mut self, token: Token<'src>) -> Result<(), SynError> {
        // TODO
        match token.kind {
            _ => {}
        }
        Ok(())
    }

    pub(super) fn output(self) -> Result<Tree<'src>, SynError> {
        // TODO
        Ok(self.tree)
    }
}

trait Stage<'src, E: 'src, T: 'src> {
    fn step(&mut self, token: Token<'src>) -> Option<Result<E, T>>;
}
