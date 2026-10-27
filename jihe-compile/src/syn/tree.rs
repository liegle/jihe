use std::{
    collections::{HashMap, hash_map::Entry},
    ops::Range,
};

use crate::{
    Lex, Spanned, SynError,
    lex::{Token, TokenSet},
    syn::{Parse, Prependable, node::Node},
};

#[derive(Debug)]
pub(crate) struct Tree<'src> {
    pub(crate) name: Range<usize>,
    pub(crate) class: Range<usize>,
    pub(crate) fields: Fields<'src>,
}

impl<'src> Parse<'src> for Tree<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<Self>, SynError> {
        let start = lex.expect_token_is(Token::Ident)?;
        let name = start.clone();
        let _ = lex.expect_token_is(Token::Eq)?;
        let class = lex.expect_token_is(Token::Ident)?;
        let (fields, end) = Fields::parse(lex)?;
        Ok((Tree { name, class, fields }, start.start..end.end))
    }
}

#[derive(Debug)]
pub(crate) enum Fields<'src> {
    Named(HashMap<&'src str, Spanned<Node>>),
    Unnamed(Vec<Spanned<Node>>),
}

impl<'src> Parse<'src> for Fields<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Spanned<Self>, SynError> {
        handle_token! { lex.expect_token()? =>
            (Token::BraceL, start) => {
                let mut fields = HashMap::<&'src str, Spanned<Node>>::new();
                let mut span = lex.expect_token_is(Token::Ident)?;
                let end = loop {
                    let name = &lex.inner().source()[span.clone()];
                    let entry = match fields.entry(name) {
                        Entry::Vacant(entry) => entry,
                        Entry::Occupied(entry) => return Err(SynError::DuplicatedStatementField {
                            name: name.to_owned(),
                            prev_span: entry.get().1.clone(),
                            span,
                        })
                    };
                    let _ = lex.expect_token_is(Token::Colon)?;
                    let (node, end) = Node::parse(lex)?;
                    entry.insert((node, span.start..end.end));
                    handle_token! { lex.expect_token()? =>
                        (Token::BraceR, end) => { break end }
                        (Token::Comma, _) => {}
                        _ => UnexpectedToken @ lex
                    }
                    handle_token! { lex.expect_token()? =>
                        (Token::BraceR, end) => { break end }
                        (Token::Ident, next) => { span = next; }
                        _ => UnexpectedToken @ lex
                    }
                };
                Ok((Self::Named(fields), start.start..end.end))
            }
            (Token::ParenL, start) => {
                let mut fields = Vec::new();
                let end = loop {
                    fields.push(Node::parse(lex)?);
                    handle_token! { lex.expect_token()? =>
                        (Token::ParenR, end) => { break end }
                        (Token::Comma, _) => {}
                        _ => UnexpectedToken @ lex
                    }
                    match lex.expect_token()? {
                        (Token::ParenR, end) => break end,
                        spanned => lex.prepend(Ok(spanned)),
                    }
                };
                Ok((Self::Unnamed(fields), start.start..end.end))
            }
            _ => UnexpectedToken @ lex
        }
    }
}

#[cfg(test)]
mod test {
    use crate::syn::node::{Binary, Primary};

    use std::assert_matches;

    use super::*;

    #[test]
    fn test_named() {
        let mut lex = Prependable::from_iter(Lex::new(
            "len = Fn { def: fn(x, y) = (x ^ 2 + y ^ 2) ^ 0.5 }",
        ));
        let spanned = Tree::parse(&mut lex);
        let Ok((tree, span)) = spanned else {
            panic!("test_named failed 1, left: {spanned:?}");
        };
        assert_eq!(
            &lex.inner().source()[span.clone()],
            "len = Fn { def: fn(x, y) = (x ^ 2 + y ^ 2) ^ 0.5 }"
        );
        assert_matches!(span, Range { start: 0, end: 50 });
        assert_eq!(tree.name, Range { start: 0, end: 3 });
        assert_eq!(tree.class, Range { start: 6, end: 8 });
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
        let Ok((tree, span)) = spanned else {
            panic!("test_unnamed failed 1, left: {spanned:?}");
        };
        assert_eq!(&lex.inner().source()[span.clone()], "alpha = Slide(0, 5)");
        assert_matches!(span, Range { start: 0, end: 19 });
        assert_eq!(tree.name, Range { start: 0, end: 5 });
        assert_eq!(tree.class, Range { start: 8, end: 13 });
        let Fields::Unnamed(fields) = tree.fields else {
            panic!("test_unnamed failed 2, left: {:?}", tree.fields);
        };
        assert_eq!(fields.len(), 2);
        let node = fields.get(0);
        let Some(node) = node else {
            panic!("test_unnamed failed 3, left: {node:?}");
        };
        let Node::Primary(Primary::Num(Range { start: 14, end: 15 })) = node.0 else {
            panic!("test_unnamed failed 4, left: {node:?}");
        };
        let node = fields.get(1);
        let Some(node) = node else {
            panic!("test_unnamed failed 5, left: {node:?}");
        };
        let Node::Primary(Primary::Num(Range { start: 17, end: 18 })) = node.0 else {
            panic!("test_unnamed failed 6, left: {node:?}");
        };
    }

    #[test]
    fn test_duplicated_statement_field() {
        let mut lex = Prependable::from_iter(Lex::new("a = Config { max_fps: 60, max_fps: 30 }"));
        assert_matches!(
            Tree::parse(&mut lex),
            Err(SynError::DuplicatedStatementField {
                name,
                prev_span: Range { start: 13, end: 24 },
                span: Range { start: 26, end: 33 }
            }) if name == "max_fps"
        );
    }
}
