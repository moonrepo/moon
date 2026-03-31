/// Configuration for a [`CasStore`](crate::CasStore).
#[derive(Debug, Clone)]
pub struct CasStoreConfig {
    /// Verify BLAKE3 hash on every read. Default: `false`.
    /// When enabled, reads are slower but detect on-disk corruption.
    pub verify_on_read: bool,

    /// Byte threshold above which to use memory-mapped I/O for hashing.
    /// Files below this size are read into a stack buffer. Default: 4 MiB.
    pub mmap_threshold: u64,
}

impl Default for CasStoreConfig {
    fn default() -> Self {
        Self {
            verify_on_read: false,
            mmap_threshold: 4 * 1024 * 1024,
        }
    }
}
