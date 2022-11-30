use crate::enums::TouchedStatus;
use crate::helpers::load_workspace;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon_logger::{color, map_list};
use moon_runner::{DepGraph, Runner};
use moon_runner_context::{ProfileType, RunnerContext};
use moon_utils::is_ci;
use moon_workspace::Workspace;
use rustc_hash::{FxHashMap, FxHashSet};
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

pub fn is_local(options: &RunOptions) -> bool {
    if options.affected {
        !options.upstream
    } else {
        !is_ci()
    }
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

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_graph = DepGraph::default();
    let touched_files = if options.affected || workspace.vcs.is_enabled() {
        query_touched_files(
            &workspace,
            &mut QueryTouchedFilesOptions {
                local: is_local(&options),
                status: options.status,
                ..QueryTouchedFilesOptions::default()
            },
        )
        .await?
    } else {
        FxHashSet::default()
    };

    // Run targets, optionally based on affected files
    let primary_targets = dep_graph.run_targets_by_id(
        target_ids,
        &workspace.projects,
        if options.affected {
            Some(&touched_files)
        } else {
            None
        },
    )?;

    if primary_targets.is_empty() {
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
            dep_graph.run_dependents_for_target(target, &workspace.projects)?;
        }
    }

    // Process all tasks in the graph
    let context = RunnerContext {
        affected: options.affected,
        initial_targets: FxHashSet::from_iter(target_ids.to_owned()),
        passthrough_args: options.passthrough,
        primary_targets: FxHashSet::from_iter(primary_targets),
        profile: options.profile,
        target_hashes: FxHashMap::default(),
        touched_files,
    };

    let mut runner = Runner::new(workspace);

    if options.report {
        runner.generate_report("runReport.json");
    }

    let results = runner.bail_on_error().run(dep_graph, Some(context)).await?;

    runner.render_stats(&results, true)?;

    Ok(())
}
