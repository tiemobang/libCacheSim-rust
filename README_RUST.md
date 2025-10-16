# libCacheSim Rust Port - Phase 1 Complete

A high-performance cache simulation library written in Rust, ported from the original C implementation.

## Features

### ✅ Phase 1 Complete (Weeks 1-10)

- **8 Cache Eviction Algorithms**
  - FIFO (First-In-First-Out)
  - LRU (Least Recently Used)
  - Clock (Second-Chance)
  - MRU (Most Recently Used)
  - Random
  - SLRU (Segmented LRU)
  - LFU (Least Frequently Used)
  - LFUDA (LFU with Dynamic Aging)

- **Trace Reading**
  - CSV format reader
  - LCS binary format reader (v1, v2, v3)
  - Auto-detection of formats
  - Iterator-based interface

- **Core Infrastructure**
  - Thread-safe atomic statistics
  - Type-safe error handling
  - Builder patterns
  - Comprehensive testing

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
libcachesim = { path = "../libcachesim" }
```

### Basic Usage

```rust
use libcachesim::{Cache, LruCache, Request};

// Create a cache with 1MB capacity
let mut cache = LruCache::new(1024 * 1024);

// Simulate cache accesses
let req = Request::new(1, 100);
cache.get(&req);

// Get statistics
let stats = cache.stats();
println!("Hit ratio: {:.2}%", stats.hit_ratio() * 100.0);
```

### Trace-based Simulation

```rust
use libcachesim::{Cache, FifoCache};
use libcachesim_trace::TraceReaderBuilder;

let mut cache = FifoCache::new(1024 * 1024);
let reader = TraceReaderBuilder::new("trace.csv").build()?;

for result in reader {
    let req = result?;
    cache.get(&req);
}

println!("Final hit ratio: {:.2}%", cache.stats().hit_ratio() * 100.0);
```

## Testing

```bash
# Run all tests
cargo test --workspace

# Run benchmarks
cargo bench -p libcachesim-eviction

# Run examples
cargo run --example basic_simulation
cargo run --example trace_simulation
```

## Project Structure

```
libCacheSim-rust/
├── libcachesim-core/       # Core types and traits
├── libcachesim-eviction/   # Eviction algorithms
├── libcachesim-trace/      # Trace I/O
├── libcachesim-datastructures/  # Custom data structures (future)
├── libcachesim-analysis/   # Analysis tools (future)
├── libcachesim-cli/        # CLI tools (future)
└── libcachesim/            # Main library
```

## Performance

Phase 1 implementation achieves:
- **34 unit tests** - 100% passing
- **10 doc tests** - 100% passing  
- **2 examples** - Working demonstrations
- **100% safe Rust** - Zero unsafe blocks
- **~2,800 lines** of implementation code

## Roadmap

### Phase 2: Advanced Algorithms (Months 4-6)
- ARC, CAR, TwoQ, LeCaR
- S3-FIFO, Sieve
- ClockPro, LIRS, QDLP

### Phase 3: ML Algorithms (Months 7-8)
- GLCache with XGBoost
- LRB, LHD

### Phase 4+: Data Structures, Analysis, CLI Tools

See [RUST_PORT_PROJECT_PLAN.md](RUST_PORT_PROJECT_PLAN.md) for complete roadmap.

## License

GPL-3.0 - See [LICENSE](LICENSE) for details.

## Contributing

This is a port of the original libCacheSim C library. Contributions following the Rust idioms and maintaining compatibility are welcome.

## Original Project

Based on [libCacheSim](https://github.com/1a1a11a/libCacheSim) by Juncheng Yang and contributors.
