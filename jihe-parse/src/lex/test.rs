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
        Some(Ok(Token { kind: Kind::Num, string: "325", range }))
            if matches!(range.start, Cursor { line: 0, col: 0 })
            && matches!(range.end, Cursor { line: 0, col: 3 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Num, string: "0.6", range }))
            if matches!(range.start, Cursor { line: 0, col: 6 })
            && matches!(range.end, Cursor { line: 0, col: 9 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Num, string: "777.", range }))
            if matches!(range.start, Cursor { line: 0, col: 10 })
            && matches!(range.end, Cursor { line: 0, col: 14 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Ident, string: "🍎", range }))
            if matches!(range.start, Cursor { line: 1, col: 1 })
            && matches!(range.end, Cursor { line: 1, col: 2 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Ident, string: "xx", range }))
            if matches!(range.start, Cursor { line: 1, col: 3 })
            && matches!(range.end, Cursor { line: 1, col: 5 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Ident, string: "yy", range }))
            if matches!(range.start, Cursor { line: 1, col: 6 })
            && matches!(range.end, Cursor { line: 1, col: 8 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::VarX, string: "x", range }))
            if matches!(range.start, Cursor { line: 1, col: 9 })
            && matches!(range.end, Cursor { line: 1, col: 10 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::VarY, string: "y", range }))
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
        Some(Ok(Token { kind: Kind::ParenL, string: "(", range }))
            if matches!(range.start, Cursor { line: 1, col: 15 })
            && matches!(range.end, Cursor { line: 1, col: 16 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::ParenR, string: ")", range }))
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
        Some(Ok(Token { kind: Kind::Pow, string: "^", range }))
            if matches!(range.start, Cursor { line: 1, col: 22 })
            && matches!(range.end, Cursor { line: 1, col: 23 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Mul, string: "*", range }))
            if matches!(range.start, Cursor { line: 1, col: 23 })
            && matches!(range.end, Cursor { line: 1, col: 24 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Div, string: "/", range }))
            if matches!(range.start, Cursor { line: 1, col: 24 })
            && matches!(range.end, Cursor { line: 1, col: 25 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Add, string: "+", range }))
            if matches!(range.start, Cursor { line: 1, col: 26 })
            && matches!(range.end, Cursor { line: 1, col: 27 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Sub, string: "-", range }))
            if matches!(range.start, Cursor { line: 1, col: 28 })
            && matches!(range.end, Cursor { line: 1, col: 29 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok(Token { kind: Kind::Eq, string: "=", range }))
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
