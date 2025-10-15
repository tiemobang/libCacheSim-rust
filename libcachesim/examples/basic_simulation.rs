//! Basic example demonstrating cache usage and comparing algorithms

use libcachesim::{Cache, ClockCache, FifoCache, LruCache, MruCache, RandomCache, Request};

fn main() {
    println!("=== libCacheSim Rust - Phase 1 Implementation ===\n");
    
    // Create a simple workload
    let workload = vec![
        Request::new(1, 100),
        Request::new(2, 100),
        Request::new(3, 100),
        Request::new(1, 100), // hit
        Request::new(4, 100),
        Request::new(2, 100), // hit
        Request::new(5, 100),
        Request::new(3, 100), // might be hit depending on algorithm
        Request::new(6, 100),
        Request::new(1, 100), // might be hit depending on algorithm
    ];
    
    let cache_size = 300; // Can hold 3 objects of 100 bytes each
    
    println!("Workload: {} requests", workload.len());
    println!("Cache size: {} bytes (can hold 3 objects of 100 bytes each)\n", cache_size);
    
    // Test FIFO
    println!("FIFO Cache:");
    run_simulation(FifoCache::new(cache_size), &workload);
    
    // Test LRU
    println!("\nLRU Cache:");
    run_simulation(LruCache::new(cache_size), &workload);
    
    // Test Clock
    println!("\nClock Cache:");
    run_simulation(ClockCache::new(cache_size), &workload);
    
    // Test MRU
    println!("\nMRU Cache:");
    run_simulation(MruCache::new(cache_size), &workload);
    
    // Test Random
    println!("\nRandom Cache:");
    run_simulation(RandomCache::new(cache_size), &workload);
}

fn run_simulation<C: Cache>(mut cache: C, workload: &[Request]) {
    for req in workload {
        cache.get(req);
    }
    
    let stats = cache.stats();
    println!("  Hits: {}, Misses: {}", stats.n_hit(), stats.n_miss());
    println!("  Hit ratio: {:.1}%, Miss ratio: {:.1}%", 
             stats.hit_ratio() * 100.0, stats.miss_ratio() * 100.0);
    println!("  Evictions: {}", stats.n_evict());
}
