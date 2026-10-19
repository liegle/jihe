#![cfg(test)]

use std::{assert_matches, ops::Range};

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

macro_rules! assert_token {
    ($lex:ident,$token:tt@$ls:literal:$cs:literal..$le:literal:$ce:literal=$value:literal) => {
        assert_matches!(
            $lex.next(),
            Some(Ok((Token::$token, Span {
                byte_span,
                cursor_span: Range {
                    start: Cursor { line: $ls, col: $cs },
                    end: Cursor { line: $le, col: $ce },
                },
                ..
            }))) if &$lex.source[byte_span.clone()] == $value
        );
    };
}

#[test]
fn test_all() {
    let mut lex =
        Lex::new("325 \t 0.6 777. # This is comment\n 🍎 xx yy x y { () } \r ^*/ + - = ,:|\\\\//");
    assert_token!(lex, Num    @ 0:0..0:3   = "325");
    assert_token!(lex, Num    @ 0:6..0:9   = "0.6");
    assert_token!(lex, Num    @ 0:10..0:14 = "777.");
    assert_token!(lex, Ident  @ 1:1..1:2   = "🍎");
    assert_token!(lex, Ident  @ 1:3..1:5   = "xx");
    assert_token!(lex, Ident  @ 1:6..1:8   = "yy");
    assert_token!(lex, Ident  @ 1:9..1:10  = "x");
    assert_token!(lex, Ident  @ 1:11..1:12 = "y");
    assert_token!(lex, BraceL @ 1:13..1:14 = "{");
    assert_token!(lex, ParenL @ 1:15..1:16 = "(");
    assert_token!(lex, ParenR @ 1:16..1:17 = ")");
    assert_token!(lex, BraceR @ 1:18..1:19 = "}");
    assert_token!(lex, Pow    @ 1:22..1:23 = "^");
    assert_token!(lex, Mul    @ 1:23..1:24 = "*");
    assert_token!(lex, Div    @ 1:24..1:25 = "/");
    assert_token!(lex, Add    @ 1:26..1:27 = "+");
    assert_token!(lex, Sub    @ 1:28..1:29 = "-");
    assert_token!(lex, Eq     @ 1:30..1:31 = "=");
    assert_token!(lex, Comma  @ 1:32..1:33 = ",");
    assert_token!(lex, Colon  @ 1:33..1:34 = ":");
    assert_token!(lex, Bar    @ 1:34..1:35 = "|");
    assert_token!(lex, SlashB @ 1:35..1:37 = "\\\\");
    assert_token!(lex, SlashF @ 1:37..1:39 = "//");
    assert_matches!(lex.next(), None);
}

#[test]
fn test_unexpected_begin() {
    let mut lex = Lex::new("  \n \t @");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedBegin {
            found: '@',
            cursor: Cursor { line: 1, col: 3 }
        }))
    );

    let mut lex = Lex::new("_ident_with_underscore'");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedBegin {
            found: '_',
            cursor: Cursor { line: 0, col: 0 }
        }))
    );
}

#[test]
fn test_unexpected_mid() {
    let mut lex = Lex::new("\\a");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedMid {
            expected: _,
            found: 'a',
            cursor: Cursor { line: 0, col: 1 }
        }))
    );
}

#[test]
fn test_unexpected_eof() {
    let mut lex = Lex::new("\\");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedEof {
            cursor: Cursor { line: 0, col: 1 }
        }))
    );
}

#[test]
fn test_underscore_at_end() {
    let mut lex = Lex::new("ident_''");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnderscoreAtEnd {
            cursor: Cursor { line: 0, col: 5 }
        }))
    );
}
