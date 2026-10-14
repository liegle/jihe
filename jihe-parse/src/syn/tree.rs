use std::collections::{HashMap, hash_map::Entry};

use crate::{
    Lex, Span, Spanned, SynError,
    lex::{Token, TokenSet},
    syn::{ExpectToken, Parse, Prependable, node::Node},
};

#[derive(Debug)]
pub(crate) struct Tree<'src> {
    pub(crate) name: &'src str,
    pub(crate) class: &'src str,
    pub(crate) fields: Fields<'src>,
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

#[derive(Debug)]
pub(crate) enum Fields<'src> {
    Named(HashMap<&'src str, Spanned<'src, Node<'src>>>),
    Unnamed(Vec<Spanned<'src, Node<'src>>>),
}

impl<'src> Parse<'src> for Fields<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<'src, Self>, SynError> {
        handle_token! {
            lex.expect_token()? => {
                (Token::BraceL, start) => {
                    let mut fields = HashMap::new();
                    let Span { mut byte_span, mut cursor_span, .. } = lex.expect_token_is(Token::Ident)?;
                    let end = loop {
                        let name = &lex.inner().source()[byte_span.clone()];
                        let Entry::Vacant(entry) = fields.entry(name) else {
                            return Err(SynError::DuplicatedStatementField {
                                name: name.to_owned(),
                                cursor_span,
                            });
                        };
                        let _ = lex.expect_token_is(Token::Colon)?;
                        let (node, end) = Node::parse(lex)?;
                        entry.insert((node, Span {
                            byte_span: byte_span.start..end.byte_span.end,
                            cursor_span: cursor_span.start..end.cursor_span.end,
                            ..Default::default()
                        }));
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
                        fields.push(Node::parse(lex)?);
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

#[cfg(test)]
mod test {
    use crate::{
        Cursor,
        syn::node::{Binary, Primary},
    };

    use std::assert_matches;

    use super::*;

    #[test]
    fn test_named() {
        let mut lex =
            Prependable::from_iter(Lex::new("len = Fn { def: (x, y) = (x ^ 2 + y ^ 2) ^ 0.5 }"));
        let spanned = Tree::parse(&mut lex);
        let Ok((tree, Span { byte_span, cursor_span, .. })) = spanned else {
            panic!("test_named failed 1, left: {spanned:?}");
        };
        assert_eq!(
            &lex.inner().source()[byte_span],
            "len = Fn { def: (x, y) = (x ^ 2 + y ^ 2) ^ 0.5 }"
        );
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 48 });
        assert_eq!(tree.name, "len");
        assert_eq!(tree.class, "Fn");
        let Fields::Named(fields) = tree.fields else {
            panic!("test_named failed 2, left: {:?}", tree.fields);
        };
        assert_eq!(fields.len(), 1);
        let node = fields.get("def");
        let Some(node) = node else {
            panic!("test_named failed 3, left: {node:?}");
        };
        let Node::Binary(Binary::Eq, ..) = node.0 else {
            panic!("test_named failed 4, left: {node:?}");
        };
    }

    #[test]
    fn test_unnamed() {
        let mut lex = Prependable::from_iter(Lex::new("alpha = Slide(0, 5)"));
        let spanned = Tree::parse(&mut lex);
        let Ok((tree, Span { byte_span, cursor_span, .. })) = spanned else {
            panic!("test_unnamed failed 1, left: {spanned:?}");
        };
        assert_eq!(&lex.inner().source()[byte_span], "alpha = Slide(0, 5)");
        assert_matches!(cursor_span.start, Cursor { line: 0, col: 0 });
        assert_matches!(cursor_span.end, Cursor { line: 0, col: 19 });
        assert_eq!(tree.name, "alpha");
        assert_eq!(tree.class, "Slide");
        let Fields::Unnamed(fields) = tree.fields else {
            panic!("test_unnamed failed 2, left: {:?}", tree.fields);
        };
        assert_eq!(fields.len(), 2);
        let node = fields.get(0);
        let Some(node) = node else {
            panic!("test_unnamed failed 3, left: {node:?}");
        };
        let Node::Primary(Primary::Num("0")) = node.0 else {
            panic!("test_unnamed failed 4, left: {node:?}");
        };
        let node = fields.get(1);
        let Some(node) = node else {
            panic!("test_unnamed failed 5, left: {node:?}");
        };
        let Node::Primary(Primary::Num("5")) = node.0 else {
            panic!("test_unnamed failed 6, left: {node:?}");
        };
    }
}
