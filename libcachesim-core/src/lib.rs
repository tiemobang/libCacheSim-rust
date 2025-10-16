//! libCacheSim Core Types and Traits
//!
//! This crate provides the core types, traits, and utilities for libCacheSim.

mod cache;
mod error;
mod request;
mod stats;

pub use cache::{Cache, CacheResult, InsertResult};
pub use error::CacheError;
pub use request::{Operation, Request};
pub use stats::CacheStats;

/// Object ID type
pub type ObjectId = u64;

/// Object size type  
pub type ObjectSize = u32;
