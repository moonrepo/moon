use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionCacheUpdateCapabilities, CacheCapabilities as BazelCacheCapabilities,
};

pub use bazel_remote_apis::build::bazel::remote::execution::v2::{
    compressor::Value as Compressor, digest_function::Value as DigestFunction,
    symlink_absolute_path_strategy::Value as AbsoluteSymlinkStrategy,
};

// Only define fields that we care about!
pub struct CacheCapabilities {
    pub digest_functions: Vec<DigestFunction>,
    pub max_batch_total_size_bytes: i64,
    pub max_cas_blob_size_bytes: i64,
    pub store_manifests: bool,
    pub supported_batch_update_compressors: Vec<Compressor>,
    pub supported_compressors: Vec<Compressor>,
    pub symlink_absolute_path_strategy: AbsoluteSymlinkStrategy,
}

impl Default for CacheCapabilities {
    fn default() -> Self {
        Self {
            digest_functions: vec![DigestFunction::Sha256],
            max_batch_total_size_bytes: 4194304, // 4mb
            max_cas_blob_size_bytes: 0,
            store_manifests: true,
            supported_batch_update_compressors: vec![Compressor::Identity],
            supported_compressors: vec![Compressor::Identity],
            symlink_absolute_path_strategy: AbsoluteSymlinkStrategy::Disallowed,
        }
    }
}

impl CacheCapabilities {
    pub fn into_bazel_capabilities(self) -> BazelCacheCapabilities {
        BazelCacheCapabilities {
            action_cache_update_capabilities: Some(ActionCacheUpdateCapabilities {
                update_enabled: self.store_manifests,
            }),
            digest_functions: self
                .digest_functions
                .into_iter()
                .map(|v| v as i32)
                .collect(),
            max_batch_total_size_bytes: self.max_batch_total_size_bytes,
            max_cas_blob_size_bytes: self.max_cas_blob_size_bytes,
            supported_batch_update_compressors: self
                .supported_batch_update_compressors
                .into_iter()
                .map(|v| v as i32)
                .collect(),
            supported_compressors: self
                .supported_compressors
                .into_iter()
                .map(|v| v as i32)
                .collect(),
            symlink_absolute_path_strategy: self.symlink_absolute_path_strategy as i32,
            ..Default::default()
        }
    }
}
