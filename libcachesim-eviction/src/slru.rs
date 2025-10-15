//! SLRU (Segmented LRU) cache implementation
//!
//! SLRU divides the cache into two segments: probation and protected.
//! Objects start in probation, and on re-access are promoted to protected.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};
use std::collections::VecDeque;

/// Entry location in SLRU
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Probation,
    Protected,
}

/// SLRU (Segmented LRU) cache implementation
///
/// SLRU maintains two LRU segments:
/// - Probation segment: For newly inserted objects
/// - Protected segment: For frequently accessed objects
///
/// On cache hit:
/// - Objects in probation are promoted to protected
/// - Objects in protected remain there (moved to MRU position)
///
/// On eviction:
/// - First try to evict from probation
/// - If probation is empty, demote LRU from protected to probation and evict from probation
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::SlruCache;
///
/// let mut cache = SlruCache::new(1024, 0.2); // 20% for probation, 80% for protected
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct SlruCache {
    capacity: u64,
    probation_capacity: u64,
    protected_capacity: u64,
    probation_used: u64,
    protected_used: u64,
    objects: HashMap<ObjectId, (u32, Segment)>,
    probation_queue: VecDeque<ObjectId>,
    protected_queue: VecDeque<ObjectId>,
    stats: CacheStats,
}

impl SlruCache {
    /// Create a new SLRU cache with the given capacity and probation ratio
    ///
    /// # Arguments
    /// * `capacity` - Total cache capacity in bytes
    /// * `probation_ratio` - Fraction of capacity for probation segment (0.0 to 1.0)
    ///
    /// Common values: 0.2 (20% probation, 80% protected)
    pub fn new(capacity: u64, probation_ratio: f64) -> Self {
        let probation_ratio = probation_ratio.clamp(0.0, 1.0);
        let probation_capacity = (capacity as f64 * probation_ratio) as u64;
        let protected_capacity = capacity - probation_capacity;
        
        SlruCache {
            capacity,
            probation_capacity,
            protected_capacity,
            probation_used: 0,
            protected_used: 0,
            objects: HashMap::new(),
            probation_queue: VecDeque::new(),
            protected_queue: VecDeque::new(),
            stats: CacheStats::new(),
        }
    }
    
    /// Move an object to the protected segment
    fn promote_to_protected(&mut self, obj_id: ObjectId, obj_size: u32) {
        // Remove from probation
        if let Some(pos) = self.probation_queue.iter().position(|&id| id == obj_id) {
            self.probation_queue.remove(pos);
            self.probation_used -= obj_size as u64;
        }
        
        // Add to protected
        self.protected_queue.push_back(obj_id);
        self.protected_used += obj_size as u64;
        
        // Update segment
        if let Some((_, segment)) = self.objects.get_mut(&obj_id) {
            *segment = Segment::Protected;
        }
    }
    
    /// Move an object to MRU position in protected segment
    fn touch_protected(&mut self, obj_id: ObjectId) {
        if let Some(pos) = self.protected_queue.iter().position(|&id| id == obj_id) {
            self.protected_queue.remove(pos);
            self.protected_queue.push_back(obj_id);
        }
    }
    
    /// Evict from probation segment
    fn evict_from_probation(&mut self) -> Option<ObjectId> {
        if let Some(obj_id) = self.probation_queue.pop_front() {
            if let Some((obj_size, _)) = self.objects.remove(&obj_id) {
                self.probation_used -= obj_size as u64;
                self.stats.inc_evict();
                return Some(obj_id);
            }
        }
        None
    }
    
    /// Demote from protected to probation, then evict from probation
    fn demote_and_evict(&mut self) -> Option<ObjectId> {
        // Move LRU from protected to probation
        if let Some(obj_id) = self.protected_queue.pop_front() {
            if let Some((obj_size, segment)) = self.objects.get_mut(&obj_id) {
                let size = *obj_size;
                *segment = Segment::Probation;
                self.protected_used -= size as u64;
                self.probation_used += size as u64;
                self.probation_queue.push_back(obj_id);
            }
        }
        
        // Now evict from probation
        self.evict_from_probation()
    }
}

