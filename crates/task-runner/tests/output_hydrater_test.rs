mod utils;

use moon_cache::{CacheMode, Manifest, ManifestFile, ManifestSource};
use moon_env_var::GlobalEnvBag;
use moon_hash::Digest;
use moon_task_runner::TaskRunState;
use moon_task_runner::output_hydrater::{HydrateFrom, HydrateOutcome};
use std::fs;
use utils::*;

fn assert_hydrated(outcome: HydrateOutcome) {
    assert!(
        matches!(
            outcome,
            HydrateOutcome::Hit | HydrateOutcome::HitFromStorage(..)
        ),
        "expected a cache hit"
    );
}

fn assert_not_hydrated(outcome: HydrateOutcome) {
    assert!(
        matches!(outcome, HydrateOutcome::Skipped | HydrateOutcome::Missed),
        "expected a cache miss"
    );
}

mod output_hydrater {
    use super::*;

    mod local_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_from_prev_outputs() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let hydrater = container.create_hydrator();
            let state = container.create_state();

            assert_hydrated(
                hydrater
                    .hydrate(HydrateFrom::PreviousOutput, "hash123", &state)
                    .await
                    .unwrap(),
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_unpack_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            let hydrater = container.create_hydrator();
            let state = container.create_state();

            assert_not_hydrated(
                hydrater
                    .hydrate(HydrateFrom::LocalArchive, "hash123", &state)
                    .await
                    .unwrap(),
            );

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_unpack_if_cache_write_only() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Write);

            let hydrater = container.create_hydrator();
            let state = container.create_state();

            assert_not_hydrated(
                hydrater
                    .hydrate(HydrateFrom::LocalArchive, "hash123", &state)
                    .await
                    .unwrap(),
            );

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn unpacks_archive_into_project() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            let hydrater = container.create_hydrator();
            let state = container.create_state();

            assert_hydrated(
                hydrater
                    .hydrate(HydrateFrom::LocalArchive, "hash123", &state)
                    .await
                    .unwrap(),
            );

