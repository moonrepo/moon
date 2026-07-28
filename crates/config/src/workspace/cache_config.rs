use crate::config_struct;
use schematic::Config;

config_struct!(
    /// Configures aspects of the content-addressable storage (CAS) cache.
    #[derive(Config)]
    pub struct CacheCasConfig {
        /// Maximum total size of the local cache, for example "10gb" or
        /// "512mib". When exceeded, the least recently used cached task outputs
        /// are evicted. Unlimited when unset.
        /// @since 2.4.0
        pub max_size: Option<String>,

        // /// Byte threshold above which to use memory-mapped I/O for hashing.
        // /// Files below this size are read into a stack buffer.
        // #[setting(default = 4_194_304)]
        // pub mmap_threshold: u64,
        /// Verify hash on every read. When enabled, reads are slower
        /// but detect on-disk corruption.
        /// @since 2.3.0
        pub verify_integrity: bool,
    }
);

config_struct!(
    /// Configures aspects of the caching engine and layer.
    #[derive(Config)]
    pub struct CacheConfig {
        /// Configures aspects of the content-addressable storage (CAS) cache.
        /// @since 2.3.0
        #[setting(nested)]
        pub cas: CacheCasConfig,
    }
);
