mod common;

use std::sync::Arc;

use aptos_in_memory_cache::{caches::sync_mutex::SyncMutexCache, Cache};
use common::{NotATransaction, TestCache};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

async fn bench() {
    let ca = SyncMutexCache::with_capacity(10000);
    let cache = Arc::new(TestCache::with_capacity(ca, 1_100, 1_000));

    let c1 = cache.clone();
    let t1 = tokio::spawn(async move {
        for i in 0..1_000_000 {
            c1.insert(i, NotATransaction::new(i as i64));
        }
    });

    let c2 = cache.clone();
    let t2 = tokio::spawn(async move {
        // loop {
        for i in 0..1_000_000 {
            c2.get(&i);
        }
        // }
    });

    tokio::select! {
        _ = t1 => (),
        _ = t2 => (),
    };
}

pub fn async_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("AsyncGroup");
    group.bench_function("async_function", |b| {
        let rt = Runtime::new().unwrap();
        b.to_async(&rt).iter(|| bench());
    });
    group.finish();
}

criterion_group!(benches, async_bench);
criterion_main!(benches);
