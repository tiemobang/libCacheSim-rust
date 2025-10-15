//! Clock cache implementation
//!
//! Clock is a simple approximation of LRU that uses a reference bit.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};

/// Entry in the Clock cache
struct ClockEntry {
    obj_size: u32,
    referenced: bool,
}

/// Clock cache implementation
///
/// Clock (also known as Second-Chance) is an approximation of LRU using a reference bit.
/// Objects are arranged in a circular list, and a "clock hand" points to the next candidate
/// for eviction. When an object needs to be evicted, the algorithm checks the reference bit:
/// - If set, clear it and move to the next object
/// - If not set, evict the object
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::ClockCache;
///
/// let mut cache = ClockCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct ClockCache {
    capacity: u64,
    used: u64,
    entries: HashMap<ObjectId, ClockEntry>,
    clock_list: Vec<ObjectId>,
    hand: usize,
    stats: CacheStats,
}

impl ClockCache {
    /// Create a new Clock cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        ClockCache {
            capacity,
            used: 0,
            entries: HashMap::new(),
            clock_list: Vec::new(),
            hand: 0,
            stats: CacheStats::new(),
        }
    }
}

impl Cache for ClockCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();
        
        if let Some(entry) = self.entries.get_mut(&req.obj_id) {
            self.stats.inc_hit();
            entry.referenced = true;
            CacheResult::Hit {
                obj_size: entry.obj_size,
            }
        } else {
            self.stats.inc_miss();
            self.insert(req);
            CacheResult::Miss
        }
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        // Check if object already exists
        if let Some(entry) = self.entries.get_mut(&req.obj_id) {
            entry.referenced = true;
            return InsertResult::Inserted;
        }
        
        // Evict objects if necessary to make room
        let mut evicted_id = None;
        while self.used + req.obj_size as u64 > self.capacity && !self.clock_list.is_empty() {
            if let Some(evict_id) = self.evict() {
                evicted_id = Some(evict_id);
            }
        }
        
        // Check if object can fit
        if req.obj_size as u64 > self.capacity {
            return InsertResult::Rejected;
        }
        
        // Insert the object
        let entry = ClockEntry {
            obj_size: req.obj_size,
            referenced: true,
        };
        self.entries.insert(req.obj_id, entry);
        self.clock_list.push(req.obj_id);
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
        if self.clock_list.is_empty() {
            return None;
        }
        
        // Find an object to evict using the clock algorithm
        loop {
            if self.hand >= self.clock_list.len() {
                self.hand = 0;
            }
            
            let obj_id = self.clock_list[self.hand];
            
            if let Some(entry) = self.entries.get_mut(&obj_id) {
                if entry.referenced {
                    // Give it another chance
                    entry.referenced = false;
                    self.hand += 1;
                } else {
                    // Evict this object
                    let obj_size = entry.obj_size;
                    self.entries.remove(&obj_id);
                    self.clock_list.remove(self.hand);
                    self.used -= obj_size as u64;
                    self.stats.inc_evict();
                    
                    // Adjust hand if necessary
                    if self.hand >= self.clock_list.len() && !self.clock_list.is_empty() {
                        self.hand = 0;
                    }
                    
                    return Some(obj_id);
                }
            } else {
                // Entry not found, skip
                self.hand += 1;
            }
        }
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.used -= entry.obj_size as u64;
            // Remove from clock list
            if let Some(pos) = self.clock_list.iter().position(|&id| id == obj_id) {
                self.clock_list.remove(pos);
                // Adjust hand if necessary
                if self.hand > pos {
                    self.hand -= 1;
                } else if self.hand >= self.clock_list.len() && !self.clock_list.is_empty() {
                    self.hand = 0;
                }
            }
            true
        } else {
            false
        }
    }
    
    fn clear(&mut self) {
        self.entries.clear();
        self.clock_list.clear();
        self.used = 0;
        self.hand = 0;
    }
    
    fn size(&self) -> u64 {
        self.used
    }
    
    fn capacity(&self) -> u64 {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.entries.len()
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
    fn test_clock_basic() {
        let mut cache = ClockCache::new(300);
        
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
    fn test_clock_eviction() {
        let mut cache = ClockCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        // Fill cache
        cache.get(&req1);
        cache.get(&req2);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.size(), 200);
        
        // This should trigger eviction
        cache.get(&req3);
        
        // After adding req3, cache should still be within capacity
        assert!(cache.size() <= cache.capacity());
        assert!(cache.len() <= 2);
        
        // Verify statistics
        assert_eq!(cache.stats().n_req(), 3);
        assert_eq!(cache.stats().n_evict(), 1);
    }
    
    #[test]
    fn test_clock_stats() {
        let mut cache = ClockCache::new(200);
        
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
