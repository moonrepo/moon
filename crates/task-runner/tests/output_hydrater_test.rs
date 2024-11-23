mod utils;

use moon_action::Operation;
use moon_cache::CacheMode;
use moon_remote::Digest;
use moon_task_runner::output_hydrater::HydrateFrom;
use std::env;
use utils::*;

fn stub_digest() -> Digest {
    Digest {
        hash: "hash123".into(),
        size_bytes: 0,
    }
}

fn stub_operation() -> Operation {
    Operation::output_hydration()
}

mod output_hydrater {
    use super::*;

    mod unpack {
        use super::*;

        // #[tokio::test]
        // async fn does_nothing_if_no_hash() {
        //     let container = TaskRunnerContainer::new("archive", "file-outputs").await;
        //     let hydrater = container.create_hydrator();

        //     assert!(!hydrater.hydrate("", HydrateFrom::LocalCache).await.unwrap());
        // }

        #[tokio::test]
        async fn does_nothing_if_from_prev_outputs() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let hydrater = container.create_hydrator();
            let digest = stub_digest();
            let mut operation = stub_operation();

            assert!(hydrater
                .hydrate(HydrateFrom::PreviousOutput, &digest, &mut operation)
                .await
                .unwrap());
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.create_hydrator();
            let digest = stub_digest();
            let mut operation = stub_operation();

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            assert!(!hydrater
                .hydrate(HydrateFrom::LocalCache, &digest, &mut operation)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_write_only() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.create_hydrator();
            let digest = stub_digest();
            let mut operation = stub_operation();

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Write);

            assert!(!hydrater
                .hydrate(HydrateFrom::LocalCache, &digest, &mut operation)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn unpacks_archive_into_project() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(!container.sandbox.path().join("project/file.txt").exists());

            let hydrater = container.create_hydrator();
            let digest = stub_digest();
            let mut operation = stub_operation();

            hydrater
                .hydrate(HydrateFrom::LocalCache, &digest, &mut operation)
                .await
                .unwrap();

            assert!(container.sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn unpacks_logs_from_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.pack_archive();

            assert!(!container
                .sandbox
                .path()
                .join(".moon/cache/states/project/file-outputs/stdout.log")
                .exists());

            let hydrater = container.create_hydrator();
            let digest = stub_digest();
            let mut operation = stub_operation();

            hydrater
                .hydrate(HydrateFrom::LocalCache, &digest, &mut operation)
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
