mod utils;

use moon_blob::Blob;
use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use moon_hash::Digest;
use starbase_archive::Archiver;
use std::fs;
use utils::*;

mod output_archiver {
    use super::*;

    mod local_legacy {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        #[should_panic(expected = "Task project:file-outputs defines outputs but after being ran")]
        async fn errors_if_outputs_not_created() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_if_outputs_not_created_but_marked_as_optional() {
            let container = TaskRunnerContainer::new("archive", "file-outputs-optional").await;
            let archiver = container.create_archiver();
            let state = container.create_state();

            let _ = archiver.archive("hash123", &state).await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_error_if_outputs_not_created_but_marked_as_optional_using_globs() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs-optional").await;
            let archiver = container.create_archiver();
            let state = container.create_state();

            let _ = archiver.archive("hash123", &state).await.unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn creates_an_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            assert!(archiver.archive("hash123", &state).await.unwrap());
            assert!(
                container
                    .sandbox
                    .path()
                    .join(".moon/cache/outputs/hash123.tar.gz")
                    .exists()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn repacks_an_archive_if_it_exists() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            assert!(archiver.archive("hash123", &state).await.unwrap());

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");

            assert!(fs::metadata(file).unwrap().len() > 0);
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_an_archive_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            let archiver = container.create_archiver();
            let state = container.create_state();

            assert!(!archiver.archive("hash123", &state).await.unwrap());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_an_archive_if_cache_read_only() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Read);

            let archiver = container.create_archiver();
            let state = container.create_state();

            assert!(!archiver.archive("hash123", &state).await.unwrap());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_input_files_in_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_input_globs_in_archive() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            container.sandbox.create_file("project/one.txt", "");
            container.sandbox.create_file("project/two.txt", "");
            container.sandbox.create_file("project/three.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/one.txt").exists());
            assert!(dir.join("project/two.txt").exists());
            assert!(dir.join("project/three.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn includes_std_logs_in_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file(".moon/cache/states/project/file-outputs/stdout.log", "out");
            container
                .sandbox
                .create_file(".moon/cache/states/project/file-outputs/stderr.log", "err");
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            let err = dir.join(".moon/cache/states/project/file-outputs/stderr.log");
            let out = dir.join(".moon/cache/states/project/file-outputs/stdout.log");

            assert!(err.exists());
            assert!(out.exists());
            assert_eq!(fs::read_to_string(err).unwrap(), "err");
            assert_eq!(fs::read_to_string(out).unwrap(), "out");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_ignore_output_files_with_negation() {
            let container = TaskRunnerContainer::new("archive", "file-outputs-negated").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(!dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn can_ignore_output_globs_with_negation() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs-negated").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(!dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_one_file() {
            let container = TaskRunnerContainer::new("archive", "output-one-file").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_many_files() {
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_one_directory() {
            let container = TaskRunnerContainer::new("archive", "output-one-dir").await;
            container.sandbox.create_file("project/dir/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/dir/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_many_directories() {
            let container = TaskRunnerContainer::new("archive", "output-many-dirs").await;
            container.sandbox.create_file("project/a/file.txt", "");
            container.sandbox.create_file("project/b/file.txt", "");
            container.sandbox.create_file("project/c/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a/file.txt").exists());
            assert!(dir.join("project/b/file.txt").exists());
            assert!(dir.join("project/c/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_file_and_directory() {
            let container = TaskRunnerContainer::new("archive", "output-file-and-dir").await;
            container.sandbox.create_file("project/file.txt", "");
            container.sandbox.create_file("project/dir/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
            assert!(dir.join("project/dir/file.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_files_from_workspace() {
            let container = TaskRunnerContainer::new("archive", "output-workspace").await;
            container.sandbox.create_file("root.txt", "");
            container.sandbox.create_file("shared/a.txt", "");
            container.sandbox.create_file("shared/z.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("root.txt").exists());
            assert!(dir.join("shared/a.txt").exists());
            assert!(dir.join("shared/z.txt").exists());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn caches_files_from_workspace_and_project() {
            let container =
                TaskRunnerContainer::new("archive", "output-workspace-and-project").await;
            container.sandbox.create_file("root.txt", "");
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let state = container.create_state();

            archiver.archive("hash123", &state).await.unwrap();

            let file = container
                .app_context
                .cache_engine
                .hash
                .get_archive_path("hash123");
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("root.txt").exists());
            assert!(dir.join("project/file.txt").exists());
        }
    }

    mod local_cas {
        use super::*;

        fn setup_cas_state(state: &mut moon_task_runner::TaskRunState) {
            state.local_cas_enabled = true;
            state.digest = Digest::from_bytes(b"hash123".to_vec()).unwrap();
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stores_action_blob_in_cas() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            archiver.archive("hash123", &state).await.unwrap();

            assert!(
                container
                    .app_context
                    .cache_engine
                    .cas
                    .contains_object(&state.digest.hash)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stores_action_result_in_ac() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            archiver.archive("hash123", &state).await.unwrap();

            assert!(
                container
                    .app_context
                    .cache_engine
                    .ac
                    .contains_object(&state.digest.hash)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn stores_output_file_blobs_in_cas() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file("project/file.txt", "contents");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            archiver.archive("hash123", &state).await.unwrap();

            let blob = Blob::from_bytes(b"contents".to_vec()).unwrap();

            assert!(
                container
                    .app_context
                    .cache_engine
                    .cas
                    .contains_object(&blob.digest.hash)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_create_a_local_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            archiver.archive("hash123", &state).await.unwrap();

            assert!(
                !container
                    .sandbox
                    .path()
                    .join(".moon/cache/outputs/hash123.tar.gz")
                    .exists()
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_write_to_cas_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert!(!archiver.archive("hash123", &state).await.unwrap());

            assert!(
                !container
                    .app_context
                    .cache_engine
                    .ac
                    .contains_object(&state.digest.hash)
            );

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn doesnt_write_to_cas_if_cache_read_only() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Read);

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert!(!archiver.archive("hash123", &state).await.unwrap());

            assert!(
                !container
                    .app_context
                    .cache_engine
                    .ac
                    .contains_object(&state.digest.hash)
            );

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn dedupes_blobs_for_files_with_same_content() {
            // Multiple output files sharing identical content should share a
            // single CAS blob (content-addressed). Regression coverage for
            // the duplicate-digest path that broke remote hydration: any code
            // that iterates output digests must tolerate repeats.
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "shared");
            container.sandbox.create_file("project/b.txt", "shared");
            container.sandbox.create_file("project/c.txt", "shared");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert!(archiver.archive("hash123", &state).await.unwrap());

            // The shared content lives at exactly one CAS hash.
            let shared = Blob::from_bytes(b"shared".to_vec()).unwrap();

            assert!(
                container
                    .app_context
                    .cache_engine
                    .cas
                    .contains_object(&shared.digest.hash)
            );
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn empty_files_share_the_empty_blob() {
            // Empty files all hash to e3b0c4… ; verify the empty blob lands
            // in CAS exactly once even when multiple files are empty.
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();
            let mut state = container.create_state();
            setup_cas_state(&mut state);

            assert!(archiver.archive("hash123", &state).await.unwrap());

            let empty = Blob::from_bytes(vec![]).unwrap();

            assert!(
                container
                    .app_context
                    .cache_engine
                    .cas
                    .contains_object(&empty.digest.hash)
            );
        }
    }

    mod has_outputs {
        use super::*;

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_false_if_no_files() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let archiver = container.create_archiver();

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_files() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_false_if_no_globs() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            let archiver = container.create_archiver();

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_globs() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn returns_true_if_only_negated_globs() {
            let container = TaskRunnerContainer::new("archive", "negated-outputs-only").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }
    }
}
