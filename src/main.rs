use allocative::{size_of_unique, Allocative};
use aptos_in_memory_cache::caches::sync_mutex::SyncMutexCache;
use aptos_in_memory_cache::{Cache, SizedCache};
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    sync::watch::{Receiver, Sender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Allocative, Debug, PartialEq)]
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

pub struct TestCache<C: SizedCache<NotATransaction> + 'static> {
    pub metadata: Arc<TestCacheMetadata>,
    pub cache: Arc<C>,
    pub insert_watch: Arc<Sender<usize>>,
    pub _cancellation_token_drop_guard: tokio_util::sync::DropGuard,
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
        let cancellation_token: CancellationToken = CancellationToken::new();
        let metadata = Arc::new(TestCacheMetadata {
            eviction_trigger_size_in_bytes,
            target_size_in_bytes,
            capacity: c.capacity(),
        });
        let cache = Arc::new(c);
        let (tx, rx) = tokio::sync::watch::channel(0_usize);
        let task = TestCache::spawn_eviction_task(rx, metadata.clone(), cache.clone());

        let cache = Self {
            metadata,
            cache,
            insert_watch: Arc::new(tx),
            _cancellation_token_drop_guard: cancellation_token.clone().drop_guard(),
            eviction_task: task,
        };

        cache
    }

    /// Perform cache eviction on a separate task.
    fn spawn_eviction_task(
        // &self,
        mut insert_watch_read: Receiver<usize>,
        metadata: Arc<TestCacheMetadata>,
        cache: Arc<C>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while insert_watch_read.changed().await.is_ok() {
                println!("evict1");
                let watermark_value = *insert_watch_read.borrow();
                let mut eviction_index = (watermark_value + 1) % metadata.capacity;

                if cache.total_size() < metadata.eviction_trigger_size_in_bytes {
                    continue;
                }

                println!("evict2");
                // Evict entries until the cache size is below the target size
                while cache.total_size() > metadata.target_size_in_bytes {
                    println!("evicting");
                    if let Some(value) = cache.evict(&eviction_index) {
                        if value.key > watermark_value {
                            cache.insert_with_size(value.key, value.value, value.size_in_bytes);
                            println!("evictied");
                            break;
                        }
                    }
                    eviction_index = (eviction_index + 1) % metadata.capacity;
                }
            }
        })
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
        let size_in_bytes = size_of_unique(&value);
        self.cache.insert_with_size(key, value, size_in_bytes);
        self.insert_watch.send(key).unwrap();
    }

    fn total_size(&self) -> u64 {
        self.cache.total_size() as u64
    }
}

#[tokio::main]
async fn main() {
    println!("hi");
    let ca = SyncMutexCache::with_capacity(2_000_00);
    let cache = Arc::new(TestCache::with_capacity(ca, 1_100, 1_000));

    let c1 = cache.clone();
    let t1 = tokio::spawn(async move {
        for i in 0..1 {
            c1.insert(i, NotATransaction::new(i as i64));
        }
        println!("lol");
    });

    let c2 = cache.clone();
    let t2 = tokio::spawn(async move {
        // loop {
        for i in 0..1 {
            c2.get(&i);
        }
        println!("hi");
        // }
    });

    tokio::select! {
        t1r = t1 => t1r.unwrap(),
        t2r = t2 => t2r.unwrap(),
    };
    println!("hi2");
}
