use crate::{CacheStats, ObjectId, Request};

/// Result of a cache get operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheResult {
    /// Object was found in cache
    Hit { obj_size: u32 },
    /// Object was not found in cache
    Miss,
}

/// Result of a cache insert operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertResult {
    /// Object was inserted successfully
    Inserted,
    /// Object was inserted and another object was evicted
    Evicted { evicted_id: ObjectId },
    /// Object was rejected (e.g., too large)
    Rejected,
}

/// Cache trait defining the interface for all cache eviction algorithms
pub trait Cache {
    /// Check if an object exists in the cache (get operation)
    /// Returns CacheResult::Hit if found, CacheResult::Miss otherwise
    fn get(&mut self, req: &Request) -> CacheResult;
    
    /// Insert an object into the cache
    /// Returns InsertResult indicating what happened
    fn insert(&mut self, req: &Request) -> InsertResult;
    
    /// Evict an object from the cache
    /// Returns the ID of the evicted object, or None if cache is empty
    fn evict(&mut self) -> Option<ObjectId>;
    
    /// Remove a specific object from the cache
    /// Returns true if the object was removed, false if it wasn't in cache
    fn remove(&mut self, obj_id: ObjectId) -> bool;
    
    /// Clear all objects from the cache
    fn clear(&mut self);
    
    /// Get the current size of the cache in bytes
    fn size(&self) -> u64;
    
    /// Get the cache capacity in bytes
    fn capacity(&self) -> u64;
    
    /// Get the number of objects in the cache
    fn len(&self) -> usize;
    
    /// Check if the cache is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Check if an object of the given size can be inserted
    fn can_insert(&self, obj_size: u32) -> bool {
        obj_size as u64 <= self.capacity()
    }
    
    /// Get cache statistics
    fn stats(&self) -> &CacheStats;
    
    /// Reset cache statistics
    fn reset_stats(&mut self);
}
