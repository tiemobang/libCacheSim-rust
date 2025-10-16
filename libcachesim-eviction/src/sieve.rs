//! Sieve cache eviction algorithm
//!
//! Sieve is a simple and efficient eviction algorithm using a visited bit
//! to achieve LRU-like behavior with lower overhead.

use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Request};
use std::collections::{HashMap, VecDeque};

struct CacheEntry {
    size: ObjectSize,
    visited: bool,
}

/// Sieve cache implementation
pub struct SieveCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    stats: CacheStats,
    queue: VecDeque<ObjectId>,
    map: HashMap<ObjectId, CacheEntry>,
    hand: usize, // Eviction pointer
}

impl SieveCache {
    /// Create a new Sieve cache
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity,
            current_size: 0,
            stats: CacheStats::new(),
            queue: VecDeque::new(),
            map: HashMap::new(),
            hand: 0,
        }
    }
    
    fn evict(&mut self) {
        while self.current_size > self.capacity {
            if self.queue.is_empty() {
                break;
            }
            
            // Wrap around if needed
            if self.hand >= self.queue.len() {
                self.hand = 0;
            }
            
            let obj_id = self.queue[self.hand];
            
            if let Some(entry) = self.map.get_mut(&obj_id) {
                if entry.visited {
                    // Clear visited bit and move to next
                    entry.visited = false;
                    self.hand += 1;
                } else {
                    // Evict this entry
                    let size = entry.size;
                    self.queue.remove(self.hand);
                    self.map.remove(&obj_id);
                    self.current_size -= size;
                    self.stats.increment_evictions();
                    
                    // Don't increment hand since we removed an element
                    if self.hand >= self.queue.len() && !self.queue.is_empty() {
                        self.hand = 0;
                    }
                }
            } else {
                // Entry not found, move to next
                self.hand += 1;
            }
        }
    }
}

impl Cache for SieveCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        let obj_id = req.obj_id;
        
        if let Some(entry) = self.map.get_mut(&obj_id) {
            self.stats.increment_hits();
            entry.visited = true; // Set visited bit
            return CacheResult::Hit { size: entry.size };
        }
        
        self.stats.increment_misses();
        CacheResult::Miss
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        let obj_id = req.obj_id;
        let obj_size = req.obj_size;
        
        // Update if already present
        if let Some(entry) = self.map.get_mut(&obj_id) {
            entry.visited = true;
            return InsertResult::Admitted;
        }
        
        // Insert new object
        let entry = CacheEntry {
            size: obj_size,
            visited: false,
        };
        
        self.queue.push_back(obj_id);
        self.map.insert(obj_id, entry);
        self.current_size += obj_size;
        
        // Evict if necessary
        self.evict();
        
        InsertResult::Admitted
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(entry) = self.map.remove(&obj_id) {
            if let Some(pos) = self.queue.iter().position(|&id| id == obj_id) {
                self.queue.remove(pos);
                self.current_size -= entry.size;
                
                // Adjust hand if needed
                if pos < self.hand {
                    self.hand -= 1;
                } else if self.hand >= self.queue.len() && !self.queue.is_empty() {
                    self.hand = 0;
                }
                
                return true;
            }
        }
        false
    }
    
    fn clear(&mut self) {
        self.queue.clear();
        self.map.clear();
        self.current_size = 0;
        self.hand = 0;
    }
    
    fn capacity(&self) -> ObjectSize {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.map.len()
    }
    
    fn current_size(&self) -> ObjectSize {
        self.current_size
    }
    
    fn stats(&self) -> &CacheStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sieve_basic() {
        let mut cache = SieveCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        let req2 = Request::builder().obj_id(2).obj_size(100).build();
        
        assert!(matches!(cache.get(&req1), CacheResult::Miss));
        cache.insert(&req1);
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
        
        assert!(matches!(cache.get(&req2), CacheResult::Miss));
        cache.insert(&req2);
        assert!(matches!(cache.get(&req2), CacheResult::Hit { .. }));
    }
    
    #[test]
    fn test_sieve_eviction() {
        let mut cache = SieveCache::new(300);
        
        // Fill cache
        for i in 1..=10 {
            let req = Request::builder().obj_id(i).obj_size(50).build();
            cache.insert(&req);
        }
        
        assert!(cache.stats().evictions() > 0);
        assert!(cache.current_size() <= cache.capacity());
    }
    
    #[test]
    fn test_sieve_visited_bit() {
        let mut cache = SieveCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        let req2 = Request::builder().obj_id(2).obj_size(100).build();
        let req3 = Request::builder().obj_id(3).obj_size(100).build();
        let req4 = Request::builder().obj_id(4).obj_size(100).build();
        
        cache.insert(&req1);
        cache.insert(&req2);
        cache.insert(&req3);
        
        // Access req1 to set visited bit
        cache.get(&req1);
        
        // Insert req4, should evict an unvisited object
        cache.insert(&req4);
        
        // req1 should still be in cache due to visited bit
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
    }
}
