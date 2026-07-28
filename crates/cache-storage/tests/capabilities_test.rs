use moon_cache_storage::{CacheCapabilities, Compressor, DigestFunction};

mod capabilities {
    use super::*;

    #[test]
    fn default_disables_size_batching() {
        let caps = CacheCapabilities::default();

        // 0 is the sentinel that routes the local backend to thread-chunking
        // instead of size-based partitioning.
        assert_eq!(caps.max_batch_total_size_bytes, 0);
        assert!(caps.store_manifests);
        assert_eq!(caps.digest_functions, vec![DigestFunction::Sha256]);
    }

    #[test]
    fn bazel_round_trip_preserves_fields() {
        let caps = CacheCapabilities {
            max_batch_total_size_bytes: 4_194_304,
            // max_cas_blob_size_bytes: 1024,
            store_manifests: true,
            supported_compressors: vec![Compressor::Identity],
            ..CacheCapabilities::default()
        };

        let restored = CacheCapabilities::from_bazel_capabilities(caps.into_bazel_capabilities());

        assert_eq!(restored.max_batch_total_size_bytes, 4_194_304);
        // assert_eq!(restored.max_cas_blob_size_bytes, 1024);
        assert!(restored.store_manifests);
        assert_eq!(restored.digest_functions, vec![DigestFunction::Sha256]);
        assert_eq!(restored.supported_compressors, vec![Compressor::Identity]);
    }

    #[test]
    fn store_manifests_maps_to_action_cache_update() {
        // `store_manifests` is carried by the action-cache-update capability,
        // so it must survive a round trip through the bazel representation.
        let disabled = CacheCapabilities {
            store_manifests: false,
            ..CacheCapabilities::default()
        };

        let restored =
            CacheCapabilities::from_bazel_capabilities(disabled.into_bazel_capabilities());

        assert!(!restored.store_manifests);
    }
}
