use crate::enums::TouchedStatus;
use crate::helpers::load_workspace;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon_action::{ActionContext, ProfileType};
use moon_logger::{color, map_list};
use moon_project_graph::project_graph::ProjectGraph;
use moon_runner::{DepGraph, Runner};
use moon_task::Target;
use moon_workspace::Workspace;
use std::collections::HashSet;
use std::string::ToString;

#[derive(Default)]
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
    base_workspace: Option<Workspace>,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = match base_workspace {
        Some(ws) => ws,
        None => load_workspace().await?,
    };
    let project_graph = ProjectGraph::generate(&workspace).await?;

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::generate(&project_graph);
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
        dep_graph.run_targets_by_id(target_ids, &touched_files)?;

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
        project_graph.load_all()?;

        for target in &primary_targets {
            dep_graph.run_target_dependents(Target::parse(target)?)?;
        }
    }

    // Process all tasks in the graph
    let context = ActionContext {
        passthrough_args: options.passthrough,
        primary_targets: HashSet::from_iter(primary_targets),
        profile: options.profile,
        touched_files: touched_files.unwrap_or_default(),
    };

    let mut runner = Runner::new();

    if options.report {
        runner.generate_report("runReport.json");
    }

    let results = runner
        .bail_on_error()
        .run(workspace, dep_graph, Some(context))
        .await?;

    runner.render_stats(&results, true)?;

    Ok(())
}
