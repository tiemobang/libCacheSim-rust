//! ARC (Adaptive Replacement Cache) implementation
//!
//! ARC dynamically balances between recency and frequency by maintaining:
//! - T1: Recent cache entries (seen once)
//! - T2: Frequent cache entries (seen multiple times)
//! - B1: Ghost entries evicted from T1
//! - B2: Ghost entries evicted from T2
//!
//! The algorithm adaptively adjusts the target size of T1 based on workload.

use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Request};
use std::collections::{HashMap, VecDeque};

/// Location of an object in ARC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Location {
    T1,
    T2,
    B1,
    B2,
}

/// Entry in the ARC cache
#[derive(Debug, Clone)]
struct ArcEntry {
    obj_id: ObjectId,
    obj_size: ObjectSize,
    location: Location,
}

/// ARC (Adaptive Replacement Cache)
///
/// # Example
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::ArcCache;
///
/// let mut cache = ArcCache::new(1000);
/// let req = Request::new(1, 100);
/// cache.insert(&req);
/// ```
pub struct ArcCache {
    capacity: ObjectSize,
    current_size: ObjectSize,
    p: ObjectSize,  // Target size for T1
    
    t1: VecDeque<ObjectId>,
    t2: VecDeque<ObjectId>,
    b1: VecDeque<ObjectId>,
    b2: VecDeque<ObjectId>,
    
    entries: HashMap<ObjectId, ArcEntry>,
    stats: CacheStats,
}

impl ArcCache {
    /// Create a new ARC cache with the given capacity
    pub fn new(capacity: ObjectSize) -> Self {
        Self {
            capacity: capacity as ObjectSize,
            current_size: 0,
            p: 0,
            t1: VecDeque::new(),
            t2: VecDeque::new(),
            b1: VecDeque::new(),
            b2: VecDeque::new(),
            entries: HashMap::new(),
            stats: CacheStats::new(),
        }
    }

