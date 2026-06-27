use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest as BazelDigest, OutputFile, OutputSymlink,
};
use moon_blob::{BlobContent, Bytes};
use moon_cache_storage::{Manifest, ManifestFile};
use moon_hash::{ContentHash, Digest};
use rustc_hash::FxHashMap;
use starbase_utils::json::serde_json;
use std::path::Path;

fn hex(seed: char) -> String {
    std::iter::repeat_n(seed, 64).collect()
}

fn digest(seed: char, size: i64) -> Digest {
    Digest {
        hash: ContentHash::from_hex(hex(seed)).unwrap(),
        size,
    }
}

fn file(bytes: Option<Bytes>, digest: Option<Digest>) -> ManifestFile {
    ManifestFile {
        bytes,
        digest,
        path: "out/a.txt".into(),
        ..Default::default()
    }
}

mod from_bazel {
    use super::*;

    #[test]
    fn empty_contents_with_digest_keeps_no_inline_bytes() {
        // moon's convention: outputs carry a digest with empty `contents`
        // (the bytes live on disk). The manifest must NOT inline empty bytes,
        // otherwise it'd store an empty blob under the real digest.
        let output = OutputFile {
            path: "out/a.txt".into(),
            digest: Some(BazelDigest {
                hash: hex('a'),
                size_bytes: 5,
            }),
            is_executable: false,
            contents: vec![],
            node_properties: None,
        };

        let manifest_file = ManifestFile::from_bazel_file(output).unwrap();

        assert!(manifest_file.bytes.is_none());
        assert!(manifest_file.digest.is_some());
    }

    #[test]
    fn non_empty_contents_are_kept_inline() {
        let output = OutputFile {
            path: "out/a.txt".into(),
            digest: Some(BazelDigest {
                hash: hex('a'),
                size_bytes: 5,
            }),
            is_executable: false,
            contents: b"hello".to_vec(),
            node_properties: None,
        };

        let manifest_file = ManifestFile::from_bazel_file(output).unwrap();

        assert_eq!(manifest_file.bytes, Some(Bytes::from_static(b"hello")));
    }

    #[test]
    fn action_result_round_trip() {
        let result = ActionResult {
            exit_code: 7,
            output_files: vec![OutputFile {
                path: "out/a.txt".into(),
                digest: Some(BazelDigest {
                    hash: hex('a'),
                    size_bytes: 5,
                }),
                is_executable: true,
                contents: vec![],
                node_properties: None,
            }],
            output_symlinks: vec![OutputSymlink {
                path: "out/link".into(),
                target: "out/a.txt".into(),
                node_properties: None,
            }],
            ..Default::default()
        };

        let manifest = Manifest::from_bazel_action_result(result).unwrap();
        assert_eq!(manifest.exit_code, 7);
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].path.as_str(), "out/a.txt");
        assert!(manifest.files[0].is_executable);
        assert_eq!(manifest.symlinks.len(), 1);

        let back = manifest.into_bazel_action_result();
        assert_eq!(back.exit_code, 7);
        assert_eq!(back.output_files.len(), 1);
        assert_eq!(back.output_files[0].path, "out/a.txt");
        assert!(back.output_files[0].is_executable);
        assert_eq!(back.output_symlinks.len(), 1);
        assert_eq!(back.output_symlinks[0].target, "out/a.txt");
    }
}

mod collect_blob_sources {
    use super::*;

    #[test]
    fn uses_file_path_when_no_inline_bytes() {
        let manifest = Manifest {
            files: vec![file(None, Some(digest('a', 5)))],
            ..Default::default()
        };

        let sources = manifest.collect_blob_inputs(Path::new("/workspace"));

        assert_eq!(sources.len(), 1);
        match &sources[0].content {
            BlobContent::File(path) => assert!(path.ends_with("out/a.txt")),
            _ => panic!("expected a file-backed source"),
        }
    }

    #[test]
    fn uses_inline_when_bytes_present() {
        let manifest = Manifest {
            files: vec![file(Some(Bytes::from_static(b"hi")), Some(digest('a', 2)))],
            ..Default::default()
        };

        let sources = manifest.collect_blob_inputs(Path::new("/workspace"));

        assert_eq!(sources.len(), 1);
        match &sources[0].content {
            BlobContent::Inline(bytes) => assert_eq!(bytes, &Bytes::from_static(b"hi")),
            _ => panic!("expected an inline source"),
        }
    }