impl Cache for SlruCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();
        
        if let Some(&(obj_size, segment)) = self.objects.get(&req.obj_id) {
            self.stats.inc_hit();
            
            match segment {
                Segment::Probation => {
                    // Promote to protected on hit
                    self.promote_to_protected(req.obj_id, obj_size);
                }
                Segment::Protected => {
                    // Move to MRU in protected
                    self.touch_protected(req.obj_id);
                }
            }
            
            CacheResult::Hit { obj_size }
        } else {
            self.stats.inc_miss();
            self.insert(req);
            CacheResult::Miss
        }
    }
    
    fn insert(&mut self, req: &Request) -> InsertResult {
        // Check if already exists
        if self.objects.contains_key(&req.obj_id) {
            return InsertResult::Inserted;
        }
        
        // Check if object can fit
        if req.obj_size as u64 > self.capacity {
            return InsertResult::Rejected;
        }
        
        // Evict until there's space
        let mut evicted_id = None;
        while self.probation_used + self.protected_used + req.obj_size as u64 > self.capacity {
            // Try probation first
            if !self.probation_queue.is_empty() {
                if let Some(id) = self.evict_from_probation() {
                    evicted_id = Some(id);
                }
            } else if !self.protected_queue.is_empty() {
                if let Some(id) = self.demote_and_evict() {
                    evicted_id = Some(id);
                }
            } else {
                break;
            }
        }
        
        // Insert into probation segment
        self.objects.insert(req.obj_id, (req.obj_size, Segment::Probation));
        self.probation_queue.push_back(req.obj_id);
        self.probation_used += req.obj_size as u64;
        self.stats.inc_insert();
        
        if let Some(evicted) = evicted_id {
            InsertResult::Evicted { evicted_id: evicted }
        } else {
            InsertResult::Inserted
        }
    }
    
    fn evict(&mut self) -> Option<ObjectId> {
        if !self.probation_queue.is_empty() {
            self.evict_from_probation()
        } else {
            self.demote_and_evict()
        }
    }
    
    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some((obj_size, segment)) = self.objects.remove(&obj_id) {
            match segment {
                Segment::Probation => {
                    self.probation_used -= obj_size as u64;
                    if let Some(pos) = self.probation_queue.iter().position(|&id| id == obj_id) {
                        self.probation_queue.remove(pos);
                    }
                }
                Segment::Protected => {
                    self.protected_used -= obj_size as u64;
                    if let Some(pos) = self.protected_queue.iter().position(|&id| id == obj_id) {
                        self.protected_queue.remove(pos);
                    }
                }
            }
            true
        } else {
            false
        }
    }
    
    fn clear(&mut self) {
        self.objects.clear();
        self.probation_queue.clear();
        self.protected_queue.clear();
        self.probation_used = 0;
        self.protected_used = 0;
    }
    
    fn size(&self) -> u64 {
        self.probation_used + self.protected_used
    }
    
    fn capacity(&self) -> u64 {
        self.capacity
    }
    
    fn len(&self) -> usize {
        self.objects.len()
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
    fn test_slru_basic() {
        let mut cache = SlruCache::new(300, 0.33); // ~100 probation, ~200 protected
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        // First access - miss, goes to probation
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.objects.get(&1), Some(&(100, Segment::Probation)));
        
        // Second access - hit, promotes to protected
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.objects.get(&1), Some(&(100, Segment::Protected)));
        
        // Add another object
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.len(), 2);
    }
    
    #[test]
    fn test_slru_promotion() {
        let mut cache = SlruCache::new(300, 0.33);
        
        let req1 = Request::new(1, 100);
        
        // Insert and check it's in probation
        cache.get(&req1);
        assert_eq!(cache.objects.get(&1), Some(&(100, Segment::Probation)));
        assert_eq!(cache.probation_used, 100);
        assert_eq!(cache.protected_used, 0);
        
        // Access again - should promote to protected
        cache.get(&req1);
        assert_eq!(cache.objects.get(&1), Some(&(100, Segment::Protected)));
        assert_eq!(cache.probation_used, 0);
        assert_eq!(cache.protected_used, 100);
    }
    
    #[test]
    fn test_slru_eviction() {
        let mut cache = SlruCache::new(250, 0.4); // ~100 probation, ~150 protected
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);
        
        // Fill probation
        cache.get(&req1);
        assert_eq!(cache.len(), 1);
        
        // Promote req1 to protected
        cache.get(&req1);
        
        // Add req2 (goes to probation)
        cache.get(&req2);
        assert_eq!(cache.len(), 2);
        
        // Add req3 - should evict req2 from probation
        cache.get(&req3);
        assert!(cache.len() <= 2);
        
        // req1 should still be there (in protected)
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
    }
    
    #[test]
    fn test_slru_stats() {
        let mut cache = SlruCache::new(200, 0.5);
        
        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        
        cache.get(&req1); // miss
        cache.get(&req1); // hit (promotion)
        cache.get(&req2); // miss
        cache.get(&req2); // hit (promotion)
        
        let stats = cache.stats();
        assert_eq!(stats.n_req(), 4);
        assert_eq!(stats.n_hit(), 2);
        assert_eq!(stats.n_miss(), 2);
        assert_eq!(stats.hit_ratio(), 0.5);
    }
}
