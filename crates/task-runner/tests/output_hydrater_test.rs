mod utils;

use bazel_remote_apis::build::bazel::remote::execution::v2::ActionResult;
use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use moon_hash::Digest;
use moon_task_runner::TaskRunState;
use moon_task_runner::output_hydrater::HydrateFrom;
use std::fs;
use utils::*;

mod output_hydrater {
    use super::*;

    mod local_legacy {
        use super::*;

        // #[tokio::test(flavor = "multi_thread")]
        // async fn does_nothing_if_no_hash() {
        //     let container = TaskRunnerContainer::new("archive", "file-outputs").await;
        //     let hydrater = container.create_hydrator();

        //     assert!(!hydrater.hydrate("", HydrateFrom::LocalCache).await.unwrap());
        // }

        #[tokio::test(flavor = "multi_thread")]
        async fn does_nothing_if_from_prev_outputs() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let hydrater = container.create_hydrator();
            let state = container.create_state();

            assert!(
                hydrater
                    .hydrate(&mut HydrateFrom::PreviousOutput, "hash123", &state)
                    .await
                    .unwrap()
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

            assert!(
                !hydrater
                    .hydrate(&mut HydrateFrom::LocalArchive, "hash123", &state)
                    .await
                    .unwrap()
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

            assert!(
                !hydrater
                    .hydrate(&mut HydrateFrom::LocalArchive, "hash123", &state)
                    .await
                    .unwrap()
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

            hydrater
                .hydrate(&mut HydrateFrom::LocalArchive, "hash123", &state)
                .await
                .unwrap();

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
                .hydrate(&mut HydrateFrom::LocalArchive, "hash123", &state)
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
        use bazel_remote_apis::build::bazel::remote::execution::v2::OutputFile;
        use moon_cache::InternalDigestExt;
        use starbase_utils::json::serde_json;

        fn setup_cas_state(state: &mut TaskRunState) {
            state.local_cas_enabled = true;
            state.bytes = b"hash123".to_vec();
            state.digest = Digest::from_bytes(&state.bytes).unwrap();
        }

        async fn populate_cas(container: &TaskRunnerContainer, state: &TaskRunState) {
            let archiver = container.create_archiver();

            archiver.archive("hash123", state).await.unwrap();
        }

        fn read_action_result(
            container: &TaskRunnerContainer,
            state: &TaskRunState,
        ) -> ActionResult {
            let bytes = container
                .app_context
                .cache_engine
                .ac
                .read(&state.digest.hash)
                .unwrap();

            serde_json::from_slice(&bytes).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_output_file_from_cas() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file("project/file.txt", "contents");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            populate_cas(&container, &state).await;

            // Remove source so we can verify hydration restores it
            fs::remove_file(container.sandbox.path().join("project/file.txt")).unwrap();

            let result = read_action_result(&container, &state);

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
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

            populate_cas(&container, &state).await;

            fs::remove_file(container.sandbox.path().join("project/file.txt")).unwrap();

            let result = read_action_result(&container, &state);

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert!(
                !container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
            );

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn falls_back_to_archive_if_cas_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            let state = container.create_state();
            let result = ActionResult::default();

            // CAS is not enabled in state, so it should fall back to unpacking the legacy archive
            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
            );

            assert!(container.sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_multiple_files_sharing_the_same_blob() {
            // Multiple output files that share content also share a CAS blob.
            // The local hydrate path reads bytes per file from the CAS, so
            // duplicates "just work" here — this test pins that behavior so
            // a future change (e.g. moving to a consuming `remove`) doesn't
            // re-introduce the bug the remote-restore path hit.
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "shared");
            container.sandbox.create_file("project/b.txt", "shared");
            container.sandbox.create_file("project/c.txt", "shared");

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            populate_cas(&container, &state).await;

            // Wipe the on-disk outputs so we know hydration restored them.
            fs::remove_file(container.sandbox.path().join("project/a.txt")).unwrap();
            fs::remove_file(container.sandbox.path().join("project/b.txt")).unwrap();
            fs::remove_file(container.sandbox.path().join("project/c.txt")).unwrap();

            let result = read_action_result(&container, &state);

            // All three output_files entries should reference the same digest.
            let digests: Vec<_> = result
                .output_files
                .iter()
                .filter_map(|f| f.digest.as_ref().map(|d| d.hash.clone()))
                .collect();
            assert_eq!(digests.len(), 3);
            assert!(digests.iter().all(|h| h == &digests[0]));

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
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

            populate_cas(&container, &state).await;

            fs::remove_dir_all(container.sandbox.path().join("project/dir")).unwrap();

            let result = read_action_result(&container, &state);

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
            );

            assert_eq!(
                fs::read_to_string(container.sandbox.path().join("project/dir/file.txt")).unwrap(),
                "contents"
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_action_result_path_outside_workspace() {
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

            let digest = container
                .app_context
                .cache_engine
                .cas
                .store_bytes(b"REMOTE_CACHE_OUTSIDE_WORKSPACE_WRITE")
                .unwrap();

            let mut result = ActionResult::default();
            result.output_files.push(OutputFile {
                path: format!("../{outside_name}"),
                digest: Some(digest.to_external_digest()),
                ..Default::default()
            });

            assert!(!outside_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!outside_path.exists());

            let _ = fs::remove_file(&outside_path);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_action_result_absolute_path_outside_workspace() {
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

            let digest = container
                .app_context
                .cache_engine
                .cas
                .store_bytes(b"REMOTE_CACHE_ABSOLUTE_PATH_WORKSPACE_WRITE")
                .unwrap();

            let mut result = ActionResult::default();
            result.output_files.push(OutputFile {
                path: outside_path.to_string_lossy().to_string(),
                digest: Some(digest.to_external_digest()),
                ..Default::default()
            });

            assert!(!outside_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!outside_path.exists());

            let _ = fs::remove_file(&outside_path);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn rejects_untrusted_action_result_undeclared_workspace_output() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            let undeclared_path = container.sandbox.path().join("project/runner.js");
            let _ = fs::remove_file(&undeclared_path);

            let digest = container
                .app_context
                .cache_engine
                .cas
                .store_bytes(b"REMOTE_CACHE_UNDECLARED_WORKSPACE_OUTPUT")
                .unwrap();

            let mut result = ActionResult::default();
            result.output_files.push(OutputFile {
                path: "project/runner.js".to_owned(),
                digest: Some(digest.to_external_digest()),
                ..Default::default()
            });

            assert!(!undeclared_path.exists());

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .is_err()
            );

            assert!(!undeclared_path.exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_empty_files_even_when_empty_blob_missing_from_cas() {
            // Action results downloaded from a remote cache reference content
            // by digest, but the bytes only land at output paths (not in the
            // local CAS). If hydration then falls back to local for any
            // reason, an empty output file's digest (e3b0c4…) would not
            // resolve in local CAS — the hydrater must handle that without
            // erroring.
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;

            let mut state = container.create_state();
            setup_cas_state(&mut state);

            // Construct an ActionResult by hand: it references the empty
            // blob, but we deliberately never populate the CAS with it.
            let empty_digest = Digest::from_bytes(b"").unwrap();
            let mut result = ActionResult::default();
            for name in ["a.txt", "b.txt", "c.txt"] {
                result.output_files.push(OutputFile {
                    path: format!("project/{name}"),
                    digest: Some(empty_digest.to_external_digest()),
                    ..Default::default()
                });
            }

            assert!(
                !container
                    .app_context
                    .cache_engine
                    .cas
                    .contains_object(&empty_digest.hash),
                "precondition: empty blob is NOT in local CAS"
            );

            assert!(
                container
                    .create_hydrator()
                    .hydrate(&mut HydrateFrom::LocalCache(result), "hash123", &state)
                    .await
                    .unwrap()
            );

            for name in ["a.txt", "b.txt", "c.txt"] {
                let path = container.sandbox.path().join("project").join(name);
                assert!(path.exists(), "{} should exist", name);
                assert_eq!(fs::metadata(&path).unwrap().len(), 0);
            }
        }
    }
}
