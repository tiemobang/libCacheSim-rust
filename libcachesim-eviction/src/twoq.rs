//! 2Q (Two Queue) cache implementation
//!
//! 2Q manages three queues:
//! - A1in: FIFO queue for first access (new objects)
//! - A1out: Ghost FIFO queue (history of evicted from A1in)
//! - Am: LRU queue for frequently accessed objects

use libcachesim_core::{
    Cache, CacheResult, InsertResult, CacheError, ObjectId, ObjectSize, Request, CacheStats,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
struct TwoQEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueueType {
    A1In,
    A1Out,
    Am,
}

/// 2Q cache implementation
pub struct TwoQCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    
    /// Size parameters
    kin_size: ObjectSize,  // A1in target size (~25%)
    kout_size: ObjectSize, // A1out target size (~50%)
    
    /// A1in: FIFO for new objects
    a1in: VecDeque<ObjectId>,
    /// A1out: Ghost FIFO
    a1out: VecDeque<ObjectId>,
    /// Am: LRU for frequent objects
    am: VecDeque<ObjectId>,
    
    entries: HashMap<ObjectId, TwoQEntry>,
    location: HashMap<ObjectId, QueueType>,
    
    stats: CacheStats,
}

impl TwoQCache {
    /// Create a new 2Q cache with the specified capacity
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity,
            current_size: 0,
            kin_size: capacity / 4,      // 25% for A1in
            kout_size: capacity / 2,     // 50% for A1out
            a1in: VecDeque::new(),
            a1out: VecDeque::new(),
            am: VecDeque::new(),
            entries: HashMap::new(),
            location: HashMap::new(),
            stats: CacheStats::new(),
        }
    }
    
    /// Create with custom size ratios
    pub fn with_ratios(capacity: ObjectSize, kin_ratio: f64, kout_ratio: f64) -> Self {
        Self {
            capacity,
            current_size: 0,
            kin_size: (capacity as f64 * kin_ratio) as ObjectSize,
            kout_size: (capacity as f64 * kout_ratio) as ObjectSize,
            a1in: VecDeque::new(),
            a1out: VecDeque::new(),
            am: VecDeque::new(),
            entries: HashMap::new(),
            location: HashMap::new(),
            stats: CacheStats::new(),
        }
    }
    
    fn get_queue_size(&self, queue: QueueType) -> ObjectSize {
        let ids = match queue {
            QueueType::A1In => &self.a1in,
            QueueType::Am => &self.am,
            QueueType::A1Out => return 0, // Ghost queue has no size
        };
        
        ids.iter()
            .filter_map(|id| self.entries.get(id))
            .map(|e| e.obj_size)
            .sum()
    }
    
    fn reclaim(&mut self, size_needed: ObjectSize) {
        // Make space by evicting from a1in or am
        while self.current_size + size_needed > self.capacity {
            let a1in_size = self.get_queue_size(QueueType::A1In);
            
            if !self.a1in.is_empty() && a1in_size > self.kin_size {
                // Evict from A1in (move to A1out)
                if let Some(victim_id) = self.a1in.pop_front() {
                    if let Some(entry) = self.entries.remove(&victim_id) {
                        self.current_size -= entry.obj_size;
                        // Add to ghost queue A1out
                        self.a1out.push_back(victim_id);
                        self.location.insert(victim_id, QueueType::A1Out);
                        
                        // Limit A1out size
                        while self.a1out.len() > self.kout_size as usize {
                            if let Some(old_id) = self.a1out.pop_front() {
                                self.location.remove(&old_id);
                            }
                        }
                        
                        self.stats.inc_evict();
                    }
                }
            } else if !self.am.is_empty() {
                // Evict from Am (LRU)
                if let Some(victim_id) = self.am.pop_front() {
                    if let Some(entry) = self.entries.remove(&victim_id) {
                        self.current_size -= entry.obj_size;
                        self.location.remove(&victim_id);
                        self.stats.inc_evict();
                    }
                }
            } else {
                break;
            }
        }
    }
}

