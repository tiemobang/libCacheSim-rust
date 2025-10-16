//! ARC (Adaptive Replacement Cache) implementation
//!
//! ARC dynamically balances between recency and frequency by maintaining two LRU lists:
//! - T1: Recent cache entries (frequency = 1)
//! - T2: Frequent cache entries (frequency >= 2)
//! - B1: Ghost entries evicted from T1
//! - B2: Ghost entries evicted from T2
//!
//! The algorithm adaptively adjusts the target size of T1 (p) based on workload characteristics.

use libcachesim_core::{
    Cache, CacheResult, InsertResult, CacheError, ObjectId, ObjectSize, Request, CacheStats,
};
use std::collections::{HashMap, VecDeque};

/// Entry in the ARC cache
#[derive(Debug, Clone)]
struct ArcEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
}

/// ARC cache implementation
///
/// Adaptive Replacement Cache balances recency and frequency dynamically.
pub struct ArcCache {
    /// Maximum cache size in bytes
    capacity: ObjectSize,
    /// Current cache size in bytes
    current_size: ObjectSize,
    /// Target size for T1 (adaptive parameter)
    p: ObjectSize,
    
    /// T1: Recent cache entries (frequency = 1)
    t1: VecDeque<ObjectId>,
    /// T2: Frequent cache entries (frequency >= 2)
    t2: VecDeque<ObjectId>,
    /// B1: Ghost entries evicted from T1
    b1: VecDeque<ObjectId>,
    /// B2: Ghost entries evicted from T2
    b2: VecDeque<ObjectId>,
    
    /// Map from object ID to entry metadata
    entries: HashMap<ObjectId, ArcEntry>,
    /// Set to track which list contains each object
    location: HashMap<ObjectId, ListType>,
    
    /// Statistics
    stats: CacheStats,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ListType {
    T1,
    T2,
    B1,
    B2,
}

impl ArcCache {
    /// Create a new ARC cache with the specified capacity
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
    
    fn replace(&mut self, req: &Request) {
        // Determine which list to evict from based on sizes and workload
        let t1_len = self.t1.len();
        let b2_in = self.location.get(&req.obj_id) == Some(&ListType::B2);
        
        if t1_len > 0 && (t1_len > self.p as usize || b2_in) {
            // Evict from T1
            if let Some(victim_id) = self.t1.pop_front() {
                if let Some(entry) = self.entries.remove(&victim_id) {
                    self.current_size -= entry.obj_size;
                    // Move to ghost list B1
                    self.b1.push_back(victim_id);
                    self.location.insert(victim_id, ListType::B1);
                    self.stats.inc_evict();
                }
            }
        } else {
            // Evict from T2
            if let Some(victim_id) = self.t2.pop_front() {
                if let Some(entry) = self.entries.remove(&victim_id) {
                    self.current_size -= entry.obj_size;
                    // Move to ghost list B2
                    self.b2.push_back(victim_id);
                    self.location.insert(victim_id, ListType::B2);
                    self.stats.inc_evict();
                }
            }
        }
        
        // Maintain ghost list sizes
        self.maintain_ghost_lists();
    }
    
    fn maintain_ghost_lists(&mut self) {
        let max_ghost_size = self.capacity as usize * 2;
        
        // Limit B1 size
        while self.b1.len() + self.t1.len() > max_ghost_size {
            if let Some(old_id) = self.b1.pop_front() {
                self.location.remove(&old_id);
            }
        }
        
        // Limit B2 size
        while self.b2.len() + self.t2.len() > max_ghost_size {
            if let Some(old_id) = self.b2.pop_front() {
                self.location.remove(&old_id);
            }
        }
    }
    
    fn adapt(&mut self, in_b1: bool, in_b2: bool) {
        // Adaptive adjustment of p based on hits in ghost lists
        let delta = if self.b1.len() >= self.b2.len() { 1 } else { self.b2.len() / self.b1.len().max(1) };
        
        if in_b1 {
            // Hit in B1: increase p (favor recency)
            self.p = (self.p + delta as ObjectSize).min(self.capacity);
        } else if in_b2 {
            // Hit in B2: decrease p (favor frequency)
            self.p = self.p.saturating_sub(delta as ObjectSize);
        }
    }
}

impl Cache for ArcCache {
    fn get(&mut self, req: &Request) -> Result<CacheResult, CacheError> {
        self.stats.inc_req();
        
        let in_t1 = self.t1.iter().any(|id| *id == req.obj_id);
        let in_t2 = self.t2.iter().any(|id| *id == req.obj_id);
        let loc = self.location.get(&req.obj_id).copied();
        
        if in_t1 {
            // Hit in T1: move to T2 (promotion)
            self.t1.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T2);
            self.stats.inc_hit();
            Ok(CacheResult::Hit)
        } else if in_t2 {
            // Hit in T2: move to end (most recent)
            self.t2.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.stats.inc_hit();
            Ok(CacheResult::Hit)
        } else {
            // Miss
            self.stats.inc_miss();
            
            // Check if in ghost lists
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
        
        // Make space if needed
        while self.current_size + req.obj_size > self.capacity {
            self.replace(req);
        }
        
        // Insert the object
        let entry = ArcEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
        };
        
        if in_b1 {
            // Was in B1 ghost list: insert into T2
            self.b1.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T2);
        } else if in_b2 {
            // Was in B2 ghost list: insert into T2
            self.b2.retain(|id| *id != req.obj_id);
            self.t2.push_back(req.obj_id);
            self.location.insert(req.obj_id, ListType::T2);
        } else {
            // New object: insert into T1
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
    fn test_arc_basic() {
        let mut cache = ArcCache::new(300);
        
        // Insert objects
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        assert!(cache.insert(&req1).is_ok());
        assert!(cache.insert(&req2).is_ok());
        assert!(cache.insert(&req3).is_ok());
        
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.size(), 300);
    }
    
    #[test]
    fn test_arc_eviction() {
        let mut cache = ArcCache::new(300);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        let req4 = Request::new(4, 100);
        
        cache.insert(&req1).unwrap();
        cache.insert(&req2).unwrap();
        cache.insert(&req3).unwrap();
        
        // Access 1 and 2 to promote them
        cache.get(&req1).unwrap();
        cache.get(&req2).unwrap();
        
        // Insert 4, should evict 3
        cache.insert(&req4).unwrap();
        
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.stats().n_evict(), 1);
    }
    
    #[test]
    fn test_arc_adaptation() {
        let mut cache = ArcCache::new(300);
        
        // Access pattern that favors recency then frequency
        for i in 1..=3 {
            let req = Request::new(i, 100);
            cache.insert(&req).unwrap();
        }
        
        // Repeated accesses to promote to T2
        let req1 = Request::new(1, 100);
        cache.get(&req1).unwrap(); // Promotes 1 to T2
        
        // Continue accessing to test adaptation
        let req4 = Request::new(4, 100);
        cache.insert(&req4).unwrap();
        
        assert_eq!(cache.stats().n_evict(), 1);
    }
}
