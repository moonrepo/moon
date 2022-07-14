use crate::enums::TouchedStatus;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use console::Term;
use moon_action::{Action, ActionContext, ActionStatus, ProfileType};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_logger::color;
use moon_project::Target;
use moon_terminal::ExtendedTerm;
use moon_utils::time;
use moon_workspace::Workspace;
use std::collections::HashSet;
use std::string::ToString;
use std::time::Duration;

pub struct RunOptions {
    pub affected: bool,
    pub dependents: bool,
    pub status: TouchedStatus,
    pub passthrough: Vec<String>,
    pub profile: Option<ProfileType>,
    pub upstream: bool,
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
            counts_message.push(color::success(format!(
                "{} completed ({} cached)",
                pass_count, cached_count
            )));
        } else {
            counts_message.push(color::success(format!("{} completed", pass_count)));
        }
    }

    if fail_count > 0 {
        counts_message.push(color::failure(format!("{} failed", fail_count)));
    }

    if invalid_count > 0 {
        counts_message.push(color::invalid(format!("{} invalid", invalid_count)));
    }

    let term = Term::buffered_stdout();
    term.write_line("")?;

    let counts_message = counts_message.join(&color::muted(", "));
    let elapsed_time = time::elapsed(duration);

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
    let mut touched_files = HashSet::new();

    if options.affected {
        touched_files = query_touched_files(
            &workspace,
            &mut QueryTouchedFilesOptions {
                local: !options.upstream,
                status: options.status,
                ..QueryTouchedFilesOptions::default()
            },
        )
        .await?;

        let inserted_count =
            dep_graph.run_target(&target, &workspace.projects, Some(&touched_files))?;

        if inserted_count == 0 {
            if matches!(options.status, TouchedStatus::All) {
                println!(
                    "Target {} not affected by touched files",
                    color::target(target_id)
                );
            } else {
                println!(
                    "Target {} not affected by touched files (using status {})",
                    color::target(target_id),
                    color::symbol(&options.status.to_string().to_lowercase())
                );
            }

            return Ok(());
        }
    } else {
        let inserted_count = dep_graph.run_target(&target, &workspace.projects, None)?;

        if inserted_count == 0 {
            println!("No tasks found for target {}", color::target(target_id));

            return Ok(());
        }
    }

    if options.dependents {
        workspace.projects.load_all()?;

        dep_graph.run_target_dependents(&target, &workspace.projects)?;
    }

    // Process all tasks in the graph
    let context = ActionContext {
        passthrough_args: options.passthrough,
        primary_targets: HashSet::from([target_id.to_owned()]),
        profile: options.profile,
        touched_files,
    };

    let mut runner = ActionRunner::new(workspace);

    let results = runner.bail_on_error().run(dep_graph, Some(context)).await?;

    // Render stats about the run
    render_result_stats(results, runner.get_duration(), false)?;

    Ok(())
}
