use std::ops::Range;

use crate::{Report, SynError};

pub(crate) enum SemError {
    SynError(SynError),
    ClassNotFound { found: String, span: Range<usize> },
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
        }
    }
}
