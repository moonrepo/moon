mod utils;

use moon_project::Project;
use moon_task::Target;
use moon_task_runner::output_archiver::OutputArchiver;
use moon_test_utils2::{generate_project_graph_from_sandbox, ProjectGraph};
use moon_workspace::Workspace;
use starbase_archive::Archiver;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::env;
use std::fs;
use std::sync::Arc;
use utils::*;

pub struct OutputArchiverContainer {
    pub project_graph: ProjectGraph,
    pub project: Arc<Project>,
    pub workspace: Workspace,
}

impl OutputArchiverContainer {
    pub fn modify_workspace(&mut self, mut op: impl FnMut(&mut Workspace)) {
        op(&mut self.workspace);
    }

    pub fn build(&self, task_id: &str) -> OutputArchiver {
        let task = self.project.get_task(task_id).unwrap();

        OutputArchiver {
            project_config: &self.project.config,
            task: &task,
            workspace: &self.workspace,
        }
    }
}

async fn generate_container() -> (Sandbox, OutputArchiverContainer) {
    let sandbox = create_sandbox("archive");
    let workspace = create_workspace(sandbox.path());
    let project_graph = generate_project_graph_from_sandbox(sandbox.path()).await;
    let project = project_graph.get("project").unwrap();

    (
        sandbox,
        OutputArchiverContainer {
            project,
            project_graph,
            workspace,
        },
    )
}

mod output_archiver {
    use super::*;

    mod pack {
        use super::*;

        #[tokio::test]
        async fn does_nothing_if_no_hash() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("file-outputs");

            assert!(archiver.archive("").await.unwrap().is_none());
        }

        #[tokio::test]
        async fn does_nothing_if_no_outputs_in_task() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("no-outputs");

            assert!(archiver.archive("hash123").await.unwrap().is_none());
        }

        #[tokio::test]
        #[should_panic(
            expected = "Task project:file-outputs defines outputs, but none exist after being ran."
        )]
        async fn errors_if_outputs_not_created() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("file-outputs");

            archiver.archive("hash123").await.unwrap();
        }

        #[tokio::test]
        async fn creates_an_archive() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");

            assert!(archiver.archive("hash123").await.unwrap().is_some());
            assert!(sandbox
                .path()
                .join(".moon/cache/outputs/hash123.tar.gz")
                .exists());
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_it_exists() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/file.txt", "");
            sandbox.create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let archiver = container.build("file-outputs");
            let file = archiver.archive("hash123").await.unwrap().unwrap();

            assert_eq!(fs::metadata(file).unwrap().len(), 0);
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_cache_disabled() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");

            env::set_var("MOON_CACHE", "off");

            assert!(archiver.archive("hash123").await.unwrap().is_none());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn doesnt_create_an_archive_if_cache_read_only() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");

            env::set_var("MOON_CACHE", "read");

            assert!(archiver.archive("hash123").await.unwrap().is_none());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn includes_input_files_in_archive() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");
            let file = archiver.archive("hash123").await.unwrap().unwrap();
            let dir = sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn includes_input_globs_in_archive() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file("project/one.txt", "");
            sandbox.create_file("project/two.txt", "");
            sandbox.create_file("project/three.txt", "");

            let archiver = container.build("glob-outputs");
            let file = archiver.archive("hash123").await.unwrap().unwrap();
            let dir = sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            assert!(dir.join("project/one.txt").exists());
            assert!(dir.join("project/two.txt").exists());
            assert!(dir.join("project/three.txt").exists());
        }

        #[tokio::test]
        async fn includes_std_logs_in_archive() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file(".moon/cache/states/project/file-outputs/stdout.log", "out");
            sandbox.create_file(".moon/cache/states/project/file-outputs/stderr.log", "err");
            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");
            let file = archiver.archive("hash123").await.unwrap().unwrap();
            let dir = sandbox.path().join("out");

            Archiver::new(&dir, &file).unpack_from_ext().unwrap();

            let err = dir.join(".moon/cache/states/project/file-outputs/stderr.log");
            let out = dir.join(".moon/cache/states/project/file-outputs/stdout.log");

            assert!(err.exists());
            assert!(out.exists());
            assert_eq!(fs::read_to_string(err).unwrap(), "err");
            assert_eq!(fs::read_to_string(out).unwrap(), "out");
        }
    }

    mod is_archivable {
        use super::*;

        #[tokio::test]
        async fn returns_based_on_type() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("build-type");

            assert!(archiver.is_archivable().unwrap());

            let archiver = container.build("run-type");

            assert!(!archiver.is_archivable().unwrap());

            let archiver = container.build("test-type");

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn can_return_true_for_run_type_if_workspace_configured() {
            let (_sandbox, mut container) = generate_container().await;

            // Project scope
            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::new("project", "run-type").unwrap());
                }
            });

            let archiver = container.build("run-type");

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn can_return_true_for_test_type_if_workspace_configured() {
            let (_sandbox, mut container) = generate_container().await;

            // All scope
            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse(":test-type").unwrap());
                }
            });

            let archiver = container.build("test-type");

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_all_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse(":no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_all_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse(":unknown-task").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_project_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::new("project", "no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_project_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::new("other-project", "no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn matches_tag_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse("#cache:no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        async fn doesnt_match_tag_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse("#other-tag:no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
        async fn errors_for_deps_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse("^:no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(!archiver.is_archivable().unwrap());
        }

        #[tokio::test]
        #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
        async fn errors_for_self_config() {
            let (_sandbox, mut container) = generate_container().await;

            container.modify_workspace(|ws| {
                if let Some(config) = Arc::get_mut(&mut ws.config) {
                    config
                        .runner
                        .archivable_targets
                        .push(Target::parse("~:no-outputs").unwrap());
                }
            });

            let archiver = container.build("no-outputs");

            assert!(!archiver.is_archivable().unwrap());
        }
    }

    mod has_outputs {
        use super::*;

        #[tokio::test]
        async fn returns_false_if_no_files() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("file-outputs");

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_files() {
            let (sandbox, container) = generate_container().await;

            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("file-outputs");

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_false_if_no_globs() {
            let (_sandbox, container) = generate_container().await;
            let archiver = container.build("glob-outputs");

            assert!(!archiver.has_outputs_been_created(false).unwrap());
        }

        #[tokio::test]
        async fn returns_true_if_globs() {
            let (sandbox, container) = generate_container().await;

            sandbox.create_file("project/file.txt", "");

            let archiver = container.build("glob-outputs");

            assert!(archiver.has_outputs_been_created(false).unwrap());
        }
    }
}
