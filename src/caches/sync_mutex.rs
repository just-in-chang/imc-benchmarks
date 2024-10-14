use crate::{SizedCache, SizedCacheEntry};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

const DEFAULT_MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

/// A cache that uses a mutex to synchronize access to the cache entries.
pub struct SyncMutexCache<T> {
    cache: Box<[SizedCacheEntry<T>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> SyncMutexCache<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(SizedCacheEntry::new());
        }

        Self {
            cache: buffer.into_boxed_slice(),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for SyncMutexCache<T> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_NUM_CACHE_ITEMS)
    }
}

impl<T> SizedCache<T> for SyncMutexCache<T> {
    fn get(&self, key: &usize) -> Option<(usize, &T)> {
        let entry = &self.cache[*key % self.capacity];
        let ptr = entry.value.load(Ordering::Acquire);

        if ptr.is_null() {
            return None;
        }

        unsafe {
            Arc::increment_strong_count(ptr);
            ptr.as_ref()
                .map(|value| (entry.key.load(Ordering::Acquire), value))
        }
    }

    fn insert_with_size(&self, key: usize, value: T, size_in_bytes: usize) -> usize {
        let entry = &self.cache[key % self.capacity];
        let arc = Arc::new(value);
        let arc_ptr = Arc::into_raw(arc);

        // Swap the old pointer atomically
        let old_ptr = entry.value.swap(arc_ptr as *mut _, Ordering::AcqRel);

        // Decrease the size if there was a previous value
        if !old_ptr.is_null() {
            // unsafe {
            //     Arc::from_raw(old_ptr);
            // }
            self.size.fetch_sub(
                entry.size_in_bytes.load(Ordering::Acquire),
                Ordering::Relaxed,
            );
        }

        // Update size and key
        entry.size_in_bytes.store(size_in_bytes, Ordering::Release);
        entry.key.store(key, Ordering::Release);

        // Increase the size
        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);

        key % self.capacity
    }

    fn evict(&self, key: &usize) -> Option<(usize, T)> {
        let entry = &self.cache[*key % self.capacity];
        let ptr = entry.value.swap(std::ptr::null_mut(), Ordering::AcqRel);

        if ptr.is_null() {
            return None;
        }

        let old_key = entry.key.swap(0, Ordering::Release);
        entry.size_in_bytes.store(0, Ordering::Release);

        unsafe {
            // Reconstruct the Arc to decrement the reference count
            let arc = Arc::from_raw(ptr);
            match Arc::try_unwrap(arc) {
                Ok(value) => Some((old_key, value)),
                Err(_) => None,
            }
        }
    }

    fn total_size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}
