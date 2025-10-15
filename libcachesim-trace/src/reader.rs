use crate::error::{Result, TraceError};
use libcachesim_core::Request;

/// Trait for reading cache traces
pub trait TraceReader: Iterator<Item = Result<Request>> {
    /// Get the total number of requests in the trace (if known)
    fn n_requests(&self) -> Option<u64> {
        None
    }
    
    /// Reset the reader to the beginning of the trace
    fn reset(&mut self) -> Result<()>;
}

/// Builder for creating trace readers
pub struct TraceReaderBuilder {
    path: String,
    format: Option<TraceFormat>,
}

/// Supported trace formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFormat {
    /// CSV format
    Csv,
    /// LCS binary format
    Lcs,
}

impl TraceReaderBuilder {
    /// Create a new builder for the given file path
    pub fn new(path: impl Into<String>) -> Self {
        TraceReaderBuilder {
            path: path.into(),
            format: None,
        }
    }
    
    /// Set the trace format explicitly
    pub fn format(mut self, format: TraceFormat) -> Self {
        self.format = Some(format);
        self
    }
    
    /// Build the trace reader
    ///
    /// If format is not specified, it will be detected from file extension
    pub fn build(self) -> Result<Box<dyn TraceReader>> {
        let format = self.format.unwrap_or_else(|| {
            // Detect format from extension
            if self.path.ends_with(".csv") {
                TraceFormat::Csv
            } else if self.path.ends_with(".lcs") || self.path.ends_with(".bin") {
                TraceFormat::Lcs
            } else {
                TraceFormat::Csv // Default to CSV
            }
        });
        
        match format {
            TraceFormat::Csv => {
                let reader = crate::csv_reader::CsvTraceReader::new(&self.path)?;
                Ok(Box::new(reader))
            }
            TraceFormat::Lcs => {
                let reader = crate::lcs_reader::LcsTraceReader::new(&self.path)?;
                Ok(Box::new(reader))
            }
        }
    }
}
