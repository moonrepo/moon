use crate::app::GlobalArgs;
use crate::app_error::{AppError, ExitCode};
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use ci_env::CiOutput;
use clap::Args;
use itertools::Itertools;
use moon::{build_action_graph, generate_project_graph};
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, RunRequirements};
use moon_action_pipeline::Pipeline;
use moon_app_components::Console;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use starbase::{system, AppResult};
use starbase_styles::color;
use std::sync::Arc;
use tracing::debug;

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

struct CiConsole<'ci> {
    inner: &'ci Console,
    output: CiOutput,
}

impl<'ci> CiConsole<'ci> {
    pub fn write_line<T: AsRef<[u8]>>(&self, data: T) -> miette::Result<()> {
        self.inner.out.write_line(data)
    }

    pub fn print_header(&self, title: &str) -> miette::Result<()> {
        self.write_line(format!("{}{}", self.output.open_log_group, title))
    }

    pub fn print_footer(&self) -> miette::Result<()> {
        if !self.output.close_log_group.is_empty() {
            self.write_line(self.output.close_log_group)?;
        }

        Ok(())
    }

    pub fn print_targets(&self, targets: &TargetList) -> miette::Result<()> {
        let mut targets_to_print = targets.clone();
        targets_to_print.sort();

        self.write_line(
            targets_to_print
                .iter()
                .map(|t| format!("  {}", color::label(&t.id)))
                .join("\n"),
        )
    }
}

/// Gather a list of files that have been modified between branches.
async fn gather_touched_files(
    console: &CiConsole<'_>,
    workspace: &Workspace,
    args: &CiArgs,
) -> AppResult<FxHashSet<WorkspaceRelativePathBuf>> {
    console.print_header("Gathering touched files")?;

    let result = query_touched_files(
        workspace,
        &QueryTouchedFilesOptions {
            default_branch: true,
            base: args.base.clone(),
            head: args.head.clone(),
            ..QueryTouchedFilesOptions::default()
        },
    )
    .await?;

    if result.shallow {
        return Err(AppError::CiNoShallowHistory.into());
    }

    console.write_line(
        result
            .files
            .iter()
            .map(|f| color::file(f.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
    )?;

    console.print_footer()?;

    Ok(result.files)
}

/// Gather runnable targets by checking if all projects/tasks are affected based on touched files.
fn gather_runnable_targets(
    console: &CiConsole<'_>,
    project_graph: &ProjectGraph,
    args: &CiArgs,
) -> AppResult<TargetList> {
    console.print_header("Gathering runnable targets")?;

    let mut targets = vec![];

    // Required for dependents
    let projects = project_graph.get_all()?;

    if args.targets.is_empty() {
        for project in projects {
            for task in project.get_tasks()? {
                if task.should_run_in_ci() {
                    targets.push(task.target.clone());
                } else {
                    debug!(
                        "Not running target {} because it either has no {} or {} is false",
                        color::label(&task.target.id),
                        color::property("outputs"),
                        color::property("runInCI"),
                    );
                }
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
    console: &CiConsole<'_>,
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
fn generate_action_graph(
    console: &CiConsole<'_>,
    project_graph: &ProjectGraph,
    targets: &TargetList,
    touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> AppResult<ActionGraph> {
    console.print_header("Generating action graph")?;

    let mut action_graph_builder = build_action_graph(project_graph)?;

    // Run dependents to ensure consumers still work correctly
    let requirements = RunRequirements {
        ci: true,
        dependents: true,
        touched_files: Some(touched_files),
        ..Default::default()
    };

    for target in targets {
        // Run the target and its dependencies
        action_graph_builder.run_task_by_target(target, &requirements)?;
    }

    let action_graph = action_graph_builder.build()?;

    console.write_line(format!("Target count: {}", targets.len()))?;
    console.write_line(format!("Action count: {}", action_graph.get_node_count()))?;
    console.print_footer()?;

    Ok(action_graph)
}

#[system]
pub async fn ci(args: ArgsRef<CiArgs>, global_args: StateRef<GlobalArgs>, resources: ResourcesMut) {
    let project_graph = { generate_project_graph(resources.get_mut::<Workspace>()).await? };
    let workspace = resources.get::<Workspace>();
    let console = CiConsole {
        inner: resources.get::<Console>(),
        output: ci_env::get_output().unwrap_or(CiOutput {
            close_log_group: "",
            open_log_group: "▪▪▪▪ ",
        }),
    };

    let touched_files = gather_touched_files(&console, workspace, args).await?;
    let targets = gather_runnable_targets(&console, &project_graph, args)?;

    if targets.is_empty() {
        console.write_line(color::invalid("No targets to run"))?;

        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&console, args, targets)?;
    let action_graph = generate_action_graph(&console, &project_graph, &targets, &touched_files)?;

    if action_graph.is_empty() {
        console.write_line(color::invalid("No targets to run based on touched files"))?;

        return Ok(());
    }

    // Process all tasks in the graph
    console.print_header("Running targets")?;

    let context = ActionContext {
        primary_targets: FxHashSet::from_iter(targets),
        touched_files,
        workspace_root: workspace.root.clone(),
        ..ActionContext::default()
    };

    let mut pipeline = Pipeline::new(workspace.to_owned(), project_graph);

    if let Some(concurrency) = &global_args.concurrency {
        pipeline.concurrency(*concurrency);
    }

    let results = pipeline
        .generate_report("ciReport.json")
        .run(
            action_graph,
            Arc::new(resources.get::<Console>().to_owned()),
            Some(context),
        )
        .await?;

    console.print_footer()?;

    // Print out a summary of any failures
    console.print_header("Summary")?;

    pipeline.render_summary(&results, console.inner)?;

    console.print_footer()?;

    // Print out the results and exit if an error occurs
    console.print_header("Stats")?;

    let failed = pipeline.render_results(&results, console.inner)?;

    pipeline.render_stats(&results, console.inner, false)?;

    console.print_footer()?;

    if failed {
        return Err(ExitCode(1).into());
    }
}
