//! 2Q (Two Queue) cache eviction algorithm
//!
//! 2Q uses three queues:
//! - A1in: First-time access queue (FIFO)
//! - A1out: Ghost queue for recently evicted from A1in
//! - Am: Main queue (LRU) for frequently accessed items

use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Request};
use std::collections::{HashMap, VecDeque};

/// 2Q cache implementation
pub struct TwoQCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    stats: CacheStats,
    
    // A1in: FIFO queue for first access
    a1in: VecDeque<ObjectId>,
    a1in_map: HashMap<ObjectId, ObjectSize>,
    a1in_size: ObjectSize,
    a1in_capacity: ObjectSize,
    
    // A1out: Ghost queue (metadata only)
    a1out: VecDeque<ObjectId>,
    a1out_set: std::collections::HashSet<ObjectId>,
    a1out_capacity: usize,
    
    // Am: LRU queue for frequently accessed
    am: VecDeque<ObjectId>,
    am_map: HashMap<ObjectId, ObjectSize>,
    am_size: ObjectSize,
}

impl TwoQCache {
    /// Create a new 2Q cache with default parameters
    /// A1in: 25% of capacity, A1out: 50% of capacity (ghost entries)
    pub fn new(capacity: ObjectSize) -> Self {
        let a1in_capacity = capacity / 4;
        let a1out_capacity = (capacity / 2) as usize;
        
        Self {
            capacity,
            current_size: 0,
            stats: CacheStats::new(),
            a1in: VecDeque::new(),
            a1in_map: HashMap::new(),
            a1in_size: 0,
            a1in_capacity,
            a1out: VecDeque::new(),
            a1out_set: std::collections::HashSet::new(),
            a1out_capacity,
            am: VecDeque::new(),
            am_map: HashMap::new(),
            am_size: 0,
        }
    }
    
    fn evict_from_a1in(&mut self) {
        while self.a1in_size > self.a1in_capacity {
            if let Some(obj_id) = self.a1in.pop_front() {
                if let Some(size) = self.a1in_map.remove(&obj_id) {
                    self.a1in_size -= size;
                    self.current_size -= size;
                    self.stats.increment_evictions();
                    
                    // Add to A1out ghost queue
                    self.a1out.push_back(obj_id);
                    self.a1out_set.insert(obj_id);
                    
                    // Maintain A1out size limit
                    while self.a1out.len() > self.a1out_capacity {
                        if let Some(old) = self.a1out.pop_front() {
                            self.a1out_set.remove(&old);
                        }
                    }
                }
            }
        }
    }
    
    fn evict_from_am(&mut self) {
        while self.current_size > self.capacity {
            if let Some(obj_id) = self.am.pop_front() {
                if let Some(size) = self.am_map.remove(&obj_id) {
                    self.am_size -= size;
                    self.current_size -= size;
                    self.stats.increment_evictions();
                }
            }
        }
    }
}

