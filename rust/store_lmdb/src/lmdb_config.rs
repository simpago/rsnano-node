#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SyncStrategy {
    /** Always flush to disk on commit. This is default. */
    Always,
    /** Do not flush meta data eagerly. This may cause loss of transactions, but maintains integrity. */
    NosyncSafe,

    /**
     * Let the OS decide when to flush to disk. On filesystems with write ordering, this has the same
     * guarantees as nosync_safe, otherwise corruption may occur on system crash.
     */
    NosyncUnsafe,
    /**
     * Use a writeable memory map. Let the OS decide when to flush to disk, and make the request asynchronous.
     * This may give better performance on systems where the database fits entirely in memory, otherwise is
     * may be slower.
     * @warning Do not use this option if external processes uses the database concurrently.
     */
    NosyncUnsafeLargeMemory,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LmdbConfig {
    pub sync: SyncStrategy,
    pub max_databases: u32,
    pub map_size: usize,
}

impl Default for LmdbConfig {
    fn default() -> Self {
        Self {
            sync: SyncStrategy::Always,
            max_databases: 128,
            map_size: 256 * 1024 * 1024 * 1024,
        }
    }
}

impl LmdbConfig {
    pub fn new() -> Self {
        Default::default()
    }
}
