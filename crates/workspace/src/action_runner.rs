use crate::action::{Action, ActionStatus};
use crate::actions::{install_node_deps, run_target, setup_toolchain, sync_project};
use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::{color, debug, error, trace};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;

const TARGET: &str = "moon:action-runner";

async fn run_action(
    workspace: Arc<RwLock<Workspace>>,
    action: &mut Action,
    action_node: &Node,
    primary_target: &str,
    passthrough_args: &[String],
) -> Result<(), WorkspaceError> {
    let result = match action_node {
        Node::InstallNodeDeps => install_node_deps(workspace).await,
        Node::RunTarget(target_id) => {
            run_target(workspace, target_id, primary_target, passthrough_args).await
        }
        Node::SetupToolchain => setup_toolchain(workspace).await,
        Node::SyncProject(project_id) => sync_project(workspace, project_id).await,
    };

    match result {
        Ok(status) => {
            action.pass(status);
        }
        Err(error) => {
            action.fail(error.to_string());

            // If these fail, we should abort instead of trying to continue
            if matches!(action_node, Node::SetupToolchain)
                || matches!(action_node, Node::InstallNodeDeps)
            {
                action.abort();
            }
        }
    }

    Ok(())
}

pub struct ActionRunner {
    bail: bool,

    pub duration: Option<Duration>,

    passthrough_args: Vec<String>,

    primary_target: String,

    workspace: Arc<RwLock<Workspace>>,
}

impl ActionRunner {
    pub fn new(workspace: Workspace) -> Self {
        debug!(target: TARGET, "Creating action runner",);

        ActionRunner {
            bail: false,
            duration: None,
            passthrough_args: Vec::new(),
            primary_target: String::new(),
            workspace: Arc::new(RwLock::new(workspace)),
        }
    }

    pub fn bail_on_error(&mut self) -> &mut Self {
        self.bail = true;
        self
    }

    pub async fn cleanup(&self) -> Result<(), WorkspaceError> {
        let workspace = self.workspace.read().await;

        // Delete all previously created runfiles
        trace!(target: TARGET, "Deleting stale runfiles");

        workspace.cache.delete_runfiles().await?;

        Ok(())
    }

    pub async fn run(&mut self, graph: DepGraph) -> Result<Vec<Action>, WorkspaceError> {
        let start = Instant::now();
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let graph = Arc::new(RwLock::new(graph));
        let passthrough_args = Arc::new(self.passthrough_args.clone());
        let primary_target = Arc::new(self.primary_target.clone());

        // Clean the runner state *before* running actions instead of after,
        // so that failing or broken builds can dig into and debug the state!
        self.cleanup().await?;

        debug!(
            target: TARGET,
            "Running {} actions across {} batches", node_count, batches_count
        );

        let mut results: Vec<Action> = vec![];

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let batch_target_name = format!("{}:batch:{}", TARGET, batch_count);
            let actions_count = batch.len();

            trace!(
                target: &batch_target_name,
                "Running {} actions",
                actions_count
            );

            let mut action_handles = vec![];

            for (i, node_index) in batch.into_iter().enumerate() {
                let action_count = i + 1;
                let workspace_clone = Arc::clone(&self.workspace);
                let graph_clone = Arc::clone(&graph);
                let passthrough_args_clone = Arc::clone(&passthrough_args);
                let primary_target_clone = Arc::clone(&primary_target);

                action_handles.push(task::spawn(async move {
                    let mut action = Action::new(node_index);
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(node_index) {
                        action.label = Some(node.label());

                        let log_target_name =
                            format!("{}:batch:{}:{}", TARGET, batch_count, action_count);
                        let log_action_label = color::muted_light(&node.label());

                        trace!(
                            target: &log_target_name,
                            "Running action {}",
                            log_action_label
                        );

                        run_action(
                            workspace_clone,
                            &mut action,
                            node,
                            &primary_target_clone,
                            &passthrough_args_clone,
                        )
                        .await?;

                        if action.has_failed() {
                            trace!(
                                target: &log_target_name,
                                "Action {} failed in {:?}",
                                log_action_label,
                                action.duration.unwrap()
                            );
                        } else {
                            trace!(
                                target: &log_target_name,
                                "Ran action {} in {:?}",
                                log_action_label,
                                action.duration.unwrap()
                            );
                        }
                    } else {
                        action.status = ActionStatus::Invalid;

                        return Err(WorkspaceError::DepGraphUnknownNode(node_index.index()));
                    }

                    Ok(action)
                }));
            }

            // Wait for all actions in this batch to complete,
            // while also handling and propagating errors
            for handle in action_handles {
                match handle.await {
                    Ok(Ok(result)) => {
                        if result.should_abort() {
                            error!(
                                target: &batch_target_name,
                                "Encountered a critical error, aborting the action runner"
                            );
                        }

                        if self.bail && result.error.is_some() || result.should_abort() {
                            return Err(WorkspaceError::ActionRunnerFailure(result.error.unwrap()));
                        }

                        results.push(result);
                    }
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(e) => {
                        return Err(WorkspaceError::ActionRunnerFailure(e.to_string()));
                    }
                }
            }
        }

        self.duration = Some(start.elapsed());

        debug!(
            target: TARGET,
            "Finished running {} actions in {:?}",
            node_count,
            self.duration.unwrap()
        );

        Ok(results)
    }

    pub fn set_passthrough_args(&mut self, args: Vec<String>) -> &mut Self {
        self.passthrough_args = args;
        self
    }

    pub fn set_primary_target(&mut self, target: &str) -> &mut Self {
        self.primary_target = target.to_owned();
        self
    }
}
