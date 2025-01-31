mod common;

use std::sync::Arc;

use aptos_in_memory_cache::{caches::sync_mutex::SyncMutexCache, AsyncCache, Cache};
use common::{sync_cache::TestCache, NotATransaction};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::{runtime::Runtime, task::JoinSet};

async fn bench_sync_cache() {
    let ca = SyncMutexCache::with_capacity(1_000_000);
    // let ca = SyncRwLockCache::with_capacity(1_000_000);
    let cache = Arc::new(TestCache::with_capacity(ca, 1_100_000, 1_000_000));
    // let cache = Arc::new(FIFOCache::new(1_000_000, 1_100_000, |key, _| Some(key + 1)));

    // let mut join_set = JoinSet::new();
    // let num = 5_000_000;

    // for _ in 0..1 {
    //     let c = cache.clone();
    //     join_set.spawn(async move {
    //         for i in 0..num {
    //             c.insert(i, NotATransaction::new(i as i64));
    //         }
    //     });
    // }

    // for _ in 0..100 {
    //     let c = cache.clone();
    //     join_set.spawn(async move {
    //         for i in 0..num {
    //             c.get(&i);
    //         }
    //     });
    // }

    // join_set.join_next().await;

    let c1 = cache.clone();
    let t1 = tokio::spawn(async move {
        for i in 0..1_000_000 {
            c1.insert(i, NotATransaction::new(i as i64));
        }

        for i in 0..1_000_000 {
            c1.get(&i);
        }
    });

    let c2 = cache.clone();
    let t2 = tokio::spawn(async move {
        loop {
            for i in 0..1_000_000 {
                c2.get(&i);
            }
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        }
    });

    tokio::select! {
        t1u = t1 => t1u.unwrap(),
        t2u = t2 => t2u.unwrap(),
    }
}

// async fn bench_async_cache() {
//     let ca = AsyncMutexCache::with_capacity(1_000_000);
//     let cache = Arc::new(AsyncTestCache::with_capacity(ca, 1_100_000, 1_000_000));

//     let c1 = cache.clone();
//     let t1 = tokio::spawn(async move {
//         for i in 0..1_000_000 {
//             c1.insert(i, NotATransaction::new(i as i64)).await;
//         }

//         for i in 0..1_000_000 {
//             c1.get(&i).await;
//         }
//     });

//     let c2 = cache.clone();
//     let t2 = tokio::spawn(async move {
//         loop {
//             for i in 0..1_000_000 {
//                 c2.get(&i).await;
//             }
//             tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
//         }
//     });

//     tokio::select! {
//         t1u = t1 => t1u.unwrap(),
//         t2u = t2 => t2u.unwrap(),
//     }
// }

pub fn sync_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("AsyncGroup");
    group.bench_function("async_sync_function", |b| {
        let rt = Runtime::new().unwrap();
        b.to_async(&rt).iter(|| bench_sync_cache());
    });
    group.finish();
}

// pub fn async_bench(c: &mut Criterion) {
//     let mut group = c.benchmark_group("AsyncGroup");
//     group.bench_function("async_function", |b| {
//         let rt = Runtime::new().unwrap();
//         b.to_async(&rt).iter(|| bench_async_cache());
//     });
//     group.finish();
// }

criterion_group!(benches, sync_bench);
criterion_main!(benches);
