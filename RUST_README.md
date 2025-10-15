# Rust Translation of libCacheSim

This directory contains a Rust translation of the libCacheSim C library for cache simulation.

## Current Status

This is an initial implementation providing:

1. **Core Cache Trait** - A Rust trait defining the interface for cache implementations
2. **FIFO Cache** - A complete Rust implementation of the First-In-First-Out cache eviction algorithm
3. **LRU Cache** - A complete Rust implementation of the Least Recently Used cache eviction algorithm
4. **Comprehensive Tests** - Full test coverage for both cache implementations

## Architecture

The Rust implementation follows these design principles:

- **Type Safety**: Uses Rust's strong type system (`ObjId`, `ObjSize` type aliases)
- **Zero-cost Abstractions**: Trait-based design for cache algorithms
- **Memory Safety**: No unsafe code, leverages Rust's ownership system
- **Idiomatic Rust**: Uses standard library collections (`HashMap`, `VecDeque`)

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
- `test_remove` - Object removal

## Usage Example

```rust
use libcachesim::{Cache, FifoCache, LruCache, Request};

// Create a 1MB FIFO cache
let mut cache = FifoCache::new(1024 * 1024);

// Access objects
let req = Request::new(1, 1024);
let hit = cache.get(&req);  // false - cache miss

// Access again
let hit = cache.get(&req);  // true - cache hit
```

## Future Work

To complete the full translation of libCacheSim, the following components would need to be implemented:

### Cache Algorithms (libCacheSim/cache/eviction/)
- [ ] ARC, Clock, ClockPro
- [ ] LFU, LFUDA
- [ ] TwoQ, Belady
- [ ] S3-FIFO, SIEVE, QDLP
- [ ] And ~20 more algorithms

### Data Structures (libCacheSim/dataStructure/)
- [ ] Hash tables
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

## Design Decisions

1. **Pure Rust Implementation**: Rather than using FFI bindings to the C code, this is a native Rust implementation. This provides better type safety, memory safety, and idiomatic Rust code.

2. **Simplified API**: The API is streamlined compared to the C version, focusing on the core cache operations while maintaining the same algorithmic behavior.

3. **Standard Collections**: Uses Rust's standard library `HashMap` and `VecDeque` instead of custom data structures, providing good performance while maintaining simplicity.

## License

Same as the original libCacheSim project - see LICENSE file in the repository root.
