use std::{
    fmt::{self, Display, Formatter},
    ops::Range,
};

use crate::{Report, SynError, sem::Dep};

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
    WrongFieldDefinition {
        field: &'static str,
        def: &'static str,
        span: Range<usize>,
    },
    RedundantField {
        span: Range<usize>,
    },
    WrongDependenceType {
        name: String,
        prev_span: Range<usize>,
        prev_dep: Dep,
        span: Range<usize>,
        dep: Dep,
    },
}

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
            Self::WrongFieldDefinition { field, def, span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Sem error: Statement field structure is wrong")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Field \"{}\" should have structure looks like {def}",
                                ariadne::Fmt::fg(field, ariadne::Color::Red),
                            )),
                    )
                    .finish()
            }
            Self::RedundantField { span } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Sem error: Redundant statement field")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message("This field should not appear"),
                    )
                    .finish()
            }
            Self::WrongDependenceType { name, prev_span, prev_dep, span, dep } => {
                ariadne::Report::build(ariadne::ReportKind::Error, span.clone())
                    .with_message("Sem error: Statement was depended but with different type")
                    .with_label(
                        ariadne::Label::new(span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "Statement \"{}\" is {}",
                                ariadne::Fmt::fg(name, ariadne::Color::Red),
                                ariadne::Fmt::fg(dep, ariadne::Color::Red)
                            )),
                    )
                    .with_label(
                        ariadne::Label::new(prev_span)
                            .with_color(ariadne::Color::Red)
                            .with_message(format!(
                                "It was depended {} here",
                                ariadne::Fmt::fg(prev_dep, ariadne::Color::Red)
                            )),
                    )
                    .finish()
            }
        }
    }
}

impl Display for Dep {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Not => write!(f, "not a dependence"),
            Self::Var => write!(f, "variable"),
            Self::Fn => write!(f, "function"),
        }
    }
}
