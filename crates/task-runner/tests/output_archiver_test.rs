mod utils;

use moon_cache::CacheMode;
use moon_env_var::GlobalEnvBag;
use moon_task::Target;
use starbase_archive::Archiver;
use std::fs;
use std::sync::Arc;
use utils::*;

mod output_archiver {
    use super::*;

    mod pack {
        use super::*;

        #[tokio::test]
        async fn does_nothing_if_no_outputs_in_task() {
            let container = TaskRunnerContainer::new("archive", "no-outputs").await;
            let archiver = container.create_archiver();

            assert!(archiver.archive("hash123", None).await.unwrap().is_none());
        }

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

    mod is_archivable {
        use super::*;

        #[tokio::test]
        async fn returns_based_on_type() {
            let container = TaskRunnerContainer::new("archive", "build-type").await;
            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());

            let container = TaskRunnerContainer::new("archive", "run-type").await;
            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());

            let container = TaskRunnerContainer::new("archive", "test-type").await;
            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn can_return_true_for_run_type_if_workspace_configured() {
            let mut container = TaskRunnerContainer::new("archive", "run-type").await;

            // Project scope
            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::new("project", "run-type").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn can_return_true_for_test_type_if_workspace_configured() {
            let mut container = TaskRunnerContainer::new("archive", "test-type").await;

            // All scope
            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse(":test-type").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_all_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse(":no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_all_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse(":unknown-task").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_project_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::new("project", "no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_project_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::new("other-project", "no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_tag_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse("#cache:no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_tag_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse("#other-tag:no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
        async fn errors_for_deps_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse("^:no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_for_self_config() {
            let mut container = TaskRunnerContainer::new("archive", "no-outputs").await;

            if let Some(config) = Arc::get_mut(&mut container.app_context.workspace_config) {
                config
                    .pipeline
                    .archivable_targets
                    .push(Target::parse("~:no-outputs").unwrap());
            }

            let archiver = container.create_archiver();

            assert!(!archiver.is_archivable().unwrap());
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
