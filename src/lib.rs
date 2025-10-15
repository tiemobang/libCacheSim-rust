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
    
    pub fn with_timestamp(obj_id: ObjId, obj_size: ObjSize, timestamp: u64) -> Self {
        Request {
            obj_id,
            obj_size,
            timestamp,
        }
    }
}

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub n_req: u64,
    pub n_hit: u64,
    pub n_miss: u64,
    pub n_evict: u64,
}

impl CacheStats {
    pub fn miss_ratio(&self) -> f64 {
        if self.n_req == 0 {
            0.0
        } else {
            self.n_miss as f64 / self.n_req as f64
        }
    }
    
    pub fn hit_ratio(&self) -> f64 {
        1.0 - self.miss_ratio()
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
    
    /// Get cache statistics
    fn get_stats(&self) -> &CacheStats;
    
    /// Reset cache statistics
    fn reset_stats(&mut self);
}

/// FIFO (First-In-First-Out) cache implementation
pub struct FifoCache {
    cache_size: u64,
    occupied_size: u64,
    objects: HashMap<ObjId, ObjSize>,
    queue: VecDeque<ObjId>,
    stats: CacheStats,
}

impl FifoCache {
    pub fn new(cache_size: u64) -> Self {
        FifoCache {
            cache_size,
            occupied_size: 0,
            objects: HashMap::new(),
            queue: VecDeque::new(),
            stats: CacheStats::default(),
        }
    }
}

impl Cache for FifoCache {
    fn get(&mut self, req: &Request) -> bool {
        self.stats.n_req += 1;
        
        if self.objects.contains_key(&req.obj_id) {
            // Cache hit
            self.stats.n_hit += 1;
            true
        } else {
            // Cache miss - need to insert
            self.stats.n_miss += 1;
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
                self.stats.n_evict += 1;
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
    
    fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
    
    fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }
}

/// LRU (Least Recently Used) cache implementation
pub struct LruCache {
    cache_size: u64,
    occupied_size: u64,
    objects: HashMap<ObjId, ObjSize>,
    access_order: VecDeque<ObjId>,
    stats: CacheStats,
}

impl LruCache {
    pub fn new(cache_size: u64) -> Self {
        LruCache {
            cache_size,
            occupied_size: 0,
            objects: HashMap::new(),
            access_order: VecDeque::new(),
            stats: CacheStats::default(),
        }
    }
}

impl Cache for LruCache {
    fn get(&mut self, req: &Request) -> bool {
        self.stats.n_req += 1;
        
        if self.objects.contains_key(&req.obj_id) {
            // Cache hit - move to back (most recently used)
            if let Some(pos) = self.access_order.iter().position(|&id| id == req.obj_id) {
                self.access_order.remove(pos);
                self.access_order.push_back(req.obj_id);
            }
            self.stats.n_hit += 1;
            true
        } else {
            // Cache miss - need to insert
            self.stats.n_miss += 1;
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
                self.stats.n_evict += 1;
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
    
    fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
    
    fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }
}

/// Clock cache implementation (approximation of LRU using reference bits)
pub struct ClockCache {
    cache_size: u64,
    occupied_size: u64,
    objects: HashMap<ObjId, (ObjSize, bool)>, // (size, reference_bit)
    circular_list: Vec<ObjId>,
    hand: usize, // Clock hand position
    stats: CacheStats,
}

impl ClockCache {
    pub fn new(cache_size: u64) -> Self {
        ClockCache {
            cache_size,
            occupied_size: 0,
            objects: HashMap::new(),
            circular_list: Vec::new(),
            hand: 0,
            stats: CacheStats::default(),
        }
    }
}

impl Cache for ClockCache {
    fn get(&mut self, req: &Request) -> bool {
        self.stats.n_req += 1;
        
        if let Some((_, ref_bit)) = self.objects.get_mut(&req.obj_id) {
            // Cache hit - set reference bit
            *ref_bit = true;
            self.stats.n_hit += 1;
            true
        } else {
            // Cache miss - need to insert
            self.stats.n_miss += 1;
            while !self.can_insert(req.obj_size) {
                self.evict();
            }
            self.insert(req);
            false
        }
    }
    
    fn insert(&mut self, req: &Request) {
        if !self.objects.contains_key(&req.obj_id) {
            self.objects.insert(req.obj_id, (req.obj_size, false));
            self.circular_list.push(req.obj_id);
            self.occupied_size += req.obj_size;
        }
    }
    
