use moon_blob::Bytes;
use moon_hash::Digest;
use moon_manifest::{Manifest, ManifestFile, ManifestSymlink, ManifestUnpacker};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use std::path::PathBuf;
use std::time::{Duration, UNIX_EPOCH};

fn digest(bytes: &[u8]) -> Option<Digest> {
    Some(Digest::from_bytes(bytes).unwrap())
}

fn unpack(sandbox: &Sandbox, manifest: &Manifest) -> miette::Result<()> {
    ManifestUnpacker::new(manifest, sandbox.path().to_path_buf()).unpack()
}

fn manifest_with(file: ManifestFile) -> Manifest {
    Manifest {
        files: vec![file],
        ..Default::default()
    }
}

mod files {
    use super::*;

    #[test]
    fn writes_a_file_from_inline_bytes() {
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"contents")),
            digest: digest(b"contents"),
            path: "out/a.txt".into(),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("out/a.txt")).unwrap(),
            "contents"
        );
    }

    #[test]
    fn writes_a_file_into_nested_directories() {
        // Nothing creates the output tree beforehand, so the unpacker has to.
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"deep")),
            digest: digest(b"deep"),
            path: "out/nested/deeper/a.txt".into(),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("out/nested/deeper/a.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn writes_an_empty_file_when_there_are_no_bytes() {
        // A size-0 output carries no bytes but still has a digest, so it must
        // land on disk as an empty file rather than be skipped.
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: None,
            digest: digest(b""),
            path: "out/empty.txt".into(),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        let path = sandbox.path().join("out/empty.txt");

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn reflinks_a_file_from_its_source_path() {
        // The local CAS hands back a file reference rather than bytes, and the
        // blob is cloned into place instead of being read through memory.
        let sandbox = create_empty_sandbox();
        sandbox.create_file("cas/blob", "cached");

        let manifest = manifest_with(ManifestFile {
            digest: digest(b"cached"),
            path: "out/a.txt".into(),
            source_path: Some(sandbox.path().join("cas/blob")),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("out/a.txt")).unwrap(),
            "cached"
        );
    }

    #[test]
    fn skips_a_file_without_a_digest() {
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"contents")),
            digest: None,
            path: "out/a.txt".into(),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert!(!sandbox.path().join("out/a.txt").exists());
    }

    #[test]
    fn applies_the_recorded_modified_time() {
        let sandbox = create_empty_sandbox();
        let modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"contents")),
            digest: digest(b"contents"),
            modified_at: Some(modified),
            path: "out/a.txt".into(),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        let actual = std::fs::metadata(sandbox.path().join("out/a.txt"))
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(actual, modified);
    }
}

#[cfg(unix)]
mod unix_modes {
    use super::*;
    use std::fs::Permissions;
    use std::os::unix::fs::PermissionsExt;

