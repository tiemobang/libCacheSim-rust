//! Example: Trace-based cache simulation
//!
//! Demonstrates reading traces and simulating cache performance

use libcachesim::{Cache, FifoCache, LruCache};
use libcachesim_trace::{TraceError, TraceFormat, TraceReaderBuilder};
use std::io::Write;
use tempfile::NamedTempFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== libCacheSim Rust - Trace-based Simulation ===\n");

    // Create a sample CSV trace
    let trace_file = create_sample_trace()?;

    println!(
        "Running simulation with trace file: {:?}\n",
        trace_file.path()
    );

    // Test with FIFO cache
    println!("--- FIFO Cache ---");
    run_simulation_with_cache(FifoCache::new(300), trace_file.path().to_str().unwrap())?;

    println!();

    // Test with LRU cache
    println!("--- LRU Cache ---");
    run_simulation_with_cache(LruCache::new(300), trace_file.path().to_str().unwrap())?;

    Ok(())
}

fn create_sample_trace() -> std::io::Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;

    // Write CSV header
    writeln!(file, "timestamp,obj_id,obj_size")?;

    // Write sample requests
    writeln!(file, "0,1,100")?;
    writeln!(file, "1,2,100")?;
    writeln!(file, "2,3,100")?;
    writeln!(file, "3,1,100")?; // Repeat access to object 1
    writeln!(file, "4,4,100")?; // This will cause eviction
    writeln!(file, "5,2,100")?; // Repeat access to object 2
    writeln!(file, "6,5,100")?;
    writeln!(file, "7,3,100")?; // Repeat access to object 3
    writeln!(file, "8,6,100")?;
    writeln!(file, "9,1,100")?; // Repeat access to object 1

    file.flush()?;
    Ok(file)
}

fn run_simulation_with_cache<C: Cache>(mut cache: C, trace_path: &str) -> Result<(), TraceError> {
    let reader = TraceReaderBuilder::new(trace_path)
        .format(TraceFormat::Csv)
        .build()?;

    let mut n_req = 0;

    for result in reader {
        let req = result?;
        cache.get(&req);
        n_req += 1;
    }

    let stats = cache.stats();
    println!("  Total requests: {}", n_req);
    println!("  Hits: {}, Misses: {}", stats.n_hit(), stats.n_miss());
    println!("  Hit ratio: {:.1}%", stats.hit_ratio() * 100.0);
    println!("  Miss ratio: {:.1}%", stats.miss_ratio() * 100.0);
    println!("  Evictions: {}", stats.n_evict());

    Ok(())
}
