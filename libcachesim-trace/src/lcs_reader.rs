//! LCS (libCacheSim) binary trace format reader
//!
//! Supports LCS versions 1, 2, and 3

use crate::error::{Result, TraceError};
use crate::reader::TraceReader;
use libcachesim_core::{Operation, Request};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// LCS format version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LcsVersion {
    /// Version 1: timestamp (u32), obj_id (u64), obj_size (u32), next_access_vtime (i64)
    V1,
    /// Version 2: V1 + operation (u8), tenant (u24)
    V2,
    /// Version 3: V2 with full 64-bit timestamp and size
    V3,
}

/// LCS trace header (simplified)
#[derive(Debug)]
struct LcsHeader {
    version: LcsVersion,
    n_requests: u64,
}

/// LCS trace reader
pub struct LcsTraceReader {
    reader: BufReader<File>,
    path: String,
    header: LcsHeader,
    header_size: u64,
}

impl LcsTraceReader {
    /// Create a new LCS trace reader
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read header
        let header = Self::read_header(&mut reader)?;
        let header_size = 256; // Standard LCS header size

        // Seek to start of data
        reader.seek(SeekFrom::Start(header_size))?;

        Ok(LcsTraceReader {
            reader,
            path: path_str,
            header,
            header_size,
        })
    }

    fn read_header(reader: &mut BufReader<File>) -> Result<LcsHeader> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;

        // Read version (simplified - just read as u64)
        let mut version_bytes = [0u8; 8];
        reader.read_exact(&mut version_bytes)?;
        let version_num = u64::from_le_bytes(version_bytes);

        let version = match version_num {
            1 => LcsVersion::V1,
            2 => LcsVersion::V2,
            3 => LcsVersion::V3,
            v => {
                return Err(TraceError::InvalidFormat(format!(
                    "Unknown LCS version: {}",
                    v
                )))
            }
        };

        // Read n_requests (at offset 16 in header)
        let mut n_req_bytes = [0u8; 8];
        reader.read_exact(&mut n_req_bytes)?;
        let n_requests = u64::from_le_bytes(n_req_bytes);

        // Skip rest of header (240 bytes remaining)
        let mut skip = vec![0u8; 240];
        reader.read_exact(&mut skip)?;

        Ok(LcsHeader {
            version,
            n_requests,
        })
    }

    fn read_request_v1(&mut self) -> Result<Request> {
        let mut buf = [0u8; 24]; // 4 + 8 + 4 + 8
        self.reader.read_exact(&mut buf)?;

        let clock_time = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64;
        let obj_id = u64::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]);
        let obj_size = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let next_access_vtime = i64::from_le_bytes([
            buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
        ]);

        Ok(Request {
            clock_time,
            obj_id,
            obj_size,
            op: Operation::Get,
            tenant_id: None,
            ttl: None,
            next_access_vtime: Some(next_access_vtime),
        })
    }

    fn read_request_v2(&mut self) -> Result<Request> {
        let mut buf = [0u8; 28]; // 4 + 8 + 4 + 4 + 8
        self.reader.read_exact(&mut buf)?;

        let clock_time = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64;
        let obj_id = u64::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]);
        let obj_size = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);

        // op (u8) + tenant (u24) packed as u32
        let op_tenant = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        let op_byte = (op_tenant & 0xFF) as u8;
        let tenant = (op_tenant >> 8) & 0xFFFFFF;

        let next_access_vtime = i64::from_le_bytes([
            buf[20], buf[21], buf[22], buf[23], buf[24], buf[25], buf[26], buf[27],
        ]);

        let op = match op_byte {
            0 => Operation::Get,
            1 => Operation::Set,
            2 => Operation::Delete,
            3 => Operation::Add,
            4 => Operation::Replace,
            _ => Operation::Get,
        };

        Ok(Request {
            clock_time,
            obj_id,
            obj_size,
            op,
            tenant_id: Some(tenant),
            ttl: None,
            next_access_vtime: Some(next_access_vtime),
        })
    }

    fn read_request_v3(&mut self) -> Result<Request> {
        let mut buf = [0u8; 36]; // 4 + 8 + 8 + 4 + 4 + 8
        self.reader.read_exact(&mut buf)?;

        let clock_time = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64;
        let obj_id = u64::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]);
        let obj_size = u64::from_le_bytes([
            buf[12], buf[13], buf[14], buf[15], buf[16], buf[17], buf[18], buf[19],
        ]) as u32; // Truncate to u32 for compatibility

        let op_tenant = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);
        let op_byte = (op_tenant & 0xFF) as u8;
        let tenant = (op_tenant >> 8) & 0xFFFFFF;

        let ttl = i32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);

        let next_access_vtime = i64::from_le_bytes([
            buf[28], buf[29], buf[30], buf[31], buf[32], buf[33], buf[34], buf[35],
        ]);

        let op = match op_byte {
            0 => Operation::Get,
            1 => Operation::Set,
            2 => Operation::Delete,
            3 => Operation::Add,
            4 => Operation::Replace,
            _ => Operation::Get,
        };

        Ok(Request {
            clock_time,
            obj_id,
            obj_size,
            op,
            tenant_id: Some(tenant),
            ttl: Some(ttl),
            next_access_vtime: Some(next_access_vtime),
        })
    }
}

