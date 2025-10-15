use thiserror::Error;

/// Errors that can occur during trace operations
#[derive(Debug, Error)]
pub enum TraceError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// CSV parsing error
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// Invalid trace format
    #[error("Invalid trace format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// End of trace
    #[error("End of trace")]
    EndOfTrace,
}

pub type Result<T> = std::result::Result<T, TraceError>;
