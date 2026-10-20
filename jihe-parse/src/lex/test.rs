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
    ($lex:ident,$token:tt=$value:literal) => {
        assert_matches!(
            $lex.next(),
            Some(Ok((Token::$token, span))) if &$lex.source[span.clone()] == $value
        );
    };
}

#[test]
fn test_all() {
    let mut lex =
        Lex::new("325 \t 0.6 777. # This is comment\n 🍎 xx yy x y { () } \r ^*/ + - = ,:|\\\\//");
    assert_token!(lex, Num = "325");
    assert_token!(lex, Num = "0.6");
    assert_token!(lex, Num = "777.");
    assert_token!(lex, Ident = "🍎");
    assert_token!(lex, Ident = "xx");
    assert_token!(lex, Ident = "yy");
    assert_token!(lex, Ident = "x");
    assert_token!(lex, Ident = "y");
    assert_token!(lex, BraceL = "{");
    assert_token!(lex, ParenL = "(");
    assert_token!(lex, ParenR = ")");
    assert_token!(lex, BraceR = "}");
    assert_token!(lex, Pow = "^");
    assert_token!(lex, Mul = "*");
    assert_token!(lex, Div = "/");
    assert_token!(lex, Add = "+");
    assert_token!(lex, Sub = "-");
    assert_token!(lex, Eq = "=");
    assert_token!(lex, Comma = ",");
    assert_token!(lex, Colon = ":");
    assert_token!(lex, Bar = "|");
    assert_token!(lex, SlashB = "\\\\");
    assert_token!(lex, SlashF = "//");
    assert_matches!(lex.next(), None);
}

#[test]
fn test_unexpected_begin() {
    let mut lex = Lex::new("  \n \t @");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedBegin {
            found: '@',
            span: Range { start: 6, end: 7 }
        }))
    );

    let mut lex = Lex::new("_ident_with_underscore'");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedBegin {
            found: '_',
            span: Range { start: 0, end: 1 }
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
            span: Range { start: 1, end: 2 }
        }))
    );
}

#[test]
fn test_unexpected_eof() {
    let mut lex = Lex::new("\\");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnexpectedEof { expected: '\\', byte: 1 }))
    );
}

#[test]
fn test_underscore_at_end() {
    let mut lex = Lex::new("ident_''");
    assert_matches!(
        lex.next(),
        Some(Err(LexError::UnderscoreAtEnd {
            span: Range { start: 5, end: 6 }
        }))
    );
}
