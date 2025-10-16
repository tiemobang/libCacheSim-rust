use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cache statistics tracking performance metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    #[serde(skip)]
    n_req: AtomicU64,
    #[serde(skip)]
    n_hit: AtomicU64,
    #[serde(skip)]
    n_miss: AtomicU64,
    #[serde(skip)]
    n_evict: AtomicU64,
    #[serde(skip)]
    n_insert: AtomicU64,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStats {
    /// Create a new statistics tracker
    pub fn new() -> Self {
        CacheStats {
            n_req: AtomicU64::new(0),
            n_hit: AtomicU64::new(0),
            n_miss: AtomicU64::new(0),
            n_evict: AtomicU64::new(0),
            n_insert: AtomicU64::new(0),
        }
    }

    /// Get the total number of requests
    pub fn n_req(&self) -> u64 {
        self.n_req.load(Ordering::Relaxed)
    }

    /// Get the number of cache hits
    pub fn n_hit(&self) -> u64 {
        self.n_hit.load(Ordering::Relaxed)
    }

    /// Get the number of cache misses
    pub fn n_miss(&self) -> u64 {
        self.n_miss.load(Ordering::Relaxed)
    }

    /// Get the number of evictions
    pub fn n_evict(&self) -> u64 {
        self.n_evict.load(Ordering::Relaxed)
    }

    /// Get the number of insertions
    pub fn n_insert(&self) -> u64 {
        self.n_insert.load(Ordering::Relaxed)
    }

    /// Increment request counter
    pub fn inc_req(&self) {
        self.n_req.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment hit counter
    pub fn inc_hit(&self) {
        self.n_hit.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment miss counter
    pub fn inc_miss(&self) {
        self.n_miss.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment eviction counter
    pub fn inc_evict(&self) {
        self.n_evict.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment insertion counter
    pub fn inc_insert(&self) {
        self.n_insert.fetch_add(1, Ordering::Relaxed);
    }

    /// Calculate hit ratio
    pub fn hit_ratio(&self) -> f64 {
        let n_req = self.n_req();
        if n_req == 0 {
            0.0
        } else {
            self.n_hit() as f64 / n_req as f64
        }
    }

    /// Calculate miss ratio
    pub fn miss_ratio(&self) -> f64 {
        1.0 - self.hit_ratio()
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.n_req.store(0, Ordering::Relaxed);
        self.n_hit.store(0, Ordering::Relaxed);
        self.n_miss.store(0, Ordering::Relaxed);
        self.n_evict.store(0, Ordering::Relaxed);
        self.n_insert.store(0, Ordering::Relaxed);
    }
}

impl Clone for CacheStats {
    fn clone(&self) -> Self {
        CacheStats {
            n_req: AtomicU64::new(self.n_req()),
            n_hit: AtomicU64::new(self.n_hit()),
            n_miss: AtomicU64::new(self.n_miss()),
            n_evict: AtomicU64::new(self.n_evict()),
            n_insert: AtomicU64::new(self.n_insert()),
        }
    }
}
