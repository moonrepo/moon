use crate::enums::TouchedStatus;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon_action::{ActionContext, ProfileType};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_logger::color;
use moon_task::Target;
use moon_workspace::Workspace;
use std::collections::HashSet;
use std::string::ToString;

pub struct RunOptions {
    pub affected: bool,
    pub dependents: bool,
    pub status: TouchedStatus,
    pub passthrough: Vec<String>,
    pub profile: Option<ProfileType>,
    pub upstream: bool,
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

    runner.render_stats(&results, true)?;

    Ok(())
}
