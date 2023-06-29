use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use ci_env::CiOutput;
use itertools::Itertools;
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_action_context::ActionContext;
use moon_action_pipeline::Pipeline;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_dep_graph::DepGraph;
use moon_logger::debug;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_terminal::safe_exit;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;

type TargetList = Vec<Target>;

const LOG_TARGET: &str = "moon:ci";

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
    options: &CiOptions,
) -> AppResult<FxHashSet<WorkspaceRelativePathBuf>> {
    print_header(provider, "Gathering touched files");

    let results = query_touched_files(
        workspace,
        &mut QueryTouchedFilesOptions {
            default_branch: true,
            base: options.base.clone().unwrap_or_default(),
            head: options.head.clone().unwrap_or_default(),
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
    touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
) -> AppResult<TargetList> {
    print_header(provider, "Gathering runnable targets");

    let mut targets = vec![];

    // Required for dependents
    for project in project_graph.get_all()? {
        for task in project.tasks.values() {
            if task.should_run_in_ci() {
                if task.is_affected(touched_files)? {
                    targets.push(task.target.clone());
                }
            } else {
                debug!(
                    target: LOG_TARGET,
                    "Not running target {} because it either has no `outputs` or `runInCI` is false",
                    color::label(&task.target.id),
                );
            }
        }
    }

    if targets.is_empty() {
        println!(
            "{}",
            color::invalid("No targets to run based on touched files")
        );
    } else {
        print_targets(&targets);
    }

    print_footer(provider);

    Ok(targets)
}

/// Distribute targets across jobs if parallelism is enabled.
fn distribute_targets_across_jobs(
    provider: &CiOutput,
    options: &CiOptions,
    targets: TargetList,
) -> TargetList {
    if options.job.is_none() || options.job_total.is_none() {
        return targets;
    }

    let job_index = options.job.unwrap_or_default();
    let job_total = options.job_total.unwrap_or_default();
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
fn generate_dep_graph(
    provider: &CiOutput,
    workspace: &Workspace,
    project_graph: &ProjectGraph,
    targets: &TargetList,
) -> AppResult<DepGraph> {
    print_header(provider, "Generating dependency graph");

    let mut dep_builder = build_dep_graph(workspace, project_graph);

    for target in targets {
        // Run the target and its dependencies
        dep_builder.run_target(target, None)?;

        // And also run its dependents to ensure consumers still work correctly
        dep_builder.run_dependents_for_target(target)?;
    }

    let dep_graph = dep_builder.build();

    println!("Target count: {}", targets.len());
    println!("Action count: {}", dep_graph.get_node_count());
    print_footer(provider);

    Ok(dep_graph)
}

pub struct CiOptions {
    pub base: Option<String>,
    pub concurrency: Option<usize>,
    pub head: Option<String>,
    pub job: Option<usize>,
    pub job_total: Option<usize>,
}

pub async fn ci(options: CiOptions) -> AppResult {
    let mut workspace = load_workspace().await?;
    let ci_provider = ci_env::get_output().unwrap_or(CiOutput {
        close_log_group: "",
        open_log_group: "▪▪▪▪ ",
    });
    let project_graph = generate_project_graph(&mut workspace).await?;
    let touched_files = gather_touched_files(&ci_provider, &workspace, &options).await?;
    let targets = gather_runnable_targets(&ci_provider, &project_graph, &touched_files)?;

    if targets.is_empty() {
        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&ci_provider, &options, targets);
    let dep_graph = generate_dep_graph(&ci_provider, &workspace, &project_graph, &targets)?;

    // Process all tasks in the graph
    print_header(&ci_provider, "Running all targets");

    let context = ActionContext {
        primary_targets: FxHashSet::from_iter(targets),
        touched_files,
        workspace_root: workspace.root.clone(),
        ..ActionContext::default()
    };

    let mut pipeline = Pipeline::new(workspace, project_graph);

    if let Some(concurrency) = options.concurrency {
        pipeline.concurrency(concurrency);
    }

    let results = pipeline
        .generate_report("ciReport.json")
        .run(dep_graph, Some(context))
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

    Ok(())
}
