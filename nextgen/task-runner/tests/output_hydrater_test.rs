mod utils;

use moon_task_runner::output_hydrater::HydrateFrom;
use starbase_archive::Archiver;
use starbase_sandbox::Sandbox;
use std::env;
use std::fs;
use std::path::PathBuf;
use utils::*;

pub fn pack_archive(sandbox: &Sandbox) -> PathBuf {
    let file = sandbox.path().join(".moon/cache/outputs/hash123.tar.gz");
    let out = ".moon/cache/states/project/file-outputs/stdout.log";
    let err = ".moon/cache/states/project/file-outputs/stderr.log";
    let txt = "project/file.txt";

    sandbox.create_file(out, "out");
    sandbox.create_file(err, "err");
    sandbox.create_file(txt, "");

    let mut archiver = Archiver::new(sandbox.path(), &file);
    archiver.add_source_file(out, None);
    archiver.add_source_file(err, None);
    archiver.add_source_file(txt, None);
    archiver.pack_from_ext().unwrap();

    // Remove sources so we can test unpacking
    fs::remove_file(sandbox.path().join(out)).unwrap();
    fs::remove_file(sandbox.path().join(err)).unwrap();
    fs::remove_file(sandbox.path().join(txt)).unwrap();

    file
}

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

            env::set_var("MOON_CACHE", "off");

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

            env::set_var("MOON_CACHE", "write");

            assert!(!hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn unpacks_archive_into_project() {
            let container = TaskRunnerContainer::new("archive").await;

            pack_archive(&container.sandbox);

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

            pack_archive(&container.sandbox);

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
