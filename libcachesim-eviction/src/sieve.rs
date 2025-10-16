//! Sieve cache eviction algorithm
//!
//! Sieve is a simple, efficient eviction algorithm that uses a single pass
//! with a "visited" bit, similar to Clock but with a different policy.

use libcachesim_core::{
    Cache, CacheResult, InsertResult, CacheError, ObjectId, ObjectSize, Request, CacheStats,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
struct SieveEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
    visited: bool,
}

/// Sieve cache implementation
pub struct SieveCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    queue: VecDeque<ObjectId>,
    entries: HashMap<ObjectId, SieveEntry>,
    hand: usize, // Clock hand position
    stats: CacheStats,
}

impl SieveCache {
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity,
            current_size: 0,
            queue: VecDeque::new(),
            entries: HashMap::new(),
            hand: 0,
            stats: CacheStats::new(),
        }
    }
    
    fn evict(&mut self) {
        let queue_len = self.queue.len();
        if queue_len == 0 {
            return;
        }
        
        // Start from hand position and sweep
        let mut checked = 0;
        
        while checked < queue_len {
            if self.hand >= self.queue.len() {
                self.hand = 0;
            }
            
            if let Some(&obj_id) = self.queue.get(self.hand) {
                if let Some(entry) = self.entries.get_mut(&obj_id) {
                    if entry.visited {
                        // Clear visited bit and move to next
                        entry.visited = false;
                        self.hand += 1;
                    } else {
                        // Evict this entry
                        let obj_size = entry.obj_size;
                        self.queue.remove(self.hand);
                        self.entries.remove(&obj_id);
                        self.current_size -= obj_size;
                        self.stats.inc_evict();
                        
                        // Don't increment hand, next element is now at this position
                        return;
                    }
                } else {
                    // Entry not found, remove from queue
                    self.queue.remove(self.hand);
                }
            }
            
            checked += 1;
        }
    }
}

impl Cache for SieveCache {
    fn get(&mut self, req: &Request) -> Result<CacheResult, CacheError> {
        self.stats.inc_req();
        
        if let Some(entry) = self.entries.get_mut(&req.obj_id) {
            entry.visited = true;
            self.stats.inc_hit();
            Ok(CacheResult::Hit)
        } else {
            self.stats.inc_miss();
            Ok(CacheResult::Miss)
        }
    }
    
    fn insert(&mut self, req: &Request) -> Result<InsertResult, CacheError> {
        if req.obj_size > self.capacity {
            return Err(CacheError::ObjectTooLarge {
                obj_size: req.obj_size as u64,
                capacity: self.capacity as u64,
            });
        }
        
        // Make space
        while self.current_size + req.obj_size > self.capacity {
            self.evict();
            if self.current_size + req.obj_size <= self.capacity {
                break;
            }
        }
        
        let entry = SieveEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
            visited: false,
        };
        
        self.queue.push_back(req.obj_id);
        self.entries.insert(req.obj_id, entry);
        self.current_size += req.obj_size;
        
        Ok(InsertResult::Inserted)
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> Result<bool, CacheError> {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.current_size -= entry.obj_size;
            self.queue.retain(|id| *id != obj_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn clear(&mut self) {
        self.queue.clear();
        self.entries.clear();
        self.current_size = 0;
        self.hand = 0;
        self.stats.reset();
    }
    
    fn capacity(&self) -> ObjectSize {
        self.capacity
    }
    
    fn size(&self) -> ObjectSize {
        self.current_size
    }
    
    fn len(&self) -> usize {
        self.entries.len()
    }
    
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
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
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        cache.insert(&req3).unwrap();
        
        assert_eq!(cache.len(), 3);
    }
    
    #[test]
    fn test_sieve_visited_bit() {
        let mut cache = SieveCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        
        // Access req1 to set visited bit
        cache.get(&req1).unwrap();
        
        // Insert req3, should prefer evicting req2
        let req3 = Request::new(3, 100);
        cache.insert(&req3).unwrap();
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().n_evict(), 1);
    }
}
