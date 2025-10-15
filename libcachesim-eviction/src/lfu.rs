//! LFU (Least Frequently Used) cache implementation
//!
//! LFU evicts the least frequently accessed object when the cache is full.

use hashbrown::HashMap;
use libcachesim_core::{Cache, CacheResult, CacheStats, InsertResult, ObjectId, Request};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Entry in the LFU cache with frequency tracking
#[derive(Debug, Clone)]
struct LfuEntry {
    obj_id: ObjectId,
    obj_size: u32,
    frequency: u32,
    last_access: u64, // For tie-breaking
}

impl PartialEq for LfuEntry {
    fn eq(&self, other: &Self) -> bool {
        self.frequency == other.frequency && self.last_access == other.last_access
    }
}

impl Eq for LfuEntry {}

impl PartialOrd for LfuEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LfuEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // For min-heap: higher frequency = lower priority (we want to evict low frequency)
        // So we reverse the comparison
        match other.frequency.cmp(&self.frequency) {
            Ordering::Equal => other.last_access.cmp(&self.last_access), // Older = higher priority to evict
            ord => ord,
        }
    }
}

/// LFU (Least Frequently Used) cache implementation
///
/// LFU tracks access frequency for each object and evicts the least
/// frequently accessed object when the cache is full. In case of ties,
/// evicts the least recently used among objects with the same frequency.
///
/// # Examples
///
/// ```
/// use libcachesim_core::{Cache, Request};
/// use libcachesim_eviction::LfuCache;
///
/// let mut cache = LfuCache::new(1024);
/// let req = Request::new(1, 100);
/// cache.get(&req);
/// ```
pub struct LfuCache {
    capacity: u64,
    used: u64,
    objects: HashMap<ObjectId, LfuEntry>,
    access_counter: u64,
    stats: CacheStats,
}

impl LfuCache {
    /// Create a new LFU cache with the given capacity in bytes
    pub fn new(capacity: u64) -> Self {
        LfuCache {
            capacity,
            used: 0,
            objects: HashMap::new(),
            access_counter: 0,
            stats: CacheStats::new(),
        }
    }

    /// Find the object with minimum frequency to evict
    fn find_min_frequency_object(&self) -> Option<ObjectId> {
        let mut heap = BinaryHeap::new();
        for entry in self.objects.values() {
            heap.push(entry.clone());
        }
        heap.pop().map(|entry| entry.obj_id)
    }
}

impl Cache for LfuCache {
    fn get(&mut self, req: &Request) -> CacheResult {
        self.stats.inc_req();
        self.access_counter += 1;

        if let Some(entry) = self.objects.get_mut(&req.obj_id) {
            self.stats.inc_hit();
            entry.frequency += 1;
            entry.last_access = self.access_counter;
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
        self.access_counter += 1;

        // Check if already exists
        if let Some(entry) = self.objects.get_mut(&req.obj_id) {
            entry.frequency += 1;
            entry.last_access = self.access_counter;
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

        // Insert the object
        let entry = LfuEntry {
            obj_id: req.obj_id,
            obj_size: req.obj_size,
            frequency: 1,
            last_access: self.access_counter,
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
        // Find object with minimum frequency
        let obj_id = self.find_min_frequency_object()?;

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
        self.access_counter = 0;
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
    fn test_lfu_basic() {
        let mut cache = LfuCache::new(300);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);

        // First access - miss
        assert_eq!(cache.get(&req1), CacheResult::Miss);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.objects.get(&1).unwrap().frequency, 1);

        // Second access - hit, frequency increases
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
        assert_eq!(cache.objects.get(&1).unwrap().frequency, 2);

        // Add another object
        assert_eq!(cache.get(&req2), CacheResult::Miss);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_lfu_eviction_by_frequency() {
        let mut cache = LfuCache::new(200);

        let req1 = Request::new(1, 100);
        let req2 = Request::new(2, 100);
        let req3 = Request::new(3, 100);

        // Insert req1 and access it multiple times (high frequency)
        cache.get(&req1); // freq=1
        cache.get(&req1); // freq=2
        cache.get(&req1); // freq=3

        // Insert req2 (low frequency)
        cache.get(&req2); // freq=1

        assert_eq!(cache.len(), 2);

        // Insert req3 - should evict req2 (lowest frequency)
        cache.get(&req3);
        assert!(cache.len() <= 2);

        // req1 should still be there (high frequency)
        assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
    }

    #[test]
    fn test_lfu_stats() {
        let mut cache = LfuCache::new(200);

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
