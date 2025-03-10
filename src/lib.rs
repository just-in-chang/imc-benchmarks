// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use futures::Stream;
use std::{hash::Hash, sync::Arc};

pub mod caches;

/// A struct that holds a single cache entry, containing its key, value, and size in bytes.
#[derive(Clone, Debug)]
pub struct SizedCacheEntry<V> {
    pub key: usize,
    pub value: V,
    pub size_in_bytes: usize,
}

/// A trait for a cache that can be used to store key-value pairs.
pub trait SizedCache<V>: Send + Sync {
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    fn get(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Inserts a given key-value pair in cache. Returns the index of the inserted entry. Panics if the insert fails.
    fn insert_with_size(&self, key: usize, value: V, size_in_bytes: usize) -> usize;

    /// Evicts the value for a given key from the cache.
    fn evict(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;

    /// Returns the capacity of the cache.
    fn capacity(&self) -> usize;
}

/// A trait for a cache that can be used to store key-value pairs.
pub trait Cache<K, V>: Send + Sync
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    fn get(&self, key: &K) -> Option<V>;

    /// Inserts a given key-value pair in cache. Panics if the insert fails.
    fn insert(&self, key: K, value: V);

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;
}

/// A trait for a cache that can be used to store key-value pairs.
#[async_trait::async_trait]
pub trait AsyncSizedCache<V>: Send + Sync
where
    V: Send + Sync,
{
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    async fn get(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Inserts a given key-value pair in cache. Returns the index of the inserted entry. Panics if the insert fails.
    async fn insert_with_size(&self, key: usize, value: Arc<V>, size_in_bytes: usize) -> usize;

    /// Evicts the value for a given key from the cache.
    async fn evict(&self, key: &usize) -> Option<SizedCacheEntry<V>>;

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;

    /// Returns the capacity of the cache.
    fn capacity(&self) -> usize;
}

/// A trait for a cache that can be used to store key-value pairs.
#[async_trait::async_trait]
pub trait AsyncCache<K, V>: Send + Sync
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Get the value for a given key. Return [`None`] if the key is not in the cache.
    async fn get(&self, key: &K) -> Option<Arc<V>>;

    /// Inserts a given key-value pair in cache. Panics if the insert fails.
    async fn insert(&self, key: K, value: V);

    /// Returns the total size of the cache.
    fn total_size(&self) -> usize;
}

pub trait OrderedCache<K, V>: Cache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Returns the first key in the cache. Returns [`None`] if the cache is empty.
    fn first_key(&self) -> Option<K>;

    /// Returns the last key in the cache. Returns [`None`] if the cache is empty.
    fn last_key(&self) -> Option<K>;
}

pub trait StreamableOrderedCache<K, V>: OrderedCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Returns the next key in the cache. Returns [`None`] if the key is not in the cache.
    fn next_key(&self, key: &K) -> Option<K>;

    /// Returns the previous key in the cache. Returns [`None`] if the next key or value is not in the cache.
    fn next_key_and_value(&self, key: &K) -> Option<(K, Arc<V>)>;

    /// Returns a stream of values in the cache.
    fn get_stream(&self, starting_key: Option<K>) -> impl Stream<Item = Arc<V>> + '_;
}
