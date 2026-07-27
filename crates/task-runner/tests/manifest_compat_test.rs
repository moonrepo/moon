use moon_hash::Digest;
use moon_task_runner::manifest_compat::ManifestBuilder;
use starbase_sandbox::create_empty_sandbox;

mod inherit_source {
    use super::*;

    #[test]
    fn records_the_file_when_it_exists() {
        // The action digest names moon's fingerprint file, which has to reach
        // the CAS or an RE-compliant backend rejects the action result. The
        // path is stored relative for the manifest and absolute for the upload,
        // since the fingerprint lives outside the task's output tree.
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "[\"abc\"]");

        let path = sandbox.path().join(".moon/cache/hashes/abc.json");
        let digest = Digest::from_file(&path).unwrap();

        let mut builder = ManifestBuilder::new(sandbox.path().to_path_buf());
        builder.inherit_source(&digest, path.clone()).unwrap();

        let source = builder
            .build()
            .digest_source
            .expect("an existing fingerprint file must be recorded");

        assert_eq!(source.digest, Some(digest));
        assert_eq!(source.path.as_str(), ".moon/cache/hashes/abc.json");
        assert_eq!(source.source_path, Some(path));
    }

    #[test]
    fn is_a_noop_when_the_file_is_missing() {
        // Archiving without a computed fingerprint leaves nothing to upload.
        // That must stay a no-op rather than recording a blob whose bytes
        // can't be read at upload time.
        let sandbox = create_empty_sandbox();

        let path = sandbox.path().join(".moon/cache/hashes/missing.json");
        let digest = Digest::from_bytes(b"missing").unwrap();

        let mut builder = ManifestBuilder::new(sandbox.path().to_path_buf());
        builder.inherit_source(&digest, path).unwrap();

        assert!(builder.build().digest_source.is_none());
    }

    #[test]
    fn carries_no_inline_bytes() {
        // The fingerprint is read from disk during upload, not held in memory —
        // inlining it would duplicate the file into every manifest.
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "[\"abc\"]");

        let path = sandbox.path().join(".moon/cache/hashes/abc.json");
        let digest = Digest::from_file(&path).unwrap();

        let mut builder = ManifestBuilder::new(sandbox.path().to_path_buf());
        builder.inherit_source(&digest, path).unwrap();

        assert!(builder.build().digest_source.unwrap().bytes.is_none());
    }

    #[test]
    fn is_not_treated_as_an_output() {
        // It's the action the manifest came from, not something to restore on
        // a cache hit, so it must not leak into the output file list.
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "[\"abc\"]");

        let path = sandbox.path().join(".moon/cache/hashes/abc.json");
        let digest = Digest::from_file(&path).unwrap();

        let mut builder = ManifestBuilder::new(sandbox.path().to_path_buf());
        builder.inherit_source(&digest, path).unwrap();

        let manifest = builder.build();

        assert!(manifest.files.is_empty());
        assert!(manifest.symlinks.is_empty());
    }
}
