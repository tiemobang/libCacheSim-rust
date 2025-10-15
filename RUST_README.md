# Rust Translation of libCacheSim

This directory contains a Rust translation of the libCacheSim C library for cache simulation.

## Current Status

This is an initial implementation providing:

1. **Core Cache Trait** - A Rust trait defining the interface for cache implementations
2. **FIFO Cache** - A complete Rust implementation of the First-In-First-Out cache eviction algorithm
3. **LRU Cache** - A complete Rust implementation of the Least Recently Used cache eviction algorithm
4. **Clock Cache** - A complete Rust implementation of the Clock algorithm (approximation of LRU using reference bits)
5. **Cache Statistics** - Track hits, misses, evictions, and calculate miss/hit ratios
6. **Workload Simulator** - Helper function to simulate cache workloads
7. **Comprehensive Tests** - Full test coverage with 9 passing tests

## Architecture

The Rust implementation follows these design principles:

- **Type Safety**: Uses Rust's strong type system (`ObjId`, `ObjSize` type aliases)
- **Zero-cost Abstractions**: Trait-based design for cache algorithms
- **Memory Safety**: No unsafe code, leverages Rust's ownership system
- **Idiomatic Rust**: Uses standard library collections (`HashMap`, `VecDeque`, `Vec`)

## Testing

Run all tests with:
```bash
cargo test
```

All tests pass successfully:
- `test_fifo_basic` - Basic FIFO operations
- `test_fifo_eviction` - FIFO eviction behavior  
- `test_lru_basic` - Basic LRU operations
- `test_lru_eviction` - LRU eviction behavior with recency
- `test_clock_basic` - Basic Clock algorithm operations
- `test_clock_eviction` - Clock algorithm reference bit behavior
- `test_cache_stats` - Cache statistics tracking
- `test_simulate_workload` - Workload simulation
- `test_remove` - Object removal

## Usage Example

```rust
use libcachesim::{Cache, FifoCache, LruCache, ClockCache, Request, simulate_cache};

// Create a 1MB FIFO cache
let mut cache = FifoCache::new(1024 * 1024);

// Access objects
let req = Request::new(1, 1024);
let hit = cache.get(&req);  // false - cache miss

// Access again
let hit = cache.get(&req);  // true - cache hit

// Get statistics
let stats = cache.get_stats();
println!("Miss ratio: {:.2}%", stats.miss_ratio() * 100.0);

// Simulate a complete workload
let mut lru_cache = LruCache::new(1024 * 1024);
let requests = vec![
    Request::new(1, 1024),
    Request::new(2, 2048),
    Request::new(1, 1024), // hit
];
let stats = simulate_cache(&mut lru_cache, &requests);
println!("Hits: {}, Misses: {}", stats.n_hit, stats.n_miss);
```

## Implemented Algorithms

### Complete Implementations
- ✅ **FIFO** (First-In-First-Out) - Basic queue-based eviction
- ✅ **LRU** (Least Recently Used) - Evicts least recently accessed items
- ✅ **Clock** - Second-chance algorithm with reference bits

### To Be Implemented
The original C library includes ~30 cache algorithms. Future work includes:
- [ ] ARC (Adaptive Replacement Cache)
- [ ] TwoQ
- [ ] LFU, LFUDA (Least Frequently Used with Dynamic Aging)
- [ ] S3-FIFO (Simple, Scalable, Size-aware FIFO)
- [ ] SIEVE
- [ ] QDLP
- [ ] And many more...

## Future Work

To complete the full translation of libCacheSim, the following components would need to be implemented:

### Data Structures (libCacheSim/dataStructure/)
- [ ] Hash tables (currently using std HashMap)
- [ ] Priority queues
- [ ] Bloom filters
- [ ] Splay trees

### Trace Reading (libCacheSim/traceReader/)
- [ ] CSV reader
- [ ] Binary format readers
- [ ] Compression support (zstd)

### Analysis Tools (libCacheSim/traceAnalyzer/)
- [ ] Reuse distance analysis
- [ ] Miss ratio curve generation
- [ ] SHARDS sampling

### Binary Tools (libCacheSim/bin/)
- [ ] Command-line cache simulator
- [ ] MRC profiler

## Building C Library (Original)

The original C library can still be built using CMake:
```bash
mkdir _build && cd _build
cmake -G Ninja ..
ninja
ctest
```

## Performance

The Rust implementation uses standard library collections which provide good performance characteristics:
- HashMap: O(1) average case lookup/insert
- VecDeque: O(1) push/pop at both ends
- Vec: O(1) indexed access

For production use, custom data structures could be implemented for further optimization.

## Design Decisions

1. **Pure Rust Implementation**: Rather than using FFI bindings to the C code, this is a native Rust implementation. This provides better type safety, memory safety, and idiomatic Rust code.

2. **Simplified API**: The API is streamlined compared to the C version, focusing on the core cache operations while maintaining the same algorithmic behavior.

3. **Standard Collections**: Uses Rust's standard library `HashMap`, `VecDeque`, and `Vec` instead of custom data structures, providing good performance while maintaining simplicity.

4. **Statistics Built-in**: Cache statistics are tracked automatically without requiring separate counters, making the API easier to use.

## License

Same as the original libCacheSim project - see LICENSE file in the repository root.

