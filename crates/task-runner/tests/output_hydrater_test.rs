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

        fn read_action_result(container: &TaskRunnerContainer, state: &TaskRunState) -> ActionResult {
            let bytes = container
                .app_context
                .cache_engine
                .ac
                .read_bytes(&state.digest.hash)
                .unwrap();

            serde_json::from_slice(&bytes).unwrap()
        }

        #[tokio::test(flavor = "multi_thread")]
        async fn hydrates_output_file_from_cas() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "contents");

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
            container.sandbox.create_file("project/file.txt", "contents");

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
    }
}
