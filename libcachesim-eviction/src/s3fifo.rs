//! S3-FIFO (Simple, Scalable, Scan-resistant FIFO) implementation
//!
//! S3-FIFO uses three FIFO queues with different purposes:
//! - Small queue: For small objects (quick filtering)
//! - Main queue: For regular objects
//! - Ghost queue: For tracking evicted objects

use libcachesim_core::{
    Cache, CacheResult, InsertResult, CacheError, ObjectId, ObjectSize, Request, CacheStats,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
struct S3FifoEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
    freq: u8, // Frequency counter (capped at 3)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueueType {
    Small,
    Main,
    Ghost,
}

/// S3-FIFO cache implementation
pub struct S3FifoCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    
    /// Size threshold for small queue
    small_threshold: ObjectSize,
    /// Target size for small queue (10% of capacity)
    small_capacity: ObjectSize,
    
    /// Small FIFO queue
    small: VecDeque<ObjectId>,
    /// Main FIFO queue
    main: VecDeque<ObjectId>,
    /// Ghost queue for evicted objects
    ghost: VecDeque<ObjectId>,
    
    entries: HashMap<ObjectId, S3FifoEntry>,
    location: HashMap<ObjectId, QueueType>,
    
    stats: CacheStats,
}

impl S3FifoCache {
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity,
            current_size: 0,
            small_threshold: 1024, // Objects <= 1KB go to small queue
            small_capacity: capacity / 10, // 10% for small queue
            small: VecDeque::new(),
            main: VecDeque::new(),
            ghost: VecDeque::new(),
            entries: HashMap::new(),
            location: HashMap::new(),
            stats: CacheStats::new(),
        }
    }
    
    fn get_queue_size(&self, queue: QueueType) -> ObjectSize {
        let ids = match queue {
            QueueType::Small => &self.small,
            QueueType::Main => &self.main,
            QueueType::Ghost => return 0,
        };
        
        ids.iter()
            .filter_map(|id| self.entries.get(id))
            .map(|e| e.obj_size)
            .sum()
    }
    
    fn evict_from_small(&mut self) -> bool {
        while let Some(victim_id) = self.small.pop_front() {
            if let Some(mut entry) = self.entries.get_mut(&victim_id) {
                if entry.freq > 0 {
                    // Decrement frequency and move to main
                    entry.freq -= 1;
                    self.small.retain(|id| *id != victim_id);
                    self.main.push_back(victim_id);
                    self.location.insert(victim_id, QueueType::Main);
                } else {
                    // Evict
                    let obj_size = entry.obj_size;
                    self.entries.remove(&victim_id);
                    self.current_size -= obj_size;
                    self.ghost.push_back(victim_id);
                    self.location.insert(victim_id, QueueType::Ghost);
                    self.stats.inc_evict();
                    
                    // Maintain ghost size
                    while self.ghost.len() > self.capacity as usize {
                        if let Some(old_id) = self.ghost.pop_front() {
                            self.location.remove(&old_id);
                        }
                    }
                    return true;
                }
            }
        }
        false
    }
    
    fn evict_from_main(&mut self) -> bool {
        while let Some(victim_id) = self.main.pop_front() {
            if let Some(mut entry) = self.entries.get_mut(&victim_id) {
                if entry.freq > 0 {
                    // Decrement and reinsert at tail
                    entry.freq -= 1;
                    self.main.push_back(victim_id);
                } else {
                    // Evict
                    let obj_size = entry.obj_size;
                    self.entries.remove(&victim_id);
                    self.current_size -= obj_size;
                    self.ghost.push_back(victim_id);
                    self.location.insert(victim_id, QueueType::Ghost);
                    self.stats.inc_evict();
                    
                    while self.ghost.len() > self.capacity as usize {
                        if let Some(old_id) = self.ghost.pop_front() {
                            self.location.remove(&old_id);
                        }
                    }
                    return true;
                }
            }
        }
        false
    }
}

impl Cache for S3FifoCache {
    fn get(&mut self, req: &Request) -> Result<CacheResult, CacheError> {
        self.stats.inc_req();
        
        if let Some(entry) = self.entries.get_mut(&req.obj_id) {
            // Increment frequency (cap at 3)
            entry.freq = entry.freq.saturating_add(1).min(3);
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
            let small_size = self.get_queue_size(QueueType::Small);
            
            if small_size > self.small_capacity || !self.main.is_empty() {
                if !self.evict_from_small() && !self.evict_from_main() {
                    break;
                }
            } else {
                if !self.evict_from_main() {
                    break;
                }
            }
        }
        
        let entry = S3FifoEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
            freq: 0,
        };
        
        // Determine which queue to use
        let is_small = req.obj_size <= self.small_threshold;
        let in_ghost = self.location.get(&req.obj_id) == Some(&QueueType::Ghost);
        
        if in_ghost {
            // Was evicted before: goes to main
            self.ghost.retain(|id| *id != req.obj_id);
            self.main.push_back(req.obj_id);
            self.location.insert(req.obj_id, QueueType::Main);
        } else if is_small {
            self.small.push_back(req.obj_id);
            self.location.insert(req.obj_id, QueueType::Small);
        } else {
            self.main.push_back(req.obj_id);
            self.location.insert(req.obj_id, QueueType::Main);
        }
        
        self.entries.insert(req.obj_id, entry);
        self.current_size += req.obj_size;
        
        Ok(InsertResult::Inserted)
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> Result<bool, CacheError> {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.current_size -= entry.obj_size;
            self.small.retain(|id| *id != obj_id);
            self.main.retain(|id| *id != obj_id);
            self.location.remove(&obj_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn clear(&mut self) {
        self.small.clear();
        self.main.clear();
        self.ghost.clear();
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
    fn test_s3fifo_basic() {
        let mut cache = S3FifoCache::new(1000);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        
        assert_eq!(cache.len(), 2);
    }
    
    #[test]
    fn test_s3fifo_small_queue() {
        let mut cache = S3FifoCache::new(10000);
        
        // Small object should go to small queue
        let req_small = Request::new(1, 512); // < 1024
        cache.insert(&req_small).unwrap();
        assert_eq!(cache.location.get(&1), Some(&QueueType::Small));
        
        // Large object should go to main queue
        let req_large = Request::new(2, 2048); // > 1024
        cache.insert(&req_large).unwrap();
        assert_eq!(cache.location.get(&2), Some(&QueueType::Main));
    }
    
    #[test]
    fn test_s3fifo_frequency() {
        let mut cache = S3FifoCache::new(1000);
        
        let req1 = Request::new(1, 100);
        cache.insert(&req1).unwrap();
        
        // Access multiple times to increase frequency
        cache.get(&req1).unwrap();
        cache.get(&req1).unwrap();
        
        let entry = cache.entries.get(&1).unwrap();
        assert!(entry.freq > 0);
    }
}
