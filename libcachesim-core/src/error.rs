use thiserror::Error;

/// Errors that can occur during cache operations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CacheError {
    /// Object is too large to fit in cache
    #[error("Object too large: size {size} exceeds capacity {capacity}")]
    ObjectTooLarge { size: u64, capacity: u64 },
    
    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    /// Object not found
    #[error("Object not found: {0}")]
    ObjectNotFound(u64),
    
    /// Cache is full and cannot evict
    #[error("Cache is full and no objects can be evicted")]
    CacheFull,
}
