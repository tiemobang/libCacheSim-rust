// libCacheSim-rust: A Rust implementation of libCacheSim
//
// This is a Rust translation of the libCacheSim C library for cache simulation.
// The original C library can be found at: https://github.com/1a1a11a/libCacheSim

use std::collections::{HashMap, VecDeque};

/// Object ID type
pub type ObjId = u64;

/// Object size type
pub type ObjSize = u64;

/// Request structure representing a cache access
#[derive(Debug, Clone)]
pub struct Request {
    pub obj_id: ObjId,
    pub obj_size: ObjSize,
    pub timestamp: u64,
}

impl Request {
    pub fn new(obj_id: ObjId, obj_size: ObjSize) -> Self {
        Request {
            obj_id,
            obj_size,
            timestamp: 0,
        }
    }
}

/// Cache trait defining the interface for all cache eviction algorithms
pub trait Cache {
    /// Check if an object exists in the cache (get operation)
    /// Returns true if the object was found (cache hit), false otherwise (cache miss)
    fn get(&mut self, req: &Request) -> bool;
    
    /// Insert an object into the cache
    fn insert(&mut self, req: &Request);
    
    /// Evict an object from the cache
    fn evict(&mut self) -> Option<ObjId>;
    
    /// Remove a specific object from the cache
    fn remove(&mut self, obj_id: ObjId) -> bool;
    
    /// Get the current size of the cache in bytes
    fn get_occupied_byte(&self) -> u64;
    
    /// Get the number of objects in the cache
    fn get_n_obj(&self) -> usize;
    
    /// Check if an object can be inserted
    fn can_insert(&self, obj_size: ObjSize) -> bool;
}

/// FIFO (First-In-First-Out) cache implementation
pub struct FifoCache {
    cache_size: u64,
    occupied_size: u64,
    objects: HashMap<ObjId, ObjSize>,
    queue: VecDeque<ObjId>,
}

impl FifoCache {
    pub fn new(cache_size: u64) -> Self {
        FifoCache {
            cache_size,
            occupied_size: 0,
            objects: HashMap::new(),
            queue: VecDeque::new(),
        }
    }
}

impl Cache for FifoCache {
    fn get(&mut self, req: &Request) -> bool {
        if self.objects.contains_key(&req.obj_id) {
            // Cache hit
            true
        } else {
            // Cache miss - need to insert
            while !self.can_insert(req.obj_size) {
                self.evict();
            }
            self.insert(req);
            false
        }
    }
    
    fn insert(&mut self, req: &Request) {
        if !self.objects.contains_key(&req.obj_id) {
            self.objects.insert(req.obj_id, req.obj_size);
            self.queue.push_back(req.obj_id);
            self.occupied_size += req.obj_size;
        }
    }
    
    fn evict(&mut self) -> Option<ObjId> {
        if let Some(obj_id) = self.queue.pop_front() {
            if let Some(obj_size) = self.objects.remove(&obj_id) {
                self.occupied_size -= obj_size;
                return Some(obj_id);
            }
        }
        None
    }
    
    fn remove(&mut self, obj_id: ObjId) -> bool {
        if let Some(obj_size) = self.objects.remove(&obj_id) {
            self.occupied_size -= obj_size;
            // Remove from queue
            if let Some(pos) = self.queue.iter().position(|&id| id == obj_id) {
                self.queue.remove(pos);
            }
            true
        } else {
            false
        }
    }
    
    fn get_occupied_byte(&self) -> u64 {
        self.occupied_size
    }
    
    fn get_n_obj(&self) -> usize {
        self.objects.len()
    }
    
    fn can_insert(&self, obj_size: ObjSize) -> bool {
        self.occupied_size + obj_size <= self.cache_size
    }
}

/// LRU (Least Recently Used) cache implementation
pub struct LruCache {
    cache_size: u64,
    occupied_size: u64,
    objects: HashMap<ObjId, ObjSize>,
    access_order: VecDeque<ObjId>,
}

impl LruCache {
    pub fn new(cache_size: u64) -> Self {
        LruCache {
            cache_size,
            occupied_size: 0,
            objects: HashMap::new(),
            access_order: VecDeque::new(),
        }
    }
}

impl Cache for LruCache {
    fn get(&mut self, req: &Request) -> bool {
        if self.objects.contains_key(&req.obj_id) {
            // Cache hit - move to back (most recently used)
            if let Some(pos) = self.access_order.iter().position(|&id| id == req.obj_id) {
                self.access_order.remove(pos);
                self.access_order.push_back(req.obj_id);
            }
            true
        } else {
            // Cache miss - need to insert
            while !self.can_insert(req.obj_size) {
                self.evict();
            }
            self.insert(req);
            false
        }
    }
    