    fn evict(&mut self) -> Option<ObjId> {
        if self.circular_list.is_empty() {
            return None;
        }
        
        // Sweep clock hand looking for object with reference bit = 0
        loop {
            if self.hand >= self.circular_list.len() {
                self.hand = 0;
            }
            
            let obj_id = self.circular_list[self.hand];
            
            if let Some((obj_size, ref_bit)) = self.objects.get_mut(&obj_id) {
                if *ref_bit {
                    // Reference bit is set, clear it and move to next
                    *ref_bit = false;
                    self.hand += 1;
                } else {
                    // Reference bit is clear, evict this object
                    let size = *obj_size;
                    self.objects.remove(&obj_id);
                    self.circular_list.remove(self.hand);
                    self.occupied_size -= size;
                    self.stats.n_evict += 1;
                    
                    // Don't increment hand since we removed an element
                    if self.hand >= self.circular_list.len() && !self.circular_list.is_empty() {
                        self.hand = 0;
                    }
                    
                    return Some(obj_id);
                }
            } else {
                // Object not found, skip it
                self.hand += 1;
            }
        }
    }
    
    fn remove(&mut self, obj_id: ObjId) -> bool {
        if let Some((obj_size, _)) = self.objects.remove(&obj_id) {
            self.occupied_size -= obj_size;
            if let Some(pos) = self.circular_list.iter().position(|&id| id == obj_id) {
                self.circular_list.remove(pos);
                // Adjust hand if necessary
                if self.hand > pos {
                    self.hand -= 1;
                } else if self.hand >= self.circular_list.len() && !self.circular_list.is_empty() {
                    self.hand = 0;
                }
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
    
    fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
    
    fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }
}

/// Simulate a workload and return cache statistics
pub fn simulate_cache<C: Cache>(cache: &mut C, requests: &[Request]) -> CacheStats {
    cache.reset_stats();
    for req in requests {
        cache.get(req);
    }
    cache.get_stats().clone()
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
    
    #[test]
    fn test_cache_stats() {
        let mut cache = FifoCache::new(100);
        
        let req1 = Request::new(1, 10);
        let req2 = Request::new(2, 20);
        
        cache.get(&req1); // miss
        cache.get(&req1); // hit
        cache.get(&req2); // miss
        cache.get(&req1); // hit
        
        let stats = cache.get_stats();
        assert_eq!(stats.n_req, 4);
        assert_eq!(stats.n_hit, 2);
        assert_eq!(stats.n_miss, 2);
        assert_eq!(stats.miss_ratio(), 0.5);
        assert_eq!(stats.hit_ratio(), 0.5);
    }
    
    #[test]
    fn test_clock_basic() {
        let mut cache = ClockCache::new(100);
        
        let req1 = Request::new(1, 10);
        let req2 = Request::new(2, 20);
        
        // First access - cache miss
        assert_eq!(cache.get(&req1), false);
        assert_eq!(cache.get_n_obj(), 1);
        
        // Second access - cache miss
        assert_eq!(cache.get(&req2), false);
        assert_eq!(cache.get_n_obj(), 2);
        
        // Access req1 again - cache hit (sets reference bit)
        assert_eq!(cache.get(&req1), true);
        assert_eq!(cache.get_n_obj(), 2);
    }
    
    #[test]
    fn test_clock_eviction() {
        let mut cache = ClockCache::new(50);
        
        let req1 = Request::new(1, 20);
        let req2 = Request::new(2, 20);
        let req3 = Request::new(3, 20);
        
        cache.get(&req1); // 20 bytes, ref bit = 0
        cache.get(&req2); // 40 bytes, ref bit = 0
        cache.get(&req1); // Access req1 again, ref bit = 1
        
        // This should skip req1 (ref bit = 1), clear it, then evict req2 (ref bit = 0)
        cache.get(&req3); // Need 20 more bytes
        
        assert_eq!(cache.get_n_obj(), 2);
        assert!(cache.objects.contains_key(&1));  // req1 still in cache
        assert!(!cache.objects.contains_key(&2)); // req2 was evicted
        assert!(cache.objects.contains_key(&3));  // req3 in cache
    }
    
    #[test]
    fn test_simulate_workload() {
        let mut cache = LruCache::new(100);
        
        let requests = vec![
            Request::new(1, 10),
            Request::new(2, 20),
            Request::new(1, 10), // hit
            Request::new(3, 30),
            Request::new(2, 20), // hit
            Request::new(1, 10), // hit
        ];
        
        let stats = simulate_cache(&mut cache, &requests);
        assert_eq!(stats.n_req, 6);
        assert_eq!(stats.n_hit, 3);
        assert_eq!(stats.n_miss, 3);
        assert_eq!(stats.miss_ratio(), 0.5);
    }
}
