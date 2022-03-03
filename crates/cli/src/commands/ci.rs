use itertools::Itertools;
use moon_logger::{color, debug};
use moon_project::{Target, TargetID, TouchedFilePaths};
use moon_utils::is_ci;
use moon_workspace::DepGraph;
use moon_workspace::{Workspace, WorkspaceError};
use std::collections::HashSet;
use std::path::PathBuf;

type TargetList = Vec<TargetID>;

const TARGET: &str = "moon:ci";

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
            .map(|t| format!("  {}", color::target(t)))
            .join("\n")
    );
}

/// Gather a list of files that have been modified between branches.
async fn gather_touched_files(workspace: &Workspace) -> Result<TouchedFilePaths, WorkspaceError> {
    print_header("Gathering touched files");

    let vcs = workspace.detect_vcs();
    let branch = vcs.get_local_branch().await?;
    let touched_files_map = if vcs.is_default_branch(&branch) {
        // On master/main, so compare against master -1 commit
        vcs.get_touched_files_against_branch("HEAD", 1).await?
    } else {
        // On a branch, so compare branch against master/main
        vcs.get_touched_files_against_branch(&branch, 0).await?
    };

    let mut touched_files_to_print = vec![];
    let touched_files: HashSet<PathBuf> = touched_files_map
        .all
        .iter()
        .map(|f| {
            touched_files_to_print.push(format!("  {}", color::path(f)));
            workspace.root.join(f)
        })
        .collect();

    touched_files_to_print.sort();

    println!("{}", touched_files_to_print.join("\n"));

    Ok(touched_files)
}

/// Gather runnable targets by checking if all projects/tasks are affected based on touched files.
fn gather_runnable_targets(
    workspace: &Workspace,
    touched_files: &TouchedFilePaths,
) -> Result<TargetList, WorkspaceError> {
    print_header("Gathering runnable targets");

    let mut targets = vec![];

    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;

        if !project.is_affected(touched_files) {
            continue;
        }

        for (task_id, task) in &project.tasks {
            let target = Target::format(&project_id, task_id)?;

            if task.should_run_in_ci() {
                if task.is_affected(touched_files)? {
                    targets.push(target);
                }
            } else {
                debug!(
                    target: TARGET,
                    "Not running target {} because it either has no `outputs` or `runInCi` is false",
                    color::target(&target),
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
) -> Result<DepGraph, WorkspaceError> {
    print_header("Generating dependency and task graphs");

    let mut dep_graph = DepGraph::default();

    for target in targets {
        dep_graph.run_target(target, &workspace.projects)?;
    }

    println!("Target count: {}", targets.len());
    println!("Node count: {}", dep_graph.graph.node_count());

    Ok(dep_graph)
}

pub struct CiOptions {
    pub job: Option<usize>,
    pub job_total: Option<usize>,
}

pub async fn ci(options: CiOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let touched_files = gather_touched_files(&workspace).await?;
    let targets = gather_runnable_targets(&workspace, &touched_files)?;

    if targets.is_empty() {
        return Ok(());
    }

    let targets = distribute_targets_across_jobs(&options, targets);
    let dep_graph = generate_dep_graph(&workspace, &targets)?;

    Ok(())
}
