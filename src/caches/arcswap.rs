use crate::{SizedCache, SizedCacheEntry};
use aarc::{AtomicArc, Snapshot};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

const MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

pub struct ArcSwapCache<T: Send + Sync + Clone + 'static> {
    cache: Box<[AtomicArc<Option<SizedCacheEntry<T>>>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> ArcSwapCache<T>
where
    T: Send + Sync + Clone + 'static,
{
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(AtomicArc::new(None));
        }

        Self {
            cache: buffer.into_boxed_slice(),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for ArcSwapCache<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::with_capacity(MAX_NUM_CACHE_ITEMS)
    }
}

impl<T> SizedCache<T> for ArcSwapCache<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn get(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        self.cache[*key % self.capacity]
            .load::<Snapshot<_>>()
            .and_then(|v| v.clone())
    }

    fn insert_with_size(&self, key: usize, value: Arc<T>, size_in_bytes: usize) -> usize {
        // Get lock for cache entry
        let index = key % self.capacity;
        let arc_swap = &self.cache[index];
        let arc = arc_swap.load::<Snapshot<_>>().and_then(|v| v.clone());

        // Update cache size
        if let Some(prev_value) = arc {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
        }

        // Update cache entry
        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);

        arc_swap.store(Some(&Arc::new(Some(SizedCacheEntry {
            key,
            value,
            size_in_bytes,
        }))));

        index
    }

    fn evict(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        // Get lock for cache entry
        let arc_swap = &self.cache[*key % self.capacity];

        // Update cache size & set previous value to none
        let arc = arc_swap.load::<Snapshot<_>>().and_then(|v| v.clone());
        if let Some(prev_value) = arc {
            self.size
                .fetch_sub(prev_value.size_in_bytes, Ordering::Relaxed);
            arc_swap.store(Some(&Arc::new(None)));
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