    fn insert(&mut self, req: &Request) {
        if !self.objects.contains_key(&req.obj_id) {
            self.objects.insert(req.obj_id, req.obj_size);
            self.access_order.push_back(req.obj_id);
            self.occupied_size += req.obj_size;
        }
    }
    
    fn evict(&mut self) -> Option<ObjId> {
        if let Some(obj_id) = self.access_order.pop_front() {
            if let Some(obj_size) = self.objects.remove(&obj_id) {
                self.occupied_size -= obj_size;
                return Some(obj_id);
            }
        }
        None
    }
    
    fn remove(&mut self, obj_id: ObjId) -> bool {
        if let Some(obj_size) = self.objects.remove(&obj_id) {
            self.occupied_size -= obj_size;
            if let Some(pos) = self.access_order.iter().position(|&id| id == obj_id) {
                self.access_order.remove(pos);
            }
            true
        } else {
            false
        }
    }
    
    fn get_occupied_byte(&self) -> u64 {
        self.occupied_size
    }
    
    fn get_n_obj(&self) -> usize {
        self.objects.len()
    }
    
    fn can_insert(&self, obj_size: ObjSize) -> bool {
        self.occupied_size + obj_size <= self.cache_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_basic() {
        let mut cache = FifoCache::new(100);
        
        let req1 = Request::new(1, 10);
        let req2 = Request::new(2, 20);
        let req3 = Request::new(3, 30);
        
        // First access - cache miss
        assert_eq!(cache.get(&req1), false);
        assert_eq!(cache.get_n_obj(), 1);
        assert_eq!(cache.get_occupied_byte(), 10);
        
        // Second access - cache miss
        assert_eq!(cache.get(&req2), false);
        assert_eq!(cache.get_n_obj(), 2);
        assert_eq!(cache.get_occupied_byte(), 30);
        
        // Access req1 again - cache hit
        assert_eq!(cache.get(&req1), true);
        assert_eq!(cache.get_n_obj(), 2);
        
        // Third access - cache miss
        assert_eq!(cache.get(&req3), false);
        assert_eq!(cache.get_n_obj(), 3);
        assert_eq!(cache.get_occupied_byte(), 60);
    }

    #[test]
    fn test_fifo_eviction() {
        let mut cache = FifoCache::new(50);
        
        let req1 = Request::new(1, 20);
        let req2 = Request::new(2, 20);
        let req3 = Request::new(3, 20);
        
        cache.get(&req1); // 20 bytes
        cache.get(&req2); // 40 bytes
        
        // This should evict req1 (first in, first out)
        cache.get(&req3); // Need 20 more bytes, evict req1
        
        assert_eq!(cache.get_n_obj(), 2);
        // Check that req2 and req3 are in cache, req1 is not
        assert!(cache.objects.contains_key(&2));
        assert!(cache.objects.contains_key(&3));
        assert!(!cache.objects.contains_key(&1)); // req1 was evicted
    }

    #[test]
    fn test_lru_basic() {
        let mut cache = LruCache::new(100);
        
        let req1 = Request::new(1, 10);
        let req2 = Request::new(2, 20);
        
        // First access - cache miss
        assert_eq!(cache.get(&req1), false);
        assert_eq!(cache.get_n_obj(), 1);
        
        // Second access - cache miss
        assert_eq!(cache.get(&req2), false);
        assert_eq!(cache.get_n_obj(), 2);
        
        // Access req1 again - cache hit
        assert_eq!(cache.get(&req1), true);
        assert_eq!(cache.get_n_obj(), 2);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(50);
        
        let req1 = Request::new(1, 20);
        let req2 = Request::new(2, 20);
        let req3 = Request::new(3, 20);
        
        cache.get(&req1); // 20 bytes
        cache.get(&req2); // 40 bytes
        cache.get(&req1); // Access req1 again (make it most recently used)
        
        // This should evict req2 (least recently used)
        cache.get(&req3); // Need 20 more bytes, evict req2
        
        assert_eq!(cache.get_n_obj(), 2);
        // Check that req1 and req3 are in cache, req2 is not
        assert!(cache.objects.contains_key(&1));  // req1 still in cache (was accessed recently)
        assert!(!cache.objects.contains_key(&2)); // req2 was evicted
        assert!(cache.objects.contains_key(&3));  // req3 still in cache
    }

    #[test]
    fn test_remove() {
        let mut cache = FifoCache::new(100);
        
        let req1 = Request::new(1, 10);
        let req2 = Request::new(2, 20);
        
        cache.get(&req1);
        cache.get(&req2);
        
        assert_eq!(cache.get_n_obj(), 2);
        assert!(cache.remove(1));
        assert_eq!(cache.get_n_obj(), 1);
        assert_eq!(cache.get_occupied_byte(), 20);
        
        // Try to remove non-existent object
        assert!(!cache.remove(999));
    }
}
