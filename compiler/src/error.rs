//! Unified error system – Polish + English messages with source locations

use ariadne::{Color, Label, Report, ReportKind, Source};
use std::ops::Range;
use thiserror::Error;

pub type AxResult<T> = Result<T, AxError>;

#[derive(Debug, Error)]
pub enum AxError {
    #[error("{0}")]
    Message(String),

    #[error("Parse error")]
    Parse {
        message_pl: String,
        message_en: String,
        span: Range<usize>,
        file: String,
        suggestion: Option<String>,
    },

    #[error("Semantic error")]
    Semantic {
        code: &'static str,
        message_pl: String,
        message_en: String,
        span: Range<usize>,
        file: String,
        suggestion: Option<String>,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Codegen error: {0}")]
    Codegen(String),
}

impl AxError {
    pub fn report(&self, source: &str) {
        match self {
            AxError::Parse { message_pl, message_en, span, file, suggestion }
            | AxError::Semantic { message_pl, message_en, span, file, suggestion, .. } => {
                let mut report = Report::build(ReportKind::Error, file.clone(), span.start)
                    .with_message(format!("{}\n{}", message_pl, message_en))
                    .with_label(
                        Label::new((file.clone(), span.clone()))
                            .with_message(message_pl)
                            .with_color(Color::Red),
                    );

                if let Some(sug) = suggestion {
                    report = report.with_help(sug);
                }

                report.finish().print((file.clone(), Source::from(source))).ok();
            }
            other => eprintln!("Error: {}", other),
        }
    }
}

// Common diagnostic codes used by the semantic analyzer
pub mod codes {
    pub const UNKNOWN_SCREEN: &str = "AX1001";
    pub const UNKNOWN_VAR: &str = "AX1042";
    pub const MISSING_ALT: &str = "AX2010";
    pub const PROVIDER_DISABLED: &str = "AX-MUSIC-042";
    pub const NO_TRANSLATE_PROVIDER: &str = "AX-TRANS-017";
    pub const ADMIN_FORBIDDEN: &str = "AX-ADMIN-001";
    pub const PERSONA_INVALID: &str = "AX-PERSONA-003";
}
