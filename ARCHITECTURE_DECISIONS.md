# Architecture Decision Records (ADR)

This document records the key architectural decisions made for the libCacheSim Rust port.

## ADR-001: Pure Rust Implementation vs FFI Bindings

**Status:** Accepted

**Context:**
We need to decide between:
1. Creating Rust FFI bindings to wrap the existing C library
2. Rewriting the library entirely in Rust

**Decision:**
We will implement a pure Rust rewrite rather than FFI bindings.

**Rationale:**

**Advantages of Pure Rust:**
- ✅ **Memory Safety:** No undefined behavior, no memory leaks, no use-after-free
- ✅ **Thread Safety:** Rust's ownership system prevents data races at compile time
- ✅ **Zero Unsafe:** Can achieve 100% safe code (except justified use cases)
- ✅ **Better Errors:** Rust's error handling with Result types is more robust
- ✅ **Maintainability:** Single language codebase is easier to maintain
- ✅ **Modern Tooling:** cargo, rustfmt, clippy provide excellent developer experience
- ✅ **No C Dependency:** Users don't need C toolchain to use the library
- ✅ **Future-Proof:** Easier to add Rust-specific features (async, etc.)

**Disadvantages:**
- ❌ **More Work:** Need to reimplement all algorithms
- ❌ **Migration Time:** Longer time to feature parity
- ❌ **Learning Curve:** Team needs Rust expertise

**Consequences:**
- Development will take 12-14 months for full feature parity
- Need strong Rust expertise on the team
- Can provide better safety guarantees than FFI approach
- Opportunity to improve API design with Rust idioms

---

## ADR-002: Workspace Structure

**Status:** Accepted

**Context:**
Need to organize the codebase for modularity and maintainability.

**Decision:**
Use a Cargo workspace with multiple crates:
- `libcachesim-core` - Core types and traits
- `libcachesim-eviction` - Eviction algorithms
- `libcachesim-datastructures` - Data structures
- `libcachesim-trace` - Trace I/O
- `libcachesim-analysis` - Analysis tools
- `libcachesim-cli` - CLI binaries
- `libcachesim` - Main library (re-exports)

**Rationale:**
- **Modularity:** Clear separation of concerns
- **Compile Time:** Can compile crates in parallel
- **Dependencies:** Each crate can have its own dependencies
- **Feature Flags:** Can conditionally compile expensive features (ML, async)
- **Testing:** Easier to test modules in isolation

**Consequences:**
- More Cargo.toml files to maintain
- Need to manage inter-crate dependencies
- Clearer architecture and boundaries

---

## ADR-003: Cache Trait Design

**Status:** Accepted

**Context:**
Need a common interface for all cache implementations.

**Decision:**
Define a `Cache` trait with core operations:

```rust
pub trait Cache {
    fn get(&mut self, req: &Request) -> CacheResult;
    fn insert(&mut self, req: &Request) -> InsertResult;
    fn evict(&mut self) -> Option<ObjectId>;
    fn remove(&mut self, obj_id: ObjectId) -> bool;
    fn clear(&mut self);
    fn size(&self) -> u64;
    fn capacity(&self) -> u64;
    fn stats(&self) -> &CacheStats;
    fn reset_stats(&mut self);
}
```

**Rationale:**
- **Polymorphism:** Can use different cache types through the trait
- **Testability:** Mock implementations for testing
- **Extensibility:** Easy to add new cache algorithms
- **Type Safety:** Compiler ensures all methods are implemented

**Alternatives Considered:**
- Enum-based dispatch: Less flexible, harder to extend
- Dynamic dispatch with Box<dyn Cache>: Runtime overhead

**Consequences:**
- All cache types must implement this interface
- Some specialized caches may need additional methods
- Can use trait objects for runtime polymorphism when needed

---

## ADR-004: Error Handling Strategy

**Status:** Accepted

**Context:**
Need consistent error handling across the library.

