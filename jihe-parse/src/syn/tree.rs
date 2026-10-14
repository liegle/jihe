use std::collections::{HashMap, hash_map::Entry};

use crate::{
    Lex, Span, Spanned, SynError,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Parse, Prependable, node::Node},
};

pub(crate) struct Tree<'src> {
    pub(super) name: &'src str,
    pub(super) class: &'src str,
    pub(super) fields: Fields<'src>,
}

impl<'src> Parse<'src> for Tree<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        let start = lex.expect_token_is(Token::Ident)?;
        let name = &lex.inner().source()[start.byte_span.clone()];
        let _ = lex.expect_token_is(Token::Eq)?;
        let class = &lex.inner().source()[lex.expect_token_is(Token::Ident)?.byte_span];
        let (fields, end) = Fields::parse(lex)?;
        Ok((Tree { name, class, fields }, Span::join(&start, &end)))
    }
}

pub(crate) enum Fields<'src> {
    Named(HashMap<&'src str, Node<'src>>),
    Unnamed(Vec<Node<'src>>),
}

impl<'src> Parse<'src> for Fields<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        handle_token! {
            lex.expect_token()? => {
                (Token::BraceL, start) => {
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
                        entry.insert(Node::parse(lex)?.0); // TODO: is span really needed to be returned?
                        handle_token! {
                            lex.expect_token()? => {
                                (Token::BraceR, end) => { break end }
                                (Token::Comma, _) => {}
                                _ => UnexpectedToken @ lex
                            }
                        }
                        handle_token! {
                            lex.expect_token()? => {
                                (Token::BraceR, end) => { break end }
                                (Token::Ident, next) => {
                                    byte_span = next.byte_span;
                                    cursor_span = next.cursor_span;
                                }
                                _ => UnexpectedToken @ lex
                            }
                        }
                    };
                    Ok((Self::Named(fields), Span::join(&start, &end)))
                }
                (Token::ParenL, start) => {
                    let mut fields = Vec::new();
                    let end = loop {
                        fields.push(Node::parse(lex)?.0); // TODO: is span really needed to be returned?
                        handle_token! {
                            lex.expect_token()? => {
                                (Token::ParenR, end) => { break end }
                                (Token::Comma, _) => {}
                                _ => UnexpectedToken @ lex
                            }
                        }
                        match lex.expect_token()? {
                            (Token::ParenR, end) => break end,
                            spanned => lex.prepend(Ok(spanned)),
                        }
                    };
                    Ok((Self::Unnamed(fields), Span::join(&start, &end)))
                }
                _ => UnexpectedToken @ lex
            }
        }
    }
}
