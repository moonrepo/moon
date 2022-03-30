use clap::ArgEnum;
use console::Term;
use moon_logger::color;
use moon_project::TouchedFilePaths;
use moon_terminal::ExtendedTerm;
use moon_utils::time;
use moon_workspace::{Action, ActionRunner, ActionStatus, DepGraph, Workspace, WorkspaceError};
use std::collections::HashSet;
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
    pub local: bool,
    pub status: RunStatus,
    pub passthrough: Vec<String>,
}

async fn get_touched_files(
    workspace: &Workspace,
    status: &RunStatus,
    local: bool,
) -> Result<TouchedFilePaths, WorkspaceError> {
    let vcs = workspace.detect_vcs();

    let touched_files = if local {
        vcs.get_touched_files().await?
    } else {
        vcs.get_touched_files_between_revisions(vcs.get_default_branch(), "HEAD")
            .await?
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cached_count = 0;
    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut invalid_count = 0;

    for result in results {
        match result.status {
            ActionStatus::Cached => {
                cached_count += 1;
                pass_count += 1;
            }
            ActionStatus::Passed | ActionStatus::Skipped => {
                pass_count += 1;
            }
            ActionStatus::Failed => {
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
    term.render_entry("Tasks", &counts_message.join(&color::muted(", ")))?;
    term.render_entry(" Time", &time::elapsed(duration))?;
    term.write_line("")?;
    term.flush()?;

    Ok(())
}

pub async fn run(target: &str, options: RunOptions) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();

    if options.affected {
        let touched_files = get_touched_files(&workspace, &options.status, options.local).await?;

        if dep_graph
            .run_target_if_touched(target, &touched_files, &workspace.projects)?
            .is_none()
        {
            if matches!(options.status, RunStatus::All) {
                println!("Target {} not affected by touched files", target);
            } else {
                println!(
                    "Target {} not affected by touched files (using status {})",
                    target,
                    color::symbol(&options.status.to_string().to_lowercase())
                );
            }

            return Ok(());
        }
    } else {
        dep_graph.run_target(target, &workspace.projects)?;
    }

    // Process all tasks in the graph
    let mut runner = ActionRunner::new(workspace);

    let results = runner
        .bail_on_error()
        .set_passthrough_args(options.passthrough)
        .set_primary_target(target)
        .run(dep_graph)
        .await?;

    // Render stats about the run
    render_result_stats(results, runner.duration.unwrap())?;

    Ok(())
}
