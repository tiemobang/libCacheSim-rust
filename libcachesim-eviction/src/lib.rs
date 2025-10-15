//! libCacheSim Eviction Algorithms
//!
//! This crate provides cache eviction algorithm implementations.

mod clock;
mod fifo;
mod lru;
mod mru;
mod random;

pub use clock::ClockCache;
pub use fifo::FifoCache;
pub use lru::LruCache;
pub use mru::MruCache;
pub use random::RandomCache;
