//! CAR (Clock with Adaptive Replacement) implementation
//!
//! CAR combines the CLOCK algorithm's efficient approximate LRU with ARC's adaptive
//! partitioning between short-term and long-term entries.

use libcachesim_core::{
    Cache, CacheResult, InsertResult, CacheError, ObjectId, ObjectSize, Request, CacheStats,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
struct CarEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
    reference_bit: bool,
}

/// CAR cache implementation
pub struct CarCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    p: ObjectSize, // Adaptive parameter
    
    /// T1: Short-term cache (frequency = 1)
    t1: VecDeque<ObjectId>,
    /// T2: Long-term cache (frequency >= 2)
    t2: VecDeque<ObjectId>,
    /// B1: Ghost entries from T1
    b1: VecDeque<ObjectId>,
    /// B2: Ghost entries from T2
    b2: VecDeque<ObjectId>,
    
    entries: HashMap<ObjectId, CarEntry>,
    location: HashMap<ObjectId, ListType>,
    
    stats: CacheStats,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ListType {
    T1,
    T2,
    B1,
    B2,
}

impl CarCache {
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity,
            current_size: 0,
            p: 0,
            t1: VecDeque::new(),
            t2: VecDeque::new(),
            b1: VecDeque::new(),
            b2: VecDeque::new(),
            entries: HashMap::new(),
            location: HashMap::new(),
            stats: CacheStats::new(),
        }
    }
    
    fn clock_evict(&mut self, from_t1: bool) -> Option<ObjectId> {
        let list = if from_t1 { &mut self.t1 } else { &mut self.t2 };
        
        // Clock sweep: find first entry with reference_bit = false
        let mut rotations = 0;
        let max_rotations = list.len();
        
        while rotations < max_rotations {
            if let Some(obj_id) = list.pop_front() {
                if let Some(entry) = self.entries.get_mut(&obj_id) {
                    if entry.reference_bit {
                        // Reset reference bit and move to back
                        entry.reference_bit = false;
                        list.push_back(obj_id);
                        rotations += 1;
                    } else {
                        // Found victim
                        return Some(obj_id);
                    }
                }
            } else {
                break;
            }
        }
        
        // Fallback: evict first entry
        list.pop_front()
    }
    
    fn replace(&mut self, in_b2: bool) {
        let t1_len = self.t1.len();
        
        if t1_len > 0 && (t1_len > self.p as usize || in_b2) {
            // Evict from T1 using CLOCK
            if let Some(victim_id) = self.clock_evict(true) {
                if let Some(entry) = self.entries.remove(&victim_id) {
                    self.current_size -= entry.obj_size;
                    self.b1.push_back(victim_id);
                    self.location.insert(victim_id, ListType::B1);
                    self.stats.inc_evict();
                }
            }
        } else {
            // Evict from T2 using CLOCK
            if let Some(victim_id) = self.clock_evict(false) {
                if let Some(entry) = self.entries.remove(&victim_id) {
                    self.current_size -= entry.obj_size;
                    self.b2.push_back(victim_id);
                    self.location.insert(victim_id, ListType::B2);
                    self.stats.inc_evict();
                }
            }
        }
        
        self.maintain_ghost_lists();
    }
    
    fn maintain_ghost_lists(&mut self) {
        let max_ghost_size = self.capacity as usize * 2;
        
        while self.b1.len() + self.t1.len() > max_ghost_size {
            if let Some(old_id) = self.b1.pop_front() {
                self.location.remove(&old_id);
            }
        }
        
        while self.b2.len() + self.t2.len() > max_ghost_size {
            if let Some(old_id) = self.b2.pop_front() {
                self.location.remove(&old_id);
            }
        }
    }
    
    fn adapt(&mut self, in_b1: bool, in_b2: bool) {
        let delta = if self.b1.len() >= self.b2.len() { 1 } else { self.b2.len() / self.b1.len().max(1) };
        
        if in_b1 {
            self.p = (self.p + delta as ObjectSize).min(self.capacity);
        } else if in_b2 {
            self.p = self.p.saturating_sub(delta as ObjectSize);
        }
    }
}

impl Cache for CarCache {
    fn get(&mut self, req: &Request) -> Result<CacheResult, CacheError> {
        self.stats.inc_req();
        
        if let Some(entry) = self.entries.get_mut(&req.obj_id) {
            // Set reference bit on hit
            entry.reference_bit = true;
            self.stats.inc_hit();
            
            // Promote from T1 to T2 if in T1
            if self.location.get(&req.obj_id) == Some(&ListType::T1) {
                self.t1.retain(|id| *id != req.obj_id);
                self.t2.push_back(req.obj_id);
                self.location.insert(req.obj_id, ListType::T2);
            }
            
            Ok(CacheResult::Hit)
        } else {
            self.stats.inc_miss();
            
            let loc = self.location.get(&req.obj_id).copied();
            let in_b1 = loc == Some(ListType::B1);
            let in_b2 = loc == Some(ListType::B2);
            
            if in_b1 || in_b2 {
                self.adapt(in_b1, in_b2);
            }
            
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
        
        let loc = self.location.get(&req.obj_id).copied();
        let in_b1 = loc == Some(ListType::B1);
        let in_b2 = loc == Some(ListType::B2);
        
        while self.current_size + req.obj_size > self.capacity {
            self.replace(in_b2);
        }
        
        let entry = CarEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
            reference_bit: true,
        };
        
        if in_b1 {
            self.b1.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T2);
        } else if in_b2 {
            self.b2.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T2);
        } else {
            self.t1.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T1);
        }
        
        self.entries.insert(req.obj_id, entry);
        self.current_size += req.obj_size;
        
        Ok(InsertResult::Inserted)
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> Result<bool, CacheError> {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.current_size -= entry.obj_size;
            self.t1.retain(|id| *id != obj_id);
            self.t2.retain(|id| *id != obj_id);
            self.location.remove(&obj_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn clear(&mut self) {
        self.t1.clear();
        self.t2.clear();
        self.b1.clear();
        self.b2.clear();
        self.entries.clear();
        self.location.clear();
        self.current_size = 0;
        self.p = 0;
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
    fn test_car_basic() {
        let mut cache = CarCache::new(300);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        assert!(cache.insert(&req1).is_ok());
        assert!(cache.insert(&req2).is_ok());
        assert!(cache.insert(&req3).is_ok());
        
        assert_eq!(cache.len(), 3);
    }
    
    #[test]
    fn test_car_clock_eviction() {
        let mut cache = CarCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        
        // Access req1 to set reference bit
        cache.get(&req1).unwrap();
        
        // Insert req3 should evict req2 (no reference bit)
        cache.insert(&req3).unwrap();
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().n_evict(), 1);
    }
}
