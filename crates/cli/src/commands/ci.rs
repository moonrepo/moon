use crate::commands::run::render_result_stats;
use console::Term;
use itertools::Itertools;
use moon_logger::{color, debug};
use moon_project::{Target, TouchedFilePaths};
use moon_terminal::helpers::{replace_style_tokens, safe_exit};
use moon_terminal::output;
use moon_utils::{is_ci, path, time};
use moon_workspace::DepGraph;
use moon_workspace::{ActionRunner, ActionStatus, Workspace, WorkspaceError};
use std::collections::HashSet;
use std::path::PathBuf;

type TargetList = Vec<Target>;

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

    let vcs = workspace.detect_vcs()?;
    let default_branch = vcs.get_default_branch();
    let current_branch = vcs.get_local_branch().await?;

    // On default branch, so compare against self -1 revision
    let touched_files_map = if vcs.is_default_branch(&current_branch) {
        vcs.get_touched_files_against_previous_revision(default_branch)
            .await?

        // On a branch, so compare branch against base/default branch
    } else {
        let base = options
            .base
            .clone()
            .unwrap_or_else(|| default_branch.to_owned());
        let head = options.head.clone().unwrap_or_else(|| String::from("HEAD"));

        vcs.get_touched_files_between_revisions(&base, &head)
            .await?
    };

    let mut touched_files_to_print = vec![];
    let touched_files: HashSet<PathBuf> = touched_files_map
        .all
        .iter()
        .map(|f| {
            touched_files_to_print.push(format!("  {}", color::file(f)));
            workspace.root.join(path::normalize_separators(f))
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
    let globally_affected = workspace.projects.is_globally_affected(touched_files);

    if globally_affected {
        debug!(
            target: TARGET,
            "Moon files touched, marking all targets as affected",
        );
    }

    for project_id in workspace.projects.ids() {
        let project = workspace.projects.load(&project_id)?;

        if !globally_affected && !project.is_affected(touched_files) {
            continue;
        }

        for (task_id, task) in &project.tasks {
            let target = Target::new(&project_id, task_id)?;

            if task.should_run_in_ci() {
                if globally_affected || task.is_affected(touched_files)? {
                    targets.push(target);
                }
            } else {
                debug!(
                    target: TARGET,
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
) -> Result<DepGraph, WorkspaceError> {
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
    let results = runner.run(dep_graph).await?;

    // Print out the results and exit if an error occurs
    let mut error_count = 0;
    let term = Term::buffered_stdout();

    print_header("Results");

    for result in &results {
        let status = match result.status {
            ActionStatus::Passed | ActionStatus::Cached | ActionStatus::Skipped => {
                color::success("pass")
            }
            ActionStatus::Failed | ActionStatus::FailedAndAbort => color::failure("fail"),
            ActionStatus::Invalid => color::invalid("warn"),
            _ => color::muted_light("oops"),
        };

        let mut meta: Vec<String> = vec![];

        if matches!(result.status, ActionStatus::Cached) {
            meta.push(String::from("cached"));
        } else if matches!(result.status, ActionStatus::Skipped) {
            meta.push(String::from("skipped"));
        } else {
            meta.push(time::elapsed(result.duration.unwrap()));
        }

        term.write_line(&format!(
            "{} {} {}",
            status,
            output::bold(result.label.as_ref().unwrap()),
            color::muted(&format!("({})", meta.join(", ")))
        ))?;

        if let Some(error) = &result.error {
            error_count += 1;
            term.write_line(&format!(
                "     {}",
                color::muted_light(&replace_style_tokens(error))
            ))?;
        }
    }

    term.flush()?;

    render_result_stats(results, runner.duration.unwrap(), true)?;

    if error_count > 0 {
        safe_exit(1);
    }

    Ok(())
}
