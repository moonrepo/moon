use crate::enums::TouchedStatus;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon_action::{ActionContext, ProfileType};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_logger::{color, map_list};
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

pub async fn run(
    target_ids: &[String],
    options: RunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let targets_list = map_list(target_ids, |id| color::target(id));
    let workspace = Workspace::load().await?;
    let primary_targets: Vec<String>;
    let inserted_count: usize;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();
    let mut touched_files = HashSet::new();

    // Run targets based on affected files
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

        let (primary_targets, inserted_count) =
            dep_graph.run_targets_by_id(target_ids, &workspace.projects, Some(&touched_files))?;

        if inserted_count == 0 {
            if matches!(options.status, TouchedStatus::All) {
                println!("Target(s) {} not affected by touched files", targets_list);
            } else {
                println!(
                    "Target(s) {} not affected by touched files (using status {})",
                    targets_list,
                    color::symbol(&options.status.to_string().to_lowercase())
                );
            }

            return Ok(());
        }

        // Otherwise explicitly run the targets
    } else {
        let (primary_targets, inserted_count) =
            dep_graph.run_targets_by_id(target_ids, &workspace.projects, None)?;

        if inserted_count == 0 {
            println!("No tasks found for target(s) {}", targets_list);

            return Ok(());
        }
    }

    // Run dependents for all primary targets
    if options.dependents {
        workspace.projects.load_all()?;

        for target in &primary_targets {
            dep_graph.run_target_dependents(Target::parse(target)?, &workspace.projects)?;
        }
    }

    // Process all tasks in the graph
    let context = ActionContext {
        passthrough_args: options.passthrough,
        primary_targets: HashSet::from_iter(primary_targets),
        profile: options.profile,
        touched_files,
    };

    let mut runner = ActionRunner::new(workspace);
    let results = runner.bail_on_error().run(dep_graph, Some(context)).await?;

    runner.render_stats(&results, true)?;

    Ok(())
}
