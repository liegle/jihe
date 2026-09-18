#![cfg(test)]

use std::assert_matches;

use super::*;
use crate::lex::token::{Kind, Token};

#[test]
fn test_none() {
    let mut lex = Lex::new("");
    assert_matches!(lex.next(), None);
}

#[test]
fn test_whitespaces() {
    let mut lex = Lex::new("   \n\t\r   ");
    assert_matches!(lex.next(), None);
}

#[test]
fn test_all() {
    let mut lex = Lex::new("325 \t 0.6 777. \n 🍎 xx yy x y { () } \r ^*/ + - = ,:");
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Number, string: "325" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Number, string: "0.6" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Number, string: "777." })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Identifier, string: "🍎" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Identifier, string: "xx" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Identifier, string: "yy" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::VariableX, string: "x" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::VariableY, string: "y" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::BraceL, string: "{" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::ParentheseL, string: "(" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::ParentheseR, string: ")" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::BraceR, string: "}" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Power, string: "^" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Multiply, string: "*" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Divide, string: "/" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Plus, string: "+" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Minus, string: "-" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Equal, string: "=" })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Comma, string: "," })));
    assert_matches!(lex.next(), Some(Ok(Token { kind: Kind::Colon, string: ":" })));
    assert_matches!(lex.next(), None);
}

#[test]
fn test_unexcepted_begin() {
    let mut lex = Lex::new("  \n \t @");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedBegin {
            found: '@',
            cursor: Cursor { line: 1, col: 3 }
        }))
    );
}
