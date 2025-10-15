//! MRU (Most Recently Used) cache implementation
//!
//! MRU evicts the most recently accessed object when the cache is full.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};
use std::collections::VecDeque;

/// MRU (Most Recently Used) cache implementation
///
/// Evicts the most recently accessed object when the cache is full.
/// This is the opposite of LRU and is useful in certain access patterns.
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::MruCache;
///
/// let mut cache = MruCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct MruCache {
    capacity: u64,
    used: u64,
    objects: HashMap<ObjectId, u32>,
    stack: VecDeque<ObjectId>, // Most recent at back
    stats: CacheStats,
}

impl MruCache {
    /// Create a new MRU cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        MruCache {
            capacity,
            used: 0,
            objects: HashMap::new(),
            stack: VecDeque::new(),
            stats: CacheStats::new(),
        }
    }
    
    /// Move an object to the back (most recently used position)
    fn touch(&mut self, obj_id: ObjectId) {
        // Remove from current position
        if let Some(pos) = self.stack.iter().position(|&id| id == obj_id) {
            self.stack.remove(pos);
        }
        // Add to back (most recent)
        self.stack.push_back(obj_id);
    }
}

impl Cache for MruCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();
        
        if let Some(&obj_size) = self.objects.get(&req.obj_id) {
            self.stats.inc_hit();
            self.touch(req.obj_id);
            CacheResult::Hit { obj_size }
        } else {
            self.stats.inc_miss();
            self.insert(req);
            CacheResult::Miss
        }
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        // Check if object already exists
        if self.objects.contains_key(&req.obj_id) {
            self.touch(req.obj_id);
            return InsertResult::Inserted;
        }
        
        // Evict objects if necessary to make room
        let mut evicted_id = None;
        while self.used + req.obj_size as u64 > self.capacity && !self.stack.is_empty() {
            if let Some(evict_id) = self.evict() {
                evicted_id = Some(evict_id);
            }
        }
        
        // Check if object can fit
        if req.obj_size as u64 > self.capacity {
            return InsertResult::Rejected;
        }
        
        // Insert the object
        self.objects.insert(req.obj_id, req.obj_size);
        self.stack.push_back(req.obj_id);
        self.used += req.obj_size as u64;
        self.stats.inc_insert();
        
        if let Some(evicted) = evicted_id {
            InsertResult::Evicted {
                evicted_id: evicted,
            }
        } else {
            InsertResult::Inserted
        }
    }
    
    fn evict(&mut self) -> Option<ObjectId> {
        // Evict from back (most recently used)
        if let Some(obj_id) = self.stack.pop_back() {
            if let Some(obj_size) = self.objects.remove(&obj_id) {
                self.used -= obj_size as u64;
                self.stats.inc_evict();
                return Some(obj_id);
            }
        }
        None
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(obj_size) = self.objects.remove(&obj_id) {
            self.used -= obj_size as u64;
            // Remove from stack
            if let Some(pos) = self.stack.iter().position(|&id| id == obj_id) {
                self.stack.remove(pos);
            }
            true
        } else {
            false
        }
    }
    
    fn clear(&mut self) {
        self.objects.clear();
        self.stack.clear();
        self.used = 0;
    }
    
    fn size(&self) -> u64 {
        self.used
    }
    
    fn capacity(&self) -> u64 {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.objects.len()
    }
    
    fn stats(&self) -> &CacheStats {
        &self.stats
    }
    
    fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mru_basic() {
        let mut cache = MruCache::new(300);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        // First access - miss
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);
        
        // Second access - hit
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.len(), 1);
        
        // Add another object
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.len(), 2);
    }
    
    #[test]
    fn test_mru_eviction() {
        let mut cache = MruCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        // Fill cache
        cache.get(&req1);
        cache.get(&req2);
        assert_eq!(cache.len(), 2);
        
        // Access req1 to make it most recent
        cache.get(&req1);
        
        // This should evict object 1 (most recently used)
        cache.get(&req3);
        assert!(cache.len() <= 2);
        
        // Objects 2 and 3 should still be there
        assert_eq!(cache.get(&req2), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.get(&req3), CacheResult::Hit { obj_size: 100 });
    }
    
    #[test]
    fn test_mru_stats() {
        let mut cache = MruCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        cache.get(&req1); // miss
        cache.get(&req1); // hit
        cache.get(&req2); // miss
        cache.get(&req2); // hit
        
        let stats = cache.stats();
        assert_eq!(stats.n_req(), 4);
        assert_eq!(stats.n_hit(), 2);
        assert_eq!(stats.n_miss(), 2);
        assert_eq!(stats.hit_ratio(), 0.5);
    }
}
