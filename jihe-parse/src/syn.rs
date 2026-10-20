pub(super) use crate::syn::{
    error::SynError,
    node::{Binary, Delimit, Node, Primary, Unary},
    tree::{Fields, Tree},
};

use crate::{
    Lex, Span, Spanned,
    lex::{Token, TokenSet},
    syn::prepend::Prependable,
};

mod error;
mod prepend;

macro_rules! handle_token {
    {
        $target:expr =>
            $((Token::$token:tt, $span:pat) => $handle:block)+
            _ => UnexpectedToken @ $lex:ident
    } => {
        match $target {
            $((Token::$token, $span) => $handle)+
            (_, Span { byte_span, .. }) => {
                return Err(SynError::UnexpectedToken {
                    expected: TokenSet::with_values([$(Token::$token),+]),
                    found: $lex.inner().source()[byte_span.clone()].to_owned(),
                    span: byte_span,
                });
            }
        }
    };
}

mod node;
mod tree;

pub(super) struct Syn<'src> {
    lex: Prependable<Lex<'src>>,
}

impl<'src> Syn<'src> {
    pub(super) fn new(lex: Lex<'src>) -> Self {
        Self { lex: Prependable::from_iter(lex) }
    }
}

impl<'src> Iterator for Syn<'src> {
    type Item = Result<Spanned<'src, Tree<'src>>, SynError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lex.next().map(|token| {
            self.lex.prepend(token);
            Tree::parse(&mut self.lex)
        })
    }
}

trait Parse<'src>: Sized {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError>;
}

trait ExpectToken<'src>: 'src {
    fn expect_token(&mut self) -> Result<Spanned<'src, Token>, SynError>;
    fn expect_token_is(&mut self, token: Token) -> Result<Span<'src>, SynError>;
}

impl<'src> ExpectToken<'src> for Prependable<Lex<'src>> {
    fn expect_token(&mut self) -> Result<Spanned<'src, Token>, SynError> {
        match self.next() {
            Some(token) => token.map_err(SynError::LexError),
            None => Err(SynError::UnexpectedEof),
        }
    }

    fn expect_token_is(&mut self, token: Token) -> Result<Span<'src>, SynError> {
        let (tok, span) = self.expect_token()?;
        if tok == token {
            Ok(span)
        } else {
            Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([token]),
                found: self.inner().source()[span.byte_span.clone()].to_owned(),
                span: span.byte_span,
            })
        }
    }
}
