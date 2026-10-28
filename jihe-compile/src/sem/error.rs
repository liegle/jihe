use std::ops::Range;

use crate::{Report, SynError};

pub(crate) enum SemError {
    SynError(SynError),
    ClassNotFound {
        found: String,
        span: Range<usize>,
    },
    FieldLost {
        field: &'static str,
        span: Range<usize>,
    },
    WrongFieldStruct {
        field: &'static str,
        stru: FieldStruct,
        span: Range<usize>,
    },
    RedundantField {
        span: Range<usize>,
    },
}

// TODO
pub(crate) enum FieldStruct {}

impl Report for SemError {
    fn report(self) -> ariadne::Report<'static> {
        match self {
            Self::SynError(e) => e.report(),
            Self::ClassNotFound { found, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Sem error: Class not found")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "\"{}\" is not a name of any statement class",
                                ariadne::Fmt::fg(found, ariadne::Color::Red)
                            )),
                    )
                    .finish()
            }
            Self::FieldLost { field, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Sem error: Required field lost")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Field \"{}\" is required in this statement",
                                ariadne::Fmt::fg(field, ariadne::Color::Red)
                            )),
                    )
                    .finish()
            }
            Self::WrongFieldStruct { field, stru, span } => {
                todo!()
            }
            Self::RedundantField { span } => {
                todo!()
            }
        }
    }
}