            assert!(container.sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn unpacks_logs_from_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(
                !container
                    .sandbox
                    .path()
                    .join(".moon/cache/states/project/file-outputs/stdout.log")
                    .exists()
            );

            let hydrater = container.create_hydrator();
            let state = container.create_state();

            hydrater
                .hydrate(HydrateFrom::LocalArchive, "hash123", &state)
                .await
                .unwrap();

            assert!(
                container
                    .sandbox
                    .path()
                    .join(".moon/cache/states/project/file-outputs/stdout.log")
                    .exists()
            );
        }
    }

    mod local_cas {
        use super::*;

        fn setup_cas_state(state: &mut TaskRunState) {
            state.local_cas_enabled = true;
            state.digest = Digest::from_bytes(b"hash123").unwrap();
        }

        /// Archive the current outputs into storage, then load the resulting
        /// manifest back as a hydration source (the storage-backed flow).
        async fn archive_and_load(
            container: &TaskRunnerContainer,
            state: &TaskRunState,
        ) -> ManifestSource {
            container
                .create_archiver()
                .archive("hash123", state)
                .await
                .unwrap();
            container.flush_storage().await;

            load_source(container, state).await
        }

        async fn load_source(
            container: &TaskRunnerContainer,
            state: &TaskRunState,
        ) -> ManifestSource {
            container
                .app_context
                .cache_engine
                .storage
                .load_manifest(&state.digest)
                .await
                .unwrap()
                .expect("manifest was stored")
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_output_file_from_cas() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file("project/file.txt", "contents");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let source = archive_and_load(&container, &state).await;

            // Remove source so we can verify hydration restores it
            fs::remove_file(container.sandbox.path().join("project/file.txt")).unwrap();

            assert_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            assert_eq!(
                fs::read_to_string(container.sandbox.path().join("project/file.txt")).unwrap(),
                "contents"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_hydrate_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file("project/file.txt", "contents");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            // Load the source while the cache is still readable.
            let source = archive_and_load(&container, &state).await;

            fs::remove_file(container.sandbox.path().join("project/file.txt")).unwrap();

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert_not_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_archive_if_cas_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            // CAS is not enabled in state, so a local source should fall back to
            // unpacking the legacy archive.
            let mut state = container.create_state();
            state.digest = Digest::from_bytes(b"hash123").unwrap();

            container
                .seed_manifest(&state.digest, Manifest::default())
                .await;
            let source = load_source(&container, &state).await;

            assert_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            assert!(container.sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_multiple_files_sharing_the_same_blob() {
            // Multiple output files that share content also share a CAS blob.
            // Hydration reads bytes per file from the manifest, so duplicate
            // digests must "just work".
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "shared");
            container.sandbox.create_file("project/b.txt", "shared");
            container.sandbox.create_file("project/c.txt", "shared");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let source = archive_and_load(&container, &state).await;

            // All three files should reference the same digest.
            let digests: Vec<_> = source
                .manifest
                .files
                .iter()
                .filter_map(|file| file.digest.as_ref().map(|digest| digest.hash.clone()))
                .collect();
            assert_eq!(digests.len(), 3);
            assert!(digests.iter().all(|hash| hash == &digests[0]));

            // Wipe the on-disk outputs so we know hydration restored them.
            fs::remove_file(container.sandbox.path().join("project/a.txt")).unwrap();
            fs::remove_file(container.sandbox.path().join("project/b.txt")).unwrap();
            fs::remove_file(container.sandbox.path().join("project/c.txt")).unwrap();

            assert_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            for name in ["a.txt", "b.txt", "c.txt"] {
                let path = container.sandbox.path().join("project").join(name);
                assert_eq!(fs::read_to_string(&path).unwrap(), "shared");
            }
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_file_inside_declared_output_directory() {
            let container = TaskRunnerContainer::new("archive", "output-one-dir").await;
            container
                .sandbox
                .create_file("project/dir/file.txt", "contents");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let source = archive_and_load(&container, &state).await;

            fs::remove_dir_all(container.sandbox.path().join("project/dir")).unwrap();

            assert_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            assert_eq!(
                fs::read_to_string(container.sandbox.path().join("project/dir/file.txt")).unwrap(),
                "contents"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_manifest_path_outside_workspace() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let outside_name = format!(
                "{}-remote-cache-outside-marker.txt",
                container
                    .sandbox
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
            );
            let outside_path = container
                .sandbox
                .path()
                .parent()
                .unwrap()
                .join(&outside_name);
            let _ = fs::remove_file(&outside_path);

            let manifest = Manifest {
                files: vec![ManifestFile {
                    path: format!("../{outside_name}").into(),
                    digest: Some(Digest::from_bytes(b"").unwrap()),
                    ..Default::default()
                }],
                ..Default::default()
            };

            container.seed_manifest(&state.digest, manifest).await;
            let source = load_source(&container, &state).await;

            assert!(!outside_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!outside_path.exists());

            let _ = fs::remove_file(&outside_path);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_manifest_absolute_path_outside_workspace() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let outside_name = format!(
                "{}-remote-cache-absolute-marker.txt",
                container
                    .sandbox
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
            );
            let outside_path = container
                .sandbox
                .path()
                .parent()
                .unwrap()
                .join(&outside_name);
            let _ = fs::remove_file(&outside_path);

            let manifest = Manifest {
                files: vec![ManifestFile {
                    path: outside_path.to_string_lossy().to_string().into(),
                    digest: Some(Digest::from_bytes(b"").unwrap()),
                    ..Default::default()
                }],
                ..Default::default()
            };

            container.seed_manifest(&state.digest, manifest).await;
            let source = load_source(&container, &state).await;

            assert!(!outside_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!outside_path.exists());

            let _ = fs::remove_file(&outside_path);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_manifest_undeclared_workspace_output() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let undeclared_path = container.sandbox.path().join("project/runner.js");
            let _ = fs::remove_file(&undeclared_path);

            let manifest = Manifest {
                files: vec![ManifestFile {
                    path: "project/runner.js".into(),
                    digest: Some(Digest::from_bytes(b"").unwrap()),
                    ..Default::default()
                }],
                ..Default::default()
            };

            container.seed_manifest(&state.digest, manifest).await;
            let source = load_source(&container, &state).await;

            assert!(!undeclared_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!undeclared_path.exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_empty_files_even_when_empty_blob_missing_from_cas() {
            // A manifest may reference the empty blob (e3b0c4…) without it being
            // present in the local CAS (e.g. bytes that only landed at output
            // paths). Empty outputs must still hydrate — reconstructed directly
            // rather than fetched.
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let empty_digest = Digest::from_bytes(b"").unwrap();
            let manifest = Manifest {
                files: ["a.txt", "b.txt", "c.txt"]
                    .into_iter()
                    .map(|name| ManifestFile {
                        path: format!("project/{name}").into(),
                        digest: Some(empty_digest.clone()),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            };

            container.seed_manifest(&state.digest, manifest).await;

            assert!(
                !container.blob_exists(&empty_digest).await,
                "precondition: empty blob is NOT in storage"
            );

            let source = load_source(&container, &state).await;

            assert_hydrated(
                container
                    .create_hydrator()
                    .hydrate(HydrateFrom::Storage(Box::new(source)), "hash123", &state)
                    .await
                    .unwrap(),
            );

            for name in ["a.txt", "b.txt", "c.txt"] {
                let path = container.sandbox.path().join("project").join(name);
                assert!(path.exists(), "{name} should exist");
                assert_eq!(fs::metadata(&path).unwrap().len(), 0);
            }
        }
    }
}