impl TraceReader for LcsTraceReader {
    fn n_requests(&self) -> Option<u64> {
        Some(self.header.n_requests)
    }

    fn reset(&mut self) -> Result<()> {
        self.reader.seek(SeekFrom::Start(self.header_size))?;
        Ok(())
    }
}

impl Iterator for LcsTraceReader {
    type Item = Result<Request>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.header.version {
            LcsVersion::V1 => self.read_request_v1(),
            LcsVersion::V2 => self.read_request_v2(),
            LcsVersion::V3 => self.read_request_v3(),
        };

        match result {
            Ok(req) => Some(Ok(req)),
            Err(TraceError::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_lcs_v1_trace() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();

        // Write header
        file.write_all(b"LCSTrace").unwrap(); // Magic
        file.write_all(&1u64.to_le_bytes()).unwrap(); // Version
        file.write_all(&2u64.to_le_bytes()).unwrap(); // N requests
        file.write_all(&vec![0u8; 240]).unwrap(); // Rest of header

        // Write request 1
        file.write_all(&0u32.to_le_bytes()).unwrap(); // clock_time
        file.write_all(&1u64.to_le_bytes()).unwrap(); // obj_id
        file.write_all(&100u32.to_le_bytes()).unwrap(); // obj_size
        file.write_all(&(-1i64).to_le_bytes()).unwrap(); // next_access_vtime

        // Write request 2
        file.write_all(&1u32.to_le_bytes()).unwrap(); // clock_time
        file.write_all(&2u64.to_le_bytes()).unwrap(); // obj_id
        file.write_all(&200u32.to_le_bytes()).unwrap(); // obj_size
        file.write_all(&(-1i64).to_le_bytes()).unwrap(); // next_access_vtime

        file.flush().unwrap();
        file
    }

    #[test]
    fn test_lcs_reader_v1() {
        let file = create_lcs_v1_trace();
        let mut reader = LcsTraceReader::new(file.path()).unwrap();

        assert_eq!(reader.n_requests(), Some(2));

        // Just verify we can read two requests
        let req1 = reader.next();
        assert!(req1.is_some());
        assert!(req1.unwrap().is_ok());

        let req2 = reader.next();
        assert!(req2.is_some());
        assert!(req2.unwrap().is_ok());

        // No more requests
        assert!(reader.next().is_none());
    }

    #[test]
    fn test_lcs_reader_reset() {
        let file = create_lcs_v1_trace();
        let mut reader = LcsTraceReader::new(file.path()).unwrap();

        // Read all
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
