use crate::app_options::AffectedOption;
// use crate::app_error::AppError;
use crate::helpers::run_action_pipeline;
use crate::prompts::select_targets;
use crate::queries::changed_files::{QueryChangedFilesOptions, query_changed_files};
use crate::session::{MoonSession, SessionResult};
use ci_env::CiOutput;
use clap::{Args, ValueEnum};
use iocraft::prelude::element;
use moon_action::{Action, ActionNode};
use moon_action_context::ActionContext;
use moon_action_graph::{
    ActionGraph, ActionGraphBuilderOptions, ActionGraphType, BuildOutput, RunRequirements,
};
use moon_affected::Depth;
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_cache::CacheMode;
use moon_common::{apply_style_tags, is_ci, is_test_env, path::WorkspaceRelativePathBuf};
use moon_console::ui::{Container, Notice, SelectOption, SelectProps, StyledText, Variant};
use moon_console::{Console, Level};
use moon_exec_plan::{ExecutionPlan, TargetsBlock};
use moon_task::{Target, TargetLocator};
use moon_vcs::ChangedStatus;
use petgraph::{Direction, graph::NodeIndex, visit::EdgeRef};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase_utils::json;
use std::fmt;
use tracing::{debug, instrument, warn};

#[derive(Clone, Debug, Default, PartialEq, ValueEnum)]
pub enum OnFailure {
    #[default]
    Bail,
    Continue,
}

impl fmt::Display for OnFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Bail => "bail",
                Self::Continue => "continue",
            }
        )
    }
}

#[with_affected_args]
#[with_shared_exec_args(passthrough)]
#[derive(Args, Clone, Debug, Default)]
pub struct ExecArgs {
    #[arg(help = "List of task targets to execute in the action pipeline")]
    pub targets: Vec<TargetLocator>,

