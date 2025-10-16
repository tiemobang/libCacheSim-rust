# Implementation Roadmap: libCacheSim Rust Port

This document provides concrete, actionable steps for implementing the Rust port of libCacheSim.

## Quick Reference

- **Total Duration:** 12-14 months
- **Team Size:** 1-2 developers
- **Phases:** 8 major phases
- **Milestones:** 5 key releases
- **Current Status:** Foundation phase in progress (3 algorithms completed)

---

## Phase 1: Foundation (Months 1-3)

### Week 1-2: Core Infrastructure Setup

**Goal:** Set up project structure and core types

**Tasks:**
- [ ] Create Cargo workspace structure
  ```bash
  cargo new --lib libcachesim-core
  cargo new --lib libcachesim-eviction
  cargo new --lib libcachesim-datastructures
  cargo new --lib libcachesim-trace
  cargo new --lib libcachesim-analysis
  cargo new --lib libcachesim-cli
  cargo new --lib libcachesim
  ```

- [ ] Set up workspace Cargo.toml
  ```toml
  [workspace]
  members = [
      "libcachesim-core",
      "libcachesim-eviction",
      "libcachesim-datastructures",
      "libcachesim-trace",
      "libcachesim-analysis",
      "libcachesim-cli",
      "libcachesim",
  ]
  
  [workspace.package]
  version = "0.1.0"
  edition = "2021"
  authors = ["Your Name <you@example.com>"]
  license = "GPL-3.0"
  ```

- [ ] Implement enhanced Request type in libcachesim-core
  ```rust
  pub struct Request {
      pub clock_time: u64,
      pub obj_id: u64,
      pub obj_size: u32,
      pub op: Operation,
      pub tenant_id: Option<u32>,
      pub ttl: Option<i32>,
      pub next_access_vtime: Option<i64>,
  }
  
  pub enum Operation {
      Get,
      Set,
      Delete,
      Add,
      Replace,
  }
  ```

