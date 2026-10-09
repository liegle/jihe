#![cfg(test)]

use std::assert_matches;

use super::*;
use crate::lex::token::Token;

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
        Some(Ok((Token::Num, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "325"
            && matches!(cursor_span.start, Cursor { line: 0, col: 0 })
            && matches!(cursor_span.end, Cursor { line: 0, col: 3 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Num, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "0.6"
            && matches!(cursor_span.start, Cursor { line: 0, col: 6 })
            && matches!(cursor_span.end, Cursor { line: 0, col: 9 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Num, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "777."
            && matches!(cursor_span.start, Cursor { line: 0, col: 10 })
            && matches!(cursor_span.end, Cursor { line: 0, col: 14 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Ident, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "🍎"
            && matches!(cursor_span.start, Cursor { line: 1, col: 1 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 2 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Ident, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "xx"
            && matches!(cursor_span.start, Cursor { line: 1, col: 3 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 5 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Ident, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "yy"
            && matches!(cursor_span.start, Cursor { line: 1, col: 6 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 8 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::VarX, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "x"
            && matches!(cursor_span.start, Cursor { line: 1, col: 9 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 10 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::VarY, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "y"
            && matches!(cursor_span.start, Cursor { line: 1, col: 11 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 12 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::BraceL, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "{"
            && matches!(cursor_span.start, Cursor { line: 1, col: 13 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 14 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::ParenL, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "("
            && matches!(cursor_span.start, Cursor { line: 1, col: 15 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 16 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::ParenR, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == ")"
            && matches!(cursor_span.start, Cursor { line: 1, col: 16 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 17 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::BraceR, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "}"
            && matches!(cursor_span.start, Cursor { line: 1, col: 18 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 19})
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Pow, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "^"
            && matches!(cursor_span.start, Cursor { line: 1, col: 22 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 23 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Mul, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "*"
            && matches!(cursor_span.start, Cursor { line: 1, col: 23 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 24 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Div, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "/"
            && matches!(cursor_span.start, Cursor { line: 1, col: 24 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 25 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Add, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "+"
            && matches!(cursor_span.start, Cursor { line: 1, col: 26 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 27 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Sub, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "-"
            && matches!(cursor_span.start, Cursor { line: 1, col: 28 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 29 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Eq, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == "="
            && matches!(cursor_span.start, Cursor { line: 1, col: 30 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 31 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Comma, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == ","
            && matches!(cursor_span.start, Cursor { line: 1, col: 32 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 33 })
    );
    assert_matches!(
        lex.next(),
        Some(Ok((Token::Colon, Span { byte_span, cursor_span, .. })))
            if &lex.source[byte_span.clone()] == ":"
            && matches!(cursor_span.start, Cursor { line: 1, col: 33 })
            && matches!(cursor_span.end, Cursor { line: 1, col: 34 })
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
