use aptos_in_memory_cache::caches::arcswap::ArcSwapCache;
use aptos_in_memory_cache::caches::fifo::FIFOCache;
use aptos_in_memory_cache::caches::sync_mutex::SyncMutexCache;
// use aptos_in_memory_cache::caches::sync_mutex::SyncMutexCache;
// use aptos_in_memory_cache::caches::sync_rwlock::SyncRwLockCache;
use aptos_in_memory_cache::{Cache, SizedCache};
use get_size::GetSize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::{JoinHandle, JoinSet};

#[derive(Clone, GetSize, Debug, PartialEq)]
pub struct NotATransaction {
    transaction_version: i64,
}

impl NotATransaction {
    pub fn new(transaction_version: i64) -> Self {
        Self {
            transaction_version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestCacheMetadata {
    pub eviction_trigger_size_in_bytes: usize,
    pub target_size_in_bytes: usize,
    pub capacity: usize,
}

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
    fn get(&self, key: &usize) -> Option<Arc<NotATransaction>> {
        self.cache.get(key).and_then(|entry| {
            if entry.key == *key {
                return Some(entry.value.clone());
            }
            None
        })
    }

    fn insert(&self, key: usize, value: NotATransaction) {
        // let size_in_bytes = size_of_unique(&value);
        let size_in_bytes = GetSize::get_size(&value);
        self.cache
            .insert_with_size(key, Arc::new(value), size_in_bytes);
        if self.cache.total_size() > self.metadata.eviction_trigger_size_in_bytes {
            self.eviction_start.store(key, Ordering::Relaxed);
            self.insert_notify.notify_one();
        }
    }

    fn total_size(&self) -> usize {
        self.cache.total_size()
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
                        cache.insert_with_size(value.key, value.value, value.size_in_bytes);
                        break;
                    }
                }
                eviction_index = (eviction_index + 1) % metadata.capacity;
            }
        }
    })
}

#[tokio::main]
async fn main() {
    let ca = SyncMutexCache::with_capacity(1_000_000);
    let cache = Arc::new(TestCache::with_capacity(ca, 1_100_000, 1_000_000));
    // let cache = Arc::new(FIFOCache::new(1_000_000, 1_100_000, |key, _| Some(key + 1)));

    let mut join_set = JoinSet::new();
    let num = 5_000_000;

    for _ in 0..1 {
        let c = cache.clone();
        join_set.spawn(async move {
            for i in 0..num {
                c.insert(i, NotATransaction::new(i as i64));
            }
        });
    }

    for _ in 0..100 {
        let c = cache.clone();
        join_set.spawn(async move {
            for i in 0..num {
                c.get(&i);
            }
        });
    }

    join_set.join_next().await;
}