**Decision:**
Use `Result<T, E>` with custom error types using `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Object too large: {size} > {capacity}")]
    ObjectTooLarge { size: u64, capacity: u64 },
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type CacheResult = Result<bool, CacheError>;
```

**Rationale:**
- **Explicit:** Errors are part of the function signature
- **Composable:** Can use `?` operator for error propagation
- **Informative:** thiserror generates good error messages
- **No Panics:** Library code should never panic (except for bugs)

**Consequences:**
- Users must handle errors explicitly
- Error types need to be well-designed
- Better error messages and debugging

---

## ADR-005: Statistics Tracking

**Status:** Accepted

**Context:**
Need to track cache statistics (hits, misses, evictions) efficiently.

**Decision:**
Use atomic counters for thread-safe statistics:

```rust
pub struct CacheStats {
    n_hit: AtomicU64,
    n_miss: AtomicU64,
    n_evict: AtomicU64,
    n_insert: AtomicU64,
}
```

**Rationale:**
- **Thread Safe:** Atomic operations are lock-free
- **Performance:** No mutex overhead for simple counters
- **Accuracy:** Exact counts (no sampling)

**Alternatives Considered:**
- Thread-local counters: Harder to aggregate
- Mutex-protected counters: Higher overhead
- No statistics: Not acceptable for a simulation library

**Consequences:**
- Slight overhead from atomic operations
- Statistics are always available
- Can safely share cache across threads

---

## ADR-006: Data Structure Choices

**Status:** Accepted

**Context:**
Need to choose data structures for cache implementations.

**Decision:**
Prefer standard library collections, custom implementations when needed:
- `HashMap<K, V>` for object storage (or `hashbrown::HashMap` for performance)
- `VecDeque<T>` for queues (FIFO, LRU)
- `Vec<T>` for arrays and circular buffers
- Custom splay tree for reuse distance
- Custom bloom filter for admission control

**Rationale:**
- **Battle-Tested:** Standard library is well-optimized and reliable
- **Maintainability:** Well-known data structures are easier to understand
- **Performance:** Good enough for most use cases
- **Custom When Needed:** Implement custom structures only when necessary

**Consequences:**
- Some algorithms may be slightly slower than custom C implementations
- Easier to maintain and understand
- Can optimize later if profiling shows bottlenecks

---

## ADR-007: LRU Implementation Strategy

**Status:** Accepted, with future optimization path

**Context:**
LRU needs O(1) get and O(1) update operations. The POC uses VecDeque which requires O(n) for moving items.

**Decision:**
Phase 1: Use HashMap + VecDeque (current POC approach)
Phase 2: Implement custom doubly-linked list for O(1) operations

**Rationale:**
- **Start Simple:** Get working implementation first
- **Optimize Later:** Profile to confirm the bottleneck
- **Proven Pattern:** Many Rust LRU implementations use similar approach

**Phase 1 (Current):**
```rust
struct LruCache {
    map: HashMap<ObjId, ObjSize>,
    queue: VecDeque<ObjId>,  // O(n) removal
    // ...
}
```

**Phase 2 (Future):**
```rust
struct LruCache {
    map: HashMap<ObjId, NonNull<Node>>,
    list: DoublyLinkedList,  // O(1) operations
    // ...
}
```

**Consequences:**
- Initial implementation may be slower for large caches
- Can optimize without changing public API
- Clear path forward when needed

---

## ADR-008: Async I/O Support

**Status:** Proposed

**Context:**
Trace files can be large. Async I/O could improve performance for parallel processing.

**Decision:**
Provide both sync and async APIs using feature flags:
- Default: Sync I/O (no tokio dependency)
- Feature "async": Async I/O (tokio dependency)

```rust
// Sync
pub fn read_trace(path: &Path) -> Result<impl Iterator<Item = Request>>;

// Async (behind feature flag)
#[cfg(feature = "async")]
pub async fn read_trace_async(path: &Path) -> Result<impl Stream<Item = Request>>;
```

