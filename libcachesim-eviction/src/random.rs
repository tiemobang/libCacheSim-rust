//! Random eviction cache implementation
//!
//! Random eviction selects a random object to evict when the cache is full.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Random eviction cache implementation
///
/// Evicts a randomly selected object when the cache is full.
/// Despite its simplicity, random eviction can perform surprisingly well in practice.
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::RandomCache;
///
/// let mut cache = RandomCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct RandomCache {
    capacity: u64,
    used: u64,
    objects: HashMap<ObjectId, u32>,
    keys: Vec<ObjectId>,
    stats: CacheStats,
}

impl RandomCache {
    /// Create a new Random cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        RandomCache {
            capacity,
            used: 0,
            objects: HashMap::new(),
            keys: Vec::new(),
            stats: CacheStats::new(),
        }
    }
}

impl Cache for RandomCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();

        if let Some(&obj_size) = self.objects.get(&req.obj_id) {
            self.stats.inc_hit();
            CacheResult::Hit { obj_size }
        } else {
            self.stats.inc_miss();
            self.insert(req);
            CacheResult::Miss
        }
    }

    fn insert(&mut self, req: &Request) -> InsertResult {
        // Check if object already exists
        if self.objects.contains_key(&req.obj_id) {
            return InsertResult::Inserted;
        }

        // Evict objects if necessary to make room
        let mut evicted_id = None;
        while self.used + req.obj_size as u64 > self.capacity && !self.keys.is_empty() {
            if let Some(evict_id) = self.evict() {
                evicted_id = Some(evict_id);
            }
        }

        // Check if object can fit
        if req.obj_size as u64 > self.capacity {
            return InsertResult::Rejected;
        }

        // Insert the object
        self.objects.insert(req.obj_id, req.obj_size);
        self.keys.push(req.obj_id);
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
        if self.keys.is_empty() {
            return None;
        }

        // Select a random key
        let mut rng = thread_rng();
        let index = *self.keys.choose(&mut rng)?;

        // Find and remove the key
        if let Some(pos) = self.keys.iter().position(|&id| id == index) {
            let obj_id = self.keys.remove(pos);
            if let Some(obj_size) = self.objects.remove(&obj_id) {
                self.used -= obj_size as u64;
                self.stats.inc_evict();
                return Some(obj_id);
            }
        }

        None
    }

    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(obj_size) = self.objects.remove(&obj_id) {
            self.used -= obj_size as u64;
            // Remove from keys
            if let Some(pos) = self.keys.iter().position(|&id| id == obj_id) {
                self.keys.remove(pos);
            }
            true
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.objects.clear();
        self.keys.clear();
        self.used = 0;
    }

    fn size(&self) -> u64 {
        self.used
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
    fn test_random_basic() {
        let mut cache = RandomCache::new(300);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);

        // First access - miss
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);

        // Second access - hit
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.len(), 1);

        // Add another object
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_random_eviction() {
        let mut cache = RandomCache::new(200);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);

        // Fill cache
        cache.get(&req1);
        cache.get(&req2);
        assert_eq!(cache.len(), 2);

        // This should evict a random object
        cache.get(&req3);
        assert!(cache.len() <= 2);
        assert!(cache.size() <= cache.capacity());

        // Verify stats
        assert_eq!(cache.stats().n_evict(), 1);
    }

    #[test]
    fn test_random_stats() {
        let mut cache = RandomCache::new(200);

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
