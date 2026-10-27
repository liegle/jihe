use std::ops::Range;

pub(super) use crate::syn::{
    error::SynError,
    node::{Binary, Delimit, Node, Primary, Unary},
    tree::{Fields, Tree},
};

use crate::{
    Lex, Spanned,
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
            (_, span) => {
                return Err(SynError::UnexpectedToken {
                    expected: TokenSet::with_values([$(Token::$token),+]),
                    found: $lex.inner().source()[span.clone()].to_owned(),
                    span,
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
    type Item = Result<Spanned<Tree<'src>>, SynError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lex.next().map(|token| {
            self.lex.prepend(token);
            Tree::parse(&mut self.lex)
        })
    }
}

trait Parse<'src>: Sized {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<Self>, SynError>;
}

impl<'src> Prependable<Lex<'src>> {
    fn expect_token(&mut self) -> Result<Spanned<Token>, SynError> {
        match self.next() {
            Some(token) => token.map_err(SynError::LexError),
            None => Err(SynError::UnexpectedEof { byte: self.inner().source().len() }),
        }
    }

    fn expect_token_is(&mut self, token: Token) -> Result<Range<usize>, SynError> {
        let (tok, span) = self.expect_token()?;
        if tok == token {
            Ok(span)
        } else {
            Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([token]),
                found: self.inner().source()[span.clone()].to_owned(),
                span,
            })
        }
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_syn() {
        let source = r#"config = Config {
                max_fps: 30
            }

            f = Fn( def(x, y, m) = x * m + ln(y) )

            alpha = Slide(0, 1)

            c = Curve {
                expr: f(1, x, 2) = y ^ 2,
                color: color(0, 1, 0, alpha)
            }"#;
        let lex = Lex::new(source);
        let mut syn = Syn::new(lex);
        assert_matches!(
            syn.next(),
            Some(Ok((
                Tree {
                    name: Range { start: 0, end: 6 },
                    class: Range { start: 9, end: 15 },
                    fields: Fields::Named(fields)
                },
                Range { start: 0, end: 59 }
            ))) if fields.get("max_fps").is_some()
        );
        assert_matches!(
            syn.next(),
            Some(Ok((
                Tree {
                    name: Range { start: 73, end: 74 },
                    class: Range { start: 77, end: 79 },
                    fields: Fields::Unnamed(fields)
                },
                Range { start: 73, end: 111 }
            ))) if fields.get(0).is_some()
        );
        assert_matches!(
            syn.next(),
            Some(Ok((
                Tree {
                    name: Range { start: 125, end: 130 },
                    class: Range { start: 133, end: 138 },
                    fields: Fields::Unnamed(fields)
                },
                Range { start: 125, end: 144 }
            ))) if fields.get(0).is_some() && fields.get(1).is_some()
        );
        assert_matches!(
            syn.next(),
            Some(Ok((
                Tree {
                    name: Range { start: 158, end: 159 },
                    class: Range { start: 162, end: 167 },
                    fields: Fields::Named(fields)
                },
                Range { start: 158, end: 270 }
            ))) if fields.get("expr").is_some() && fields.get("color").is_some()
        );
        assert_matches!(syn.next(), None);
    }
}
