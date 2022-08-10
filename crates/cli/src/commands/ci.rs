use crate::enums::TouchedStatus;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use itertools::Itertools;
use moon_action::ActionContext;
use moon_action_runner::{ActionRunner, DepGraph, DepGraphError};
use moon_logger::{color, debug};
use moon_project::ProjectError;
use moon_task::{Target, TouchedFilePaths};
use moon_terminal::safe_exit;
use moon_utils::is_ci;
use moon_workspace::{Workspace, WorkspaceError};

type TargetList = Vec<Target>;

const LOG_TARGET: &str = "moon:ci";

fn print_header(title: &str) {
    let prefix = if is_ci() { "--- " } else { "" };

    println!("{}{}", prefix, title);
}

fn print_targets(targets: &TargetList) {
    let mut targets_to_print = targets.clone();
    targets_to_print.sort();

    println!(
        "{}",
        targets_to_print
            .iter()
            .map(|t| format!("  {}", color::target(&t.id)))
            .join("\n")
    );
}

/// Gather a list of files that have been modified between branches.
async fn gather_touched_files(
    workspace: &Workspace,
    options: &CiOptions,
) -> Result<TouchedFilePaths, WorkspaceError> {
    print_header("Gathering touched files");

    query_touched_files(
        workspace,
        &mut QueryTouchedFilesOptions {
            default_branch: true,
            base: options.base.clone().unwrap_or_default(),
            head: options.head.clone().unwrap_or_default(),
            local: false,
            log: true,
            status: TouchedStatus::All,
        },
    )
    .await
}

/// Gather runnable targets by checking if all projects/tasks are affected based on touched files.
fn gather_runnable_targets(
    workspace: &Workspace,
    touched_files: &TouchedFilePaths,
) -> Result<TargetList, ProjectError> {
    print_header("Gathering runnable targets");

    let mut targets = vec![];

    // Required for dependents
    workspace.projects.load_all()?;

    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;

        for (task_id, task) in &project.tasks {
            let target = Target::new(&project.id, task_id)?;

            if task.should_run_in_ci() {
                if task.is_affected(touched_files)? {
                    targets.push(target);
                }
            } else {
                debug!(
                    target: LOG_TARGET,
                    "Not running target {} because it either has no `outputs` or `runInCI` is false",
                    color::target(&target.id),
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

    Ok(targets)
}

/// Distribute targets across jobs if parallelism is enabled.
fn distribute_targets_across_jobs(options: &CiOptions, targets: TargetList) -> TargetList {
    if options.job.is_none() || options.job_total.is_none() {
        return targets;
    }

    let job_index = options.job.unwrap();
    let job_total = options.job_total.unwrap();
    let batch_size = targets.len() / job_total;
    let batched_targets;

    print_header("Distributing targets across jobs");
    println!("Job index: {}", job_index);
    println!("Job total: {}", job_index);
    println!("Batch size: {}", batch_size);
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

    batched_targets
}

/// Generate a dependency graph with the runnable targets.
fn generate_dep_graph(
    workspace: &Workspace,
    targets: &TargetList,
) -> Result<DepGraph, DepGraphError> {
    print_header("Generating dependency graph");

    let mut dep_graph = DepGraph::default();

    for target in targets {
        // Run the target and its dependencies
        dep_graph.run_target(target, &workspace.projects, None)?;

        // And also run its dependents to ensure consumers still work correctly
        dep_graph.run_target_dependents(target, &workspace.projects)?;
    }

    println!("Target count: {}", targets.len());
    println!("Action count: {}", dep_graph.graph.node_count());

    Ok(dep_graph)
}

pub struct CiOptions {
    pub base: Option<String>,
    pub head: Option<String>,
    pub job: Option<usize>,
    pub job_total: Option<usize>,
}

pub async fn ci(options: CiOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let touched_files = gather_touched_files(&workspace, &options).await?;
    let targets = gather_runnable_targets(&workspace, &touched_files)?;

    if targets.is_empty() {
        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&options, targets);
    let dep_graph = generate_dep_graph(&workspace, &targets)?;

    // Process all tasks in the graph
    print_header("Running all targets");

    let mut runner = ActionRunner::new(workspace);
    let results = runner
        .run(
            dep_graph,
            Some(ActionContext {
                touched_files,
                ..ActionContext::default()
            }),
        )
        .await?;

    // Print out the results and exit if an error occurs
    print_header("Results");

    runner.render_results(&results)?;
    runner.render_stats(&results, false)?;

    if runner.has_failed() {
        safe_exit(1);
    }

    Ok(())
}
