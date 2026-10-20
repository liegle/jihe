use std::ops::Range;

use crate::{
    LexError, Report,
    lex::{Token, TokenSet},
};

#[derive(Debug)]
pub enum SynError {
    LexError(LexError),
    UnexpectedEof,
    UnexpectedToken {
        expected: TokenSet,
        found: String,
        span: Range<usize>,
    },
    DuplicatedStatementField {
        name: String,
        prev_span: Range<usize>,
        span: Range<usize>,
    },
    UnaryInsideExpr {
        span: Range<usize>,
    },
    DismatchDelimit {
        expected: Token,
        found: Token,
        span: Range<usize>,
    },
    CommaAsBinary {
        // comma is single byte so there's no need to use span here
        byte: usize,
    },
}

impl Report for SynError {
    fn report(self) -> ariadne::Report<'static> {
        match self {
            Self::LexError(e) => e.report(),
            Self::UnexpectedEof => ariadne::Report::build(ariadne::ReportKind::Error, 0..0)
                .with_message("Syn error: Unexpected end of file")
                .with_note("It's expected to be more tokens input")
                .finish(),
            Self::UnexpectedToken { expected, found, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Syn error: Unexpected token")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Expected {}, found {}",
                                ariadne::Fmt::fg(format!("{expected}"), ariadne::Color::Green),
                                ariadne::Fmt::fg(format!("{found}"), ariadne::Color::Red)
                            )),
                    )
                    .finish()
            }
            Self::DuplicatedStatementField { name, prev_span, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Syn error: Duplicated statement field")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "The field name {} is duplicated",
                                ariadne::Fmt::fg(name, ariadne::Color::Red)
                            )),
                    )
                    .with_label(
                        ariadne::Label::new(prev_span)
                            .with_color(ariadne::Color::Yellow)
                            .with_message("A field with the same name is already defined here"),
                    )
                    .finish()
            }
            Self::UnaryInsideExpr { span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Syn error: Unary operator inside expression")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message("Unary operator must be start of en expression"),
                    )
                    .finish()
            }
            Self::DismatchDelimit { expected, found, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Syn error: Dismatch delimiter")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Expected {}, found {}",
                                ariadne::Fmt::fg(format!("{expected}"), ariadne::Color::Green),
                                ariadne::Fmt::fg(format!("{found}"), ariadne::Color::Red),
                            )),
                    )
                    .finish()
            }
            Self::CommaAsBinary { byte } => {
                ariadne::Report::build(ariadne::ReportKind::Error, byte..(byte + 1))
                    .with_message("Syn error: Comma use as binary operator")
                    .with_label(
                        ariadne::Label::new(byte..(byte + 1))
                            .with_color(ariadne::Color::Red)
                            .with_message(
                                "Comma should only be used to divide function arguements",
                            ),
                    )
                    .finish()
            }
        }
    }
}
