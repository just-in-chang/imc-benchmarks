use get_size::GetSize;

pub mod async_cache;
pub mod sync_cache;

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
