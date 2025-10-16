//! S3-FIFO (Simple Scalable Scan-resistant FIFO) cache eviction algorithm
//!
//! S3-FIFO uses three FIFO queues with frequency tracking to achieve
//! scan resistance with low overhead.

use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Request};
use std::collections::{HashMap, VecDeque};

struct CacheEntry {
    size: ObjectSize,
    freq: u8,
}

/// S3-FIFO cache implementation
pub struct S3FifoCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    stats: CacheStats,
    
    // Small queue (S) - initial insertion
    small_queue: VecDeque<ObjectId>,
    small_map: HashMap<ObjectId, CacheEntry>,
    small_size: ObjectSize,
    small_capacity: ObjectSize,
    
    // Main queue (M)
    main_queue: VecDeque<ObjectId>,
    main_map: HashMap<ObjectId, CacheEntry>,
    main_size: ObjectSize,
    
    // Ghost queue (G) - metadata only
    ghost_queue: VecDeque<ObjectId>,
    ghost_set: std::collections::HashSet<ObjectId>,
    ghost_capacity: usize,
}

impl S3FifoCache {
    /// Create a new S3-FIFO cache
    /// Small queue is 10% of capacity by default
    pub fn new(capacity: ObjectSize) -> Self {
        let small_capacity = capacity / 10;
        let ghost_capacity = 1000; // Fixed size for ghost entries
        
        Self {
            capacity,
            current_size: 0,
            stats: CacheStats::new(),
            small_queue: VecDeque::new(),
            small_map: HashMap::new(),
            small_size: 0,
            small_capacity,
            main_queue: VecDeque::new(),
            main_map: HashMap::new(),
            main_size: 0,
            ghost_queue: VecDeque::new(),
            ghost_set: std::collections::HashSet::new(),
            ghost_capacity,
        }
    }
    
