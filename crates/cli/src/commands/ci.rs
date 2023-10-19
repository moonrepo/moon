use crate::app::GlobalArgs;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use ci_env::CiOutput;
use clap::Args;
use itertools::Itertools;
use moon::{build_action_graph, generate_project_graph};
use moon_action_context::ActionContext;
use moon_action_graph::{ActionGraph, RunRequirements};
use moon_action_pipeline::Pipeline;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_terminal::safe_exit;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use starbase::{system, AppResult};
use starbase_styles::color;
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

fn print_header(provider: &CiOutput, title: &str) {
    println!("{}{}", provider.open_log_group, title);
}

fn print_footer(provider: &CiOutput) {
    if !provider.close_log_group.is_empty() {
        println!("{}", provider.close_log_group);
    }
}

fn print_targets(targets: &TargetList) {
    let mut targets_to_print = targets.clone();
    targets_to_print.sort();

    println!(
        "{}",
        targets_to_print
            .iter()
            .map(|t| format!("  {}", color::label(&t.id)))
            .join("\n")
    );
}

/// Gather a list of files that have been modified between branches.
async fn gather_touched_files(
    provider: &CiOutput,
    workspace: &Workspace,
    args: &CiArgs,
) -> AppResult<FxHashSet<WorkspaceRelativePathBuf>> {
    print_header(provider, "Gathering touched files");

    let results = query_touched_files(
        workspace,
        &QueryTouchedFilesOptions {
            default_branch: true,
            base: args.base.clone(),
            head: args.head.clone(),
            log: true,
            ..QueryTouchedFilesOptions::default()
        },
    )
    .await?;

    print_footer(provider);

    Ok(results)
}

/// Gather runnable targets by checking if all projects/tasks are affected based on touched files.
fn gather_runnable_targets(
    provider: &CiOutput,
    project_graph: &ProjectGraph,
    args: &CiArgs,
) -> AppResult<TargetList> {
    print_header(provider, "Gathering runnable targets");

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

    print_targets(&targets);
    print_footer(provider);

    Ok(targets)
}

/// Distribute targets across jobs if parallelism is enabled.
fn distribute_targets_across_jobs(
    provider: &CiOutput,
    args: &CiArgs,
    targets: TargetList,
) -> TargetList {
    if args.job.is_none() || args.job_total.is_none() {
        return targets;
    }

    let job_index = args.job.unwrap_or_default();
    let job_total = args.job_total.unwrap_or_default();
    let batch_size = targets.len() / job_total;
    let batched_targets;

    print_header(provider, "Distributing targets across jobs");
    println!("Job index: {job_index}");
    println!("Job total: {job_index}");
    println!("Batch size: {batch_size}");
    println!("Batched targets:");

    if job_index == 0 {
        batched_targets = targets[0..batch_size].to_vec();
    } else if job_index == job_total - 1 {
        batched_targets = targets[(batch_size * job_index)..].to_vec();
    } else {
        batched_targets =
            targets[(batch_size * job_index)..(batch_size * (job_index + 1))].to_vec();
    }

    print_targets(&batched_targets);
    print_footer(provider);

    batched_targets
}

/// Generate a dependency graph with the runnable targets.
fn generate_action_graph(
    provider: &CiOutput,
    project_graph: &ProjectGraph,
    targets: &TargetList,
    touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> AppResult<ActionGraph> {
    print_header(provider, "Generating action graph");

    let mut action_graph_builder = build_action_graph(project_graph)?;

    // Run dependents to ensure consumers still work correctly
    let requirements = RunRequirements {
        dependents: true,
        touched_files: Some(touched_files),
        ..Default::default()
    };

    for target in targets {
        // Run the target and its dependencies
        action_graph_builder.run_task_by_target(target, &requirements)?;
    }

    let action_graph = action_graph_builder.build()?;

    println!("Target count: {}", targets.len());
    println!("Action count: {}", action_graph.get_node_count());
    print_footer(provider);

    Ok(action_graph)
}

#[system]
pub async fn ci(
    args: ArgsRef<CiArgs>,
    global_args: StateRef<GlobalArgs>,
    workspace: ResourceMut<Workspace>,
) {
    let ci_provider = ci_env::get_output().unwrap_or(CiOutput {
        close_log_group: "",
        open_log_group: "▪▪▪▪ ",
    });
    let project_graph = generate_project_graph(workspace).await?;
    let touched_files = gather_touched_files(&ci_provider, workspace, args).await?;
    let targets = gather_runnable_targets(&ci_provider, &project_graph, args)?;

    if targets.is_empty() {
        println!("{}", color::invalid("No targets to run"));

        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&ci_provider, args, targets);
    let action_graph =
        generate_action_graph(&ci_provider, &project_graph, &targets, &touched_files)?;

    if action_graph.is_empty() {
        println!(
            "{}",
            color::invalid("No targets to run based on touched files")
        );

        return Ok(());
    }

    // Process all tasks in the graph
    print_header(&ci_provider, "Running targets");

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
        .run(action_graph, Some(context))
        .await?;

    print_footer(&ci_provider);

    print_header(&ci_provider, "Summary");

    pipeline.render_summary(&results)?;

    // Print out the results and exit if an error occurs
    print_header(&ci_provider, "Stats");

    let failed = pipeline.render_results(&results)?;

    pipeline.render_stats(&results, false)?;

    if failed {
        safe_exit(1);
    }
}
