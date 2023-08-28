use crate::app::GlobalArgs;
use crate::enums::{CacheMode, TouchedStatus};
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use clap::Args;
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
use starbase::{system, AppResult};
use starbase_styles::color;
use std::env;
use std::string::ToString;

const HEADING_AFFECTED: &str = "Affected by changes";
const HEADING_DEBUGGING: &str = "Debugging";

#[derive(Args, Clone, Debug, Default)]
pub struct RunArgs {
    #[arg(required = true, help = "List of targets (scope:task) to run")]
    pub targets: Vec<String>,

    #[arg(
        long,
        help = "Run dependents of the primary targets, as well as dependencies"
    )]
    pub dependents: bool,

    #[arg(
        long,
        short = 'f',
        help = "Force run and ignore touched files and affected status"
    )]
    pub force: bool,

    #[arg(long, short = 'i', help = "Run the target interactively")]
    pub interactive: bool,

    #[arg(long, help = "Focus target(s) based on the result of a query")]
    pub query: Option<String>,

    #[arg(
        short = 'u',
        long = "updateCache",
        help = "Bypass cache and force update any existing items"
    )]
    pub update_cache: bool,

    // Debugging
    #[arg(
        value_enum,
        long,
        help = "Record and generate a profile for ran tasks",
        help_heading = HEADING_DEBUGGING,
    )]
    pub profile: Option<ProfileType>,

    // Affected
    #[arg(
        long,
        help = "Only run target if affected by touched files",
        help_heading = HEADING_AFFECTED,
        group = "affected-args"
    )]
    pub affected: bool,

    #[arg(
        long,
        help = "Determine affected against remote by comparing against a base revision",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    pub remote: bool,

    #[arg(
        value_enum,
        long,
        help = "Filter affected files based on a touched status",
        help_heading = HEADING_AFFECTED,
        requires = "affected-args",
    )]
    pub status: Vec<TouchedStatus>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    pub passthrough: Vec<String>,
}

pub fn is_local(args: &RunArgs) -> bool {
    if args.affected {
        !args.remote
    } else {
        !is_ci()
    }
}

pub async fn run_target(
    target_ids: &[String],
    args: &RunArgs,
    concurrency: Option<usize>,
    workspace: Workspace,
    project_graph: ProjectGraph,
) -> AppResult {
    // Force cache to update using write-only mode
    if args.update_cache {
        env::set_var("MOON_CACHE", CacheMode::Write.to_string());
    }

    let should_run_affected = !args.force && args.affected;

    // Always query for a touched files list as it'll be used by many actions
    let touched_files = if !args.force && (args.affected || workspace.vcs.is_enabled()) {
        let local = is_local(args);

        query_touched_files(
            &workspace,
            &QueryTouchedFilesOptions {
                default_branch: !local && !is_test_env(),
                local,
                status: args.status.clone(),
                ..QueryTouchedFilesOptions::default()
            },
        )
        .await?
    } else {
        FxHashSet::default()
    };

    // Generate a dependency graph for all the targets that need to be ran
    let mut dep_builder = build_dep_graph(&project_graph);

    if let Some(query_input) = &args.query {
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
            let status_list = if args.status.is_empty() {
                color::symbol(TouchedStatus::All.to_string())
            } else {
                map_list(&args.status, |s| color::symbol(s.to_string()))
            };

            println!(
                "Target(s) {targets_list} not affected by touched files (using status {status_list})"
            );
        } else {
            println!("No tasks found for target(s) {targets_list}");
        }

        if let Some(query_input) = &args.query {
            println!("Using query {}", color::shell(query_input));
        }

        return Ok(());
    }

    // Interactive can only run against 1 task
    if args.interactive && primary_targets.len() > 1 {
        return Err(miette!(
            "Only 1 target can be ran as interactive. Requires a fully qualified project target."
        ));
    }

    // Run dependents for all primary targets
    if args.dependents {
        for target in &primary_targets {
            dep_builder.run_dependents_for_target(target)?;
        }
    }

    // Process all tasks in the graph
    let context = ActionContext {
        affected_only: should_run_affected,
        initial_targets: FxHashSet::from_iter(target_ids.to_owned()),
        interactive: args.interactive,
        passthrough_args: args.passthrough.to_owned(),
        primary_targets: FxHashSet::from_iter(primary_targets),
        profile: args.profile.to_owned(),
        touched_files,
        workspace_root: workspace.root.clone(),
        ..ActionContext::default()
    };

    let dep_graph = dep_builder.build();
    let mut pipeline = Pipeline::new(workspace, project_graph);

    if let Some(concurrency) = concurrency {
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

#[system]
pub async fn run(args: ArgsRef<RunArgs>, global_args: StateRef<GlobalArgs>) {
    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace).await?;

    run_target(
        &args.targets,
        args,
        global_args.concurrency,
        workspace,
        project_graph,
    )
    .await?;
}
