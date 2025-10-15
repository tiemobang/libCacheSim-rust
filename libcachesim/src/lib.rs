//! libCacheSim - A high-performance cache simulation library
//!
//! This is the main crate that re-exports all libCacheSim functionality.
//!
//! # Examples
//!
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

// Re-export core types and traits
pub use libcachesim_core::{
    Cache, CacheError, CacheResult, CacheStats, InsertResult, ObjectId, ObjectSize, Operation,
    Request,
};

// Re-export eviction algorithms
pub use libcachesim_eviction::{
    ClockCache, FifoCache, LfuCache, LfudaCache, LruCache, MruCache, RandomCache, SlruCache,
};

// Convenience prelude module
pub mod prelude {
    pub use libcachesim_core::{Cache, CacheResult, Request};
    pub use libcachesim_eviction::*;
}

