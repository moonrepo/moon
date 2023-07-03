use crate::enums::{CacheMode, TouchedStatus};
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use miette::miette;
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_action_context::{ActionContext, ProfileType};
use moon_action_pipeline::Pipeline;
use moon_common::is_test_env;
use moon_logger::map_list;
use moon_project_graph::ProjectGraph;
use moon_utils::is_ci;
use moon_workspace::Workspace;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;
use std::env;
use std::string::ToString;

#[derive(Default)]
pub struct RunOptions {
    pub affected: bool,
    pub concurrency: Option<usize>,
    pub dependents: bool,
    pub force: bool,
    pub interactive: bool,
    pub passthrough: Vec<String>,
    pub profile: Option<ProfileType>,
    pub query: Option<String>,
    pub remote: bool,
    pub status: Vec<TouchedStatus>,
    pub update_cache: bool,
}

pub fn is_local(options: &RunOptions) -> bool {
    if options.affected {
        !options.remote
    } else {
        !is_ci()
    }
}

pub async fn run_target(
    target_ids: &[String],
    options: RunOptions,
    workspace: Workspace,
    project_graph: ProjectGraph,
) -> AppResult {
    // Force cache to update using write-only mode
    if options.update_cache {
        env::set_var("MOON_CACHE", CacheMode::Write.to_string());
    }

    let should_run_affected = !options.force && options.affected;

    // Always query for a touched files list as it'll be used by many actions
    let touched_files = if !options.force && (options.affected || workspace.vcs.is_enabled()) {
        let local = is_local(&options);

        query_touched_files(
            &workspace,
            &mut QueryTouchedFilesOptions {
                default_branch: !local && !is_test_env(),
                local,
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

    if let Some(query_input) = &options.query {
        dep_builder.set_query(query_input)?;
    }

    // Run targets, optionally based on affected files
    let primary_targets = dep_builder.run_targets_by_id(
        target_ids,
        if should_run_affected {
            Some(&touched_files)
        } else {
            None
        },
    )?;

    if primary_targets.is_empty() {
        let targets_list = map_list(target_ids, |id| color::label(id));

        if should_run_affected {
            let status_list = if options.status.is_empty() {
                color::symbol(TouchedStatus::All.to_string())
            } else {
                map_list(&options.status, |s| color::symbol(s.to_string()))
            };

            println!(
                "Target(s) {targets_list} not affected by touched files (using status {status_list})"
            );
        } else {
            println!("No tasks found for target(s) {targets_list}");
        }

        if let Some(query_input) = &options.query {
            println!("Using query {}", color::shell(query_input));
        }

        return Ok(());
    }

    // Interactive can only run against 1 task
    if options.interactive && primary_targets.len() > 1 {
        return Err(miette!(
            "Only 1 target can be ran as interactive. Requires a fully qualified project target."
        ));
    }

    // Run dependents for all primary targets
    if options.dependents {
        for target in &primary_targets {
            dep_builder.run_dependents_for_target(target)?;
        }
    }

    // Process all tasks in the graph
    let context = ActionContext {
        affected_only: should_run_affected,
        initial_targets: FxHashSet::from_iter(target_ids.to_owned()),
        interactive: options.interactive,
        passthrough_args: options.passthrough,
        primary_targets: FxHashSet::from_iter(primary_targets),
        profile: options.profile,
        touched_files,
        workspace_root: workspace.root.clone(),
        ..ActionContext::default()
    };

    let dep_graph = dep_builder.build();
    let mut pipeline = Pipeline::new(workspace, project_graph);

    if let Some(concurrency) = options.concurrency {
        pipeline.concurrency(concurrency);
    }

    let results = pipeline
        .bail_on_error()
        .generate_report("runReport.json")
        .run(dep_graph, Some(context))
        .await?;

    pipeline.render_stats(&results, true)?;

    Ok(())
}

pub async fn run(target_ids: &[String], options: RunOptions) -> AppResult {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;

    run_target(target_ids, options, workspace, project_graph).await?;

    Ok(())
}
