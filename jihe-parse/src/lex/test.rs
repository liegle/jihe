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
    let mut lex =
        Lex::new("325 \t 0.6 777. # This is comment\n 🍎 xx yy x y { () } \r ^*/ + - = ,:");
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Number, string: "325", range }))
            if matches!(range.start, Cursor { line: 0, col: 0 })
            && matches!(range.end, Cursor { line: 0, col: 3 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Number, string: "0.6", range }))
            if matches!(range.start, Cursor { line: 0, col: 6 })
            && matches!(range.end, Cursor { line: 0, col: 9 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Number, string: "777.", range }))
            if matches!(range.start, Cursor { line: 0, col: 10 })
            && matches!(range.end, Cursor { line: 0, col: 14 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Identifier, string: "🍎", range }))
            if matches!(range.start, Cursor { line: 1, col: 1 })
            && matches!(range.end, Cursor { line: 1, col: 2 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Identifier, string: "xx", range }))
            if matches!(range.start, Cursor { line: 1, col: 3 })
            && matches!(range.end, Cursor { line: 1, col: 5 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Identifier, string: "yy", range }))
            if matches!(range.start, Cursor { line: 1, col: 6 })
            && matches!(range.end, Cursor { line: 1, col: 8 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::VariableX, string: "x", range }))
            if matches!(range.start, Cursor { line: 1, col: 9 })
            && matches!(range.end, Cursor { line: 1, col: 10 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::VariableY, string: "y", range }))
            if matches!(range.start, Cursor { line: 1, col: 11 })
            && matches!(range.end, Cursor { line: 1, col: 12 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::BraceL, string: "{", range }))
            if matches!(range.start, Cursor { line: 1, col: 13 })
            && matches!(range.end, Cursor { line: 1, col: 14 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::ParentheseL, string: "(", range }))
            if matches!(range.start, Cursor { line: 1, col: 15 })
            && matches!(range.end, Cursor { line: 1, col: 16 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::ParentheseR, string: ")", range }))
            if matches!(range.start, Cursor { line: 1, col: 16 })
            && matches!(range.end, Cursor { line: 1, col: 17 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::BraceR, string: "}", range }))
            if matches!(range.start, Cursor { line: 1, col: 18 })
            && matches!(range.end, Cursor { line: 1, col: 19 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Power, string: "^", range }))
            if matches!(range.start, Cursor { line: 1, col: 22 })
            && matches!(range.end, Cursor { line: 1, col: 23 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Multiply, string: "*", range }))
            if matches!(range.start, Cursor { line: 1, col: 23 })
            && matches!(range.end, Cursor { line: 1, col: 24 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Divide, string: "/", range }))
            if matches!(range.start, Cursor { line: 1, col: 24 })
            && matches!(range.end, Cursor { line: 1, col: 25 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Plus, string: "+", range }))
            if matches!(range.start, Cursor { line: 1, col: 26 })
            && matches!(range.end, Cursor { line: 1, col: 27 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Minus, string: "-", range }))
            if matches!(range.start, Cursor { line: 1, col: 28 })
            && matches!(range.end, Cursor { line: 1, col: 29 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Equal, string: "=", range }))
            if matches!(range.start, Cursor { line: 1, col: 30 })
            && matches!(range.end, Cursor { line: 1, col: 31 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Comma, string: ",", range }))
            if matches!(range.start, Cursor { line: 1, col: 32 })
            && matches!(range.end, Cursor { line: 1, col: 33 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Colon, string: ":", range }))
            if matches!(range.start, Cursor { line: 1, col: 33 })
            && matches!(range.end, Cursor { line: 1, col: 34 })
    );
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
