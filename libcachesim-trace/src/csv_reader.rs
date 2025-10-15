//! CSV trace reader implementation

use crate::error::{Result, TraceError};
use crate::reader::TraceReader;
use csv::{Reader, ReaderBuilder};
use libcachesim_core::{Operation, Request};
use std::fs::File;
use std::path::Path;

/// CSV trace reader
///
/// Supports CSV format with columns: timestamp, obj_id, obj_size
/// Optional columns: operation, tenant_id, ttl, next_access_vtime
///
/// # Format Examples
///
/// Simple (3 columns):
/// ```csv
/// timestamp,obj_id,obj_size
/// 0,1,100
/// 1,2,200
/// ```
///
/// Extended (with operation):
/// ```csv
/// timestamp,obj_id,obj_size,operation
/// 0,1,100,get
/// 1,2,200,set
/// ```
pub struct CsvTraceReader {
    reader: Reader<File>,
    path: String,
    n_requests: Option<u64>,
}

impl CsvTraceReader {
    /// Create a new CSV trace reader
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path)?;
        
        let reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(file);
        
        Ok(CsvTraceReader {
            reader,
            path: path_str,
            n_requests: None,
        })
    }
    
    /// Parse an operation string
    fn parse_operation(op_str: &str) -> Operation {
        match op_str.to_lowercase().as_str() {
            "get" | "read" => Operation::Get,
            "set" | "write" => Operation::Set,
            "delete" | "del" => Operation::Delete,
            "add" => Operation::Add,
            "replace" => Operation::Replace,
            _ => Operation::Get, // Default
        }
    }
}

impl TraceReader for CsvTraceReader {
    fn n_requests(&self) -> Option<u64> {
        self.n_requests
    }
    
    fn reset(&mut self) -> Result<()> {
        let file = File::open(&self.path)?;
        self.reader = ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(file);
        Ok(())
    }
}

impl Iterator for CsvTraceReader {
    type Item = Result<Request>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.records().next() {
            Some(Ok(record)) => {
                // Parse required fields
                let clock_time = match record.get(0).and_then(|s| s.parse().ok()) {
                    Some(t) => t,
                    None => return Some(Err(TraceError::MissingField("timestamp".to_string()))),
                };
                
                let obj_id = match record.get(1).and_then(|s| s.parse().ok()) {
                    Some(id) => id,
                    None => return Some(Err(TraceError::MissingField("obj_id".to_string()))),
                };
                
                let obj_size = match record.get(2).and_then(|s| s.parse().ok()) {
                    Some(size) => size,
                    None => return Some(Err(TraceError::MissingField("obj_size".to_string()))),
                };
                
                // Parse optional fields
                let op = record.get(3).map(Self::parse_operation).unwrap_or(Operation::Get);
                let tenant_id = record.get(4).and_then(|s| s.parse().ok());
                let ttl = record.get(5).and_then(|s| s.parse().ok());
                let next_access_vtime = record.get(6).and_then(|s| s.parse().ok());
                
                Some(Ok(Request {
                    clock_time,
                    obj_id,
                    obj_size,
                    op,
                    tenant_id,
                    ttl,
                    next_access_vtime,
                }))
            }
            Some(Err(e)) => Some(Err(TraceError::from(e))),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_csv_reader_basic() {
        // Create a temporary CSV file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "timestamp,obj_id,obj_size").unwrap();
        writeln!(file, "0,1,100").unwrap();
        writeln!(file, "1,2,200").unwrap();
        writeln!(file, "2,3,300").unwrap();
        file.flush().unwrap();
        
        let mut reader = CsvTraceReader::new(file.path()).unwrap();
        
        // Read first request
        let req1 = reader.next().unwrap().unwrap();
        assert_eq!(req1.clock_time, 0);
        assert_eq!(req1.obj_id, 1);
        assert_eq!(req1.obj_size, 100);
        
        // Read second request
        let req2 = reader.next().unwrap().unwrap();
        assert_eq!(req2.clock_time, 1);
        assert_eq!(req2.obj_id, 2);
        assert_eq!(req2.obj_size, 200);
        
        // Read third request
        let req3 = reader.next().unwrap().unwrap();
        assert_eq!(req3.clock_time, 2);
        assert_eq!(req3.obj_id, 3);
        assert_eq!(req3.obj_size, 300);
        
        // Should be no more requests
        assert!(reader.next().is_none());
    }
    
    #[test]
    fn test_csv_reader_with_operation() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "timestamp,obj_id,obj_size,operation").unwrap();
        writeln!(file, "0,1,100,get").unwrap();
        writeln!(file, "1,2,200,set").unwrap();
        writeln!(file, "2,3,300,delete").unwrap();
        file.flush().unwrap();
        
        let mut reader = CsvTraceReader::new(file.path()).unwrap();
        
        let req1 = reader.next().unwrap().unwrap();
        assert_eq!(req1.op, Operation::Get);
        
        let req2 = reader.next().unwrap().unwrap();
        assert_eq!(req2.op, Operation::Set);
        
        let req3 = reader.next().unwrap().unwrap();
        assert_eq!(req3.op, Operation::Delete);
    }
    
    #[test]
    fn test_csv_reader_reset() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "timestamp,obj_id,obj_size").unwrap();
        writeln!(file, "0,1,100").unwrap();
        writeln!(file, "1,2,200").unwrap();
        file.flush().unwrap();
        
        let mut reader = CsvTraceReader::new(file.path()).unwrap();
        
        // Read all requests
        assert!(reader.next().is_some());
        assert!(reader.next().is_some());
        assert!(reader.next().is_none());
        
        // Reset and read again
        reader.reset().unwrap();
        assert!(reader.next().is_some());
        assert!(reader.next().is_some());
        assert!(reader.next().is_none());
    }
}