impl Cache for TwoQCache {
    fn get(&mut self, req: &Request) -> Result<CacheResult, CacheError> {
        self.stats.inc_req();
        
        let loc = self.location.get(&req.obj_id).copied();
        
        match loc {
            Some(QueueType::A1In) => {
                // Hit in A1in
                self.stats.inc_hit();
                Ok(CacheResult::Hit)
            }
            Some(QueueType::Am) => {
                // Hit in Am: move to end (MRU position)
                self.am.retain(|id| *id != req.obj_id);
                self.am.push_back(req.obj_id);
                self.stats.inc_hit();
                Ok(CacheResult::Hit)
            }
            Some(QueueType::A1Out) => {
                // In ghost queue: miss but will be promoted on insert
                self.stats.inc_miss();
                Ok(CacheResult::Miss)
            }
            None => {
                self.stats.inc_miss();
                Ok(CacheResult::Miss)
            }
        }
    }
    
    fn insert(&mut self, req: &Request) -> Result<InsertResult, CacheError> {
        if req.obj_size > self.capacity {
            return Err(CacheError::ObjectTooLarge {
                obj_size: req.obj_size as u64,
                capacity: self.capacity as u64,
            });
        }
        
        let loc = self.location.get(&req.obj_id).copied();
        
        // Make space
        self.reclaim(req.obj_size);
        
        let entry = TwoQEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
        };
        
        match loc {
            Some(QueueType::A1Out) => {
                // Was in ghost queue: promote to Am
                self.a1out.retain(|id| *id != req.obj_id);
                self.am.push_back(req.obj_id);
                self.location.insert(req.obj_id, QueueType::Am);
            }
            Some(QueueType::A1In) => {
                // Already in A1in: promote to Am
                self.a1in.retain(|id| *id != req.obj_id);
                self.am.push_back(req.obj_id);
                self.location.insert(req.obj_id, QueueType::Am);
            }
            Some(QueueType::Am) => {
                // Already in Am: just update
                self.am.retain(|id| *id != req.obj_id);
                self.am.push_back(req.obj_id);
            }
            None => {
                // New object: add to A1in
                self.a1in.push_back(req.obj_id);
                self.location.insert(req.obj_id, QueueType::A1In);
            }
        }
        
        self.entries.insert(req.obj_id, entry);
        self.current_size += req.obj_size;
        
        Ok(InsertResult::Inserted)
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> Result<bool, CacheError> {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.current_size -= entry.obj_size;
            self.a1in.retain(|id| *id != obj_id);
            self.am.retain(|id| *id != obj_id);
            self.location.remove(&obj_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn clear(&mut self) {
        self.a1in.clear();
        self.a1out.clear();
        self.am.clear();
        self.entries.clear();
        self.location.clear();
        self.current_size = 0;
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
    fn test_twoq_basic() {
        let mut cache = TwoQCache::new(300);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        cache.insert(&req3).unwrap();
        
        assert_eq!(cache.len(), 3);
    }
    
    #[test]
    fn test_twoq_promotion() {
        let mut cache = TwoQCache::new(300);
        
        let req1 = Request::new(1, 100);
        
        // First insert: goes to A1in
        cache.insert(&req1).unwrap();
        assert_eq!(cache.location.get(&1), Some(&QueueType::A1In));
        
        // Access again: should promote to Am
        cache.get(&req1).unwrap();
        cache.insert(&req1).unwrap();
        assert_eq!(cache.location.get(&1), Some(&QueueType::Am));
    }
    
    #[test]
    fn test_twoq_ghost_queue() {
        let mut cache = TwoQCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        
        // Fill A1in
        cache.insert(&req3).unwrap();
        
        // Should have evicted something to A1out
        assert!(cache.stats().n_evict() >= 1);
    }
}
