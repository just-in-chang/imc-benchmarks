use std::time::Duration;

use aptos_in_memory_cache::{Cache, SizedCache};
use common::{sync_cache::TestCache, NotATransaction};
use get_size::GetSize;

pub mod common;

#[cfg(test)]
mod tests {
    use aptos_in_memory_cache::caches::atomicptr::AtomicPtrCache;

    use super::*;
    // use aptos_in_memory_cache::caches::sync_mutex::ArcSwapCache;

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_insert_out_of_order() {
        let cache = AtomicPtrCache::with_capacity(10);
        test_insert_out_of_order_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_array_wrap_around() {
        let cache = AtomicPtrCache::with_capacity(10);
        test_array_wrap_around_impl(cache);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_on_size_limit() {
        let cache = AtomicPtrCache::with_capacity(10);
        test_eviction_on_size_limit_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_out_of_order_inserts() {
        let cache = AtomicPtrCache::with_capacity(20);
        test_eviction_out_of_order_inserts_impl(cache).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn test_eviction_with_array_wrap_around() {
        let cache = AtomicPtrCache::with_capacity(10);
        test_eviction_with_array_wrap_around_impl(cache).await;
    }
}

pub async fn test_insert_out_of_order_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 150, 100);
    let key = 100;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 101;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 105;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 103;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 102;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 104;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));
}

pub fn test_array_wrap_around_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 150, 100);
    let key = 7;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 8;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 12;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 10;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 9;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));

    let key = 11;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value.clone());
    assert_eq!(cache.get(&key).clone(), Some(value));
}

pub async fn test_eviction_on_size_limit_impl<C: SizedCache<NotATransaction> + 'static>(c: C) {
    let cache = TestCache::with_capacity(c, 56, 48);

    // Insert initial items
    for i in 0..6 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    assert_eq!(cache.total_size(), 6 * NotATransaction::new(0).get_size());

    tokio::time::sleep(Duration::from_micros(1)).await;

    // This insert should trigger eviction
    let key = 6;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    // assert_eq!(cache.total_size(), 6 * NotATransaction::new(0).get_size());

    // Further inserts to ensure eviction continues correctly
    for i in 7..10 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 6 * NotATransaction::new(0).get_size());
}

pub async fn test_eviction_out_of_order_inserts_impl<C: SizedCache<NotATransaction> + 'static>(
    c: C,
) {
    let cache = TestCache::with_capacity(c, 88, 80);

    // Insert items out of order
    let keys = [0, 5, 1, 3, 7, 2, 6, 4, 9, 8];
    for &key in &keys {
        let value = NotATransaction::new(key as i64);
        cache.insert(key, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 10 * NotATransaction::new(0).get_size());

    tokio::time::sleep(Duration::from_micros(1)).await;

    // This insert should trigger eviction
    let key = 10;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    // Wait for eviction to occur
    tokio::time::sleep(Duration::from_micros(1)).await;

    // assert_eq!(cache.total_size(), 10 * NotATransaction::new(0).get_size());

    tokio::time::sleep(Duration::from_micros(1)).await;

    // Further inserts to ensure eviction continues correctly
    let key = 11;
    let value = NotATransaction::new(key as i64);
    cache.insert(key, value);

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 10 * NotATransaction::new(0).get_size());
}

pub async fn test_eviction_with_array_wrap_around_impl<C: SizedCache<NotATransaction> + 'static>(
    c: C,
) {
    let cache = TestCache::with_capacity(c, 48, 40);

    // Insert items to fill the cache
    for i in 5..10 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 5 * NotATransaction::new(0).get_size());

    tokio::time::sleep(Duration::from_micros(1)).await;

    // Insert more items to cause wrap-around
    for i in 10..12 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 5 * NotATransaction::new(0).get_size());

    // Insert even more items to fully wrap-around
    for i in 12..15 {
        let value = NotATransaction::new(i as i64);
        cache.insert(i, value);
    }

    tokio::time::sleep(Duration::from_micros(1)).await;

    assert_eq!(cache.total_size(), 5 * NotATransaction::new(0).get_size());
}
