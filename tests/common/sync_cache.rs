use crate::common::{NotATransaction, TestCacheMetadata};
use aptos_in_memory_cache::{Cache, SizedCache};
use get_size::GetSize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct TestCache<C: SizedCache<NotATransaction> + 'static> {
    pub metadata: Arc<TestCacheMetadata>,
    pub cache: Arc<C>,
    pub insert_notify: Arc<Notify>,
    pub eviction_start: Arc<AtomicUsize>,
    pub eviction_task: JoinHandle<()>,
}

impl<C: SizedCache<NotATransaction> + 'static> Drop for TestCache<C> {
    fn drop(&mut self) {
        self.eviction_task.abort();
    }
}

impl<C: SizedCache<NotATransaction> + 'static> TestCache<C> {
    pub fn with_capacity(
        c: C,
        eviction_trigger_size_in_bytes: usize,
        target_size_in_bytes: usize,
    ) -> Self {
        let metadata = Arc::new(TestCacheMetadata {
            eviction_trigger_size_in_bytes,
            target_size_in_bytes,
            capacity: c.capacity(),
        });
        let cache = Arc::new(c);
        let insert_notify = Arc::new(Notify::new());
        let highest_key = Arc::new(AtomicUsize::new(0));
        let task = spawn_eviction_task(
            insert_notify.clone(),
            highest_key.clone(),
            metadata.clone(),
            cache.clone(),
        );

        let cache = Self {
            metadata,
            cache,
            insert_notify,
            eviction_start: highest_key,
            eviction_task: task,
        };

        cache
    }
}

impl<C: SizedCache<NotATransaction> + 'static> Cache<usize, NotATransaction> for TestCache<C> {
    fn get(&self, key: &usize) -> Option<NotATransaction> {
        self.cache.get(key).and_then(|entry| {
            if entry.key == *key {
                return Some(entry.value.clone());
            }
            None
        })
    }

    fn insert(&self, key: usize, value: NotATransaction) {
        let size_in_bytes = value.get_size();
        self.cache.insert_with_size(key, value, size_in_bytes);
        if self.cache.total_size() > self.metadata.eviction_trigger_size_in_bytes {
            self.eviction_start.store(key, Ordering::Relaxed);
            self.insert_notify.notify_one();
        }
    }

    fn total_size(&self) -> usize {
        self.cache.total_size() as usize
    }
}

/// Perform cache eviction on a separate task.
fn spawn_eviction_task<C: SizedCache<NotATransaction> + 'static>(
    insert_notify: Arc<Notify>,
    highest_key: Arc<AtomicUsize>,
    metadata: Arc<TestCacheMetadata>,
    cache: Arc<C>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            insert_notify.notified().await;
            let watermark_value = highest_key.load(Ordering::Relaxed);
            let mut eviction_index = (watermark_value + 1) % metadata.capacity;

            // Evict entries until the cache size is below the target size
            while cache.total_size() > metadata.target_size_in_bytes {
                if let Some(value) = cache.evict(&eviction_index) {
                    if value.key > watermark_value {
                        cache.insert_with_size(value.key, value.value.clone(), value.size_in_bytes);
                        break;
                    }
                }
                eviction_index = (eviction_index + 1) % metadata.capacity;
            }
        }
    })
}