    fn mode_of(path: PathBuf) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn applies_the_recorded_unix_mode() {
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"#!/bin/sh")),
            digest: digest(b"#!/bin/sh"),
            path: "out/run.sh".into(),
            unix_mode: Some(0o755),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(mode_of(sandbox.path().join("out/run.sh")), 0o755);
    }

    #[test]
    fn restores_write_access_when_reflinking_a_read_only_source() {
        // A reflink clones the source's permissions. A CAS blob stored before
        // objects were normalized can be 0444, and opening the clone for
        // writing then fails with EACCES — so writability has to be restored
        // before the handle is taken to apply the mtime and mode.
        let sandbox = create_empty_sandbox();
        sandbox.create_file("cas/blob", "cached");

        let source = sandbox.path().join("cas/blob");
        std::fs::set_permissions(&source, Permissions::from_mode(0o444)).unwrap();

        let manifest = manifest_with(ManifestFile {
            digest: digest(b"cached"),
            modified_at: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            path: "out/a.txt".into(),
            source_path: Some(source),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(
            std::fs::read_to_string(sandbox.path().join("out/a.txt")).unwrap(),
            "cached"
        );
    }

    #[test]
    fn a_read_only_source_still_honors_the_recorded_mode() {
        // Restoring write access is a means to apply the recorded metadata, so
        // the mode the manifest asked for must be what actually lands — the
        // file must not be left writable just because the clone was patched.
        let sandbox = create_empty_sandbox();
        sandbox.create_file("cas/blob", "cached");

        let source = sandbox.path().join("cas/blob");
        std::fs::set_permissions(&source, Permissions::from_mode(0o444)).unwrap();

        let manifest = manifest_with(ManifestFile {
            digest: digest(b"cached"),
            path: "out/a.txt".into(),
            source_path: Some(source),
            unix_mode: Some(0o444),
            ..Default::default()
        });

        unpack(&sandbox, &manifest).unwrap();

        assert_eq!(mode_of(sandbox.path().join("out/a.txt")), 0o444);
    }
}

mod symlinks {
    use super::*;

    #[test]
    fn links_to_the_target() {
        let sandbox = create_empty_sandbox();

        let manifest = Manifest {
            files: vec![ManifestFile {
                bytes: Some(Bytes::from_static(b"contents")),
                digest: digest(b"contents"),
                path: "out/a.txt".into(),
                ..Default::default()
            }],
            symlinks: vec![ManifestSymlink {
                path: "out/link.txt".into(),
                target: "out/a.txt".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        unpack(&sandbox, &manifest).unwrap();

        let link = sandbox.path().join("out/link.txt");

        assert!(link.is_symlink());
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            sandbox.path().join("out/a.txt")
        );
        assert_eq!(std::fs::read_to_string(link).unwrap(), "contents");
    }

    #[test]
    fn links_into_nested_directories() {
        let sandbox = create_empty_sandbox();

        let manifest = Manifest {
            symlinks: vec![ManifestSymlink {
                path: "out/nested/link.txt".into(),
                target: "out/a.txt".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        unpack(&sandbox, &manifest).unwrap();

        assert!(sandbox.path().join("out/nested/link.txt").is_symlink());
    }
}

mod path_containment {
    use super::*;

    /// Assert the write was refused by the containment check specifically, so
    /// these can't pass on an unrelated failure (a missing parent dir, say).
    fn assert_outside_workspace(result: miette::Result<()>) {
        let error = result.unwrap_err().to_string();

        assert!(
            error.contains("exists outside of the workspace"),
            "expected a containment rejection, got: {error}"
        );
    }

    fn manifest_at(path: &str) -> Manifest {
        manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"pwned")),
            digest: digest(b"pwned"),
            path: path.into(),
            ..Default::default()
        })
    }

    #[test]
    fn rejects_a_rooted_file_path() {
        // The manifest comes from a shared cache, so a path that escapes the
        // workspace must be refused rather than written. Windows treats this as
        // rooted-but-not-absolute, which is why the guard checks both.
        let sandbox = create_empty_sandbox();

        assert_outside_workspace(unpack(&sandbox, &manifest_at("/tmp/moon-escape.txt")));
        assert!(!sandbox.path().join("tmp/moon-escape.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn rejects_a_drive_absolute_file_path() {
        // The shape Windows actually considers absolute, and the one that would
        // genuinely escape the workspace if it slipped through.
        let sandbox = create_empty_sandbox();

        assert_outside_workspace(unpack(
            &sandbox,
            &manifest_at(r"C:\Windows\moon-escape.txt"),
        ));
        assert!(!PathBuf::from(r"C:\Windows\moon-escape.txt").exists());
    }

    #[cfg(windows)]
    #[test]
    fn rejects_a_unc_file_path() {
        let sandbox = create_empty_sandbox();

        assert_outside_workspace(unpack(
            &sandbox,
            &manifest_at(r"\\server\share\moon-escape.txt"),
        ));
    }

    #[test]
    fn rejects_a_file_path_that_traverses_out_of_the_workspace() {
        let sandbox = create_empty_sandbox();

        let manifest = manifest_with(ManifestFile {
            bytes: Some(Bytes::from_static(b"pwned")),
            digest: digest(b"pwned"),
            path: "../moon-escape.txt".into(),
            ..Default::default()
        });

        assert_outside_workspace(unpack(&sandbox, &manifest));
        assert!(
            !sandbox
                .path()
                .parent()
                .unwrap()
                .join("moon-escape.txt")
                .exists()
        );
    }

    #[test]
    fn rejects_a_symlink_path_that_traverses_out_of_the_workspace() {
        let sandbox = create_empty_sandbox();

        let manifest = Manifest {
            symlinks: vec![ManifestSymlink {
                path: "../moon-escape.txt".into(),
                target: "out/a.txt".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_outside_workspace(unpack(&sandbox, &manifest));
        assert!(
            !sandbox
                .path()
                .parent()
                .unwrap()
                .join("moon-escape.txt")
                .exists()
        );
    }

    #[test]
    fn rejects_a_symlink_target_that_traverses_out_of_the_workspace() {
        // The link path is inside the workspace but the target isn't, which
        // would otherwise plant a link pointing at an arbitrary file.
        let sandbox = create_empty_sandbox();

        let manifest = Manifest {
            symlinks: vec![ManifestSymlink {
                path: "out/link.txt".into(),
                target: "../../etc/passwd".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let error = unpack(&sandbox, &manifest).unwrap_err().to_string();

        assert!(
            error.contains("is a symlink to"),
            "expected a symlink-target rejection, got: {error}"
        );
        assert!(!sandbox.path().join("out/link.txt").exists());
    }
}
