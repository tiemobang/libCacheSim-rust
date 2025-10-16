//! libCacheSim Trace I/O
//!
//! This crate provides trace reading and writing functionality.

mod csv_reader;
mod error;
mod lcs_reader;
mod reader;

pub use csv_reader::CsvTraceReader;
pub use error::TraceError;
pub use lcs_reader::{LcsTraceReader, LcsVersion};
pub use reader::{TraceFormat, TraceReader, TraceReaderBuilder};
