//! libCacheSim - A high-performance cache simulation library
//!
//! This is the main crate that re-exports all libCacheSim functionality.
//!
//! # Examples
//!
//! ## Basic Cache Usage
//! ```
//! use libcachesim::{Cache, FifoCache, Request};
//!
//! let mut cache = FifoCache::new(1024);
//! let req = Request::new(1, 100);
//! cache.get(&req);
//!
//! let stats = cache.stats();
//! println!("Hit ratio: {:.2}%", stats.hit_ratio() * 100.0);
//! ```
//!
//! ## Trace-based Simulation
//! ```no_run
//! use libcachesim::{Cache, LruCache};
//! use libcachesim_trace::{TraceReaderBuilder};
//!
//! let mut cache = LruCache::new(1024);
//! let reader = TraceReaderBuilder::new("trace.csv").build().unwrap();
//!
//! for result in reader {
//!     let req = result.unwrap();
//!     cache.get(&req);
//! }
//! ```

// Re-export core types and traits
pub use libcachesim_core::{
    Cache, CacheError, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Operation,
    Request,
};

// Re-export eviction algorithms
pub use libcachesim_eviction::{
    // Phase 1: Basic algorithms
    ClockCache, FifoCache, LfuCache, LfudaCache, LruCache, MruCache, RandomCache, SlruCache,
    // Phase 2: Advanced algorithms (in progress)
    ArcCache, S3FifoCache, SieveCache, TwoQCache,
};

// Re-export trace reading (at top level for convenience)
pub use libcachesim_trace;

// Convenience prelude module
pub mod prelude {
    pub use libcachesim_core::{Cache, CacheResult, Request};
    pub use libcachesim_eviction::*;
    pub use libcachesim_trace::{TraceReader, TraceReaderBuilder};
}