**Rationale:**
- **Flexibility:** Users can choose based on their needs
- **Zero Cost:** Async support is optional
- **Future Proof:** Ready for async ecosystem

**Consequences:**
- More code to maintain (two versions)
- Need to test both sync and async paths
- Better ecosystem integration

---

## ADR-009: Machine Learning Integration

**Status:** Accepted

**Context:**
GLCache and LRB use XGBoost for predictions. Rust has xgboost bindings.

**Decision:**
Use conditional compilation for ML features:
- Default: No ML dependencies
- Feature "ml": Include xgboost-rs and related dependencies

```rust
#[cfg(feature = "ml")]
pub mod glcache {
    use xgboost;
    // ...
}
```

**Rationale:**
- **Optional:** Not all users need ML algorithms
- **Binary Size:** Avoid large dependencies for users who don't need them
- **Build Time:** Faster builds without ML
- **Platform Support:** XGBoost may not be available on all platforms

**Consequences:**
- More complex build configuration
- Need to document feature flags
- Better for users who don't need ML

---

## ADR-010: Testing Strategy

**Status:** Accepted

**Context:**
Need comprehensive testing to ensure correctness.

**Decision:**
Multi-layered testing approach:

1. **Unit Tests:** Test individual components
2. **Integration Tests:** Test component interactions
3. **Property-Based Tests:** Use proptest for invariant checking
4. **Reference Tests:** Compare against C version output
5. **Benchmark Tests:** Track performance regressions

**Rationale:**
- **Comprehensive:** Catch different classes of bugs
- **Confidence:** Multiple levels of verification
- **Regression Prevention:** Automated testing catches issues early

**Consequences:**
- More test code to write and maintain
- Longer CI times
- Higher confidence in correctness

---

## ADR-011: Serialization Support

**Status:** Accepted

**Context:**
Need to export statistics, configurations, and results in various formats.

**Decision:**
Use `serde` for serialization with multiple format support:
- JSON (serde_json)
- CSV (csv crate with serde)
- Binary (bincode for internal use)

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub n_hit: u64,
    pub n_miss: u64,
    // ...
}
```

**Rationale:**
- **Standard:** serde is the de-facto standard in Rust
- **Flexible:** Support multiple formats easily
- **Performance:** Very efficient serialization

**Consequences:**
- All relevant types need to derive Serialize/Deserialize
- Backward compatibility considerations for data formats

---

## ADR-012: CLI Argument Parsing

**Status:** Accepted

**Context:**
Need user-friendly CLI tools with consistent interfaces.

**Decision:**
Use `clap` v4 with derive macros:

```rust
#[derive(Parser)]
#[command(name = "cachesim")]
#[command(about = "Cache simulator", long_about = None)]
struct Args {
    /// Trace file path
    #[arg(short, long)]
    trace: PathBuf,
    
    /// Cache algorithm
    #[arg(short, long)]
    algorithm: Algorithm,
    
    /// Cache size in bytes
    #[arg(short, long)]
    size: u64,
}
```

**Rationale:**
- **User Friendly:** Automatic help generation
- **Type Safe:** Arguments are parsed into proper types
- **Validation:** Built-in validation and error messages
- **Completions:** Can generate shell completions

**Consequences:**
- Larger binary size (acceptable for CLI tools)
- Learning curve for clap's derive API
- Excellent user experience

---

## ADR-013: Benchmarking Framework

**Status:** Accepted

**Context:**
Need to measure and track performance.

**Decision:**
Use `criterion` for benchmarking:

```rust
fn bench_lru(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru");
    
    for size in [1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                let mut cache = LruCache::new(size);
                b.iter(|| cache.get(&request));
            },
        );
    }
}
```

**Rationale:**
- **Statistical:** Measures with statistical rigor
- **Regression Detection:** Tracks performance over time
- **Visualization:** Generates charts
- **Standard:** De-facto standard for Rust benchmarking

**Consequences:**
- Benchmarks take time to run
- Need to maintain benchmark code
- Can track performance regressions

---

## ADR-014: Memory Allocator

**Status:** Proposed

**Context:**
C version uses tcmalloc for better performance. Rust's default allocator is good but we might need better.

**Decision:**
Default: System allocator (jemalloc on Unix, system on Windows)
Optional: Allow users to choose allocator

```rust
#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

