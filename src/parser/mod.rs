pub mod builder;
pub mod grammar;
pub mod imports;

use anyhow::Result;

use crate::ast::nodes::Program;
use crate::ast::span::Span;
use crate::errors::diagnostic::VerunError;
use builder::build_program;
use grammar::parse;
pub use imports::{parse_file_with_imports, LoadedProgram};

pub fn parse_source(source: &str) -> Result<Program> {
    let pairs = parse(source).map_err(|e| {
        let span = match e.location {
            pest::error::InputLocation::Pos(p) => Span::new(p, p.saturating_add(1).min(source.len())),
            pest::error::InputLocation::Span((s, end)) => Span::new(s, end),
        };
        let message = match &e.variant {
            pest::error::ErrorVariant::ParsingError { positives, negatives } => {
                let pos_str: Vec<String> = positives.iter().map(|r| format!("{:?}", r)).collect();
                let neg_str: Vec<String> = negatives.iter().map(|r| format!("{:?}", r)).collect();
                if !positives.is_empty() && !negatives.is_empty() {
                    format!("expected {}; unexpected {}", pos_str.join(", "), neg_str.join(", "))
                } else if !positives.is_empty() {
                    format!("expected {}", pos_str.join(", "))
                } else {
                    format!("unexpected {}", neg_str.join(", "))
                }
            }
            pest::error::ErrorVariant::CustomError { message } => message.clone(),
        };
        VerunError::ParseError { message, span: Some(span) }
    })?;
    build_program(pairs, source)
}
