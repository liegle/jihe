use std::ops::Range;

use crate::Report;

#[derive(Debug)]
pub enum LexError {
    UnexpectedBegin {
        found: char,
        span: Range<usize>,
    },
    UnexpectedMid {
        expected: char,
        found: char,
        span: Range<usize>,
    },
    UnexpectedEof {
        expected: char,
        byte: usize,
    },
    UnderscoreAtEnd {
        span: Range<usize>,
    },
}

impl Report for LexError {
    fn report(self) -> ariadne::Report<'static> {
        match self {
            Self::UnexpectedBegin { found, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Lex error: Unexpected token begin")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Char '{}' is not a beginning of any known token kind",
                                ariadne::Fmt::fg(found, ariadne::Color::Red)
                            )),
                    )
                    .finish()
            }
            Self::UnexpectedMid { expected, found, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Lex error: Unexpected token mid")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Char '{}' is expected to be '{}' here",
                                ariadne::Fmt::fg(found, ariadne::Color::Red),
                                ariadne::Fmt::fg(expected, ariadne::Color::Green)
                            )),
                    )
                    .finish()
            }
            Self::UnexpectedEof { expected, byte } => {
                ariadne::Report::build(ariadne::ReportKind::Error, byte..byte)
                    .with_message("Lex error: Unexpected end of file")
                    .with_label(ariadne::Label::new(byte..byte).with_message(format!(
                        "Its expected to be an additional '{}' here",
                        ariadne::Fmt::fg(expected, ariadne::Color::Green)
                    )))
                    .finish()
            }
            Self::UnderscoreAtEnd { span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Lex error: Underscore at end of ident")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message("Underscore should not appear at end of identifier"),
                    )
                    .finish()
            }
        }
    }
}
