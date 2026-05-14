use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures aspects of the content-addressable storage (CAS) cache.
    #[derive(Config)]
    pub struct CacheCasConfig {
        /// Byte threshold above which to use memory-mapped I/O for hashing.
        /// Files below this size are read into a stack buffer.
        #[setting(default = 4_194_304)]
        pub mmap_threshold: u64,

        /// Verify hash on every read/write. When enabled, operations
        /// are slower but detect on-disk corruption.
        pub verify_integrity: bool,
    }
);

config_struct!(
    /// Configures aspects of the caching engine and layer.
    #[derive(Config)]
    pub struct CacheConfig {
        /// Configures aspects of the content-addressable storage (CAS) cache.
        #[setting(nested)]
        pub cas: CacheCasConfig,
    }
);
