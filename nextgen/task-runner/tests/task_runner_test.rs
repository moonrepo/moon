mod utils;

use moon_task_runner::output_hydrater::HydrateFrom;
use std::env;
use utils::*;

mod task_runner {
    use super::*;

    mod is_cached {
        use super::*;

        #[tokio::test]
        async fn returns_none_by_default() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");

            assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
        }

        #[tokio::test]
        async fn sets_the_hash_to_cache() {
            let container = TaskRunnerContainer::new("runner").await;
            let mut runner = container.create_runner("base");

            runner.is_cached("hash123").await.unwrap();

            assert_eq!(runner.cache.data.hash, "hash123");
        }

        mod previous_output {
            use super::*;

            #[tokio::test]
            async fn returns_if_hashes_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                );
            }

            #[tokio::test]
            async fn skips_if_hashes_dont_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "otherhash456".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_codes_dont_match() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 2;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_outputs_dont_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("outputs");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn returns_if_outputs_do_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                runner.cache.data.exit_code = 0;
                runner.cache.data.hash = "hash123".into();
                container.sandbox.create_file("project/file.txt", "");

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::PreviousOutput)
                );
            }
        }

        mod local_cache {
            use super::*;

            #[tokio::test]
            async fn returns_if_archive_exists() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                assert_eq!(
                    runner.is_cached("hash123").await.unwrap(),
                    Some(HydrateFrom::LocalCache)
                );
            }

            #[tokio::test]
            async fn skips_if_archive_doesnt_exist() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);
            }

            #[tokio::test]
            async fn skips_if_cache_isnt_readable() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                env::set_var("MOON_CACHE", "off");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);

                env::remove_var("MOON_CACHE");
            }

            #[tokio::test]
            async fn skips_if_cache_is_writeonly() {
                let container = TaskRunnerContainer::new("runner").await;
                let mut runner = container.create_runner("base");

                container
                    .sandbox
                    .create_file(".moon/cache/outputs/hash123.tar.gz", "");

                env::set_var("MOON_CACHE", "write");

                assert_eq!(runner.is_cached("hash123").await.unwrap(), None);

                env::remove_var("MOON_CACHE");
            }
        }
    }
}
