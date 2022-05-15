use clap::ArgEnum;
use console::Term;
use moon_logger::color;
use moon_project::{Target, TouchedFilePaths};
use moon_terminal::ExtendedTerm;
use moon_utils::time;
use moon_workspace::{Action, ActionRunner, ActionStatus, DepGraph, Workspace, WorkspaceError};
use std::collections::HashSet;
use std::env;
use std::string::ToString;
use std::time::Duration;
use strum_macros::Display;

#[derive(ArgEnum, Clone, Debug, Display)]
pub enum RunStatus {
    Added,
    All,
    Deleted,
    Modified,
    Staged,
    Unstaged,
    Untracked,
}

impl Default for RunStatus {
    fn default() -> Self {
        RunStatus::All
    }
}

pub struct RunOptions {
    pub affected: bool,
    pub dependents: bool,
    pub status: RunStatus,
    pub passthrough: Vec<String>,
    pub upstream: bool,
}

async fn get_touched_files(
    workspace: &Workspace,
    status: &RunStatus,
    upstream: bool,
) -> Result<TouchedFilePaths, WorkspaceError> {
    let vcs = workspace.detect_vcs();

    let touched_files = if upstream {
        vcs.get_touched_files_between_revisions(vcs.get_default_branch(), "HEAD")
            .await?
    } else {
        vcs.get_touched_files().await?
    };

    let files = match status {
        RunStatus::Added => touched_files.added,
        RunStatus::All => touched_files.all,
        RunStatus::Deleted => touched_files.deleted,
        RunStatus::Modified => touched_files.modified,
        RunStatus::Staged => touched_files.staged,
        RunStatus::Unstaged => touched_files.unstaged,
        RunStatus::Untracked => touched_files.untracked,
    };

    let mut touched = HashSet::new();

    for file in &files {
        touched.insert(workspace.root.join(file));
    }

    Ok(touched)
}

pub fn render_result_stats(
    results: Vec<Action>,
    duration: Duration,
    in_actions_context: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cached_count = 0;
    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut invalid_count = 0;

    let filtered_results = if in_actions_context {
        results
    } else {
        results
            .into_iter()
            .filter(|result| match &result.label {
                Some(l) => l.contains("RunTarget"),
                None => false,
            })
            .collect()
    };

    for result in filtered_results {
        match result.status {
            ActionStatus::Cached => {
                cached_count += 1;
                pass_count += 1;
            }
            ActionStatus::Passed | ActionStatus::Skipped => {
                pass_count += 1;
            }
            ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                fail_count += 1;
            }
            ActionStatus::Invalid => {
                invalid_count += 1;
            }
            _ => {}
        }
    }

    let mut counts_message = vec![];

    if pass_count > 0 {
        if cached_count > 0 {
            counts_message.push(color::success(&format!(
                "{} completed ({} cached)",
                pass_count, cached_count
            )));
        } else {
            counts_message.push(color::success(&format!("{} completed", pass_count)));
        }
    }

    if fail_count > 0 {
        counts_message.push(color::failure(&format!("{} failed", fail_count)));
    }

    if invalid_count > 0 {
        counts_message.push(color::invalid(&format!("{} invalid", invalid_count)));
    }

    let term = Term::buffered_stdout();
    term.write_line("")?;

    let counts_message = counts_message.join(&color::muted(", "));
    let elapsed_time = match env::var("MOON_TEST") {
        Ok(_) => String::from("100ms"), // Snapshots
        Err(_) => time::elapsed(duration),
    };

    if in_actions_context {
        term.render_entry("Actions", &counts_message)?;
        term.render_entry("   Time", &elapsed_time)?;
    } else {
        term.render_entry("Tasks", &counts_message)?;
        term.render_entry(" Time", &elapsed_time)?;
    }

    term.write_line("")?;
    term.flush()?;

    Ok(())
}

pub async fn run(target_id: &str, options: RunOptions) -> Result<(), Box<dyn std::error::Error>> {
    let target = Target::parse(target_id)?;
    let workspace = Workspace::load().await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();

    if options.affected {
        let touched_files =
            get_touched_files(&workspace, &options.status, options.upstream).await?;
        let inserted_count =
            dep_graph.run_target(&target, &workspace.projects, Some(&touched_files))?;

        if inserted_count == 0 {
            if matches!(options.status, RunStatus::All) {
                println!("Target {} not affected by touched files", target_id);
            } else {
                println!(
                    "Target {} not affected by touched files (using status {})",
                    target_id,
                    color::symbol(&options.status.to_string().to_lowercase())
                );
            }

            return Ok(());
        }
    } else {
        dep_graph.run_target(&target, &workspace.projects, None)?;
    }

    if options.dependents {
        dep_graph.run_target_dependents(&target, &workspace.projects)?;
    }

    // Process all tasks in the graph
    let mut runner = ActionRunner::new(workspace);

    let results = runner
        .bail_on_error()
        .set_passthrough_args(options.passthrough)
        .set_primary_target(target_id)
        .run(dep_graph)
        .await?;

    // Render stats about the run
    render_result_stats(results, runner.duration.unwrap(), false)?;

    Ok(())
}
