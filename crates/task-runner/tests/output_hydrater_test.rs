mod utils;

use moon_cache::CacheMode;
use moon_task_runner::output_hydrater::HydrateFrom;
use std::env;
use utils::*;

mod output_hydrater {
    use super::*;

    mod unpack {
        use super::*;

        #[tokio::test]
        async fn does_nothing_if_no_hash() {
            let container = TaskRunnerContainer::new("archive").await;
            let hydrater = container.create_hydrator("file-outputs");

            assert!(!hydrater.hydrate("", HydrateFrom::LocalCache).await.unwrap());
        }

        #[tokio::test]
        async fn does_nothing_if_from_prev_outputs() {
            let container = TaskRunnerContainer::new("archive").await;
            let hydrater = container.create_hydrator("file-outputs");

            assert!(hydrater
                .hydrate("hash123", HydrateFrom::PreviousOutput)
                .await
                .unwrap());
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.create_hydrator("file-outputs");

            container.workspace.cache_engine.force_mode(CacheMode::Off);

            assert!(!hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_write_only() {
            let container = TaskRunnerContainer::new("archive").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.create_hydrator("file-outputs");

            container
                .workspace
                .cache_engine
                .force_mode(CacheMode::Write);

            assert!(!hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn unpacks_archive_into_project() {
            let container = TaskRunnerContainer::new("archive").await;
            container.pack_archive("file-outputs");

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            let hydrater = container.create_hydrator("file-outputs");
            hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap();

            assert!(container.sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn unpacks_logs_from_archive() {
            let container = TaskRunnerContainer::new("archive").await;
            container.pack_archive("file-outputs");

            assert!(!container
                .sandbox
                .path()
                .join(".moon/cache/states/project/file-outputs/stdout.log")
                .exists());

            let hydrater = container.create_hydrator("file-outputs");
            hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap();

            assert!(container
                .sandbox
                .path()
                .join(".moon/cache/states/project/file-outputs/stdout.log")
                .exists());
        }
    }
}
