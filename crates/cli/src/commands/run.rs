use console::Term;
use humantime::format_duration;
use moon_logger::color;
use moon_terminal::output::label_moon;
use moon_terminal::ExtendedTerm;
use moon_workspace::{DepGraph, TaskResult, TaskResultStatus, TaskRunner, Workspace};
use std::time::Duration;

pub fn render_result_stats(
    results: Vec<TaskResult>,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let total_count = results.len();
    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut invalid_count = 0;

    for result in results {
        match result.status {
            TaskResultStatus::Passed => {
                pass_count += 1;
            }
            TaskResultStatus::Failed => {
                fail_count += 1;
            }
            TaskResultStatus::Invalid => {
                invalid_count += 1;
            }
            _ => {}
        }
    }

    let ran_message = format!("Ran {} tasks in {}", total_count, format_duration(duration));
    let mut counts_message = vec![];

    if pass_count > 0 {
        counts_message.push(color::success(&format!("{} completed", pass_count)));
    }

    if fail_count > 0 {
        counts_message.push(color::failure(&format!("{} failed", fail_count)));
    }

    if invalid_count > 0 {
        counts_message.push(color::invalid(&format!("{} invalid", invalid_count)));
    }

    let contents = format!(
        "{}\n{}",
        ran_message,
        counts_message.join(&color::muted(", "))
    );

    // let term = Term::buffered_stdout();
    // term.write_line("")?;
    // term.write_line(&contents)?;
    // term.write_line("")?;
    // term.flush()?;

    let term = Term::buffered_stdout();
    term.write_line("")?;
    term.write_line(&label_moon())?;
    term.render_entry("Tasks", &counts_message.join(&color::muted(", ")))?;
    term.render_entry(" Time", &format_duration(duration).to_string())?;
    term.write_line("")?;
    term.flush()?;

    Ok(())
}

pub async fn run(target: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();
    dep_graph.run_target(target, &workspace.projects)?;

    // Process all tasks in the graph
    let mut runner = TaskRunner::new(workspace);
    let results = runner.set_primary_target(target).run(dep_graph).await?;

    // Render stats about the run
    render_result_stats(results, runner.duration.unwrap())?;

    Ok(())
}
