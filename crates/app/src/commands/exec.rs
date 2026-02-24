// use crate::app_error::AppError;
use crate::helpers::run_action_pipeline;
use crate::prompts::select_targets;
use crate::queries::changed_files::{QueryChangedFilesOptions, query_changed_files};
use crate::session::MoonSession;
use ci_env::CiOutput;
use clap::{Args, ValueEnum};
use iocraft::prelude::element;
use moon_action::Action;
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, ActionGraphBuilderOptions, RunRequirements};
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_app_macros::{with_affected_args, with_shared_exec_args};
use moon_cache::CacheMode;
use moon_common::{apply_style_tags, is_ci, is_test_env, path::WorkspaceRelativePathBuf};
use moon_console::ui::{Container, Notice, SelectOption, SelectProps, StyledText, Variant};
use moon_console::{Console, Level};
use moon_task::{Target, TargetLocator};
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase::AppResult;
use std::fmt;
use std::sync::Arc;
use tracing::{debug, instrument};

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
}

#[instrument(skip(session))]
pub async fn exec(session: MoonSession, mut args: ExecArgs) -> AppResult {
    if args.targets.is_empty() {
        let workspace_graph = session.get_workspace_graph().await?;
        let tasks = workspace_graph.get_tasks()?;

        let targets = select_targets(&session.console, &[], || {
            Ok(SelectProps {
                label: "Which task(s) to run?".into(),
                options: tasks
                    .iter()
                    .map(|task| {
                        SelectOption::new(&task.target).description_opt(task.description.clone())
                    })
                    .collect(),
                multiple: true,
                ..Default::default()
            })
        })
        .await?;

        for target in targets {
            args.targets.push(TargetLocator::Qualified(target));
        }
    }

    let executor = ExecWorkflow::new(session, args)?;
    let exit_code = executor.execute().await?;

    Ok(exit_code)
}

pub struct ExecWorkflow {
    args: ExecArgs,
    console: Arc<Console>,
    session: MoonSession,

    last_title: String,
    summary: Level,
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
    pub fn new(session: MoonSession, args: ExecArgs) -> miette::Result<Self> {
        Ok(Self {
            affected: args
                .affected
                .as_ref()
                .is_some_and(|affected| match affected {
                    Some(inner) => inner.is_enabled(),
                    None => true, // No arg value
                }),
            summary: args
                .summary
                .clone()
                .map(|sum| sum.unwrap_or_default())
                .unwrap_or_default()
                .to_level(),
            ci_check: !args.ignore_ci_checks,
            ci_env: args.ci.unwrap_or(is_ci()),
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
        })
    }

    pub async fn execute(mut self) -> miette::Result<Option<u8>> {
        // Force cache to update using write-only mode
        if self.args.force {
            self.affected = false;
            self.session
                .get_cache_engine()?
                .force_mode(CacheMode::Write);
        }

        // Step 1
        let changed_files = self.load_changed_files().await?;

        // Step 2
        let (action_context, action_graph) = self.build_action_graph(changed_files).await?;

        if self.targets.is_empty() {
            let message = if self.affected {
                if self.args.status.is_empty() {
                    "No tasks affected by changed files.".to_string()
                } else {
                    format!(
                        "No tasks affected by changed files with status {}.",
                        self.args
                            .status
                            .iter()
                            .map(|status| format!("<symbol>{status}</symbol>"))
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

                        #((self.args.targets.len() < 15).then(|| {
                            element! {
                                StyledText(
                                    content: format!(
                                        "For targets {}.",
                                        self
                                            .args
                                            .targets
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

        let failed = results.into_iter().any(|result| {
            if result.has_failed() {
                !result.allow_failure
            } else {
                false
            }
        });

        if failed {
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

        let mut base = self.args.base.clone();
        let mut head = self.args.head.clone();

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

        if self.affected {
            self.print(format!(
                "Affected by changes: {}",
                if self.args.status.is_empty() {
                    "<symbol>all</symbol>".to_owned()
                } else {
                    self.args
                        .status
                        .iter()
                        .map(|status| format!("<symbol>{status}</symbol>"))
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
            status: self.args.status.clone(),
            stdin: self.args.stdin,
        };

        if let Some(Some(by)) = &self.args.affected {
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
                self.args.upstream.unwrap_or(UpstreamScope::Deep),
                self.args.downstream.unwrap_or(DownstreamScope::None),
                self.ci_env && self.ci_check,
            )?;
        }

        // Always sync workspace in CI
        if self.ci_env {
            action_graph_builder.sync_workspace().await?;
        }

        // Insert targets into the graph
        let partition = action_graph_builder
            .run_tasks(
                &self.args.targets,
                RunRequirements {
                    ci: self.ci_env,
                    ci_check: self.ci_check,
                    dependencies: self.args.upstream.unwrap_or(UpstreamScope::Deep),
                    dependents: self.args.downstream.unwrap_or(DownstreamScope::None),
                    include_relations: self.args.include_relations,
                    interactive: self.args.interactive,
                    job: self.args.job,
                    job_total: self.args.job_total,
                    skip_affected: !self.affected,
                },
            )
            .await?;

        // Build the graph
        let (action_context, action_graph) = action_graph_builder.build();

        if let Some(job_index) = &self.args.job
            && let Some(job_total) = &self.args.job_total
        {
            self.print(format!("Job index: <property>{job_index}</property>"))?;
            self.print(format!("Job total: <property>{job_total}</property>"))?;
            self.print(format!(
                "Partition size: <property>{}</property>",
                partition.size.unwrap_or_default()
            ))?;
        }

        self.print(format!(
            "Action count: <property>{}</property>",
            action_graph.get_node_count()
        ))?;

        self.print(format!(
            "Requested targets: <mutedlight>{}</mutedlight>",
            self.args.targets.len()
        ))?;

        for target in &self.args.targets {
            self.print(format!("\t<label>{}</label>", target.as_str()))?;
        }

        self.print(format!(
            "Resolved targets: <mutedlight>{}</mutedlight>",
            partition.targets.len()
        ))?;

        for target in partition.targets.values() {
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
            } else if !state.projects.is_empty() {
                self.print(format!(
                    "\t<id>{target}</id> affected by project <id>{}</id>",
                    state.projects.iter().next().unwrap()
                ))?;
            } else if !state.upstream.is_empty() {
                if self.args.include_relations {
                    self.print(format!(
                        "\t<id>{target}</id> affected by dependency task <label>{}</label>",
                        state.upstream.iter().next().unwrap()
                    ))?;
                }
            } else if !state.downstream.is_empty() {
                if self.args.include_relations {
                    self.print(format!(
                        "\t<id>{target}</id> affected by dependent task <label>{}</label>",
                        state.downstream.iter().next().unwrap()
                    ))?;
                }
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

        action_context
            .initial_targets
            .extend(self.args.targets.clone());
        action_context.passthrough_args = self.args.passthrough.clone();

        let results = run_action_pipeline(&self.session, action_context, action_graph).await?;

        Ok(results)
    }

    fn should_print(&self) -> bool {
        !self.console.out.is_quiet() && self.summary.is(Level::Three) && !self.test_env
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
}
