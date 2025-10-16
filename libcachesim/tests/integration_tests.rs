//! Integration tests for libcachesim
//!
//! Tests the full library API and integration between components

use libcachesim::{Cache, CacheResult, FifoCache, LruCache, Request};

#[test]
fn test_basic_cache_workflow() {
    let mut cache = FifoCache::new(300);

    // Insert objects
    let req1 = Request::new(1, 100);
    let req2 = Request::new(2, 100);
    let req3 = Request::new(3, 100);

    // All should be misses initially
    assert_eq!(cache.get(&req1), CacheResult::Miss);
    assert_eq!(cache.get(&req2), CacheResult::Miss);
    assert_eq!(cache.get(&req3), CacheResult::Miss);

    // Access them again - should be hits
    assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
    assert_eq!(cache.get(&req2), CacheResult::Hit { obj_size: 100 });
    assert_eq!(cache.get(&req3), CacheResult::Hit { obj_size: 100 });

    // Stats should reflect this
    let stats = cache.stats();
    assert_eq!(stats.n_req(), 6);
    assert_eq!(stats.n_hit(), 3);
    assert_eq!(stats.n_miss(), 3);
    assert_eq!(stats.hit_ratio(), 0.5);
}

#[test]
fn test_cache_eviction() {
    let mut cache = LruCache::new(200);

    let req1 = Request::new(1, 100);
    let req2 = Request::new(2, 100);
    let req3 = Request::new(3, 100);

    // Fill cache
    cache.get(&req1);
    cache.get(&req2);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.size(), 200);

    // Access req1 to make it recent
    cache.get(&req1);

    // Add req3 - should evict req2
    cache.get(&req3);

    // Verify eviction happened
    assert!(cache.stats().n_evict() > 0);

    // req1 and req3 should be present
    assert_eq!(cache.get(&req1), CacheResult::Hit { obj_size: 100 });
    assert_eq!(cache.get(&req3), CacheResult::Hit { obj_size: 100 });
}

#[test]
fn test_cache_capacity() {
    let mut cache = FifoCache::new(1000);

    assert_eq!(cache.capacity(), 1000);
    assert_eq!(cache.size(), 0);
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());

    // Add one object
    cache.get(&Request::new(1, 100));
    assert_eq!(cache.size(), 100);
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
}

#[test]
fn test_cache_clear() {
    let mut cache = FifoCache::new(1000);

    // Add some objects
    cache.get(&Request::new(1, 100));
    cache.get(&Request::new(2, 200));
    cache.get(&Request::new(3, 300));

    assert_eq!(cache.len(), 3);
    assert_eq!(cache.size(), 600);

    // Clear
    cache.clear();

    assert_eq!(cache.len(), 0);
    assert_eq!(cache.size(), 0);
    assert!(cache.is_empty());
}

#[test]
fn test_cache_remove() {
    let mut cache = FifoCache::new(1000);

    cache.get(&Request::new(1, 100));
    cache.get(&Request::new(2, 200));

    assert_eq!(cache.len(), 2);

    // Remove object 1
    assert!(cache.remove(1));
    assert_eq!(cache.len(), 1);

    // Remove non-existent object
    assert!(!cache.remove(999));
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_multiple_cache_types() {
    let workload: Vec<Request> = vec![
        Request::new(1, 100),
        Request::new(2, 100),
        Request::new(1, 100), // Hit
        Request::new(3, 100),
        Request::new(2, 100), // Hit
    ];

    // Test with FIFO
    let mut fifo = FifoCache::new(300);
    for req in &workload {
        fifo.get(req);
    }
    assert_eq!(fifo.stats().n_hit(), 2);

    // Test with LRU
    let mut lru = LruCache::new(300);
    for req in &workload {
        lru.get(req);
    }
    assert_eq!(lru.stats().n_hit(), 2);
}