    fn evict_from_small(&mut self) {
        while self.small_size > self.small_capacity {
            if let Some(obj_id) = self.small_queue.front().copied() {
                if let Some(entry) = self.small_map.get(&obj_id) {
                    if entry.freq > 0 {
                        // Promote to main queue
                        self.small_queue.pop_front();
                        let mut entry = self.small_map.remove(&obj_id).unwrap();
                        self.small_size -= entry.size;
                        
                        entry.freq = 0; // Reset frequency
                        let size = entry.size;
                        self.main_queue.push_back(obj_id);
                        self.main_map.insert(obj_id, entry);
                        self.main_size += size;
                    } else {
                        // Evict from small
                        self.small_queue.pop_front();
                        let entry = self.small_map.remove(&obj_id).unwrap();
                        self.small_size -= entry.size;
                        self.current_size -= entry.size;
                        self.stats.increment_evictions();
                        
                        // Add to ghost
                        self.add_to_ghost(obj_id);
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    fn evict_from_main(&mut self) {
        while self.current_size > self.capacity {
            if let Some(obj_id) = self.main_queue.front().copied() {
                if let Some(entry) = self.main_map.get_mut(&obj_id) {
                    if entry.freq > 0 {
                        // Move to back and decrement frequency
                        entry.freq -= 1;
                        self.main_queue.pop_front();
                        self.main_queue.push_back(obj_id);
                    } else {
                        // Evict
                        self.main_queue.pop_front();
                        let entry = self.main_map.remove(&obj_id).unwrap();
                        self.main_size -= entry.size;
                        self.current_size -= entry.size;
                        self.stats.increment_evictions();
                        
                        self.add_to_ghost(obj_id);
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    fn add_to_ghost(&mut self, obj_id: ObjectId) {
        self.ghost_queue.push_back(obj_id);
        self.ghost_set.insert(obj_id);
        
        while self.ghost_queue.len() > self.ghost_capacity {
            if let Some(old) = self.ghost_queue.pop_front() {
                self.ghost_set.remove(&old);
            }
        }
    }
}

impl Cache for S3FifoCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        let obj_id = req.obj_id;
        
        // Check main queue
        if let Some(entry) = self.main_map.get_mut(&obj_id) {
            self.stats.increment_hits();
            entry.freq = entry.freq.saturating_add(1).min(3); // Cap at 3
            return CacheResult::Hit { size: entry.size };
        }
        
        // Check small queue
        if let Some(entry) = self.small_map.get_mut(&obj_id) {
            self.stats.increment_hits();
            entry.freq = entry.freq.saturating_add(1).min(3);
            return CacheResult::Hit { size: entry.size };
        }
        
        self.stats.increment_misses();
        CacheResult::Miss
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        let obj_id = req.obj_id;
        let obj_size = req.obj_size;
        
        // Update if already in main
        if let Some(entry) = self.main_map.get_mut(&obj_id) {
            entry.freq = entry.freq.saturating_add(1).min(3);
            return InsertResult::Admitted;
        }
        
        // Update if already in small
        if let Some(entry) = self.small_map.get_mut(&obj_id) {
            entry.freq = entry.freq.saturating_add(1).min(3);
            return InsertResult::Admitted;
        }
        
        // Check if in ghost (recently evicted)
        if self.ghost_set.contains(&obj_id) {
            self.ghost_set.remove(&obj_id);
            
            // Insert directly to main with higher initial frequency
            let entry = CacheEntry { size: obj_size, freq: 1 };
            self.main_queue.push_back(obj_id);
            self.main_map.insert(obj_id, entry);
            self.main_size += obj_size;
            self.current_size += obj_size;
            
            self.evict_from_main();
            return InsertResult::Admitted;
        }
        
        // New object - insert to small queue
        let entry = CacheEntry { size: obj_size, freq: 0 };
        self.small_queue.push_back(obj_id);
        self.small_map.insert(obj_id, entry);
        self.small_size += obj_size;
        self.current_size += obj_size;
        
        self.evict_from_small();
        self.evict_from_main();
        
        InsertResult::Admitted
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        // Check main
        if let Some(entry) = self.main_map.remove(&obj_id) {
            if let Some(pos) = self.main_queue.iter().position(|&id| id == obj_id) {
                self.main_queue.remove(pos);
                self.main_size -= entry.size;
                self.current_size -= entry.size;
                return true;
            }
        }
        
        // Check small
        if let Some(entry) = self.small_map.remove(&obj_id) {
            if let Some(pos) = self.small_queue.iter().position(|&id| id == obj_id) {
                self.small_queue.remove(pos);
                self.small_size -= entry.size;
                self.current_size -= entry.size;
                return true;
            }
        }
        
        false
    }
    
    fn clear(&mut self) {
        self.small_queue.clear();
        self.small_map.clear();
        self.small_size = 0;
        self.main_queue.clear();
        self.main_map.clear();
        self.main_size = 0;
        self.ghost_queue.clear();
        self.ghost_set.clear();
        self.current_size = 0;
    }
    
    fn capacity(&self) -> ObjectSize {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.small_map.len() + self.main_map.len()
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
    fn test_s3fifo_basic() {
        let mut cache = S3FifoCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        
        assert!(matches!(cache.get(&req1), CacheResult::Miss));
        cache.insert(&req1);
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
    }
    
    #[test]
    fn test_s3fifo_frequency() {
        let mut cache = S3FifoCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        
        cache.insert(&req1);
        
        // Multiple accesses increase frequency
        for _ in 0..5 {
            cache.get(&req1);
        }
        
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
    }
    
    #[test]
    fn test_s3fifo_eviction() {
        let mut cache = S3FifoCache::new(300);
        
        // Fill cache
        for i in 1..=10 {
            let req = Request::builder().obj_id(i).obj_size(50).build();
            cache.insert(&req);
        }
        
        assert!(cache.stats().evictions() > 0);
    }
}
