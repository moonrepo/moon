use crate::enums::{CacheMode, TouchedStatus};
use crate::helpers::AnyError;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_logger::{color, map_list};
use moon_project_graph::ProjectGraph;
use moon_runner::Runner;
use moon_runner_context::{ProfileType, RunnerContext};
use moon_utils::is_ci;
use moon_workspace::Workspace;
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;
use std::string::ToString;

#[derive(Default)]
pub struct RunOptions {
    pub affected: bool,
    pub dependents: bool,
    pub status: Vec<TouchedStatus>,
    pub passthrough: Vec<String>,
    pub profile: Option<ProfileType>,
    pub report: bool,
    pub update_cache: bool,
    pub upstream: bool,
}

pub fn is_local(options: &RunOptions) -> bool {
    if options.affected {
        !options.upstream
    } else {
        !is_ci()
    }
}

pub async fn run_target(
    target_ids: &[String],
    options: RunOptions,
    workspace: Workspace,
    project_graph: ProjectGraph,
) -> Result<(), AnyError> {
    // Force cache to update using write-only mode
    if options.update_cache {
        env::set_var("MOON_CACHE", CacheMode::Write.to_string());
    }

    // Always query for a touched files list as it'll be used by many actions
    let touched_files = if options.affected || workspace.vcs.is_enabled() {
        query_touched_files(
            &workspace,
            &mut QueryTouchedFilesOptions {
                local: is_local(&options),
                status: options.status.clone(),
                ..QueryTouchedFilesOptions::default()
            },
        )
        .await?
    } else {
        FxHashSet::default()
    };

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    // Run targets, optionally based on affected files
    let primary_targets = dep_builder.run_targets_by_id(
        target_ids,
        if options.affected {
            Some(&touched_files)
        } else {
            None
        },
    )?;

    if primary_targets.is_empty() {
        let targets_list = map_list(target_ids, |id| color::target(id));

        if options.affected {
            let status_list = if options.status.is_empty() {
                color::symbol(TouchedStatus::All.to_string())
            } else {
                map_list(&options.status, |s| color::symbol(s.to_string()))
            };

            println!(
                "Target(s) {} not affected by touched files (using status {})",
                targets_list, status_list
            );
        } else {
            println!("No tasks found for target(s) {}", targets_list);
        }

        return Ok(());
    }

    // Run dependents for all primary targets
    if options.dependents {
        for target in &primary_targets {
            dep_builder.run_dependents_for_target(target)?;
        }
    }

    // Process all tasks in the graph
    let context = RunnerContext {
        affected_only: options.affected,
        initial_targets: FxHashSet::from_iter(target_ids.to_owned()),
        passthrough_args: options.passthrough,
        primary_targets: FxHashSet::from_iter(primary_targets),
        profile: options.profile,
        target_hashes: FxHashMap::default(),
        touched_files,
    };

    let dep_graph = dep_builder.build();
    let mut runner = Runner::new(workspace);

    if options.report {
        runner.generate_report("runReport.json");
    }

    let results = runner
        .bail_on_error()
        .run(dep_graph, project_graph, Some(context))
        .await?;

    runner.render_stats(&results, true)?;

    Ok(())
}

pub async fn run(target_ids: &[String], options: RunOptions) -> Result<(), AnyError> {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace)?;

    run_target(target_ids, options, workspace, project_graph).await?;

    Ok(())
}
