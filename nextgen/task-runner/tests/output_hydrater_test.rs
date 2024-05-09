mod utils;

use moon_project::Project;
use moon_task_runner::output_hydrater::{HydrateFrom, OutputHydrater};
use moon_test_utils2::{generate_project_graph_from_sandbox, ProjectGraph};
use moon_workspace::Workspace;
use starbase_archive::Archiver;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use utils::*;

pub struct OutputHydraterContainer {
    pub project_graph: ProjectGraph,
    pub project: Arc<Project>,
    pub workspace: Workspace,
}

impl OutputHydraterContainer {
    pub fn create_archive(&self, sandbox: &Sandbox) -> PathBuf {
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

    pub fn build(&self, task_id: &str) -> OutputHydrater {
        let task = self.project.get_task(task_id).unwrap();

        OutputHydrater {
            task,
            workspace: &self.workspace,
        }
    }
}

async fn generate_container() -> (Sandbox, OutputHydraterContainer) {
    let sandbox = create_sandbox("archive");
    let workspace = create_workspace(sandbox.path());
    let project_graph = generate_project_graph_from_sandbox(sandbox.path()).await;
    let project = project_graph.get("project").unwrap();

    (
        sandbox,
        OutputHydraterContainer {
            project,
            project_graph,
            workspace,
        },
    )
}

mod output_hydrater {
    use super::*;

    mod unpack {
        use super::*;

        #[tokio::test]
        async fn does_nothing_if_no_hash() {
            let (_sandbox, container) = generate_container().await;
            let hydrater = container.build("file-outputs");

            assert!(!hydrater.hydrate("", HydrateFrom::LocalCache).await.unwrap());
        }

        #[tokio::test]
        async fn does_nothing_if_from_prev_outputs() {
            let (_sandbox, container) = generate_container().await;
            let hydrater = container.build("file-outputs");

            assert!(hydrater
                .hydrate("hash123", HydrateFrom::PreviousOutput)
                .await
                .unwrap());
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_disabled() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.build("file-outputs");

            env::set_var("MOON_CACHE", "off");

            assert!(!hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn doesnt_unpack_if_cache_write_only() {
            let (sandbox, container) = generate_container().await;
            sandbox.create_file(".moon/cache/outputs/hash123.tar.gz", "");

            let hydrater = container.build("file-outputs");

            env::set_var("MOON_CACHE", "write");

            assert!(!hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap());

            env::remove_var("MOON_CACHE");
        }

        #[tokio::test]
        async fn unpacks_archive_into_project() {
            let (sandbox, container) = generate_container().await;
            container.create_archive(&sandbox);

            assert!(!sandbox.path().join("project/file.txt").exists());

            let hydrater = container.build("file-outputs");
            hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap();

            assert!(sandbox.path().join("project/file.txt").exists());
        }

        #[tokio::test]
        async fn unpacks_logs_from_archive() {
            let (sandbox, container) = generate_container().await;
            container.create_archive(&sandbox);

            assert!(!sandbox
                .path()
                .join(".moon/cache/states/project/file-outputs/stdout.log")
                .exists());

            let hydrater = container.build("file-outputs");
            hydrater
                .hydrate("hash123", HydrateFrom::LocalCache)
                .await
                .unwrap();

            assert!(sandbox
                .path()
                .join(".moon/cache/states/project/file-outputs/stdout.log")
                .exists());
        }
    }
}
