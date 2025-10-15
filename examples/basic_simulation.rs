// Example: Basic cache simulation comparing FIFO, LRU, and Clock algorithms

use libcachesim::{ClockCache, FifoCache, LruCache, Request, simulate_cache};

fn main() {
    println!("=== libCacheSim-rust Example ===\n");

    // Create a workload with some repeated accesses
    // Objects 1, 2, 3 fit in cache, then we access 4, 5, 6
    // Then we access 1, 2, 3 again to show difference between algorithms
    let workload = vec![
        Request::new(1, 100), // miss, insert 1
        Request::new(2, 100), // miss, insert 2
        Request::new(3, 100), // miss, insert 3 (cache full)
        Request::new(1, 100), // hit (1 already in cache)
        Request::new(2, 100), // hit (2 already in cache)
        Request::new(4, 100), // miss, evict and insert 4
        Request::new(5, 100), // miss, evict and insert 5
        Request::new(1, 100), // depends on algorithm
        Request::new(2, 100), // depends on algorithm
        Request::new(3, 100), // depends on algorithm
    ];

    println!("Workload: {} requests", workload.len());
    println!("Cache size: 300 bytes (can hold 3 objects of 100 bytes each)\n");
    println!("Access pattern: 1,2,3,1,2,4,5,1,2,3");
    println!("  - First 3 accesses: all misses (cold start)");
    println!("  - Accesses to 1,2: hits");
    println!("  - Accesses to 4,5: misses (require eviction)");
    println!("  - Final accesses to 1,2,3: depends on algorithm\n");

    // Test FIFO cache
    let mut fifo = FifoCache::new(300);
    let fifo_stats = simulate_cache(&mut fifo, &workload);
    println!("FIFO Cache:");
    println!("  Hits: {}, Misses: {}", fifo_stats.n_hit, fifo_stats.n_miss);
    println!(
        "  Hit ratio: {:.1}%, Miss ratio: {:.1}%",
        fifo_stats.hit_ratio() * 100.0,
        fifo_stats.miss_ratio() * 100.0
    );
    println!("  Evictions: {}", fifo_stats.n_evict);
    println!("  (FIFO doesn't consider recency, evicts oldest entries)\n");

    // Test LRU cache
    let mut lru = LruCache::new(300);
    let lru_stats = simulate_cache(&mut lru, &workload);
    println!("LRU Cache:");
    println!("  Hits: {}, Misses: {}", lru_stats.n_hit, lru_stats.n_miss);
    println!(
        "  Hit ratio: {:.1}%, Miss ratio: {:.1}%",
        lru_stats.hit_ratio() * 100.0,
        lru_stats.miss_ratio() * 100.0
    );
    println!("  Evictions: {}", lru_stats.n_evict);
    println!("  (LRU considers recency, keeps recently used items)\n");

    // Test Clock cache
    let mut clock = ClockCache::new(300);
    let clock_stats = simulate_cache(&mut clock, &workload);
    println!("Clock Cache:");
    println!(
        "  Hits: {}, Misses: {}",
        clock_stats.n_hit, clock_stats.n_miss
    );
    println!(
        "  Hit ratio: {:.1}%, Miss ratio: {:.1}%",
        clock_stats.hit_ratio() * 100.0,
        clock_stats.miss_ratio() * 100.0
    );
    println!("  Evictions: {}", clock_stats.n_evict);
    println!("  (Clock approximates LRU with reference bits)\n");

    // Compare results
    println!("=== Comparison ===");
    let best = if lru_stats.hit_ratio() > fifo_stats.hit_ratio()
        && lru_stats.hit_ratio() >= clock_stats.hit_ratio()
    {
        "LRU"
    } else if clock_stats.hit_ratio() > fifo_stats.hit_ratio() {
        "Clock"
    } else {
        "FIFO"
    };
    println!("Best hit ratio: {}", best);
    println!(
        "\nFor this workload with temporal locality, {} performs best.",
        best
    );
}