- [ ] Define error types using thiserror
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum CacheError {
      #[error("Object too large")]
      ObjectTooLarge,
      #[error("Invalid operation")]
      InvalidOperation,
  }
  ```

- [ ] Implement CacheStats with atomic counters
  ```rust
  pub struct CacheStats {
      n_hit: AtomicU64,
      n_miss: AtomicU64,
      n_evict: AtomicU64,
      n_insert: AtomicU64,
  }
  ```

- [ ] Set up GitHub Actions CI
  ```yaml
  # .github/workflows/ci.yml
  name: CI
  on: [push, pull_request]
  jobs:
    test:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        - uses: actions-rs/toolchain@v1
        - run: cargo test --all
        - run: cargo clippy -- -D warnings
  ```

**Deliverables:**
- Workspace structure created
- Core types defined
- CI pipeline running

---

### Week 3-4: Migrate POC Algorithms

**Goal:** Move existing FIFO, LRU, Clock to new structure

**Tasks:**
- [ ] Create libcachesim-eviction/src/fifo.rs
  - Move FIFO implementation from POC
  - Add tests
  - Add documentation

- [ ] Create libcachesim-eviction/src/lru.rs
  - Move LRU implementation from POC
  - Add tests
  - Add documentation

- [ ] Create libcachesim-eviction/src/clock.rs
  - Move Clock implementation from POC
  - Add tests
  - Add documentation

- [ ] Update implementations to use new Request type

- [ ] Ensure all POC tests pass

**Deliverables:**
- 3 algorithms in new structure
- Tests passing
- Documentation updated

---

### Week 5: Simple Eviction Algorithms

**Goal:** Implement MRU and Random

**Tasks:**
- [ ] Implement MRU (Most Recently Used)
  ```rust
  pub struct MruCache {
      map: HashMap<ObjId, ObjSize>,
      stack: VecDeque<ObjId>,
      capacity: u64,
      used: u64,
      stats: CacheStats,
  }
  ```

- [ ] Implement Random eviction
  ```rust
  use rand::seq::SliceRandom;
  
  pub struct RandomCache {
      map: HashMap<ObjId, ObjSize>,
      keys: Vec<ObjId>,
      capacity: u64,
      used: u64,
      stats: CacheStats,
      rng: ThreadRng,
  }
  ```

- [ ] Write comprehensive tests
  - Basic operations
  - Eviction behavior
  - Edge cases

- [ ] Add benchmarks
  ```rust
  fn bench_random_cache(c: &mut Criterion) {
      c.bench_function("random_get", |b| {
          let mut cache = RandomCache::new(1024);
          b.iter(|| cache.get(&request));
      });
  }
  ```

**Deliverables:**
- MRU and Random algorithms
- Tests passing
- Benchmarks created

---

### Week 6: SLRU and LFU

**Goal:** Implement Segmented LRU and LFU

**Tasks:**
- [ ] Implement SLRU
  ```rust
  pub struct SlruCache {
      probation: LruCache,
      protected: LruCache,
      capacity: u64,
      protected_ratio: f64,
      stats: CacheStats,
  }
  ```

- [ ] Implement LFU with min-heap
  ```rust
  use std::collections::BinaryHeap;
  
  pub struct LfuCache {
      map: HashMap<ObjId, LfuEntry>,
      freq_heap: BinaryHeap<Reverse<LfuEntry>>,
      capacity: u64,
      stats: CacheStats,
  }
  
  struct LfuEntry {
      obj_id: ObjId,
      obj_size: ObjSize,
      frequency: u32,
      last_access: u64,
  }
  ```

- [ ] Implement LFUDA (LFU with Dynamic Aging)
  ```rust
  pub struct LfudaCache {
      map: HashMap<ObjId, LfudaEntry>,
      age: u64,
      capacity: u64,
      stats: CacheStats,
  }
  
  struct LfudaEntry {
      obj_size: ObjSize,
      frequency: u64,
      insert_age: u64,
  }
  ```

- [ ] Tests for all three algorithms

**Deliverables:**
- SLRU, LFU, LFUDA implemented
- Tests passing
- 8 total algorithms completed

---

### Week 7-8: Basic Trace Reading

**Goal:** Implement CSV and simple binary trace readers

**Tasks:**
- [ ] Define TraceReader trait
  ```rust
  pub trait TraceReader: Iterator<Item = Result<Request, TraceError>> {
      fn reset(&mut self) -> Result<(), TraceError>;
      fn seek(&mut self, position: f64) -> Result<(), TraceError>;
      fn total_requests(&self) -> Option<u64>;
  }
  ```

- [ ] Implement CSV reader
  ```rust
  use csv::Reader;
  
  pub struct CsvTraceReader {
      reader: Reader<BufReader<File>>,
      path: PathBuf,
  }
  
  impl Iterator for CsvTraceReader {
      type Item = Result<Request, TraceError>;
      
      fn next(&mut self) -> Option<Self::Item> {
          // Parse CSV row to Request
      }
  }
  ```

- [ ] Implement LCS format reader (v1, v2, v3)
  ```rust
  pub struct LcsTraceReader {
      reader: BufReader<File>,
      version: u32,
      header: LcsHeader,
  }
  ```

- [ ] Add tests with sample trace files
  - Create test traces in data/
  - Test parsing
  - Test iteration
  - Test seeking

**Deliverables:**
- CSV reader working
- LCS reader working
- Sample trace files
- Tests passing

---

### Week 9-10: Testing Infrastructure

**Goal:** Comprehensive testing framework

**Tasks:**
- [ ] Set up property-based testing
  ```rust
  use proptest::prelude::*;
  
  proptest! {
      #[test]
      fn test_cache_invariants(
          requests in vec(any::<Request>(), 0..1000),
          capacity in 100u64..10000u64
      ) {
          let mut cache = FifoCache::new(capacity);
          for req in requests {
              cache.get(&req);
              assert!(cache.size() <= capacity);
          }
      }
  }
  ```

- [ ] Add integration tests
  ```rust
  #[test]
  fn test_workload_simulation() {
      let trace = CsvTraceReader::open("data/test.csv").unwrap();
      let mut cache = LruCache::new(1024);
      
      for request in trace {
          cache.get(&request.unwrap());
      }
      
      assert!(cache.stats().hit_ratio() > 0.4);
  }
  ```

- [ ] Set up code coverage with tarpaulin
  ```yaml
  # .github/workflows/coverage.yml
  - uses: actions-rs/tarpaulin@v0.1
    with:
      args: '--all-features --workspace'
  - uses: codecov/codecov-action@v3
  ```

- [ ] Create reference tests against C version
  ```rust
  #[test]
  fn test_lru_matches_c_version() {
      let rust_stats = run_rust_simulation();
      let c_stats = load_c_reference_output();
      
      assert_eq!(rust_stats.n_hit, c_stats.n_hit);
      assert_eq!(rust_stats.n_miss, c_stats.n_miss);
  }
  ```

**Deliverables:**
- Property-based tests
- Integration tests
- Code coverage >60%
- Reference testing framework

---

## Phase 2: Advanced Algorithms (Months 4-6)

### Week 11-12: ARC and CAR

**Goal:** Implement adaptive caching algorithms

**Tasks:**
- [ ] Study ARC paper and C implementation
- [ ] Implement ARC
  ```rust
  pub struct ArcCache {
      t1: LruCache,  // Recent items
      t2: LruCache,  // Frequent items
      b1: GhostCache,  // Ghost entries for t1
      b2: GhostCache,  // Ghost entries for t2
      p: usize,  // Target size for t1
      capacity: u64,
  }
  ```

- [ ] Implement CAR (Clock with Adaptive Replacement)
  ```rust
  pub struct CarCache {
      t1: ClockCache,
      t2: ClockCache,
      b1: GhostCache,
      b2: GhostCache,
      p: usize,
      capacity: u64,
  }
  ```

- [ ] Tests comparing against C version behavior

**Deliverables:**
- ARC implemented and tested
- CAR implemented and tested
- Documentation explaining adaptive behavior

---

### Week 13-14: TwoQ and LeCaR

**Goal:** More adaptive algorithms

**Tasks:**
- [ ] Implement TwoQ
  ```rust
  pub struct TwoQCache {
      am: LruCache,  // Main cache
      a1in: FifoCache,  // FIFO for new items
      a1out: GhostCache,  // Ghost cache
      kin: usize,
      kout: usize,
  }
  ```

- [ ] Implement LeCaR (Learning Cache Replacement)
  ```rust
  pub struct LeCarCache {
      lru: LruCache,
      lfu: LfuCache,
      lru_history: GhostCache,
      lfu_history: GhostCache,
      w_lru: f64,  // Weight for LRU
      learning_rate: f64,
  }
  ```

- [ ] Implement Cacheus
  ```rust
  pub struct CacheusCache {
      cache: HashMap<ObjId, CacheusEntry>,
      priority_queue: BinaryHeap<CacheusEntry>,
      capacity: u64,
  }
  
  struct CacheusEntry {
      obj_id: ObjId,
      utility: f64,
      last_access: u64,
  }
  ```

**Deliverables:**
- TwoQ, LeCaR, Cacheus implemented
- Tests passing
- Performance benchmarks

---

### Week 15-16: S3-FIFO and Sieve

**Goal:** Modern scan-resistant algorithms

**Tasks:**
- [ ] Implement S3-FIFO
  ```rust
  pub struct S3FifoCache {
      small: FifoCache,  // Small objects
      main: FifoCache,   // Main queue
      ghost: GhostCache, // Ghost entries
      small_size_threshold: u64,
  }
  ```

- [ ] Implement Sieve
  ```rust
  pub struct SieveCache {
      cache: HashMap<ObjId, SieveEntry>,
      hand: usize,  // Clock hand
      objects: Vec<ObjId>,
      capacity: u64,
  }
  
  struct SieveEntry {
      obj_size: ObjSize,
      visited: bool,
  }
  ```

- [ ] Comprehensive tests for scan resistance
  ```rust
  #[test]
  fn test_scan_resistance() {
      let mut cache = S3FifoCache::new(1024);
      
      // Simulate sequential scan
      for i in 0..10000 {
          cache.get(&Request::new(i, 100));
      }
      
      // Popular items should still be in cache
      assert!(cache.get(&Request::new(1, 100)));
  }
  ```

**Deliverables:**
- S3-FIFO and Sieve implemented
- Scan resistance validated
- Performance comparison

---

### Week 17-18: ClockPro and LIRS

**Goal:** Complex eviction algorithms

**Tasks:**
- [ ] Implement ClockPro
  ```rust
  pub struct ClockProCache {
      cold: Vec<ClockProEntry>,
      hot: Vec<ClockProEntry>,
      hand_cold: usize,
      hand_hot: usize,
      hand_test: usize,
      m_cold: usize,
  }
  
  struct ClockProEntry {
      obj_id: ObjId,
      obj_size: ObjSize,
      referenced: bool,
      is_test: bool,
  }
  ```

- [ ] Implement LIRS
  ```rust
  pub struct LirsCache {
      lir_stack: VecDeque<ObjId>,
      hir_list: VecDeque<ObjId>,
      map: HashMap<ObjId, LirsEntry>,
      lirs_limit: usize,
  }
  
  struct LirsEntry {
      obj_size: ObjSize,
      is_lir: bool,
      in_stack: bool,
  }
  ```

- [ ] Extensive testing for correctness

**Deliverables:**
- ClockPro and LIRS working
- Reference tests passing
- Documentation

---

### Week 19-20: QDLP and WTinyLFU

**Goal:** Recent research algorithms

**Tasks:**
- [ ] Implement QDLP
  ```rust
  pub struct QdlpCache {
      probation: FifoCache,
      protected: LruCache,
      capacity: u64,
  }
  ```

- [ ] Implement WTinyLFU
  ```rust
  pub struct WTinyLfuCache {
      window: LruCache,
      main: SlruCache,
      doorkeeper: BloomFilter,
      sketch: CountMinSketch,
  }
  ```

- [ ] Implement Hyperbolic cache
  ```rust
  pub struct HyperbolicCache {
      map: HashMap<ObjId, HyperbolicEntry>,
      capacity: u64,
  }
  
  struct HyperbolicEntry {
      obj_size: ObjSize,
      frequency: u32,
      recency: u64,
      cost: u32,
  }
  ```

**Deliverables:**
- QDLP, WTinyLFU, Hyperbolic working
- 20+ algorithms completed
- Benchmarks updated

---

### Week 21-22: Belady Variants

**Goal:** Optimal algorithms (require future knowledge)

**Tasks:**
- [ ] Implement Belady (MIN)
  ```rust
  pub struct BeladyCache {
      map: HashMap<ObjId, BeladyEntry>,
      future_accesses: HashMap<ObjId, VecDeque<u64>>,
      capacity: u64,
  }
  
  struct BeladyEntry {
      obj_size: ObjSize,
      next_access: Option<u64>,
  }
  ```

- [ ] Implement BeladySize
- [ ] Implement FIFO-Belady hybrid
- [ ] Implement LRU-Belady hybrid
- [ ] Implement Sieve-Belady hybrid

- [ ] Two-pass trace processing for optimal algorithms
  ```rust
  pub fn compute_future_accesses(
      trace: &[Request]
  ) -> HashMap<ObjId, VecDeque<u64>> {
      let mut future = HashMap::new();
      for (idx, req) in trace.iter().enumerate().rev() {
          future.entry(req.obj_id)
              .or_insert_with(VecDeque::new)
              .push_front(idx as u64);
      }
      future
  }
  ```

**Deliverables:**
- All Belady variants working
- Future access preprocessing
- Comparison framework for optimal vs practical

---

## Phase 3: Machine Learning (Months 7-8)

### Week 23-24: GLCache Data Preparation

**Goal:** Feature extraction for GLCache

**Tasks:**
- [ ] Add xgboost feature flag
  ```toml
  [features]
  ml = ["xgboost", "ndarray"]
  
  [dependencies]
  xgboost = { version = "0.2", optional = true }
  ndarray = { version = "0.15", optional = true }
  ```

- [ ] Implement feature extraction
  ```rust
  #[cfg(feature = "ml")]
  pub struct FeatureExtractor {
      // Track features per object
      features: HashMap<ObjId, Features>,
  }
  
  struct Features {
      frequency: u32,
      recency: u64,
      size: u32,
      tenant: u32,
      // ... more features
  }
  ```

- [ ] Data collection for training
  ```rust
  pub struct TrainingDataCollector {
      samples: Vec<TrainingSample>,
  }
  
  struct TrainingSample {
      features: Vec<f32>,
      label: f32,  // Time until next access
  }
  ```

**Deliverables:**
- Feature extraction working
- Training data collection
- Feature tests

---

### Week 25-26: GLCache Training and Inference

**Goal:** Complete GLCache implementation

**Tasks:**
- [ ] Implement training pipeline
  ```rust
  #[cfg(feature = "ml")]
  pub fn train_model(
      samples: &[TrainingSample]
  ) -> Result<Booster, XGBoostError> {
      let dtrain = DMatrix::from_dense(&features, num_rows)?;
      dtrain.set_labels(&labels)?;
      
      let params = vec![
          ("max_depth", "6"),
          ("eta", "0.3"),
          ("objective", "reg:squarederror"),
      ];
      
      let booster = Booster::train(&dtrain, &params, 100)?;
      Ok(booster)
  }
  ```

- [ ] Implement inference
  ```rust
  pub struct GlCache {
      cache: HashMap<ObjId, GlEntry>,
      model: Booster,
      feature_extractor: FeatureExtractor,
      capacity: u64,
  }
  
  impl GlCache {
      fn predict_next_access(&self, obj_id: ObjId) -> f32 {
          let features = self.feature_extractor.extract(obj_id);
          let dmatrix = DMatrix::from_dense(&features, 1).unwrap();
          self.model.predict(&dmatrix).unwrap()[0]
      }
  }
  ```

- [ ] Model persistence
  ```rust
  impl GlCache {
      pub fn save_model(&self, path: &Path) -> Result<()> {
          self.model.save(path)?;
          Ok(())
      }
      
      pub fn load_model(path: &Path, capacity: u64) -> Result<Self> {
          let model = Booster::load(path)?;
          Ok(GlCache { model, capacity, .. })
      }
  }
  ```

**Deliverables:**
- GLCache fully working
- Training pipeline
- Model save/load
- Tests with sample model

---

### Week 27-28: LRB Implementation

**Goal:** Learning-based Relaxed Belady

**Tasks:**
- [ ] Implement LRB feature computation
- [ ] Implement model training
- [ ] Implement inference engine
- [ ] Tests and benchmarks

**Deliverables:**
- LRB working
- Comparison with GLCache
- Documentation

---

### Week 29-30: LHD Implementation

**Goal:** Learning Hyperbolic Discounting

**Tasks:**
- [ ] Implement LHD algorithm
- [ ] Learning components
- [ ] Tests and benchmarks

**Deliverables:**
- LHD working
- All ML algorithms completed
- Performance analysis

---

## Phase 4: Data Structures (Month 9)

### Week 31: Bloom Filter and Count-Min Sketch

**Goal:** Probabilistic data structures

**Tasks:**
- [ ] Implement Bloom filter
  ```rust
  pub struct BloomFilter {
      bits: BitVec,
      num_hashes: usize,
      num_bits: usize,
  }
  
  impl BloomFilter {
      pub fn new(capacity: usize, fpr: f64) -> Self {
          let num_bits = optimal_bits(capacity, fpr);
          let num_hashes = optimal_hashes(capacity, num_bits);
          BloomFilter {
              bits: BitVec::from_elem(num_bits, false),
              num_hashes,
              num_bits,
          }
      }
      
      pub fn insert(&mut self, item: &[u8]) { /* ... */ }
      pub fn contains(&self, item: &[u8]) -> bool { /* ... */ }
  }
  ```

- [ ] Implement Count-Min Sketch
  ```rust
  pub struct CountMinSketch {
      table: Vec<Vec<u32>>,
      width: usize,
      depth: usize,
  }
  ```

**Deliverables:**
- Bloom filter working
- Count-Min Sketch working
- Tests for false positive rates

---

### Week 32: Splay Tree

**Goal:** Splay tree for reuse distance

**Tasks:**
- [ ] Implement splay tree
  ```rust
  pub struct SplayTree<K, V> {
      root: Option<Box<Node<K, V>>>,
      size: usize,
  }
  
  struct Node<K, V> {
      key: K,
      value: V,
      left: Option<Box<Node<K, V>>>,
      right: Option<Box<Node<K, V>>>,
  }
  
  impl<K: Ord, V> SplayTree<K, V> {
      fn splay(&mut self, key: &K) { /* ... */ }
      pub fn insert(&mut self, key: K, value: V) { /* ... */ }
      pub fn find(&mut self, key: &K) -> Option<&V> { /* ... */ }
  }
  ```

- [ ] Tests for splay operations
- [ ] Benchmarks vs standard BTreeMap

**Deliverables:**
- Splay tree working
- Used in reuse distance calculation

---

### Week 33: Hash Functions

**Goal:** High-performance hashing

**Tasks:**
- [ ] Integrate xxHash
  ```rust
  use twox_hash::XxHash64;
  
  pub fn xx_hash(data: &[u8]) -> u64 {
      let mut hasher = XxHash64::default();
      hasher.write(data);
      hasher.finish()
  }
  ```

- [ ] Integrate MurmurHash3
- [ ] Benchmark different hash functions
- [ ] Choose best defaults

**Deliverables:**
- Hash functions integrated
- Performance comparison
- Best defaults chosen

---

### Week 34: Advanced Structures

**Goal:** Consistent hashing and sparse maps

**Tasks:**
- [ ] Implement Ketama (consistent hashing)
  ```rust
  pub struct Ketama {
      ring: BTreeMap<u64, NodeId>,
      replicas: usize,
  }
  
  impl Ketama {
      pub fn get_node(&self, key: &[u8]) -> NodeId {
          let hash = xx_hash(key);
          self.ring.range(hash..).next()
              .or_else(|| self.ring.iter().next())
              .map(|(_, node)| *node)
              .unwrap()
      }
  }
  ```

- [ ] Evaluate existing sparse map crates
- [ ] Use hashbrown for HashMap optimizations

**Deliverables:**
- Consistent hashing working
- HashMap optimizations applied
- Benchmarks showing improvements

---

## Phase 5: Admission & Prefetching (Month 10)

### Week 35-36: Admission Policies

**Goal:** Implement admission control

**Tasks:**
- [ ] Implement AdaptSize
  ```rust
  pub struct AdaptSize {
      ewma: ExponentialMovingAverage,
      bloomfilter: BloomFilter,
      threshold: f64,
  }
  
  impl AdaptSize {
      pub fn should_admit(&mut self, req: &Request) -> bool {
          let predicted_size = self.ewma.predict();
          req.obj_size < predicted_size * self.threshold
      }
  }
  ```

- [ ] Implement Bloom filter admission
- [ ] Implement probability-based admission
- [ ] Implement size-based admission

- [ ] Integration with cache implementations
  ```rust
  pub struct CacheWithAdmission<C: Cache, A: AdmissionPolicy> {
      cache: C,
      admission: A,
  }
  
  impl<C: Cache, A: AdmissionPolicy> Cache for CacheWithAdmission<C, A> {
      fn get(&mut self, req: &Request) -> CacheResult {
          let result = self.cache.get(req);
          if !result && self.admission.should_admit(req) {
              self.cache.insert(req);
          }
          result
      }
  }
  ```

**Deliverables:**
- All admission policies working
- Integration framework
- Tests and benchmarks

---

### Week 37-38: Prefetching Algorithms

**Goal:** Implement prefetching

**Tasks:**
- [ ] Implement OBL (Object-Based Lookahead)
  ```rust
  pub struct OblPrefetcher {
      correlation_matrix: HashMap<ObjId, Vec<ObjId>>,
      confidence_threshold: f64,
  }
  
  impl OblPrefetcher {
      pub fn predict_next(&self, current: ObjId) -> Vec<ObjId> {
          self.correlation_matrix.get(&current)
              .map(|v| v.clone())
              .unwrap_or_default()
      }
  }
  ```

- [ ] Implement Mithril
- [ ] Implement PG (Prediction by Graph)

- [ ] Integration with caches
  ```rust
  pub struct CacheWithPrefetch<C: Cache, P: Prefetcher> {
      cache: C,
      prefetcher: P,
  }
  ```

**Deliverables:**
- Prefetching algorithms working
- Integration with caches
- Performance analysis

---

## Phase 6: Analysis Tools (Months 11-12)

### Week 39-40: Complete Trace Readers

**Goal:** All trace format support

**Tasks:**
- [ ] Implement remaining binary formats
  - VSCSI
  - Oracle formats
  - TWR formats
  - Valpin

- [ ] Add zstd compression support
  ```rust
  pub struct ZstdTraceReader<R: TraceReader> {
      inner: R,
      decompressor: ZstdDecoder,
  }
  ```

- [ ] Implement async I/O version
  ```rust
  #[cfg(feature = "async")]
  pub struct AsyncTraceReader {
      file: tokio::fs::File,
      buffer: BytesMut,
  }
  
  impl Stream for AsyncTraceReader {
      type Item = Result<Request, TraceError>;
      // ...
  }
  ```

**Deliverables:**
- All trace formats supported
- Compression working
- Async I/O optional

---

### Week 41-42: Sampling Techniques

**Goal:** Implement trace sampling

**Tasks:**
- [ ] Implement SHARDS
  ```rust
  pub struct ShardsSampler {
      hash_functions: Vec<HashFunction>,
      threshold: u64,
      buckets: Vec<HashSet<ObjId>>,
  }
  
  impl ShardsSampler {
      pub fn should_sample(&mut self, obj_id: ObjId) -> bool {
          let hash = self.hash_functions[0](obj_id);
          hash < self.threshold
      }
  }
  ```

- [ ] Implement spatial sampling
- [ ] Implement temporal sampling
- [ ] Adaptive sampling

**Deliverables:**
- Sampling algorithms working
- Sampling rate adaptation
- Tests validating accuracy

---

### Week 43-44: MRC Generation

**Goal:** Miss Ratio Curve profiling

**Tasks:**
- [ ] Implement SHARDS-based MRC
  ```rust
  pub struct ShardsMrcProfiler {
      sampler: ShardsSampler,
      counters: Vec<Counter>,
      cache_sizes: Vec<u64>,
  }
  
  impl ShardsMrcProfiler {
      pub fn profile(&mut self, trace: impl TraceReader) -> MrcCurve {
          for request in trace {
              self.process_request(&request?);
          }
          self.compute_mrc()
      }
  }
  ```

- [ ] Implement MiniSim
- [ ] Fixed-rate and fixed-size sampling
- [ ] Splay tree-based reuse distance

**Deliverables:**
- MRC profiling working
- Multiple methods implemented
- Accuracy validation

---

### Week 45-46: Trace Analysis

**Goal:** Comprehensive trace analysis tools

**Tasks:**
- [ ] Implement popularity analysis
  ```rust
  pub struct PopularityAnalyzer {
      frequency_map: HashMap<ObjId, u32>,
  }
  
  impl PopularityAnalyzer {
      pub fn analyze(&mut self, trace: impl TraceReader) -> PopularityStats {
          // Compute frequency distribution
          // Fit Zipf distribution
          // Calculate alpha parameter
      }
  }
  ```

- [ ] Implement reuse distance analysis
- [ ] Implement size distribution analysis
- [ ] Implement temporal pattern detection
- [ ] Implement scan detection
- [ ] Working set analysis

- [ ] Parallel analysis with rayon
  ```rust
  use rayon::prelude::*;
  
  pub fn parallel_analysis(
      traces: Vec<PathBuf>
  ) -> Vec<AnalysisResult> {
      traces.par_iter()
          .map(|path| analyze_trace(path))
          .collect()
  }
  ```

**Deliverables:**
- Complete analysis framework
- Parallel processing
- Rich statistics output

---

## Phase 7: CLI Tools (Month 13)

### Week 47-48: cachesim CLI

**Goal:** Cache simulation tool

**Tasks:**
- [ ] Implement argument parsing
  ```rust
  #[derive(Parser)]
  struct CachesimArgs {
      #[arg(short, long)]
      trace: PathBuf,
      
      #[arg(short, long)]
      algorithms: Vec<Algorithm>,
      
      #[arg(short, long)]
      size: ByteSize,
      
      #[arg(long, default_value = "json")]
      output_format: OutputFormat,
  }
  ```

- [ ] Implement cache initialization
  ```rust
  fn create_cache(algo: Algorithm, size: u64) -> Box<dyn Cache> {
      match algo {
          Algorithm::Fifo => Box::new(FifoCache::new(size)),
          Algorithm::Lru => Box::new(LruCache::new(size)),
          // ...
      }
  }
  ```

- [ ] Implement parallel simulation
  ```rust
  let results: Vec<_> = algorithms.par_iter()
      .map(|algo| {
          let mut cache = create_cache(algo, size);
          simulate(&mut cache, &trace)
      })
      .collect();
  ```

- [ ] Output formatting
  ```rust
  match output_format {
      OutputFormat::Json => serde_json::to_writer_pretty(stdout(), &results)?,
      OutputFormat::Csv => write_csv(&results)?,
      OutputFormat::Text => print_table(&results),
  }
  ```

**Deliverables:**
- cachesim binary working
- Multiple output formats
- Parallel execution
- Progress bars

---

### Week 49: MRC Profiler CLI

**Goal:** MRC profiling tool

**Tasks:**
- [ ] Implement CLI
  ```rust
  #[derive(Parser)]
  struct MrcArgs {
      #[arg(short, long)]
      trace: PathBuf,
      
      #[arg(long, default_value = "0.01")]
      sample_rate: f64,
      
      #[arg(long)]
      sizes: Option<Vec<u64>>,
  }
  ```

- [ ] Implement profiling logic
- [ ] Output generation
- [ ] Visualization data export

**Deliverables:**
- mrc binary working
- Fast MRC profiling
- Output formats

---

### Week 50: Trace Analyzer CLI

**Goal:** Trace analysis tool

**Tasks:**
- [ ] Implement CLI
  ```rust
  #[derive(Parser)]
  struct AnalyzerArgs {
      #[arg(short, long)]
      trace: PathBuf,
      
      #[arg(long)]
      analyses: Vec<AnalysisType>,
      
      #[arg(short, long)]
      output: Option<PathBuf>,
  }
  ```

- [ ] Implement analysis orchestration
- [ ] Report generation
- [ ] Visualization support

**Deliverables:**
- traceanalyzer binary working
- Rich analysis reports
- Multiple output formats

---

### Week 51: Trace Utils

**Goal:** Trace manipulation tools

**Tasks:**
- [ ] Trace conversion
  ```rust
  pub fn convert_trace(
      input: PathBuf,
      input_format: Format,
      output: PathBuf,
      output_format: Format,
  ) -> Result<()> {
      let reader = open_trace(&input, input_format)?;
      let writer = create_writer(&output, output_format)?;
      
      for request in reader {
          writer.write(&request?)?;
      }
      
      Ok(())
  }
  ```

- [ ] Trace filtering
- [ ] Trace printing
- [ ] Format validation

**Deliverables:**
- traceutils binary working
- Format conversion
- Filtering capabilities

---

## Phase 8: Polish & Release (Month 14)

### Week 52-53: Integration Testing

**Goal:** End-to-end validation

**Tasks:**
- [ ] Cross-validation with C version
  ```bash
  # Run same trace through both versions
  ./libCacheSim/build/bin/cachesim -t trace.csv -a lru -s 1MB > c_output.json
  cargo run --bin cachesim -- -t trace.csv -a lru -s 1MB > rust_output.json
  
  # Compare outputs
  diff c_output.json rust_output.json
  ```

- [ ] Trace compatibility tests
- [ ] Performance regression tests
- [ ] Memory leak checks with valgrind
- [ ] Stress testing with large traces

**Deliverables:**
- Validated against C version
- No regressions
- Stress tested

---

### Week 54: Documentation

**Goal:** Complete documentation

**Tasks:**
- [ ] API documentation
  - Ensure all public items documented
  - Run `cargo doc --all --no-deps`
  - Fix all missing docs warnings

- [ ] User guide
  - Installation instructions
  - Quick start tutorial
  - Algorithm overview
  - CLI tools usage
  - API examples

- [ ] Migration guide
  - C to Rust API mapping
  - Behavior differences
  - Migration checklist

- [ ] Contributing guide
  - Development setup
  - Code style
  - Testing requirements
  - PR process

**Deliverables:**
- Complete documentation
- Examples working
- Guides published

---

### Week 55: Ecosystem & Release

**Goal:** Publish and integrate

**Tasks:**
- [ ] Prepare for crates.io
  ```toml
  [package]
  name = "libcachesim"
  version = "1.0.0"
  description = "High-performance cache simulation library"
  license = "GPL-3.0"
  repository = "https://github.com/tiemobang/libCacheSim-rust"
  keywords = ["cache", "simulation", "algorithms"]
  categories = ["simulation", "algorithms"]
  ```

- [ ] Publish to crates.io
  ```bash
  cargo publish -p libcachesim-core
  cargo publish -p libcachesim-datastructures
  cargo publish -p libcachesim-eviction
  cargo publish -p libcachesim-trace
  cargo publish -p libcachesim-analysis
  cargo publish -p libcachesim-cli
  cargo publish -p libcachesim
  ```

- [ ] Create GitHub release
  - Tag v1.0.0
  - Release notes
  - Binary releases for major platforms

- [ ] Performance comparison report
  - Benchmark against C version
  - Document results
  - Publish findings

- [ ] Docker images
  ```dockerfile
  FROM rust:1.70 as builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release
  
  FROM debian:bookworm-slim
  COPY --from=builder /app/target/release/cachesim /usr/local/bin/
  ENTRYPOINT ["cachesim"]
  ```

**Deliverables:**
- Published to crates.io
- GitHub release created
- Docker images available
- Performance report published

---

## Tracking Progress

### Checklist Summary

**Phase 1: Foundation** (Weeks 1-10)
- [ ] Workspace setup
- [ ] Core types
- [ ] 8+ basic algorithms
- [ ] Trace reading (CSV, LCS)
- [ ] Testing infrastructure

**Phase 2: Advanced Algorithms** (Weeks 11-22)
- [ ] Adaptive algorithms (4+)
- [ ] Modern FIFO variants (4+)
- [ ] Complex algorithms (4+)
- [ ] Belady variants (5+)

**Phase 3: ML** (Weeks 23-30)
- [ ] GLCache
- [ ] LRB
- [ ] LHD

**Phase 4: Data Structures** (Weeks 31-34)
- [ ] Bloom filter
- [ ] Count-Min Sketch
- [ ] Splay tree
- [ ] Hash functions

**Phase 5: Admission & Prefetch** (Weeks 35-38)
- [ ] 4+ admission policies
- [ ] 3+ prefetch algorithms

**Phase 6: Analysis** (Weeks 39-46)
- [ ] All trace formats
- [ ] Sampling techniques
- [ ] MRC generation
- [ ] Trace analysis

**Phase 7: CLI Tools** (Weeks 47-51)
- [ ] cachesim
- [ ] mrc
- [ ] traceanalyzer
- [ ] traceutils

**Phase 8: Polish** (Weeks 52-55)
- [ ] Integration testing
- [ ] Documentation
- [ ] Release

### Current Status

✅ **Completed:**
- Basic workspace structure
- 3 cache algorithms (FIFO, LRU, Clock)
- Basic Request type
- Initial tests

🚧 **In Progress:**
- None (awaiting plan approval)

📋 **Next Steps:**
1. Review and approve this roadmap
2. Begin Week 1 tasks
3. Set up workspace structure
4. Implement enhanced core types

---

## Appendix: Quick Commands

### Development

```bash
# Build everything
cargo build --all --all-features

# Run tests
cargo test --all --all-features

# Run clippy
cargo clippy --all --all-features -- -D warnings

# Format code
cargo fmt --all

# Generate docs
cargo doc --all --no-deps --open

# Run benchmarks
cargo bench --all

# Check coverage
cargo tarpaulin --all-features --workspace --timeout 300

# Release build
cargo build --release --all
```

### CI/CD

```bash
# Local CI simulation
act -j test

# Publish (maintainers only)
cargo publish --dry-run
cargo publish
```

This roadmap provides concrete, actionable steps for each week of the project. Follow it sequentially, adjusting as needed based on progress and discoveries.
