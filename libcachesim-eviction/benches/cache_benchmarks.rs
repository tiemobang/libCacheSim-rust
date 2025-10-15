use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use libcachesim_core::{Cache, Request};
use libcachesim_eviction::{ClockCache, FifoCache, LruCache, MruCache, RandomCache};

fn bench_fifo_get(c: &mut Criterion) {
    c.bench_function("fifo_get", |b| {
        let mut cache = FifoCache::new(10000);
        let req = Request::new(1, 100);
        cache.get(&req); // Prime the cache

        b.iter(|| cache.get(black_box(&req)));
    });
}

fn bench_lru_get(c: &mut Criterion) {
    c.bench_function("lru_get", |b| {
        let mut cache = LruCache::new(10000);
        let req = Request::new(1, 100);
        cache.get(&req); // Prime the cache

        b.iter(|| cache.get(black_box(&req)));
    });
}

fn bench_cache_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_comparison");
    let cache_size = 10000;

    // Create a sequence of requests
    let requests: Vec<Request> = (0..1000).map(|i| Request::new(i % 100, 100)).collect();

    group.bench_with_input(
        BenchmarkId::new("FIFO", cache_size),
        &requests,
        |b, reqs| {
            b.iter(|| {
                let mut cache = FifoCache::new(cache_size);
                for req in reqs {
                    cache.get(black_box(req));
                }
            });
        },
    );

    group.bench_with_input(BenchmarkId::new("LRU", cache_size), &requests, |b, reqs| {
        b.iter(|| {
            let mut cache = LruCache::new(cache_size);
            for req in reqs {
                cache.get(black_box(req));
            }
        });
    });

    group.bench_with_input(
        BenchmarkId::new("Clock", cache_size),
        &requests,
        |b, reqs| {
            b.iter(|| {
                let mut cache = ClockCache::new(cache_size);
                for req in reqs {
                    cache.get(black_box(req));
                }
            });
        },
    );

    group.bench_with_input(BenchmarkId::new("MRU", cache_size), &requests, |b, reqs| {
        b.iter(|| {
            let mut cache = MruCache::new(cache_size);
            for req in reqs {
                cache.get(black_box(req));
            }
        });
    });

    group.bench_with_input(
        BenchmarkId::new("Random", cache_size),
        &requests,
        |b, reqs| {
            b.iter(|| {
                let mut cache = RandomCache::new(cache_size);
                for req in reqs {
                    cache.get(black_box(req));
                }
            });
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_fifo_get,
    bench_lru_get,
    bench_cache_comparison
);
criterion_main!(benches);
