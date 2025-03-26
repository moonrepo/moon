mod utils;

use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use starbase_archive::Archiver;
use std::fs;
use utils::*;

mod output_archiver {
    use super::*;

    mod pack {
        use super::*;

        #[tokio::test]
        #[should_panic(expected = "Task project:file-outputs defines outputs but after being ran")]
        async fn errors_if_outputs_not_created() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let archiver = container.create_archiver();

            archiver.archive("hash123", None).await.unwrap();
        }

        #[tokio::test]
        async fn creates_an_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.archive("hash123", None).await.unwrap().is_some());
            assert!(
                container
                    .sandbox
                    .path()
                    .join(".moon/cache/outputs/hash123.tar.gz")
                    .exists()
            );
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_it_exists() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");
            container
                .sandbox
                .create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();

            assert_eq!(fs::metadata(file).unwrap().len(), 0);
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_cache_disabled() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Off);

            assert!(archiver.archive("hash123", None).await.unwrap().is_none());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_cache_read_only() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            container
                .app_context
                .cache_engine
                .force_mode(CacheMode::Read);

            assert!(archiver.archive("hash123", None).await.unwrap().is_none());

            GlobalEnvBag::instance().remove("MOON_CACHE");
        }

        #[tokio::test]
        async fn includes_input_files_in_archive() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn includes_input_globs_in_archive() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            container.sandbox.create_file("project/one.txt", "");
            container.sandbox.create_file("project/two.txt", "");
            container.sandbox.create_file("project/three.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/one.txt").exists());
            assert!(dir.join("project/two.txt").exists());
            assert!(dir.join("project/three.txt").exists());
        }

        #[tokio::test]
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

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            let err = dir.join(".moon/cache/states/project/file-outputs/stderr.log");
            let out = dir.join(".moon/cache/states/project/file-outputs/stdout.log");

            assert!(err.exists());
            assert!(out.exists());
            assert_eq!(fs::read_to_string(err).unwrap(), "err");
            assert_eq!(fs::read_to_string(out).unwrap(), "out");
        }

        #[tokio::test]
        async fn can_ignore_output_files_with_negation() {
            let container = TaskRunnerContainer::new("archive", "file-outputs-negated").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(!dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test]
        async fn can_ignore_output_globs_with_negation() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs-negated").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(!dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test]
        async fn caches_one_file() {
            let container = TaskRunnerContainer::new("archive", "output-one-file").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn caches_many_files() {
            let container = TaskRunnerContainer::new("archive", "output-many-files").await;
            container.sandbox.create_file("project/a.txt", "");
            container.sandbox.create_file("project/b.txt", "");
            container.sandbox.create_file("project/c.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a.txt").exists());
            assert!(dir.join("project/b.txt").exists());
            assert!(dir.join("project/c.txt").exists());
        }

        #[tokio::test]
        async fn caches_one_directory() {
            let container = TaskRunnerContainer::new("archive", "output-one-dir").await;
            container.sandbox.create_file("project/dir/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/dir/file.txt").exists());
        }

        #[tokio::test]
        async fn caches_many_directories() {
            let container = TaskRunnerContainer::new("archive", "output-many-dirs").await;
            container.sandbox.create_file("project/a/file.txt", "");
            container.sandbox.create_file("project/b/file.txt", "");
            container.sandbox.create_file("project/c/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/a/file.txt").exists());
            assert!(dir.join("project/b/file.txt").exists());
            assert!(dir.join("project/c/file.txt").exists());
        }

        #[tokio::test]
        async fn caches_file_and_directory() {
            let container = TaskRunnerContainer::new("archive", "output-file-and-dir").await;
            container.sandbox.create_file("project/file.txt", "");
            container.sandbox.create_file("project/dir/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
            assert!(dir.join("project/dir/file.txt").exists());
        }

        #[tokio::test]
        async fn caches_files_from_workspace() {
            let container = TaskRunnerContainer::new("archive", "output-workspace").await;
            container.sandbox.create_file("root.txt", "");
            container.sandbox.create_file("shared/a.txt", "");
            container.sandbox.create_file("shared/z.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("root.txt").exists());
            assert!(dir.join("shared/a.txt").exists());
            assert!(dir.join("shared/z.txt").exists());
        }

        #[tokio::test]
        async fn caches_files_from_workspace_and_project() {
            let container =
                TaskRunnerContainer::new("archive", "output-workspace-and-project").await;
            container.sandbox.create_file("root.txt", "");
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            let file = archiver.archive("hash123", None).await.unwrap().unwrap();
            let dir = container.sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("root.txt").exists());
            assert!(dir.join("project/file.txt").exists());
        }
    }

    mod has_outputs {
        use super::*;

        #[tokio::test]
        async fn returns_false_if_no_files() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            let archiver = container.create_archiver();

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_files() {
            let container = TaskRunnerContainer::new("archive", "file-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_false_if_no_globs() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            let archiver = container.create_archiver();

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_globs() {
            let container = TaskRunnerContainer::new("archive", "glob-outputs").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_only_negated_globs() {
            let container = TaskRunnerContainer::new("archive", "negated-outputs-only").await;
            container.sandbox.create_file("project/file.txt", "");

            let archiver = container.create_archiver();

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }
    }
}
