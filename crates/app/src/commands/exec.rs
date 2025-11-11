use crate::app_error::AppError;
use crate::queries::changed_files::{QueryChangedFilesOptions, query_changed_files};
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, ActionGraphBuilderOptions, RunRequirements};
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_common::{is_ci, is_test_env, path::WorkspaceRelativePathBuf};
use moon_console::Console;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_task::TargetLocator;
use moon_vcs::ChangedStatus;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use std::sync::Arc;
use tracing::{debug, instrument};

const HEADING_AFFECTED: &str = "Affected checks";
const HEADING_GRAPH: &str = "Graph relationships";

#[derive(Args, Clone, Debug)]
pub struct ExecArgs {
    #[arg(help = "Task targets to execute in the action pipeline")]
    pub targets: Vec<TargetLocator>,

    #[arg(
        long,
        env = "MOON_NO_ACTIONS",
        help = "Run the pipeline without sync and setup related actions"
    )]
    pub no_actions: bool,

    #[arg(long, help = "Filter task targets based on the result of a query")]
    pub query: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    pub passthrough: Vec<String>,

    // AFFECTED
    #[arg(
        long,
        help = "Base branch, commit, or revision to compare against",
        help_heading = HEADING_AFFECTED,
    )]
    pub base: Option<String>,

    #[arg(
        long,
        help = "Current branch, commit, or revision to compare with",
        help_heading = HEADING_AFFECTED,
    )]
    pub head: Option<String>,

    #[arg(
        long,
        help = "Filter changed files based on a changed status",
        help_heading = HEADING_AFFECTED
    )]
    pub status: Vec<ChangedStatus>,

    #[arg(
        long,
        help = "Accept changed files from stdin for affected checks",
        help_heading = HEADING_AFFECTED,
    )]
    pub stdin: bool,

    // GRAPH
    #[arg(
        long,
        alias = "dependents",
        default_value_t = DownstreamScope::Direct,
        help = "Control the depth of downstream dependents",
        help_heading = HEADING_GRAPH,
    )]
    pub downstream: DownstreamScope,

    #[arg(
        long,
        alias = "dependencies",
        default_value_t = UpstreamScope::Deep,
        help = "Control the depth of upstream dependencies",
        help_heading = HEADING_GRAPH,
    )]
    pub upstream: UpstreamScope,
}

#[instrument(skip(session))]
pub async fn exec(session: MoonSession, args: ExecArgs) -> AppResult {
    if args.targets.is_empty() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "At least 1 task target is required for executing the action pipeline.")
                }
            }
        })?;
    } else {
        ExecWorkflow::new(session, args).await?.execute().await?;
    }

    Ok(None)
}

pub struct ExecWorkflow {
    args: ExecArgs,
    console: Arc<Console>,
    session: MoonSession,
    workspace_graph: Arc<WorkspaceGraph>,

    /// Whether we should run affected logic or not
    affected: bool,

    /// Whether we should apply `runInCI` checks
    ci_check: bool,

    /// Are we in a CI environment?
    ci_env: bool,

    /// Node indexes for targets inserted into the graph.
    node_indexes: FxHashSet<NodeIndex>,

    /// Are we in a test environment?
    test_env: bool,
}

impl ExecWorkflow {
    pub async fn new(session: MoonSession, args: ExecArgs) -> miette::Result<Self> {
        let ci_env = is_ci();

        Ok(Self {
            affected: true,
            ci_check: ci_env,
            ci_env,
            console: session.get_console()?,
            node_indexes: FxHashSet::default(),
            test_env: is_test_env(),
            workspace_graph: session.get_workspace_graph().await?,
            args,
            session,
        })
    }

    pub async fn execute(&mut self) -> miette::Result<()> {
        let changed_files = self.load_changed_files().await?;
        let (action_context, action_graph) = self.build_action_graph(changed_files).await?;
        let action_graph = self.partition_action_graph(action_graph).await?;

        Ok(())
    }

    // Step 1
    async fn load_changed_files(&mut self) -> miette::Result<FxHashSet<WorkspaceRelativePathBuf>> {
        debug!("Step 1: Loading changed files");

        let vcs = self.session.get_vcs_adapter()?;

        if !vcs.is_enabled() {
            self.affected = false;

            debug!("VCS not enabled, skipping changed and affected checks");

            return Ok(FxHashSet::default());
        }

        let mut base = self.args.base.clone();
        let mut head = self.args.head.clone();

        // If we're in CI, extract PR information for base and head
        if self.ci_env
            && let Some(env) = ci_env::get_environment()
        {
            let is_pr = env.request_id.is_some_and(|id| !id.is_empty());

            if base.is_none() {
                if env.base_revision.is_some() {
                    base = env.base_revision;
                } else if is_pr && env.base_branch.is_some() {
                    base = env.base_branch;
                }
            }

            if head.is_none() && env.head_revision.is_some() {
                head = env.head_revision;
            }
        }

        let result = query_changed_files(
            &vcs,
            QueryChangedFilesOptions {
                default_branch: self.ci_env && !self.test_env,
                base,
                head,
                local: !self.ci_env,
                status: self.args.status.clone(),
                stdin: self.args.stdin,
            },
        )
        .await?;

        if result.shallow {
            if self.ci_env {
                return Err(AppError::CiNoShallowHistory.into());
            } else {
                self.affected = false;
            }
        }

        Ok(result.files)
    }

    // Step 2
    async fn build_action_graph(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<(ActionContext, ActionGraph)> {
        debug!("Step 2: Building action graph");

        let mut action_graph_builder = if self.args.no_actions {
            self.session
                .build_action_graph_with_options(ActionGraphBuilderOptions::new(false))
                .await?
        } else {
            self.session.build_action_graph().await?
        };

        if let Some(query_input) = &self.args.query {
            action_graph_builder.set_query(query_input)?;
        }

        // Always pass changed files, even if not checking affected,
        // as it's required for plugins, contexts, and more
        action_graph_builder.set_changed_files(changed_files)?;

        // Only track affected if enabled
        if self.affected {
            action_graph_builder.track_affected(
                self.args.upstream,
                self.args.downstream,
                self.ci_check,
            )?;
        }

        // Always sync workspace in CI
        if self.ci_env {
            action_graph_builder.sync_workspace().await?;
        }

        // Insert targets into the graph
        let reqs = RunRequirements {
            ci: self.ci_env,
            ci_check: self.ci_check,
            dependents: self.args.downstream != DownstreamScope::None,
            interactive: false, // TODO
            skip_affected: !self.affected,
        };

        for target_locator in &self.args.targets {
            self.node_indexes.extend(
                action_graph_builder
                    .run_task_by_target_locator(target_locator, &reqs)
                    .await?,
            );
        }

        // Build the graph
        let (mut action_context, action_graph) = action_graph_builder.build();

        action_context
            .initial_targets
            .extend(self.args.targets.clone());

        debug!("Target count: {}", self.args.targets.len());
        debug!("Action count: {}", action_graph.get_node_count());

        Ok((action_context, action_graph))
    }

    async fn partition_action_graph(
        &self,
        mut action_graph: ActionGraph,
    ) -> miette::Result<ActionGraph> {
        Ok(action_graph)
    }
}