impl Cache for TwoQCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        let obj_id = req.obj_id;
        
        // Check Am queue (frequently accessed)
        if let Some(&size) = self.am_map.get(&obj_id) {
            self.stats.increment_hits();
            
            // Move to back (most recently used)
            if let Some(pos) = self.am.iter().position(|&id| id == obj_id) {
                self.am.remove(pos);
                self.am.push_back(obj_id);
            }
            
            return CacheResult::Hit { size };
        }
        
        // Check A1in queue (first access)
        if let Some(&size) = self.a1in_map.get(&obj_id) {
            self.stats.increment_hits();
            
            // Promote to Am queue
            if let Some(pos) = self.a1in.iter().position(|&id| id == obj_id) {
                self.a1in.remove(pos);
                self.a1in_map.remove(&obj_id);
                self.a1in_size -= size;
                
                self.am.push_back(obj_id);
                self.am_map.insert(obj_id, size);
                self.am_size += size;
                
                self.evict_from_am();
            }
            
            return CacheResult::Hit { size };
        }
        
        self.stats.increment_misses();
        CacheResult::Miss
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        let obj_id = req.obj_id;
        let obj_size = req.obj_size;
        
        // If in Am, move to back
        if self.am_map.contains_key(&obj_id) {
            if let Some(pos) = self.am.iter().position(|&id| id == obj_id) {
                self.am.remove(pos);
                self.am.push_back(obj_id);
            }
            return InsertResult::Admitted;
        }
        
        // If in A1in, promote to Am
        if let Some(&size) = self.a1in_map.get(&obj_id) {
            if let Some(pos) = self.a1in.iter().position(|&id| id == obj_id) {
                self.a1in.remove(pos);
                self.a1in_map.remove(&obj_id);
                self.a1in_size -= size;
                self.current_size -= size;
            }
            
            self.am.push_back(obj_id);
            self.am_map.insert(obj_id, obj_size);
            self.am_size += obj_size;
            self.current_size += obj_size;
            
            self.evict_from_am();
            return InsertResult::Admitted;
        }
        
        // Check if in A1out (second access after eviction)
        if self.a1out_set.contains(&obj_id) {
            // Insert directly into Am
            self.am.push_back(obj_id);
            self.am_map.insert(obj_id, obj_size);
            self.am_size += obj_size;
            self.current_size += obj_size;
            
            self.a1out_set.remove(&obj_id);
            
            self.evict_from_am();
            return InsertResult::Admitted;
        }
        
        // First access - insert into A1in
        self.a1in.push_back(obj_id);
        self.a1in_map.insert(obj_id, obj_size);
        self.a1in_size += obj_size;
        self.current_size += obj_size;
        
        self.evict_from_a1in();
        
        InsertResult::Admitted
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        // Check Am
        if let Some(size) = self.am_map.remove(&obj_id) {
            if let Some(pos) = self.am.iter().position(|&id| id == obj_id) {
                self.am.remove(pos);
                self.am_size -= size;
                self.current_size -= size;
                return true;
            }
        }
        
        // Check A1in
        if let Some(size) = self.a1in_map.remove(&obj_id) {
            if let Some(pos) = self.a1in.iter().position(|&id| id == obj_id) {
                self.a1in.remove(pos);
                self.a1in_size -= size;
                self.current_size -= size;
                return true;
            }
        }
        
        false
    }
    
    fn clear(&mut self) {
        self.a1in.clear();
        self.a1in_map.clear();
        self.a1in_size = 0;
        self.a1out.clear();
        self.a1out_set.clear();
        self.am.clear();
        self.am_map.clear();
        self.am_size = 0;
        self.current_size = 0;
    }
    
    fn capacity(&self) -> ObjectSize {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.a1in_map.len() + self.am_map.len()
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
    fn test_2q_basic() {
        let mut cache = TwoQCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        let req2 = Request::builder().obj_id(2).obj_size(100).build();
        
        // First access - goes to A1in
        assert!(matches!(cache.get(&req1), CacheResult::Miss));
        cache.insert(&req1);
        
        // Second access - stays in A1in but gets hit
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
        
        // Different object
        assert!(matches!(cache.get(&req2), CacheResult::Miss));
    }
    
    #[test]
    fn test_2q_promotion() {
        let mut cache = TwoQCache::new(300);
        let req1 = Request::builder().obj_id(1).obj_size(100).build();
        
        // First access
        cache.insert(&req1);
        
        // Second access promotes to Am
        cache.get(&req1);
        
        // Third access should still hit in Am
        assert!(matches!(cache.get(&req1), CacheResult::Hit { .. }));
    }
    
    #[test]
    fn test_2q_eviction() {
        let mut cache = TwoQCache::new(300);
        
        // Fill A1in beyond capacity
        for i in 1..=10 {
            let req = Request::builder().obj_id(i).obj_size(50).build();
            cache.insert(&req);
        }
        
        // Should have evicted some from A1in
        assert!(cache.stats().evictions() > 0);
    }
}
