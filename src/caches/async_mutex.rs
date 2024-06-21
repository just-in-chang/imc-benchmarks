use crate::{AsyncSizedCache, SizedCacheEntry};
use tokio::sync::Mutex;
// use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

type CacheEntry<T> = Arc<Mutex<Option<SizedCacheEntry<T>>>>;

const MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

pub struct AsyncMutexCache<T: Send + Sync + Clone> {
    cache: Box<[CacheEntry<T>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> AsyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Arc::new(Mutex::new(None)));
        }

        Self {
            cache: buffer.into_boxed_slice(),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for AsyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    fn default() -> Self {
        Self::with_capacity(MAX_NUM_CACHE_ITEMS)
    }
}

#[async_trait::async_trait]
impl<T> AsyncSizedCache<T> for AsyncMutexCache<T>
where
    T: Send + Sync + Clone,
{
    async fn get(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        let arc = self.cache[*key % self.capacity].clone();
        let lock = arc.lock().await;
        lock.clone()
    }

    async fn insert_with_size(&self, key: usize, value: Arc<T>, size_in_bytes: usize) -> usize {
        // Get lock for cache entry
        let index = key % self.capacity;
        let arc = self.cache[index].clone();
        let mut lock = arc.lock().await;

        // Update cache size
        if let Some(prev_value) = &*lock {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
        }

        // Update cache entry
        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);
        *lock = Some(SizedCacheEntry {
            key,
            value,
            size_in_bytes,
        });

        index
    }

    async fn evict(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        // Get lock for cache entry
        let arc = self.cache[*key % self.capacity].clone();
        let mut lock = arc.lock().await;

        // Update cache size & set previous value to none
        if let Some(prev_value) = lock.take() {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
            return Some(prev_value);
        }
        None
    }

    fn total_size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}
