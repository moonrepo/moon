use crate::app_error::{AppError, ExitCode};
use crate::components::run_action_pipeline;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use crate::session::CliSession;
use ci_env::CiOutput;
use clap::Args;
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, RunRequirements};
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_console::Console;
use moon_project_graph::ProjectGraph;
use moon_task::{Target, TargetLocator};
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;
use std::sync::Arc;
use tracing::instrument;

type TargetList = Vec<Target>;

const HEADING_PARALLELISM: &str = "Parallelism and distribution";

#[derive(Args, Clone, Debug)]
pub struct CiArgs {
    #[arg(help = "List of targets (scope:task) to run")]
    targets: Vec<Target>,

    #[arg(long, help = "Base branch, commit, or revision to compare against")]
    base: Option<String>,

    #[arg(long, help = "Current branch, commit, or revision to compare with")]
    head: Option<String>,

    #[arg(long, help = "Index of the current job", help_heading = HEADING_PARALLELISM)]
    job: Option<usize>,

    #[arg(long = "jobTotal", help = "Total amount of jobs to run", help_heading = HEADING_PARALLELISM)]
    job_total: Option<usize>,
}

struct CiConsole {
    inner: Arc<Console>,
    output: CiOutput,
    last_title: String,
}

impl CiConsole {
    pub fn write_line<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        self.inner.out.write_line(data)
    }

    pub fn print_header(&mut self, title: &str) -> miette::Result<()> {
        self.last_title = title.to_owned();
        self.write_line(self.output.open_log_group.replace("{name}", title))
    }

    pub fn print_footer(&mut self) -> miette::Result<()> {
        if !self.output.close_log_group.is_empty() {
            self.write_line(
                self.output
                    .close_log_group
                    .replace("{name}", &self.last_title),
            )?;
        }

        self.last_title = String::new();

        Ok(())
    }

    pub fn print_targets(&self, targets: &TargetList) -> miette::Result<()> {
        let mut targets_to_print = targets.clone();
        targets_to_print.sort();

        self.write_line(
            targets_to_print
                .iter()
                .map(|t| format!("  {}", color::label(&t.id)))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

async fn generate_project_graph(session: &CliSession) -> AppResult<Arc<ProjectGraph>> {
    session.get_project_graph().await
}

/// Gather a list of files that have been modified between branches.
async fn gather_touched_files(
    console: &mut CiConsole,
    session: &CliSession,
    args: &CiArgs,
) -> AppResult<FxHashSet<WorkspaceRelativePathBuf>> {
    console.print_header("Gathering touched files")?;

    let mut base = args.base.clone();
    let mut head = args.head.clone();

    if let Some(env) = ci_env::get_environment() {
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

    let vcs = session.get_vcs_adapter()?;
    let result = query_touched_files(
        &vcs,
        &QueryTouchedFilesOptions {
            default_branch: true,
            base,
            head,
            ..QueryTouchedFilesOptions::default()
        },
    )
    .await?;

    if result.shallow {
        return Err(AppError::CiNoShallowHistory.into());
    }

    let mut files = result
        .files
        .iter()
        .map(|f| format!("  {}", color::file(f.as_str())))
        .collect::<Vec<_>>();
    files.sort();

    console.write_line(files.join("\n"))?;
    console.print_footer()?;

    Ok(result.files)
}

/// Gather potential runnable targets.
async fn gather_potential_targets(
    console: &mut CiConsole,
    project_graph: &ProjectGraph,
    args: &CiArgs,
) -> AppResult<TargetList> {
    console.print_header("Gathering potential targets")?;

    let mut targets = vec![];

    // Required for dependents
    let projects = project_graph.get_all()?;

    if args.targets.is_empty() {
        for project in projects {
            for task in project.get_tasks()? {
                targets.push(task.target.clone());
            }
        }
    } else {
        targets.extend(args.targets.clone());
    }

    console.print_targets(&targets)?;
    console.print_footer()?;

    Ok(targets)
}

/// Distribute targets across jobs if parallelism is enabled.
fn distribute_targets_across_jobs(
    console: &mut CiConsole,
    args: &CiArgs,
    targets: TargetList,
) -> AppResult<TargetList> {
    if args.job.is_none() || args.job_total.is_none() {
        return Ok(targets);
    }

    let job_index = args.job.unwrap_or_default();
    let job_total = args.job_total.unwrap_or_default();
    let batch_size = targets.len() / job_total;
    let batched_targets;

    console.print_header("Distributing targets across jobs")?;
    console.write_line(format!("Job index: {job_index}"))?;
    console.write_line(format!("Job total: {job_index}"))?;
    console.write_line(format!("Batch size: {batch_size}"))?;
    console.write_line("Batched targets:")?;

    if job_index == 0 {
        batched_targets = targets[0..batch_size].to_vec();
    } else if job_index == job_total - 1 {
        batched_targets = targets[(batch_size * job_index)..].to_vec();
    } else {
        batched_targets =
            targets[(batch_size * job_index)..(batch_size * (job_index + 1))].to_vec();
    }

    console.print_targets(&batched_targets)?;
    console.print_footer()?;

    Ok(batched_targets)
}

/// Generate a dependency graph with the runnable targets.
async fn generate_action_graph(
    console: &mut CiConsole,
    session: &CliSession,
    project_graph: &ProjectGraph,
    targets: &TargetList,
    touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> AppResult<(ActionGraph, ActionContext)> {
    console.print_header("Generating action graph")?;

    let mut action_graph_builder = session.build_action_graph(project_graph).await?;
    action_graph_builder.set_touched_files(touched_files)?;
    action_graph_builder.set_affected_scopes(UpstreamScope::Direct, DownstreamScope::Deep)?;

    // Run dependents to ensure consumers still work correctly
    action_graph_builder.run_from_requirements(RunRequirements {
        ci: true,
        ci_check: true,
        dependents: true,
        target_locators: FxHashSet::from_iter(
            targets
                .iter()
                .map(|target| TargetLocator::Qualified(target.to_owned())),
        ),
        ..Default::default()
    })?;

    let action_context = action_graph_builder.build_context();
    let action_graph = action_graph_builder.build();

    console.write_line(format!("Target count: {}", targets.len()))?;
    console.write_line(format!("Action count: {}", action_graph.get_node_count()))?;
    console.print_footer()?;

    Ok((action_graph, action_context))
}

#[instrument(skip_all)]
pub async fn ci(session: CliSession, args: CiArgs) -> AppResult {
    let mut console = CiConsole {
        inner: session.get_console()?,
        output: ci_env::get_output().unwrap_or(CiOutput {
            close_log_group: "",
            open_log_group: "▪▪▪▪ {name}",
        }),
        last_title: String::new(),
    };

    let project_graph = generate_project_graph(&session).await?;
    let touched_files = gather_touched_files(&mut console, &session, &args).await?;
    let targets = gather_potential_targets(&mut console, &project_graph, &args).await?;

    if targets.is_empty() {
        console.write_line(color::invalid("No tasks to run"))?;

        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&mut console, &args, targets)?;
    let (action_graph, action_context) = generate_action_graph(
        &mut console,
        &session,
        &project_graph,
        &targets,
        &touched_files,
    )
    .await?;

    if action_graph.is_empty() {
        console.write_line(color::invalid("No tasks affected based on touched files"))?;

        return Ok(());
    }

    // Process all tasks in the graph
    console.print_header("Running tasks")?;

    let results = run_action_pipeline(&session, action_context, action_graph).await?;

    console.print_footer()?;

    let failed = results.iter().any(|result| {
        if result.has_failed() {
            !result.allow_failure
        } else {
            false
        }
    });

    if failed {
        return Err(ExitCode(1).into());
    }

    Ok(())
}