    #[test]
    fn includes_stderr_and_stdout_when_present() {
        let manifest = Manifest {
            stderr_bytes: Some(Bytes::from_static(b"err")),
            stderr_digest: Some(digest('b', 3)),
            stdout_bytes: Some(Bytes::from_static(b"out")),
            stdout_digest: Some(digest('c', 3)),
            ..Default::default()
        };

        let sources = manifest.collect_blob_inputs(Path::new("/workspace"));

        assert_eq!(sources.len(), 2);
    }
}

mod hydration {
    use super::*;

    #[test]
    fn empty_manifest_is_hydrated() {
        assert!(Manifest::default().is_hydrated());
    }

    #[test]
    fn digest_without_bytes_is_not_hydrated() {
        let manifest = Manifest {
            files: vec![file(None, Some(digest('a', 5)))],
            ..Default::default()
        };

        assert!(!manifest.is_hydrated());
    }

    #[test]
    fn collect_unhydrated_skips_already_present_bytes() {
        let manifest = Manifest {
            stderr_bytes: None,
            stderr_digest: Some(digest('b', 1)),
            // stdout already hydrated → excluded.
            stdout_bytes: Some(Bytes::from_static(b"y")),
            stdout_digest: Some(digest('c', 1)),
            files: vec![file(None, Some(digest('a', 5)))],
            ..Default::default()
        };

        let unhydrated = manifest.collect_unhydrated_blob_digests();

        assert_eq!(unhydrated.len(), 2);
        assert!(unhydrated.contains(&digest('b', 1)));
        assert!(unhydrated.contains(&digest('a', 5)));
    }

    #[test]
    fn hydrate_fills_bytes_from_blob_map() {
        let mut manifest = Manifest {
            stderr_bytes: None,
            stderr_digest: Some(digest('b', 3)),
            files: vec![file(None, Some(digest('a', 4)))],
            ..Default::default()
        };

        let mut blobs: FxHashMap<Digest, BlobContent> = FxHashMap::default();
        blobs.insert(
            digest('b', 3),
            BlobContent::Inline(Bytes::from_static(b"err")),
        );
        blobs.insert(
            digest('a', 4),
            BlobContent::Inline(Bytes::from_static(b"file")),
        );

        manifest.hydrate(&blobs);

        assert_eq!(manifest.stderr_bytes, Some(Bytes::from_static(b"err")));
        assert_eq!(manifest.files[0].bytes, Some(Bytes::from_static(b"file")));
        assert!(manifest.is_hydrated());
    }
}

mod serialization {
    use super::*;

    #[test]
    fn byte_fields_are_never_serialized() {
        let manifest = Manifest {
            exit_code: 2,
            stderr_bytes: Some(Bytes::from_static(b"stderr-secret")),
            stderr_digest: Some(digest('b', 13)),
            files: vec![file(
                Some(Bytes::from_static(b"file-secret")),
                Some(digest('a', 11)),
            )],
            ..Default::default()
        };

        let json = serde_json::to_string(&manifest).unwrap();

        // The transient byte fields must be skipped entirely.
        assert!(!json.contains("stderr_bytes"));
        assert!(!json.contains("stdout_bytes"));
        assert!(!json.contains("\"bytes\""));
    }

    #[test]
    fn round_trip_drops_bytes_but_keeps_digests() {
        let manifest = Manifest {
            exit_code: 2,
            stderr_bytes: Some(Bytes::from_static(b"stderr-secret")),
            stderr_digest: Some(digest('b', 13)),
            files: vec![file(
                Some(Bytes::from_static(b"file-secret")),
                Some(digest('a', 11)),
            )],
            ..Default::default()
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let restored: Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.exit_code, 2);
        assert!(restored.stderr_bytes.is_none());
        assert_eq!(restored.stderr_digest, Some(digest('b', 13)));
        assert!(restored.files[0].bytes.is_none());
        assert_eq!(restored.files[0].digest, Some(digest('a', 11)));
    }
}
