//! libCacheSim Eviction Algorithms
//!
//! This crate provides cache eviction algorithm implementations.

// Phase 1: Basic algorithms
mod clock;
mod fifo;
mod lfu;
mod lfuda;
mod lru;
mod mru;
mod random;
mod slru;

pub use clock::ClockCache;
pub use fifo::FifoCache;
pub use lfu::LfuCache;
pub use lfuda::LfudaCache;
pub use lru::LruCache;
pub use mru::MruCache;
pub use random::RandomCache;
pub use slru::SlruCache;
