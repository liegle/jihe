#![cfg(test)]

use std::assert_matches;

use super::*;
use crate::token::Kind;

#[test]
fn test_none() {
    let mut lexer = Lexer::new("");
    assert_matches!(lexer.next(), None);
}

#[test]
fn test_whitespaces() {
    let mut lexer = Lexer::new("   \n\t\r   ");
    assert_matches!(lexer.next(), None);
}

#[test]
fn test_all() {
    let mut lexer = Lexer::new("325 \t 0.6 \n 🍎 xx yy x y { () } \r ^*/ + - = ,:");
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Integer, bytes })) if bytes == (0..3));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Fraction, bytes })) if bytes == (6..9));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (12..16));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (17..19));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Identifier, bytes })) if bytes == (20..22));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::VariableX, bytes })) if bytes == (23..24));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::VariableY, bytes })) if bytes == (25..26));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::BraceL, bytes })) if bytes == (27..28));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::ParentheseL, bytes })) if bytes == (29..30));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::ParentheseR, bytes })) if bytes == (30..31));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::BraceR, bytes })) if bytes == (32..33));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Power, bytes })) if bytes == (36..37));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Multiply, bytes })) if bytes == (37..38));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Divide, bytes })) if bytes == (38..39));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Plus, bytes })) if bytes == (40..41));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Minus, bytes })) if bytes == (42..43));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Equal, bytes })) if bytes == (44..45));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Comma, bytes })) if bytes == (46..47));
    assert_matches!(lexer.next(), Some(Ok(Token { kind: Kind::Colon, bytes })) if bytes == (47..48));
    assert_matches!(lexer.next(), None);
}

#[test]
fn test_unexcepted_begin() {
    let mut lexer = Lexer::new("  \n \t @");
    assert_matches!(
        lexer.next(),
        Some(Err(LexerError::UnexpectedBegin {
            found: '@',
            cursor: Cursor { line: 1, col: 3 }
        }))
    );
}
