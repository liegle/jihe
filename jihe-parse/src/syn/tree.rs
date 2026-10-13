use std::collections::{HashMap, hash_map::Entry};

use crate::{
    Lex, Span, Spanned, SynError,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Prependable, Syn, expr::Expr},
};

pub(crate) struct Tree<'src> {
    pub(super) statements: Vec<Statement<'src>>,
}

pub(crate) struct Statement<'src> {
    pub(super) name: &'src str,
    pub(super) class: &'src str,
    pub(super) fields: Fields<'src>,
}

pub(crate) enum Fields<'src> {
    Named(HashMap<&'src str, Expr<'src>>),
    Unnamed(Vec<Expr<'src>>),
}

impl<'src> Syn<'src> for Fields<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        match lex.expect_token()? {
            (Token::BraceL, start) => Ok(Self::parse_named(lex, &start)?),
            (Token::ParenL, start) => Ok(Self::parse_unnamed(lex, &start)?),
            (_, Span { byte_span, cursor_span, .. }) => Err(SynError::UnexpectedToken {
                expected: TokenSet::with_values([Token::BraceL, Token::ParenL]),
                found: lex.inner().source()[byte_span].to_owned(),
                cursor_span,
            }),
        }
    }
}

impl<'src> Fields<'src> {
    fn parse_named(
        lex: &mut Prependable<Lex<'src>>,
        start: &Span<'src>,
    ) -> Result<Spanned<'src, Self>, SynError> {
        let mut fields = HashMap::new();
        let Span { mut byte_span, mut cursor_span, .. } = lex.expect_token_is(Token::Ident)?;
        let end = loop {
            let name = &lex.inner().source()[byte_span];
            let Entry::Vacant(entry) = fields.entry(name) else {
                return Err(SynError::DuplicatedStatementField {
                    name: name.to_owned(),
                    cursor_span,
                });
            };
            let _ = lex.expect_token_is(Token::Colon)?;
            entry.insert(Expr::parse(lex)?.0); // TODO: is span really needed to be returned?
            match lex.expect_token()? {
                (Token::BraceR, end) => break end,
                (Token::Comma, _) => match lex.expect_token()? {
                    (Token::BraceR, end) => break end,
                    (Token::Ident, next) => {
                        byte_span = next.byte_span;
                        cursor_span = next.cursor_span;
                    }
                    (_, Span { byte_span, cursor_span, .. }) => {
                        return Err(SynError::UnexpectedToken {
                            expected: TokenSet::with_values([Token::BraceR, Token::Ident]),
                            found: lex.inner().source()[byte_span].to_owned(),
                            cursor_span,
                        });
                    }
                },
                (_, Span { byte_span, cursor_span, .. }) => {
                    return Err(SynError::UnexpectedToken {
                        expected: TokenSet::with_values([Token::BraceR, Token::Comma]),
                        found: lex.inner().source()[byte_span].to_owned(),
                        cursor_span,
                    });
                }
            }
        };
        Ok((Self::Named(fields), Span::join(start, &end)))
    }

    fn parse_unnamed(
        lex: &mut Prependable<Lex<'src>>,
        start: &Span<'src>,
    ) -> Result<Spanned<'src, Self>, SynError> {
        let mut fields = Vec::new();
        let end = loop {
            fields.push(Expr::parse(lex)?.0); // TODO: is span really needed to be returned?
            match lex.expect_token()? {
                (Token::ParenR, end) => break end,
                (Token::Comma, _) => match lex.expect_token()? {
                    (Token::ParenR, end) => break end,
                    spanned => lex.prepend(Ok(spanned)),
                },
                (_, Span { byte_span, cursor_span, .. }) => {
                    return Err(SynError::UnexpectedToken {
                        expected: TokenSet::with_values([Token::BraceR, Token::Comma]),
                        found: lex.inner().source()[byte_span].to_owned(),
                        cursor_span,
                    });
                }
            }
        };
        Ok((Self::Unnamed(fields), Span::join(start, &end)))
    }
}
