use crate::{
    Lex, Spanned, lex::{Token, TokenSet}, syn::{
        prepend::Prependable,
        tree::{Class, Statement},
    }
};
pub(super) use {error::SynError, tree::Tree};

mod error;
mod expr;
mod prepend;
mod tree;

pub(super) fn syn<'src>(lex: Lex<'src>) -> Result<Tree<'src>, SynError> {
    let mut lex = Prependable::from_iter(lex);
    Tree::parse(&mut lex)
}

trait Syn<'src>: Sized {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError>;
}

trait ExpectToken<'src>: 'src {
    fn next_token(&mut self) -> Result<Spanned<'src, Token>, SynError>;
    fn expect_token(&mut self, token: Token) -> Result<Spanned<'src, Token>, SynError>;
    fn next_if_token(&mut self, token: Token);
}

impl<'src> ExpectToken<'src> for Prependable<Lex<'src>> {
    fn next_token(&mut self) -> Result<Spanned<'src, Token>, SynError> {
        match self.next() {
            Some(token) => token.map_err(SynError::LexError),
            None => Err(SynError::UnexpectedEof),
        }
    }

    fn expect_token(&mut self, token: Token) -> Result<Spanned<'src, Token>, SynError> {
        let spanned = self.next_token()?;
        if spanned.value == token {
            Ok(spanned)
        } else {
            Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([token]),
                found: spanned.string().to_owned(),
                cursor_span: spanned.cursor_span,
            })
        }
    }

    fn next_if_token(&mut self, token: Token) {
        self.next_if(|next| matches!(next, Ok(Spanned { value: t, .. }) if token == *t));
    }
}

impl<'src> Syn<'src> for Tree<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError> {
        let mut statements = Vec::new();
        while let Some(token) = lex.next() {
            lex.prepend(token);
            statements.push(Statement::parse(lex)?);
        }

        Ok(Tree { statements })
    }
}

impl<'src> Syn<'src> for Statement<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError> {
        let name = lex.expect_token(Token::Ident)?.string();
        let _ = lex.expect_token(Token::Colon)?;
        let class = Class::parse(lex)?;
        Ok(Statement { name, class })
    }
}