    fn replace(&mut self, req_size: ObjectSize) {
        while self.current_size + req_size > self.capacity {
            if !self.t1.is_empty() && (self.t1_size() > self.p || (self.in_b2(self.t1.front().copied().unwrap()) && self.t1_size() == self.p)) {
                // Evict from T1 to B1
                if let Some(victim_id) = self.t1.pop_front() {
                    if let Some(entry) = self.entries.get_mut(&victim_id) {
                        self.current_size -= entry.obj_size as ObjectSize;
                        entry.location = Location::B1;
                        self.b1.push_back(victim_id);
                        // Limit B1 size
                        while self.b1.len() > self.capacity as usize {
                            if let Some(id) = self.b1.pop_front() {
                                self.entries.remove(&id);
                            }
                        }
                    }
                }
            } else if !self.t2.is_empty() {
                // Evict from T2 to B2
                if let Some(victim_id) = self.t2.pop_front() {
                    if let Some(entry) = self.entries.get_mut(&victim_id) {
                        self.current_size -= entry.obj_size as ObjectSize;
                        entry.location = Location::B2;
                        self.b2.push_back(victim_id);
                        // Limit B2 size
                        while self.b2.len() > self.capacity as usize {
                            if let Some(id) = self.b2.pop_front() {
                                self.entries.remove(&id);
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }
    }

    fn t1_size(&self) -> ObjectSize {
        self.t1.iter()
            .filter_map(|id| self.entries.get(id))
            .map(|e| e.obj_size as ObjectSize)
            .sum()
    }

    fn in_b2(&self, obj_id: ObjectId) -> bool {
        self.entries.get(&obj_id)
            .map(|e| e.location == Location::B2)
            .unwrap_or(false)
    }
}

impl Cache for ArcCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();
        
        // Check location first
        let location = self.entries.get(&req.obj_id).map(|e| (e.location, e.obj_size));
        
        if let Some((loc, obj_size)) = location {
            match loc {
                Location::T1 | Location::T2 => {
                    // Hit in cache
                    self.stats.inc_hit();
                    
                    // Move to T2 (promote)
                    if loc == Location::T1 {
                        self.t1.retain(|&id| id != req.obj_id);
                        self.t2.push_back(req.obj_id);
                        if let Some(e) = self.entries.get_mut(&req.obj_id) {
                            e.location = Location::T2;
                        }
                    } else {
                        // Already in T2, move to back
                        self.t2.retain(|&id| id != req.obj_id);
                        self.t2.push_back(req.obj_id);
                    }
                    
                    CacheResult::Hit { obj_size }
                }
                _ => {
                    self.stats.inc_miss();
                    CacheResult::Miss
                }
            }
        } else {
            self.stats.inc_miss();
            CacheResult::Miss
        }
    }

    fn insert(&mut self, req: &Request) -> InsertResult {
        if req.obj_size as ObjectSize > self.capacity {
            return InsertResult::Rejected;
        }

        // Check if in ghost lists
        let in_b1 = self.entries.get(&req.obj_id)
            .map(|e| e.location == Location::B1)
            .unwrap_or(false);
        let in_b2 = self.entries.get(&req.obj_id)
            .map(|e| e.location == Location::B2)
            .unwrap_or(false);

        if in_b1 {
            // Adapt p
            let delta = if self.b1.len() >= self.b2.len() { 1 } else { self.b2.len() / self.b1.len() };
            self.p = (self.p + delta as ObjectSize).min(self.capacity);
            self.replace(req.obj_size);
            
            // Move to T2
            self.b1.retain(|&id| id != req.obj_id);
            self.t2.push_back(req.obj_id);
            if let Some(entry) = self.entries.get_mut(&req.obj_id) {
                entry.location = Location::T2;
                entry.obj_size = req.obj_size;
            }
            self.current_size += req.obj_size as ObjectSize;
            self.stats.inc_insert();
            InsertResult::Inserted
        } else if in_b2 {
            // Adapt p
            let delta = if self.b2.len() >= self.b1.len() { 1 } else { self.b1.len() / self.b2.len() };
            self.p = self.p.saturating_sub(delta as ObjectSize);
            self.replace(req.obj_size);
            
            // Move to T2
            self.b2.retain(|&id| id != req.obj_id);
            self.t2.push_back(req.obj_id);
            if let Some(entry) = self.entries.get_mut(&req.obj_id) {
                entry.location = Location::T2;
                entry.obj_size = req.obj_size;
            }
            self.current_size += req.obj_size as ObjectSize;
            self.stats.inc_insert();
            InsertResult::Inserted
        } else {
            // New entry
            self.replace(req.obj_size);
            
            let entry = ArcEntry {
                obj_id: req.obj_id,
                obj_size: req.obj_size,
                location: Location::T1,
            };
            self.entries.insert(req.obj_id, entry);
            self.t1.push_back(req.obj_id);
            self.current_size += req.obj_size as ObjectSize;
            self.stats.inc_insert();
            InsertResult::Inserted
        }
    }

    fn evict(&mut self) -> Option<ObjectId> {
        // Evict from T1 first if non-empty
        if let Some(victim_id) = self.t1.pop_front() {
            if let Some(entry) = self.entries.remove(&victim_id) {
                self.current_size -= entry.obj_size as ObjectSize;
                self.stats.inc_evict();
                return Some(victim_id);
            }
        }
        
        // Otherwise evict from T2
        if let Some(victim_id) = self.t2.pop_front() {
            if let Some(entry) = self.entries.remove(&victim_id) {
                self.current_size -= entry.obj_size as ObjectSize;
                self.stats.inc_evict();
                return Some(victim_id);
            }
        }
        
        None
    }

    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(entry) = self.entries.remove(&obj_id) {
            self.current_size -= entry.obj_size as ObjectSize;
            
            match entry.location {
                Location::T1 => self.t1.retain(|&id| id != obj_id),
                Location::T2 => self.t2.retain(|&id| id != obj_id),
                Location::B1 => self.b1.retain(|&id| id != obj_id),
                Location::B2 => self.b2.retain(|&id| id != obj_id),
            }
            true
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.t1.clear();
        self.t2.clear();
        self.b1.clear();
        self.b2.clear();
        self.entries.clear();
        self.current_size = 0;
        self.p = 0;
    }

    fn size(&self) -> u64 {
        self.current_size as u64
    }

    fn capacity(&self) -> u64 {
        self.capacity as u64
    }

    fn len(&self) -> usize {
        self.t1.len() + self.t2.len()
    }

    fn stats(&self) -> &CacheStats {
        &self.stats
    }

    fn reset_stats(&mut self) {
        self.stats = CacheStats::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_basic() {
        let mut cache = ArcCache::new(300);
        
        let req1 = Request::new(1, 100);
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        cache.insert(&req1);
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.size(), 100);
    }

    #[test]
    fn test_arc_adaptation() {
        let mut cache = ArcCache::new(300);
        
        // Insert 3 objects
        for i in 1..=3 {
            let req = Request::new(i, 100);
            cache.insert(&req);
        }
        
        // Access first two twice (should be in T2)
        for i in 1..=2 {
            let req = Request::new(i, 100);
            cache.get(&req);
        }
        
        // Insert 4th object (should evict 3 from T1)
        let req4 = Request::new(4, 100);
        cache.insert(&req4);
        
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_arc_stats() {
        let mut cache = ArcCache::new(200);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        cache.insert(&req1);
        cache.get(&req1);  // hit
        cache.get(&req2);  // miss
        
        assert_eq!(cache.stats().n_hit(), 1);
        assert_eq!(cache.stats().n_miss(), 1);  // Only req2 miss
        assert_eq!(cache.stats().n_req(), 2);  // Two gets
    }
}
