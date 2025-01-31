use crate::{SizedCache, SizedCacheEntry};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

type CacheEntry<T> = AtomicPtr<SizedCacheEntry<T>>;

const DEFAULT_MAX_NUM_CACHE_ITEMS: usize = 1_000_000;

/// A cache that uses atomic pointers to synchronize access to the cache entries.
pub struct AtomicPtrCache<T: Send + Sync + Clone> {
    cache: Box<[CacheEntry<T>]>,
    capacity: usize,
    size: AtomicUsize,
}

impl<T> AtomicPtrCache<T>
where
    T: Send + Sync + Clone,
{
    pub fn with_capacity(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(AtomicPtr::new(std::ptr::null_mut()));
        }

        Self {
            cache: buffer.into_boxed_slice(),
            capacity,
            size: AtomicUsize::new(0),
        }
    }
}

impl<T> Default for AtomicPtrCache<T>
where
    T: Send + Sync + Clone,
{
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_NUM_CACHE_ITEMS)
    }
}

impl<T> Drop for AtomicPtrCache<T>
where
    T: Send + Sync + Clone,
{
    fn drop(&mut self) {
        for entry in self.cache.iter() {
            let ptr = entry.load(Ordering::Acquire);
            if !ptr.is_null() {
                unsafe {
                    drop(Box::from_raw(ptr));
                }
            }
        }
    }
}

impl<T> SizedCache<T> for AtomicPtrCache<T>
where
    T: Send + Sync + Clone,
{
    fn get(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        let ptr = self.cache[*key % self.capacity].load(Ordering::Acquire);
        unsafe { (!ptr.is_null()).then(|| (*ptr).clone()) }
    }

    fn insert_with_size(&self, key: usize, value: T, size_in_bytes: usize) -> usize {
        let index = key % self.capacity;
        let entry = SizedCacheEntry {
            key,
            value,
            size_in_bytes,
        };

        let old_ptr = self.cache[index].swap(Box::into_raw(Box::new(entry)), Ordering::AcqRel);

        // Clean up old entry and update size
        unsafe {
            if !old_ptr.is_null() {
                self.size
                    .fetch_sub((*old_ptr).size_in_bytes, Ordering::Relaxed);
                drop(Box::from_raw(old_ptr));
            }
        }

        self.size.fetch_add(size_in_bytes, Ordering::Relaxed);
        index
    }

    fn evict(&self, key: &usize) -> Option<SizedCacheEntry<T>> {
        let index = key % self.capacity;
        let old_ptr = self.cache[index].swap(std::ptr::null_mut(), Ordering::AcqRel);

        unsafe {
            if !old_ptr.is_null() {
                let entry = Box::from_raw(old_ptr);
                self.size.fetch_sub(entry.size_in_bytes, Ordering::Relaxed);
                return Some((*entry).clone());
            }
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
