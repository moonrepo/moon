use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest as BazelDigest, ExecutedActionMetadata, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::Timestamp;
use moon_blob::{BlobContent, Bytes};
use moon_cache_storage::{Manifest, ManifestFile};
use moon_hash::{ContentHash, Digest};
use rustc_hash::FxHashMap;
use starbase_sandbox::create_empty_sandbox;
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
    fn empty_stdio_raw_with_digest_keeps_no_inline_bytes() {
        // Inlining the raw output is optional for a server, so a digest can arrive
        // without bytes. Treating the empty raw as the payload would mark the manifest
        // hydrated and replay no output on a cache hit.
        let result = ActionResult {
            stdout_digest: Some(BazelDigest {
                hash: hex('c'),
                size_bytes: 13,
            }),
            stdout_raw: vec![],
            stderr_digest: Some(BazelDigest {
                hash: hex('d'),
                size_bytes: 4,
            }),
            stderr_raw: vec![],
            ..Default::default()
        };

        let manifest = Manifest::from_bazel_action_result(result).unwrap();

        assert!(manifest.stdout_bytes.is_none());
        assert!(manifest.stderr_bytes.is_none());
        assert!(!manifest.is_hydrated());
        assert_eq!(manifest.collect_unhydrated_blob_digests().len(), 2);
    }

    #[test]
    fn upload_never_populates_the_raw_stdio_fields() {
        // The RE API reserves `stdout_raw`/`stderr_raw` for server responses; a client
        // that populates them on `UpdateActionResult` is rejected by backends that
        // validate the contract. The digests carry the output instead.
        let manifest = Manifest {
            stdout_bytes: Some(Bytes::from_static(b"out")),
            stdout_digest: Some(digest('c', 3)),
            stderr_bytes: Some(Bytes::from_static(b"err")),
            stderr_digest: Some(digest('d', 3)),
            ..Default::default()
        };

        let result = manifest.into_bazel_action_result();

        assert!(result.stdout_raw.is_empty());
        assert!(result.stderr_raw.is_empty());
        assert!(result.stdout_digest.is_some());
        assert!(result.stderr_digest.is_some());
    }

    #[test]
    fn round_trips_both_upload_timestamps() {
        // Each timestamp must survive independently — an implementation that
        // consumes the metadata while reading the first one silently drops the
        // second (`upload_started_at` came back `None`).
        use std::time::{Duration, UNIX_EPOCH};

        let result = ActionResult {
            execution_metadata: Some(ExecutedActionMetadata {
                output_upload_start_timestamp: Some(Timestamp {
                    seconds: 100,
                    nanos: 1,
                }),
                output_upload_completed_timestamp: Some(Timestamp {
                    seconds: 200,
                    nanos: 2,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let manifest = Manifest::from_bazel_action_result(result).unwrap();

        assert_eq!(
            manifest.upload_started_at,
            Some(UNIX_EPOCH + Duration::new(100, 1))
        );
        assert_eq!(
            manifest.upload_completed_at,
            Some(UNIX_EPOCH + Duration::new(200, 2))
        );

        let metadata = manifest
            .into_bazel_action_result()
            .execution_metadata
            .unwrap();

        assert_eq!(
            metadata.output_upload_start_timestamp.map(|ts| ts.seconds),
            Some(100)
        );
        assert_eq!(
            metadata
                .output_upload_completed_timestamp
                .map(|ts| ts.seconds),
            Some(200)
        );
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

    #[test]
    fn empty_file_uses_inline_empty_blob_not_a_path() {
        // A size-0 output must become a shared inline empty blob, never a
        // File(path): the path isn't materialized when warming runs, and empty
        // files dedupe to the one empty digest in CAS.
        let manifest = Manifest {
            files: vec![
                file(None, Some(digest('a', 0))),
                file(Some(Bytes::from_static(b"hi")), Some(digest('c', 2))),
            ],
            ..Default::default()
        };

        let sources = manifest.collect_blob_inputs(Path::new("/workspace"));

        assert_eq!(sources.len(), 2);

        let empty = sources
            .iter()
            .find(|source| source.digest == digest('a', 0))
            .expect("empty file should still produce a blob input");

        match &empty.content {
            BlobContent::Inline(bytes) => assert!(bytes.is_empty()),
            _ => panic!("size-0 output must be an inline empty blob, not a file path"),
        }
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

        manifest.hydrate(&blobs).unwrap();

        assert_eq!(manifest.stderr_bytes, Some(Bytes::from_static(b"err")));
        assert_eq!(manifest.files[0].bytes, Some(Bytes::from_static(b"file")));
        assert!(manifest.is_hydrated());
    }

    #[test]
    fn hydrate_reads_stdio_from_file_refs_but_defers_output_files() {
        // The local backend hands back file references, not inline bytes. stdio
        // must be read into memory (it can't be reflinked), while output files
        // keep the reference as a reflink source for the hydrater.
        let sandbox = create_empty_sandbox();
        sandbox.create_file("err", "boom");
        sandbox.create_file("out.txt", "file contents");

        let mut manifest = Manifest {
            stderr_bytes: None,
            stderr_digest: Some(digest('b', 4)),
            files: vec![file(None, Some(digest('a', 13)))],
            ..Default::default()
        };

        let mut blobs: FxHashMap<Digest, BlobContent> = FxHashMap::default();
        blobs.insert(
            digest('b', 4),
            BlobContent::File(sandbox.path().join("err")),
        );
        blobs.insert(
            digest('a', 13),
            BlobContent::File(sandbox.path().join("out.txt")),
        );

        manifest.hydrate(&blobs).unwrap();

        // stderr was read from disk into memory.
        assert_eq!(manifest.stderr_bytes, Some(Bytes::from_static(b"boom")));
        // The output file is left as a reflink source, not loaded into memory.
        assert!(manifest.files[0].bytes.is_none());
        assert_eq!(
            manifest.files[0].source_path,
            Some(sandbox.path().join("out.txt"))
        );
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
