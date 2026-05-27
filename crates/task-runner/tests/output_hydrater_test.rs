mod utils;

use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use moon_task_runner::output_hydrater::HydrateFrom;
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
}
