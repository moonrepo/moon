use moon_hash::Digest;
use moon_manifest::ManifestPacker;
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

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_source(&digest, path.clone()).unwrap();

        let source = packer
            .pack()
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

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_source(&digest, path).unwrap();

        assert!(packer.pack().digest_source.is_none());
    }

    #[test]
    fn carries_no_inline_bytes() {
        // The fingerprint is read from disk during upload, not held in memory —
        // inlining it would duplicate the file into every manifest.
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "[\"abc\"]");

        let path = sandbox.path().join(".moon/cache/hashes/abc.json");
        let digest = Digest::from_file(&path).unwrap();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_source(&digest, path).unwrap();

        assert!(packer.pack().digest_source.unwrap().bytes.is_none());
    }

    #[test]
    fn is_not_treated_as_an_output() {
        // It's the action the manifest came from, not something to restore on
        // a cache hit, so it must not leak into the output file list.
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".moon/cache/hashes/abc.json", "[\"abc\"]");

        let path = sandbox.path().join(".moon/cache/hashes/abc.json");
        let digest = Digest::from_file(&path).unwrap();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_source(&digest, path).unwrap();

        let manifest = packer.pack();

        assert!(manifest.files.is_empty());
        assert!(manifest.symlinks.is_empty());
    }
}

mod inherit_output {
    use super::*;

    #[test]
    fn records_a_file_with_its_digest_and_paths() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("out/a.txt", "contents");

        let abs_path = sandbox.path().join("out/a.txt");

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_output(abs_path.clone()).unwrap();

        let manifest = packer.pack();

        assert_eq!(manifest.files.len(), 1);

        let file = &manifest.files[0];

        assert_eq!(file.path.as_str(), "out/a.txt");
        assert_eq!(file.source_path, Some(abs_path));
        assert_eq!(file.digest, Some(Digest::from_bytes(b"contents").unwrap()));
        // Bytes stay on disk and are read at upload time, never held here.
        assert!(file.bytes.is_none());
        assert!(file.modified_at.is_some());
    }

    #[test]
    fn records_a_directory_as_its_individual_files() {
        // Outputs can be declared as a directory, but the CAS addresses files,
        // so the tree has to be flattened — recursively.
        let sandbox = create_empty_sandbox();
        sandbox.create_file("out/a.txt", "a");
        sandbox.create_file("out/nested/b.txt", "b");

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_output(sandbox.path().join("out")).unwrap();

        let manifest = packer.pack();
        let mut paths = manifest
            .files
            .iter()
            .map(|file| file.path.to_string())
            .collect::<Vec<_>>();
        paths.sort();

        assert_eq!(paths, vec!["out/a.txt", "out/nested/b.txt"]);
    }

    #[test]
    fn records_a_symlink_rather_than_its_contents() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("out/a.txt", "contents");

        #[cfg(unix)]
        std::os::unix::fs::symlink(
            sandbox.path().join("out/a.txt"),
            sandbox.path().join("out/link.txt"),
        )
        .unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            sandbox.path().join("out/a.txt"),
            sandbox.path().join("out/link.txt"),
        )
        .unwrap();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer
            .inherit_output(sandbox.path().join("out/link.txt"))
            .unwrap();

        let manifest = packer.pack();

        assert!(manifest.files.is_empty());
        assert_eq!(manifest.symlinks.len(), 1);
        assert_eq!(manifest.symlinks[0].path.as_str(), "out/link.txt");
        assert_eq!(manifest.symlinks[0].target.as_str(), "out/a.txt");
    }

    #[test]
    fn ignores_a_path_that_does_not_exist() {
        // Optional outputs may simply not have been produced.
        let sandbox = create_empty_sandbox();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer
            .inherit_output(sandbox.path().join("out/missing.txt"))
            .unwrap();

        let manifest = packer.pack();

        assert!(manifest.files.is_empty());
        assert!(manifest.symlinks.is_empty());
    }

    #[test]
    fn errors_when_the_output_is_outside_the_workspace() {
        let sandbox = create_empty_sandbox();
        let outside = create_empty_sandbox();
        outside.create_file("a.txt", "contents");

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        let error = packer
            .inherit_output(outside.path().join("a.txt"))
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("exists outside of the workspace"),
            "expected a containment rejection, got: {error}"
        );
    }

    #[test]
    fn errors_when_a_symlink_targets_outside_the_workspace() {
        // Packing the link would otherwise record a target that can't be
        // resolved on the machine that restores it.
        let sandbox = create_empty_sandbox();
        let outside = create_empty_sandbox();
        outside.create_file("a.txt", "contents");

        #[cfg(unix)]
        std::os::unix::fs::symlink(
            outside.path().join("a.txt"),
            sandbox.path().join("link.txt"),
        )
        .unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(
            outside.path().join("a.txt"),
            sandbox.path().join("link.txt"),
        )
        .unwrap();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        let error = packer
            .inherit_output(sandbox.path().join("link.txt"))
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("is a symlink to"),
            "expected a symlink-target rejection, got: {error}"
        );
    }
}

mod inherit_operation {
    use super::*;
    use moon_action::Operation;

    #[test]
    fn records_the_exit_code_and_console_output() {
        // The daemon and the cache both replay this output on a hit, and it
        // exists only in memory here, so it's digested and inlined.
        let sandbox = create_empty_sandbox();

        let mut operation = Operation::task_execution("build");
        operation.finish_from_output(None, b"built ok\n".to_vec(), b"warning issued\n".to_vec());

        operation.get_exec_output_mut().unwrap().exit_code = Some(2);

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_operation(&operation).unwrap();

        let manifest = packer.pack();

        assert_eq!(manifest.exit_code, 2);
        assert_eq!(manifest.stdout_bytes.as_deref(), Some(&b"built ok\n"[..]));
        assert_eq!(
            manifest.stderr_bytes.as_deref(),
            Some(&b"warning issued\n"[..])
        );
        assert_eq!(
            manifest.stdout_digest,
            Some(Digest::from_bytes(b"built ok\n").unwrap())
        );
        assert_eq!(
            manifest.stderr_digest,
            Some(Digest::from_bytes(b"warning issued\n").unwrap())
        );
    }

    #[test]
    fn leaves_stdio_unset_when_nothing_was_captured() {
        let sandbox = create_empty_sandbox();

        let mut operation = Operation::task_execution("build");
        operation.finish_from_output(None, vec![], vec![]);

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_operation(&operation).unwrap();

        let manifest = packer.pack();

        assert_eq!(manifest.exit_code, 0);
        assert!(manifest.stdout_digest.is_none());
        assert!(manifest.stderr_digest.is_none());
    }

    #[test]
    fn is_a_noop_for_a_non_execution_operation() {
        let sandbox = create_empty_sandbox();

        let operation = Operation::output_hydration();

        let mut packer = ManifestPacker::new(sandbox.path().to_path_buf());
        packer.inherit_operation(&operation).unwrap();

        let manifest = packer.pack();

        assert_eq!(manifest.exit_code, 0);
        assert!(manifest.stdout_bytes.is_none());
        assert!(manifest.stderr_bytes.is_none());
    }
}
