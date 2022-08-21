use crate::enums::TouchedStatus;
use crate::helpers::load_workspace;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon_action::{ActionContext, ProfileType};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_logger::{color, map_list};
use moon_task::Target;
use std::collections::HashSet;
use std::string::ToString;

pub struct RunOptions {
    pub affected: bool,
    pub dependents: bool,
    pub status: TouchedStatus,
    pub passthrough: Vec<String>,
    pub profile: Option<ProfileType>,
    pub report: bool,
    pub upstream: bool,
}

pub async fn run(
    target_ids: &[String],
    options: RunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();
    let touched_files = if options.affected {
        Some(
            query_touched_files(
                &workspace,
                &mut QueryTouchedFilesOptions {
                    local: !options.upstream,
                    status: options.status,
                    ..QueryTouchedFilesOptions::default()
                },
            )
            .await?,
        )
    } else {
        None
    };

    // Run targets, optionally based on affected files
    let (primary_targets, inserted_count) =
        dep_graph.run_targets_by_id(target_ids, &workspace.projects, &touched_files)?;

    if inserted_count == 0 {
        let targets_list = map_list(target_ids, |id| color::target(id));

        if options.affected {
            if matches!(options.status, TouchedStatus::All) {
                println!("Target(s) {} not affected by touched files", targets_list);
            } else {
                println!(
                    "Target(s) {} not affected by touched files (using status {})",
                    targets_list,
                    color::symbol(&options.status.to_string().to_lowercase())
                );
            }
        } else {
            println!("No tasks found for target(s) {}", targets_list);
        }

        return Ok(());
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
        touched_files: touched_files.unwrap_or_default(),
    };

    let mut runner = ActionRunner::new(workspace);

    if options.report {
        runner.generate_report();
    }

    let results = runner.bail_on_error().run(dep_graph, Some(context)).await?;

    runner.render_stats(&results, true)?;

    Ok(())
}
