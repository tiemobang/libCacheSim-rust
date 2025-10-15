use serde::{Deserialize, Serialize};

/// Operation type for a cache request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// Read/Get operation
    Get,
    /// Write/Set operation
    Set,
    /// Delete operation
    Delete,
    /// Add operation (insert only if not present)
    Add,
    /// Replace operation (update only if present)
    Replace,
}

impl Default for Operation {
    fn default() -> Self {
        Operation::Get
    }
}

/// Request structure representing a cache access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Timestamp of the request
    pub clock_time: u64,
    
    /// Object identifier
    pub obj_id: u64,
    
    /// Size of the object in bytes
    pub obj_size: u32,
    
    /// Operation type
    pub op: Operation,
    
    /// Tenant/namespace identifier (optional)
    pub tenant_id: Option<u32>,
    
    /// Time-to-live in seconds (optional)
    pub ttl: Option<i32>,
    
    /// Next access time in virtual time (for oracle algorithms)
    pub next_access_vtime: Option<i64>,
}

impl Request {
    /// Create a new request with just object ID and size
    pub fn new(obj_id: u64, obj_size: u32) -> Self {
        Request {
            clock_time: 0,
            obj_id,
            obj_size,
            op: Operation::Get,
            tenant_id: None,
            ttl: None,
            next_access_vtime: None,
        }
    }
    
    /// Create a new request with timestamp
    pub fn with_timestamp(obj_id: u64, obj_size: u32, clock_time: u64) -> Self {
        Request {
            clock_time,
            obj_id,
            obj_size,
            op: Operation::Get,
            tenant_id: None,
            ttl: None,
            next_access_vtime: None,
        }
    }
    
    /// Builder for creating requests with all fields
    pub fn builder() -> RequestBuilder {
        RequestBuilder::default()
    }
}

/// Builder for Request
#[derive(Debug, Default)]
pub struct RequestBuilder {
    clock_time: u64,
    obj_id: u64,
    obj_size: u32,
    op: Operation,
    tenant_id: Option<u32>,
    ttl: Option<i32>,
    next_access_vtime: Option<i64>,
}

impl RequestBuilder {
    pub fn clock_time(mut self, clock_time: u64) -> Self {
        self.clock_time = clock_time;
        self
    }
    
    pub fn obj_id(mut self, obj_id: u64) -> Self {
        self.obj_id = obj_id;
        self
    }
    
    pub fn obj_size(mut self, obj_size: u32) -> Self {
        self.obj_size = obj_size;
        self
    }
    
    pub fn op(mut self, op: Operation) -> Self {
        self.op = op;
        self
    }
    
    pub fn tenant_id(mut self, tenant_id: u32) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }
    
    pub fn ttl(mut self, ttl: i32) -> Self {
        self.ttl = Some(ttl);
        self
    }
    
    pub fn next_access_vtime(mut self, vtime: i64) -> Self {
        self.next_access_vtime = Some(vtime);
        self
    }
    
    pub fn build(self) -> Request {
        Request {
            clock_time: self.clock_time,
            obj_id: self.obj_id,
            obj_size: self.obj_size,
            op: self.op,
            tenant_id: self.tenant_id,
            ttl: self.ttl,
            next_access_vtime: self.next_access_vtime,
        }
    }
}
