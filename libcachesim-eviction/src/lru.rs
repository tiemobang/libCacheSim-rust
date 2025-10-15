//! LRU (Least Recently Used) cache implementation
//!
//! LRU evicts the least recently accessed object when the cache is full.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};
use std::collections::VecDeque;

/// LRU (Least Recently Used) cache implementation
///
/// Evicts the least recently accessed object when the cache is full.
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::LruCache;
///
/// let mut cache = LruCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct LruCache {
    capacity: u64,
    used: u64,
    objects: HashMap<ObjectId, u32>,
    queue: VecDeque<ObjectId>, // Most recent at back
    stats: CacheStats,
}

impl LruCache {
    /// Create a new LRU cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        LruCache {
            capacity,
            used: 0,
            objects: HashMap::new(),
            queue: VecDeque::new(),
            stats: CacheStats::new(),
        }
    }
    
    /// Move an object to the back (most recently used position)
    fn touch(&mut self, obj_id: ObjectId) {
        // Remove from current position (O(n) operation)
        if let Some(pos) = self.queue.iter().position(|&id| id == obj_id) {
            self.queue.remove(pos);
        }
        // Add to back (most recent)
        self.queue.push_back(obj_id);
    }
}

impl Cache for LruCache {
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
        while self.used + req.obj_size as u64 > self.capacity && !self.queue.is_empty() {
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
        self.queue.push_back(req.obj_id);
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
        // Evict from front (least recently used)
        if let Some(obj_id) = self.queue.pop_front() {
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
            // Remove from queue
            if let Some(pos) = self.queue.iter().position(|&id| id == obj_id) {
                self.queue.remove(pos);
            }
            true
        } else {
            false
        }
    }
    
    fn clear(&mut self) {
        self.objects.clear();
        self.queue.clear();
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
    fn test_lru_basic() {
        let mut cache = LruCache::new(300);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        // First access - miss
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);
        
        // Second access - hit
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.len(), 1);
        
        // Add more objects
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.get(&req3), CacheResult::Miss);
        assert_eq!(cache.len(), 3);
    }
    
    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        // Fill cache
        cache.get(&req1);
        cache.get(&req2);
        assert_eq!(cache.len(), 2);
        
        // Access req1 to make it more recent than req2
        cache.get(&req1);
        
        // This should evict object 2 (least recently used)
        cache.get(&req3);
        // After eviction and insertion, we should have req1 and req3
        assert!(cache.len() <= 2);
        
        // Objects 1 and 3 should still be there
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.get(&req3), CacheResult::Hit { obj_size: 100 });
    }
    
    #[test]
    fn test_lru_stats() {
        let mut cache = LruCache::new(200);
        
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
