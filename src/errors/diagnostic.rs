use thiserror::Error;

use crate::ast::span::Span;

#[derive(Debug, Error, Clone)]
pub enum VerunError {
    #[error("parse error: {message}")]
    ParseError { message: String, span: Option<Span> },

    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Option<Span>,
    },

    #[error("undefined variable: {name}")]
    UndefinedVariable { name: String, span: Option<Span> },

    #[error("undefined type: {name}")]
    UndefinedType { name: String, span: Option<Span> },

    #[error("duplicate definition: {name}")]
    DuplicateDefinition { name: String, span: Option<Span> },

    #[error("invalid invariant: {message}")]
    InvalidInvariant { message: String, span: Option<Span> },

    #[error("verification failed: {message}")]
    VerificationFailed { message: String, span: Option<Span> },

    #[error("runtime error: {message}")]
    RuntimeError { message: String, span: Option<Span> },

    #[error("codegen error: {message}")]
    CodegenError { message: String },

    #[error("invalid enum variant: {variant} is not a variant of {enum_name}")]
    InvalidEnumVariant {
        enum_name: String,
        variant: String,
        span: Option<Span>,
    },

    #[error("uninitialized field: {name}")]
    UninitializedField { name: String, span: Option<Span> },

    #[error("old() can only be used in postconditions (ensure blocks)")]
    OldOutsideEnsure { span: Option<Span> },

    #[error("state '{name}' has no init block")]
    MissingInit { name: String, span: Option<Span> },

    #[error("transition '{name}' has no body statements")]
    EmptyTransitionBody { name: String, span: Option<Span> },

    #[error("non-exhaustive match: missing variant {missing}")]
    NonExhaustiveMatch { missing: String, span: Option<Span> },

    #[error("old() cannot be applied to parameter '{name}': only state fields are allowed")]
    OldOnParameter { name: String, span: Option<Span> },
}

impl VerunError {
    pub fn span(&self) -> Option<Span> {
        match self {
            VerunError::ParseError { span, .. } => *span,
            VerunError::TypeMismatch { span, .. } => *span,
            VerunError::UndefinedVariable { span, .. } => *span,
            VerunError::UndefinedType { span, .. } => *span,
            VerunError::DuplicateDefinition { span, .. } => *span,
            VerunError::InvalidInvariant { span, .. } => *span,
            VerunError::VerificationFailed { span, .. } => *span,
            VerunError::RuntimeError { span, .. } => *span,
            VerunError::CodegenError { .. } => None,
            VerunError::InvalidEnumVariant { span, .. } => *span,
            VerunError::UninitializedField { span, .. } => *span,
            VerunError::OldOutsideEnsure { span, .. } => *span,
            VerunError::MissingInit { span, .. } => *span,
            VerunError::EmptyTransitionBody { span, .. } => *span,
            VerunError::NonExhaustiveMatch { span, .. } => *span,
            VerunError::OldOnParameter { span, .. } => *span,
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            VerunError::ParseError { .. } => Severity::Error,
            VerunError::TypeMismatch { .. } => Severity::Error,
            VerunError::UndefinedVariable { .. } => Severity::Error,
            VerunError::UndefinedType { .. } => Severity::Error,
            VerunError::DuplicateDefinition { .. } => Severity::Error,
            VerunError::InvalidInvariant { .. } => Severity::Error,
            VerunError::VerificationFailed { .. } => Severity::Error,
            VerunError::RuntimeError { .. } => Severity::Error,
            VerunError::CodegenError { .. } => Severity::Error,
            VerunError::InvalidEnumVariant { .. } => Severity::Error,
            VerunError::UninitializedField { .. } => Severity::Error,
            VerunError::OldOutsideEnsure { .. } => Severity::Error,
            VerunError::MissingInit { .. } => Severity::Warning,
            VerunError::EmptyTransitionBody { .. } => Severity::Warning,
            VerunError::NonExhaustiveMatch { .. } => Severity::Error,
            VerunError::OldOnParameter { .. } => Severity::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