**Rationale:**
- **Flexibility:** Users can choose based on their needs
- **Performance:** jemalloc can be faster for some workloads
- **Compatibility:** System allocator works everywhere

**Consequences:**
- Need to benchmark different allocators
- Platform-specific behavior
- Optional dependency management

---

## ADR-015: Documentation Generation

**Status:** Accepted

**Context:**
Need comprehensive documentation for users and developers.

**Decision:**
Use rustdoc with examples:

```rust
/// LRU cache implementation.
///
/// # Examples
///
/// ```
/// use libcachesim::LruCache;
/// let mut cache = LruCache::new(1024);
/// ```
///
/// # Performance
///
/// - Get: O(n) in current implementation
/// - Insert: O(1)
/// - Evict: O(1)
pub struct LruCache {
    // ...
}
```

**Rationale:**
- **Integrated:** Documentation lives with code
- **Tested:** Examples in docs are tested
- **Standard:** Familiar to Rust developers
- **Automatic:** Easy to generate and publish

**Consequences:**
- Need to write comprehensive doc comments
- Examples must be maintained
- Better documentation quality

---

## ADR-016: Version Compatibility

**Status:** Accepted

**Context:**
Need to decide on Minimum Supported Rust Version (MSRV).

**Decision:**
MSRV: Rust 1.70 (or latest stable - 2 versions)
- Use stable Rust features only
- Update MSRV conservatively

**Rationale:**
- **Accessibility:** More users can use the library
- **Stability:** Stable features are well-tested
- **CI:** Can test multiple Rust versions

**Consequences:**
- Can't use latest language features immediately
- Need to test on multiple Rust versions
- Broader compatibility

---

## ADR-017: Trace Format Extensibility

**Status:** Accepted

**Context:**
Need to support multiple trace formats and allow custom formats.

**Decision:**
Define a TraceReader trait:

```rust
pub trait TraceReader: Iterator<Item = Result<Request, TraceError>> {
    fn reset(&mut self) -> Result<(), TraceError>;
    fn seek(&mut self, position: f64) -> Result<(), TraceError>;
    fn total_requests(&self) -> Option<u64>;
}
```

**Rationale:**
- **Extensibility:** Users can implement custom readers
- **Standard:** Uses Iterator trait familiar to Rust developers
- **Flexibility:** Different readers for different formats

**Consequences:**
- All trace readers must implement this trait
- Consistent interface across formats
- Easy to add new formats

---

## ADR-018: Parallel Processing

**Status:** Accepted

**Context:**
Trace analysis can benefit from parallel processing.

**Decision:**
Use `rayon` for data parallelism:

```rust
use rayon::prelude::*;

let results: Vec<_> = caches
    .par_iter_mut()
    .map(|cache| simulate(cache, &trace))
    .collect();
```

**Rationale:**
- **Easy:** rayon makes parallelism simple
- **Safe:** Rust's type system prevents data races
- **Performance:** Can use all CPU cores
- **Standard:** rayon is widely used in Rust

**Consequences:**
- Need to ensure caches are thread-safe
- Increased complexity for parallel code
- Better performance on multi-core systems

---

## Summary

These architecture decisions provide a foundation for the Rust port of libCacheSim. Key themes:

1. **Safety:** Leverage Rust's safety features
2. **Performance:** Match or exceed C version
3. **Maintainability:** Clean, idiomatic Rust
4. **Flexibility:** Modular design with feature flags
5. **Quality:** Comprehensive testing and documentation

These decisions will be reviewed and updated as the project progresses.