    #[arg(
        long,
        help = "Execute the pipeline as if it's a CI environment",
        default_missing_value = "true",
        num_args=0..=1
    )]
    pub ci: Option<bool>,

    #[arg(
        long,
        help = "Ignore \"run in CI\" task checks",
        help_heading = super::HEADING_WORKFLOW,
    )]
    pub ignore_ci_checks: bool,

    #[arg(
        long,
        env = "MOON_ON_FAILURE",
        help = "When a task fails, either bail the pipeline, or continue executing",
        help_heading = super::HEADING_WORKFLOW,
        default_value_t,
    )]
    pub on_failure: OnFailure,

    #[arg(
        long,
        help = "Filter tasks based on the result of a query",
        help_heading = super::HEADING_WORKFLOW,
    )]
    pub query: Option<String>,

    // Skip persistent tasks when the targets were not explicitly requested,
    // and were determined by the command instead (like `moon ci` and `moon check`)
    #[arg(skip)]
    pub skip_persistent: bool,

    // Enabled internally by `moon query pre-exec` to reuse CI resolution.
    #[arg(skip)]
    pub emit_action_graph: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedActionGraph {
    schema_version: u8,
    requested_targets: Vec<String>,
    primary_targets: Vec<String>,
    actions: Vec<ResolvedAction>,
    edges: Vec<ResolvedEdge>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedAction {
    id: String,
    kind: &'static str,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

#[derive(Serialize)]
struct ResolvedEdge {
    from: String,
    to: String,
    #[serde(rename = "type")]
    kind: moon_config::TaskDependencyType,
}

#[instrument(skip(session))]
pub async fn exec(session: MoonSession, args: ExecArgs) -> SessionResult {
    let mut plan = ExecutionPlan::default();

    // Load the execution plan if provided
    if let Some(rel_plan_path) = &args.plan {
        let plan_path = if rel_plan_path.is_absolute() {
            rel_plan_path.to_path_buf()
        } else {
            session.working_dir.join(rel_plan_path)
        };

        debug!(plan = ?plan_path, "Received an execution plan, attempting to load");

        plan = json::read_file(plan_path)?;
    };

    // Inherit CLI targets if the plan does not provide them
    if !args.targets.is_empty() {
        if plan.targets.is_empty() {
            plan.targets = TargetsBlock::Included(args.targets.clone());
        } else {
            warn!(
                "Targets were passed on the command line, but the provided execution plan already defines a list of targets!"
            );
        }
    }

    // Otherwise prompt for a list of targets
    if plan.targets.is_empty() && !args.emit_action_graph {
        let workspace_graph = session.get_workspace_graph().await?;

        let targets = select_targets(&session.console, &[], || {
            Ok(SelectProps {
                label: "Which task(s) to run?".into(),
                options: workspace_graph
                    .get_tasks_unexpanded()?
                    .into_iter()
                    .map(|task| {
                        SelectOption::new(&task.target).description_opt(task.description.clone())
                    })
                    .collect(),
                multiple: true,
                ..Default::default()
            })
        })
        .await?;

        plan.targets =
            TargetsBlock::Included(targets.into_iter().map(TargetLocator::Qualified).collect());
    }

    let executor = ExecWorkflow::new(session, args, plan)?;
    let exit_code = executor.execute().await?;

    Ok(exit_code)
}

pub struct ExecWorkflow {
    args: ExecArgs,
    console: Console,
    plan: ExecutionPlan,
    session: MoonSession,

    last_title: String,
    summary: Option<Level>,
    ui: CiOutput,

    /// Whether we should run affected logic or not
    affected: bool,

    /// Whether we should apply `runInCI` checks
    ci_check: bool,

    /// Are we in a CI environment?
    ci_env: bool,

    /// The current step in the process
    step: u8,

    /// Task targets inserted into the graph
    targets: FxHashMap<NodeIndex, Target>,

    /// Are we in a test environment?
    test_env: bool,
}

impl ExecWorkflow {
    pub fn new(session: MoonSession, args: ExecArgs, plan: ExecutionPlan) -> miette::Result<Self> {
        Ok(Self {
            affected: args
                .affected
                .as_ref()
                .is_some_and(|affected| match affected {
                    Some(inner) => inner.is_enabled(),
                    None => true, // No arg value
                })
                || plan.affected.is_some(),
            summary: args
                .summary
                .clone()
                .map(|sum| sum.unwrap_or_default().to_level()),
            ci_check: !plan
                .pipeline
                .ignore_ci_checks
                .unwrap_or(args.ignore_ci_checks),
            ci_env: plan.pipeline.ci.or(args.ci).unwrap_or_else(is_ci),
            console: session.get_console()?,
            session,
            step: 0,
            targets: FxHashMap::default(),
            test_env: is_test_env(),
            args,
            ui: ci_env::get_output().unwrap_or(CiOutput {
                close_log_group: "",
                open_log_group: "▮▮▮▮ {name}",
            }),
            last_title: String::new(),
            plan,
        })
    }

    pub async fn execute(mut self) -> miette::Result<Option<u8>> {
        // Force cache to update using write-only mode. We don't disable
        // affected tracking here, otherwise `affectedFiles` would receive no
        // files when combined with `--affected --force`. Instead, forcing only
        // bypasses the affected *selection* filter (see `skip_affected` below),
        // so unaffected tasks still run while affected files are still passed.
        if self.args.force {
            self.session
                .get_cache_engine()?
                .force_mode(CacheMode::Write);
        }

        // Step 1
        let changed_files = self.load_changed_files().await?;

        // Step 2
        let (action_context, action_graph) = self.build_action_graph(changed_files).await?;

        // CI graph export stops before the action pipeline creates a report or
        // runs any action. An affected run with no targets has an empty graph
        // even if the builder inserted setup nodes.
        if self.args.emit_action_graph {
            self.emit_action_graph(if self.targets.is_empty() {
                None
            } else {
                Some(&action_graph)
            })?;

            return Ok(if self.targets.is_empty() && !self.affected {
                Some(1)
            } else {
                None
            });
        }

        if self.targets.is_empty() {
            let status = self.get_status();
            let targets = self.get_targets();

            let message = if self.affected {
                if status.is_empty() {
                    "No tasks affected by changed files.".to_string()
                } else {
                    format!(
                        "No tasks affected by changed files with status {}.",
                        status
                            .iter()
                            .map(|item| format!("<symbol>{item}</symbol>"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            } else {
                "No tasks found.".to_string()
            };

            self.console.render_err(element! {
                Container {
                    Notice(variant: Variant::Caution) {
                        StyledText(content: format!("{message} Unable to execute action pipeline."))

                        #(self.args.query.as_ref().map(|query| {
                            element! {
                                StyledText(content: format!("Using query <shell>{query}</shell>."))
                            }
                        }))

                        #((targets.len() < 15).then(|| {
                            element! {
                                StyledText(
                                    content: format!(
                                        "For targets {}.",
                                        targets
                                            .iter()
                                            .map(|target| format!("<id>{}</id>", target.as_str()))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )
                                )
                            }
                        }))
                    }
                }
            })?;

            return Ok(if self.affected { None } else { Some(1) });
        }

        // Step 3
        self.display_affected(&action_context)?;

        // Step 4
        let results = self
            .execute_action_pipeline(action_context, action_graph)
            .await?;

        let targets = self.get_targets();
        let is_single_target = targets.len() == 1
            && targets
                .first()
                .is_some_and(|target| target.is_fully_qualified());

        let mut any_failed = false;
        // Set when the pipeline was stopped by a signal (130 / 143).
        let mut exit_code = self.session.exit_code.get();

        for result in &results {
            if !result.has_failed() || result.allow_failure {
                continue;
            }

            any_failed = true;

            if is_single_target
                && exit_code.is_none()
                && let Some(code) = result.get_exit_code()
                && code != 0
            {
                exit_code = u8::try_from(code).ok();
            }
        }

        if let Some(code) = exit_code {
            self.session.exit_code.set(code);

            return Ok(Some(code));
        }

        if any_failed {
            return Ok(Some(1));
        }

        Ok(None)
    }

    // Step 1
    async fn load_changed_files(&mut self) -> miette::Result<FxHashSet<WorkspaceRelativePathBuf>> {
        self.print_step("Loading changed files")?;

        let vcs = self.session.get_vcs_adapter()?;

        if !vcs.is_enabled() {
            self.affected = false;

            debug!("VCS not enabled, skipping changed and affected checks");

            return Ok(FxHashSet::default());
        }

        let mut base = self.get_base();
        let mut head = self.get_head();

        // If we're in CI, extract PR information for base and head
        if self.ci_env
            && !self.test_env
            && (base.is_none() || head.is_none())
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

        self.print(format!(
            "Base revision: <symbol>{}</symbol>",
            base.as_deref().unwrap_or("N/A")
        ))?;

        self.print(format!(
            "Head revision: <symbol>{}</symbol>",
            head.as_deref().unwrap_or("HEAD")
        ))?;

        let status = self.get_status();
        let stdin = self.get_stdin();

        if self.affected {
            self.print(format!(
                "Affected by changes: {}",
                if status.is_empty() {
                    "<symbol>all</symbol>".to_owned()
                } else {
                    status
                        .iter()
                        .map(|item| format!("<symbol>{item}</symbol>"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ))?;
        }

        let mut options = QueryChangedFilesOptions {
            default_branch: !self.test_env,
            base,
            head,
            local: !self.ci_env,
            status: status.to_owned(),
            stdin,
        };

        if let Some(source) = self
            .plan
            .affected
            .as_ref()
            .and_then(|aff| aff.source.as_ref())
        {
            options.apply_affected(&AffectedOption::String(source.into()));
        } else if let Some(Some(by)) = &self.args.affected {
            options.apply_affected(by);
        }

        let result = query_changed_files(&vcs, options).await?;

        // Without this check, the newlines in the file list
        // will cause the message to break out of the tracing debug!
        if self.should_print() {
            let mut files = result
                .files
                .iter()
                .map(|file| format!("\t<file>{}</file>", file.as_str()))
                .collect::<Vec<_>>();
            files.sort();

            self.print("Changed files:")?;
            self.print(files.join("\n"))?;
        }

        if result.shallow {
            // if self.ci_env {
            //     return Err(AppError::CiNoShallowHistory.into());
            // } else {
            self.affected = false;
            // }
        }

        Ok(result.files)
    }

    // Step 2
    async fn build_action_graph(
        &mut self,
        changed_files: FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<(ActionContext, ActionGraph)> {
        self.print_step("Building action graph")?;

        let mut action_graph_builder = if self.get_no_actions() {
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
        action_graph_builder.set_changed_files(changed_files);

        // Only track affected if enabled
        let affected_depth = self.get_affected_depth();
        let dependents = self.get_dependents();
        let no_dependencies = self.get_no_dependencies();

        if self.affected {
            action_graph_builder.track_affected(affected_depth, self.ci_env && self.ci_check);
        }

        // Always sync workspace in CI
        if self.ci_env {
            action_graph_builder.sync_workspace();
        }

        // Insert targets into the graph
        let job = self.get_job();
        let job_total = self.get_job_total();

        action_graph_builder.set_requirements(RunRequirements {
            ci: self.ci_env,
            ci_check: self.ci_check,
            no_dependencies,
            dependents,
            interactive: self.args.interactive,
            job,
            job_total,
            // Forcing runs tasks even when not affected, but still
            // tracks affected files for the `affectedFiles` option.
            skip_affected: !self.affected || self.args.force,
            skip_persistent: self.args.skip_persistent,
        });

        let request = action_graph_builder.run_tasks_with_plan(&self.plan);

        // Build the graph
        let BuildOutput {
            context: action_context,
            graph: action_graph,
            mut report,
        } = action_graph_builder.build().await?;
        let partition = report.take_partition(request).unwrap_or_default();
        let targets = self.get_targets();

        if let Some(index) = &job
            && let Some(total) = &job_total
        {
            self.print(format!("Job index: <property>{index}</property>"))?;
            self.print(format!("Job total: <property>{total}</property>"))?;
            self.print(format!(
                "Partition size: <property>{}</property>",
                partition.size.unwrap_or_default()
            ))?;
        }

        if self.affected {
            self.print(format!(
                "Affected depth: <property>{affected_depth}</property>"
            ))?;
        }

        self.print(format!("Dependents: <property>{dependents}</property>"))?;

        if no_dependencies {
            self.print("Skipping dependencies: <property>yes</property>")?;
        }

        self.print(format!(
            "Action count: <property>{}</property>",
            action_graph.get_node_count()
        ))?;

        self.print(format!(
            "Requested targets: <mutedlight>{}</mutedlight>",
            targets.len()
        ))?;

        for target in targets {
            self.print(format!("\t<label>{}</label>", target.as_str()))?;
        }

        self.print(format!(
            "Resolved targets: <mutedlight>{}</mutedlight>",
            partition.targets.len()
        ))?;

        for (_, target) in &partition.targets {
            self.print(format!("\t<id>{}</id>", target.as_str()))?;
        }

        self.targets.extend(partition.targets);

        Ok((action_context, action_graph))
    }

    // Step 3
    fn display_affected(&mut self, context: &ActionContext) -> miette::Result<()> {
        let Some(affected) = &context.affected else {
            return Ok(());
        };

        self.print_step("Tracking affected tasks")?;

        for (target, state) in &affected.tasks {
            if !state.env.is_empty() {
                self.print(format!(
                    "\t<id>{target}</id> affected by environment variable <property>{}</property>",
                    state.env.iter().next().unwrap()
                ))?;
            } else if !state.files.is_empty() {
                self.print(format!(
                    "\t<id>{target}</id> affected by file <file>{}</file>",
                    state.files.iter().next().unwrap()
                ))?;
            } else if !state.dependencies.is_empty() {
                let dependency = state.dependencies.iter().next().unwrap();

                self.print(if state.distance > 1 {
                    format!(
                        "\t<id>{target}</id> affected by dependency task <label>{dependency}</label> (<property>{}</property> hops away)",
                        state.distance
                    )
                } else {
                    format!(
                        "\t<id>{target}</id> affected by dependency task <label>{dependency}</label>"
                    )
                })?;
            } else {
                self.print(format!("\t<id>{target}</id> affected"))?;
            }
        }

        Ok(())
    }

    // Step 4
    async fn execute_action_pipeline(
        &mut self,
        mut action_context: ActionContext,
        action_graph: ActionGraph,
    ) -> miette::Result<Vec<Action>> {
        self.print_step("Executing action pipeline")?;

        action_context.initial_targets.extend(
            self.get_targets()
                .into_iter()
                .map(|target| target.to_owned())
                .collect::<Vec<_>>(),
        );

        action_context.output_style = self.args.output_style.map(|style| style.to_output_style());
        action_context.passthrough_args = self.args.passthrough.clone();

        let results =
            run_action_pipeline(&self.session, action_context, action_graph, self.summary).await?;

        Ok(results)
    }

    fn should_print(&self) -> bool {
        !self.args.emit_action_graph
            && !self.test_env
            && !self.console.out.is_quiet()
            && self.summary.is_some_and(|level| level.is(Level::Three))
    }

    fn print_header(&mut self, title: &str) -> miette::Result<()> {
        self.last_title = title.to_owned();

        if self.should_print() {
            self.console
                .out
                .write_line(self.ui.open_log_group.replace("{name}", title))?;
        } else {
            debug!("Step {}: {title}", self.step);
        }

        Ok(())
    }

    fn print_footer(&mut self) -> miette::Result<()> {
        if self.should_print() && !self.ui.close_log_group.is_empty() {
            self.console
                .out
                .write_line(self.ui.close_log_group.replace("{name}", &self.last_title))?;
        }

        self.last_title = String::new();

        Ok(())
    }

    fn print_step(&mut self, message: &str) -> miette::Result<()> {
        if self.step > 0 {
            self.print_footer()?;
        }

        self.step += 1;
        self.print_header(message)?;

        Ok(())
    }

    fn print(&self, message: impl AsRef<str>) -> miette::Result<()> {
        let message = apply_style_tags(message.as_ref());

        if self.should_print() {
            self.console.out.write_line(message)?;
        } else {
            debug!("{message}");
        }

        Ok(())
    }

    fn get_base(&self) -> Option<String> {
        self.plan
            .affected
            .as_ref()
            .and_then(|aff| aff.base.clone())
            .or(self.args.base.clone())
    }

    fn get_affected_depth(&self) -> Depth {
        self.plan
            .affected
            .as_ref()
            .and_then(|aff| aff.depth)
            .or(self.args.affected_depth)
            .unwrap_or(Depth::Deep)
    }

    fn get_dependents(&self) -> Depth {
        self.plan
            .graph
            .dependents
            .or(self.args.dependents)
            .unwrap_or(Depth::NONE)
    }

    fn get_head(&self) -> Option<String> {
        self.plan
            .affected
            .as_ref()
            .and_then(|aff| aff.head.clone())
            .or(self.args.head.clone())
    }

    fn get_job(&self) -> Option<usize> {
        self.plan.pipeline.job.or(self.args.job)
    }

    fn get_job_total(&self) -> Option<usize> {
        self.plan.pipeline.job_total.or(self.args.job_total)
    }

    fn get_no_dependencies(&self) -> bool {
        self.plan
            .graph
            .no_dependencies
            .unwrap_or(self.args.no_dependencies)
    }

    fn get_no_actions(&self) -> bool {
        self.plan
            .pipeline
            .no_actions
            .unwrap_or(self.args.no_actions)
    }

    fn get_stdin(&self) -> bool {
        self.plan
            .affected
            .as_ref()
            .and_then(|aff| aff.stdin)
            .unwrap_or(self.args.stdin)
    }

    fn get_status(&self) -> &[ChangedStatus] {
        self.plan
            .affected
            .as_ref()
            .map(|aff| &aff.status)
            .unwrap_or(&self.args.status)
    }

    fn get_targets(&self) -> Vec<&TargetLocator> {
        match &self.plan.targets {
            TargetsBlock::Partitioned { jobs } => jobs.iter().flatten().collect(),
            TargetsBlock::Filtered { include } => Vec::from_iter(include),
            TargetsBlock::Included(targets) => Vec::from_iter(targets),
        }
    }

    fn emit_action_graph(&self, action_graph: Option<&ActionGraph>) -> miette::Result<()> {
        let mut requested_targets = self
            .get_targets()
            .into_iter()
            .map(|target| target.as_str().to_owned())
            .collect::<Vec<_>>();
        requested_targets.sort();
        requested_targets.dedup();

        let mut primary_targets = self
            .targets
            .values()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        primary_targets.sort();
        primary_targets.dedup();

        let mut report = ResolvedActionGraph {
            schema_version: 1,
            requested_targets,
            primary_targets,
            actions: vec![],
            edges: vec![],
        };

        if let Some(action_graph) = action_graph {
            // The pipeline performs this validation too, before dispatch.
            action_graph.sort_topological()?;

            let graph = action_graph.get_inner_graph();
            let mut nodes = graph
                .node_indices()
                .map(|index| (index, &graph[index]))
                .collect::<Vec<_>>();

            // Use the structure around actions with the same public identity
            // to assign repeatable graph-local IDs. Different task instances
            // can share a target but have different arguments or environment.
            // Neither is used to sort or exposed in the JSON.
            let ranks = Self::rank_action_nodes(graph);
            nodes.sort_by_cached_key(|(index, _)| ranks[index]);

            let mut ids = FxHashMap::default();

            for (position, (index, node)) in nodes.into_iter().enumerate() {
                let id = format!("action-{:04}", position + 1);
                ids.insert(index, id.clone());

                let (kind, target) = match node {
                    ActionNode::None => ("none", None),
                    ActionNode::InstallDependencies(_) => ("install-dependencies", None),
                    ActionNode::RunTask(inner) => ("run-task", Some(inner.target.to_string())),
                    ActionNode::SetupEnvironment(_) => ("setup-environment", None),
                    ActionNode::SetupProto(_) => ("setup-proto", None),
                    ActionNode::SetupToolchain(_) => ("setup-toolchain", None),
                    ActionNode::SyncProject(_) => ("sync-project", None),
                    ActionNode::SyncWorkspace => ("sync-workspace", None),
                };

                report.actions.push(ResolvedAction {
                    id,
                    kind,
                    label: node.label(),
                    target,
                });
            }

            for edge in graph.edge_references() {
                report.edges.push(ResolvedEdge {
                    from: ids[&edge.source()].clone(),
                    to: ids[&edge.target()].clone(),
                    kind: *edge.weight(),
                });
            }

            report.edges.sort_by_cached_key(|edge| {
                (
                    edge.from.clone(),
                    edge.to.clone(),
                    format!("{:?}", edge.kind),
                )
            });
        }

        self.console.out.write_line(json::format(&report, true)?)?;

        Ok(())
    }

    fn action_identity(node: &ActionNode) -> String {
        match node {
            ActionNode::None => String::new(),
            ActionNode::InstallDependencies(inner) => {
                let mut members = inner.members.clone().unwrap_or_default();
                members.sort();

                format!(
                    "{:?}",
                    (
                        &inner.toolchain_id,
                        &inner.root,
                        &inner.owner_project_id,
                        &inner.project_id,
                        members,
                    )
                )
            }
            ActionNode::RunTask(inner) => format!(
                "{:?}",
                (
                    &inner.target,
                    inner.persistent,
                    inner.interactive,
                    inner.priority,
                )
            ),
            ActionNode::SetupEnvironment(inner) => format!(
                "{:?}",
                (
                    &inner.toolchain_id,
                    &inner.root,
                    &inner.owner_project_id,
                    &inner.project_id,
                )
            ),
            ActionNode::SetupProto(inner) => inner.version.to_string(),
            ActionNode::SetupToolchain(inner) => inner.toolchain.target(),
            ActionNode::SyncProject(inner) => inner.project_id.to_string(),
            ActionNode::SyncWorkspace => String::new(),
        }
    }

    fn rank_action_nodes(graph: &ActionGraphType) -> FxHashMap<NodeIndex, usize> {
        let base_keys = graph
            .node_indices()
            .map(|index| {
                (
                    index,
                    (graph[index].label(), Self::action_identity(&graph[index])),
                )
            })
            .collect::<Vec<_>>();
        let mut unique_keys = base_keys
            .iter()
            .map(|(_, key)| key.clone())
            .collect::<Vec<_>>();
        unique_keys.sort();
        unique_keys.dedup();

        let mut ranks = FxHashMap::default();
        for (index, key) in base_keys {
            ranks.insert(index, unique_keys.binary_search(&key).unwrap());
        }

        if unique_keys.len() == graph.node_count() {
            return ranks;
        }

        // Each round splits equal-rank groups according to both their
        // dependencies and dependents. The old rank leads the sort key, so
        // groups only split, and convergence takes at most node_count rounds.
        for _ in 0..graph.node_count() {
            let signatures = graph
                .node_indices()
                .map(|index| {
                    let mut dependencies = graph
                        .edges_directed(index, Direction::Outgoing)
                        .map(|edge| (Self::edge_identity(edge.weight()), ranks[&edge.target()]))
                        .collect::<Vec<_>>();
                    dependencies.sort();

                    let mut dependents = graph
                        .edges_directed(index, Direction::Incoming)
                        .map(|edge| (Self::edge_identity(edge.weight()), ranks[&edge.source()]))
                        .collect::<Vec<_>>();
                    dependents.sort();

                    (index, (ranks[&index], dependencies, dependents))
                })
                .collect::<Vec<_>>();
            let mut unique_signatures = signatures
                .iter()
                .map(|(_, signature)| signature.clone())
                .collect::<Vec<_>>();
            unique_signatures.sort();
            unique_signatures.dedup();

            let mut next = FxHashMap::default();
            for (index, signature) in signatures {
                next.insert(index, unique_signatures.binary_search(&signature).unwrap());
            }

            if next == ranks {
                break;
            }
            ranks = next;
        }

        ranks
    }

    fn edge_identity(weight: &moon_config::TaskDependencyType) -> u8 {
        match weight {
            moon_config::TaskDependencyType::Cleanup => 0,
            moon_config::TaskDependencyType::Required => 1,
            moon_config::TaskDependencyType::Wait => 2,
            moon_config::TaskDependencyType::Optional => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_action::RunTaskNode;
    use moon_config::TaskDependencyType;

    fn graph_with_duplicate_target(
        reverse_insertion: bool,
    ) -> (ActionGraphType, NodeIndex, NodeIndex) {
        let shared = Target::parse("proj:shared").unwrap();
        let plain = ActionNode::run_task(RunTaskNode::new(shared.clone()));
        let mut configured = RunTaskNode::new(shared);
        configured.args.push("--force".into());
        configured.env.insert("TOKEN".into(), Some("secret".into()));
        let configured = ActionNode::run_task(configured);
        let parent = ActionNode::run_task(RunTaskNode::new(Target::parse("proj:parent").unwrap()));
        let prep = ActionNode::run_task(RunTaskNode::new(Target::parse("proj:prep").unwrap()));

        let mut graph = ActionGraphType::new();
        let (plain_index, configured_index, parent_index, prep_index) = if reverse_insertion {
            let prep_index = graph.add_node(prep);
            let configured_index = graph.add_node(configured);
            let plain_index = graph.add_node(plain);
            let parent_index = graph.add_node(parent);
            (plain_index, configured_index, parent_index, prep_index)
        } else {
            let parent_index = graph.add_node(parent);
            let plain_index = graph.add_node(plain);
            let configured_index = graph.add_node(configured);
            let prep_index = graph.add_node(prep);
            (plain_index, configured_index, parent_index, prep_index)
        };

        graph.add_edge(plain_index, prep_index, TaskDependencyType::Required);
        graph.add_edge(configured_index, prep_index, TaskDependencyType::Required);
        graph.add_edge(parent_index, configured_index, TaskDependencyType::Required);

        (graph, plain_index, configured_index)
    }

    #[test]
    fn ranks_duplicate_task_targets_by_graph_structure() {
        let (first, first_plain, first_configured) = graph_with_duplicate_target(false);
        let first_ranks = ExecWorkflow::rank_action_nodes(&first);
        let (second, second_plain, second_configured) = graph_with_duplicate_target(true);
        let second_ranks = ExecWorkflow::rank_action_nodes(&second);

        assert_ne!(first_ranks[&first_plain], first_ranks[&first_configured]);
        assert_eq!(first_ranks[&first_plain], second_ranks[&second_plain]);
        assert_eq!(
            first_ranks[&first_configured],
            second_ranks[&second_configured]
        );
    }
}
