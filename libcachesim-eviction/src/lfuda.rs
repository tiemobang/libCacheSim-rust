//! LFUDA (LFU with Dynamic Aging) cache implementation
//!
//! LFUDA improves LFU by adding dynamic aging to prevent cache pollution.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};

/// Entry in the LFUDA cache
#[derive(Debug, Clone)]
struct LfudaEntry {
    obj_size: u32,
    frequency: u64, // Key value = frequency + age
}

/// LFUDA (LFU with Dynamic Aging) cache implementation
///
/// LFUDA improves upon LFU by adding a dynamic aging factor. Each object
/// has a key value equal to its frequency plus the current cache age.
/// When an object is evicted, the cache age is updated to the evicted
/// object's key value. This prevents newly inserted objects from being
/// immediately evicted and helps the cache adapt to changing workloads.
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::LfudaCache;
///
/// let mut cache = LfudaCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct LfudaCache {
    capacity: u64,
    used: u64,
    objects: HashMap<ObjectId, LfudaEntry>,
    age: u64, // Current cache age
    stats: CacheStats,
}

impl LfudaCache {
    /// Create a new LFUDA cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        LfudaCache {
            capacity,
            used: 0,
            objects: HashMap::new(),
            age: 0,
            stats: CacheStats::new(),
        }
    }

    /// Find the object with minimum key value to evict
    fn find_min_key_object(&self) -> Option<(ObjectId, u64)> {
        self.objects
            .iter()
            .min_by_key(|(_, entry)| entry.frequency)
            .map(|(&id, entry)| (id, entry.frequency))
    }
}

impl Cache for LfudaCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();

        if let Some(entry) = self.objects.get_mut(&req.obj_id) {
            self.stats.inc_hit();
            entry.frequency += 1;
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
        // Check if already exists
        if let Some(entry) = self.objects.get_mut(&req.obj_id) {
            entry.frequency += 1;
            return InsertResult::Inserted;
        }

        // Check if object can fit
        if req.obj_size as u64 > self.capacity {
            return InsertResult::Rejected;
        }

        // Evict until there's space
        let mut evicted_id = None;
        while self.used + req.obj_size as u64 > self.capacity && !self.objects.is_empty() {
            if let Some(evict_id) = self.evict() {
                evicted_id = Some(evict_id);
            }
        }

        // Insert the object with initial frequency = current age + 1
        let entry = LfudaEntry {
            obj_size: req.obj_size,
            frequency: self.age + 1,
        };
        self.objects.insert(req.obj_id, entry);
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
        // Find object with minimum key value
        let (obj_id, min_key) = self.find_min_key_object()?;

        // Update age to the evicted object's key value
        self.age = min_key;

        if let Some(entry) = self.objects.remove(&obj_id) {
            self.used -= entry.obj_size as u64;
            self.stats.inc_evict();
            Some(obj_id)
        } else {
            None
        }
    }

    fn remove(&mut self, obj_id: ObjectId) -> bool {
        if let Some(entry) = self.objects.remove(&obj_id) {
            self.used -= entry.obj_size as u64;
            true
        } else {
            false
        }
    }

    fn clear(&mut self) {
        self.objects.clear();
        self.used = 0;
        self.age = 0;
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
    fn test_lfuda_basic() {
        let mut cache = LfudaCache::new(300);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);

        // First access - miss
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);

        // Check initial frequency includes age
        assert!(cache.objects.get(&1).unwrap().frequency >= 1);

        // Second access - hit, frequency increases
        let initial_freq = cache.objects.get(&1).unwrap().frequency;
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.objects.get(&1).unwrap().frequency, initial_freq + 1);

        // Add another object
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lfuda_aging() {
        let mut cache = LfudaCache::new(200);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);

        // Insert req1 and access it multiple times
        cache.get(&req1);
        cache.get(&req1);
        cache.get(&req1);

        // Insert req2
        cache.get(&req2);

        let initial_age = cache.age;

        // Insert req3 - will cause eviction
        cache.get(&req3);

        // Age should have increased after eviction
        assert!(cache.age >= initial_age);
    }

    #[test]
    fn test_lfuda_eviction() {
        let mut cache = LfudaCache::new(200);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);

        // Insert and access req1 multiple times (higher frequency)
        cache.get(&req1);
        cache.get(&req1);
        cache.get(&req1);

        // Insert req2 (lower frequency)
        cache.get(&req2);

        assert_eq!(cache.len(), 2);

        // Insert req3 - should trigger eviction
        cache.get(&req3);
        assert!(cache.len() <= 2);

        // Verify statistics
        assert!(cache.stats().n_evict() > 0);
    }

    #[test]
    fn test_lfuda_stats() {
        let mut cache = LfudaCache::new(200);

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
