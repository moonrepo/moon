#![allow(dead_code)]

use moon_action::{ActionNode, RunTaskNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_blob::{BlobContent, BlobInput, Bytes};
use moon_cache::Manifest;
use moon_env_var::GlobalEnvBag;
use moon_hash::Digest;
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_task_runner::TaskRunState;
use moon_task_runner::TaskRunner;
use moon_task_runner::command_builder::CommandBuilder;
use moon_task_runner::output_archiver::OutputArchiver;
use moon_task_runner::output_hydrater::OutputHydrater;
use moon_task_runner::task_executor::TaskExecutor;
use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
use starbase_archive::Archiver;
use starbase_sandbox::{Sandbox, create_sandbox};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn create_node(task: &Task) -> ActionNode {
    ActionNode::RunTask(Box::new(RunTaskNode::new(task.target.clone())))
}

pub struct TaskRunnerContainer {
    pub sandbox: Sandbox,
    pub app_context: Arc<AppContext>,
    pub env_bag: GlobalEnvBag,
    pub project: Arc<Project>,
    pub project_id: String,
    pub task: Arc<Task>,
    pub task_id: String,
    pub workspace_graph: WorkspaceGraph,
}

impl TaskRunnerContainer {
    pub async fn new_for_project(fixture: &str, project_id: &str, task_id: &str) -> Self {
        let sandbox = create_sandbox(fixture);
        let mut mocker = WorkspaceMocker::new(sandbox.path())
            .load_default_configs()
            .with_global_envs()
            .with_default_projects();

        if fixture.contains("toolchain") {
            mocker = mocker.with_test_toolchains();
        }

        if fixture.contains("extension") {
            mocker = mocker.with_test_extensions();
        }

        let app_context = mocker.mock_app_context();
        let workspace_graph = mocker.mock_workspace_graph().await;
        let project = workspace_graph.get_project(project_id).unwrap();
        let task = workspace_graph
            .get_task_from_project(project_id, task_id)
            .unwrap();

        Self {
            sandbox,
            app_context: Arc::new(app_context),
            env_bag: GlobalEnvBag::default(),
            workspace_graph,
            project,
            project_id: project_id.to_owned(),
            task,
            task_id: task_id.to_owned(),
        }
    }

    pub async fn new_os(fixture: &str, task_id: &str) -> Self {
        Self::new_for_project(
            fixture,
            if cfg!(windows) { "windows" } else { "unix" },
            task_id,
        )
        .await
    }

    pub async fn new(fixture: &str, task_id: &str) -> Self {
        Self::new_for_project(fixture, "project", task_id).await
    }

    pub fn create_archiver(&self) -> OutputArchiver<'_> {
        OutputArchiver::new(&self.app_context, &self.task).unwrap()
    }

    pub fn create_hydrator(&self) -> OutputHydrater<'_> {
        OutputHydrater::new(&self.app_context, &self.task).unwrap()
    }

    pub fn create_state(&self) -> TaskRunState {
        TaskRunState::new(&self.app_context, &self.task)
    }

    /// Archiving into storage is fire-and-forget; flush the background queue so
    /// the CAS/AC writes are observable before asserting on them.
    pub async fn flush_storage(&self) {
        self.app_context
            .cache_engine
            .storage
            .wait_for_background_tasks()
            .await
            .unwrap();
    }

    /// Whether a manifest for the given digest exists in any storage backend.
    pub async fn manifest_exists(&self, digest: &Digest) -> bool {
        self.app_context
            .cache_engine
            .storage
            .load_manifest(digest)
            .await
            .unwrap()
            .is_some()
    }

    /// Whether a blob for the given digest exists in the local storage backend.
    pub async fn blob_exists(&self, digest: &Digest) -> bool {
        let backend = self.app_context.cache_engine.storage.get_backends()[0].clone();

        backend
            .find_missing_blobs(vec![digest.clone()])
            .await
            .unwrap()
            .is_empty()
    }

    /// Persist a manifest directly into the local storage backend, so it can be
    /// loaded back as a hydration source.
    pub async fn seed_manifest(&self, digest: &Digest, manifest: Manifest) {
        let backend = self.app_context.cache_engine.storage.get_backends()[0].clone();

        backend
            .store_manifest(digest.clone(), manifest)
            .await
            .unwrap();
    }

    /// Persist an inline blob into the local storage backend's CAS and return
    /// its digest.
    pub async fn seed_blob(&self, content: &'static [u8]) -> Digest {
        let digest = Digest::from_bytes(content).unwrap();
        let backend = self.app_context.cache_engine.storage.get_backends()[0].clone();

        backend
            .store_blobs(
                vec![BlobInput {
                    content: BlobContent::Inline(Bytes::from_static(content)),
                    digest: digest.clone(),
                }],
                false,
            )
            .await
            .unwrap();

        digest
    }

    pub async fn create_command(&self, context: ActionContext) -> Command {
        self.create_command_with_config(context, |_, _| {}).await
    }

    pub async fn create_command_with_config(
        &self,
        context: ActionContext,
        mut op: impl FnMut(&mut Task, &mut ActionNode),
    ) -> Command {
        let mut task = self.task.as_ref().to_owned();
        let mut node = create_node(&task);

        op(&mut task, &mut node);

        self.internal_create_command(&context, &task, &node).await
    }

    pub async fn create_command_executor(&self, context: &ActionContext) -> TaskExecutor<'_> {
        let node = create_node(&self.task);

        TaskExecutor::new(
            &self.app_context,
            &self.project,
            &self.task,
            &node,
            self.internal_create_command(context, &self.task, &node)
                .await,
        )
    }

    pub fn create_runner(&self) -> TaskRunner<'_> {
        TaskRunner::new(&self.app_context, &self.project, &self.task).unwrap()
    }

    pub fn create_action_node(&self) -> ActionNode {
        create_node(&self.task)
    }

    pub fn pack_archive(&self) -> PathBuf {
        let sandbox = &self.sandbox;
        let file = sandbox.path().join(".moon/cache/outputs/hash123.tar.gz");

        let out = format!(
            ".moon/cache/states/{}/{}/stdout.log",
            self.project_id, self.task_id,
        );

        let err = format!(
            ".moon/cache/states/{}/{}/stderr.log",
            self.project_id, self.task_id,
        );

        let txt = format!("{}/file.txt", self.project_id);

        sandbox.create_file(&out, "stdout");
        sandbox.create_file(&err, "stderr");
        sandbox.create_file(&txt, "content");

        let mut archiver = Archiver::new(sandbox.path(), &file);
        archiver.add_source_file(&out, None);
        archiver.add_source_file(&err, None);
        archiver.add_source_file(&txt, None);
        archiver.pack_from_ext().unwrap();

        // Remove sources so we can test unpacking
        fs::remove_file(sandbox.path().join(out)).unwrap();
        fs::remove_file(sandbox.path().join(err)).unwrap();
        fs::remove_file(sandbox.path().join(txt)).unwrap();

        file
    }

    pub async fn create_check_command(&self, check: &moon_task::TaskCheckEntry) -> Command {
        let task = self.task.as_ref();

        let mut builder = CommandBuilder::new(&self.app_context, &self.project, task);
        builder.set_env_bag(&self.env_bag);
        builder.build_check(check).await.unwrap()
    }

    async fn internal_create_command(
        &self,
        context: &ActionContext,
        task: &Task,
        node: &ActionNode,
    ) -> Command {
        let mut builder = CommandBuilder::new(&self.app_context, &self.project, task);
        builder.set_env_bag(&self.env_bag);
        builder.build(context, node, "abc123").await.unwrap()
    }
}
