use crate::experiments::run_action_pipeline;
use crate::queries::touched_files::{query_touched_files, QueryTouchedFilesOptions};
use crate::session::CliSession;
use clap::Args;
use moon_action_context::{ActionContext, ProfileType};
use moon_action_graph::RunRequirements;
use moon_cache::CacheMode;
use moon_common::{is_ci, is_test_env};
use moon_task::TargetLocator;
use moon_vcs::TouchedStatus;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_styles::color;
use std::string::ToString;
use tracing::instrument;

const HEADING_AFFECTED: &str = "Affected by changes";
const HEADING_DEBUGGING: &str = "Debugging";

#[derive(Args, Clone, Debug, Default)]
pub struct RunArgs {
    #[arg(required = true, help = "List of targets to run")]
    pub targets: Vec<TargetLocator>,

    #[arg(long, help = "Run dependents of the primary targets")]
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
        long,
        short = 's',
        help = "Include a summary of all actions that were processed in the pipeline"
    )]
    pub summary: bool,

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
    session: &CliSession,
    args: &RunArgs,
    target_locators: &[TargetLocator],
) -> AppResult {
    let console = &session.console;
    let cache_engine = session.get_cache_engine()?;
    let project_graph = session.get_project_graph().await?;
    let vcs = session.get_vcs_adapter()?;

    // Force cache to update using write-only mode
    if args.update_cache {
        cache_engine.force_mode(CacheMode::Write);
    }

    let mut should_run_affected = !args.force && args.affected;

    // Always query for a touched files list as it'll be used by many actions
    let touched_files = if vcs.is_enabled() {
        let local = is_local(args);
        let result = query_touched_files(
            &vcs,
            &QueryTouchedFilesOptions {
                default_branch: !local && !is_test_env(),
                local,
                status: args.status.clone(),
                ..QueryTouchedFilesOptions::default()
            },
        )
        .await?;

        if result.shallow {
            should_run_affected = false;
        }

        result.files
    } else {
        FxHashSet::default()
    };

    // Generate a dependency graph for all the targets that need to be ran
    let mut action_graph_builder = session.build_action_graph(&project_graph).await?;

    if let Some(query_input) = &args.query {
        action_graph_builder.set_query(query_input)?;
    }

    // Run targets, optionally based on affected files
    let mut primary_targets = vec![];
    let mut requirements = RunRequirements {
        ci: is_ci(),
        ci_check: false,
        dependents: args.dependents,
        initial_locators: target_locators.iter().collect(),
        resolved_locators: vec![],
        interactive: args.interactive,
        touched_files: if should_run_affected {
            Some(&touched_files)
        } else {
            None
        },
    };

    for locator in target_locators {
        primary_targets.extend(
            action_graph_builder
                .run_task_by_target_locator(locator, &mut requirements)?
                .0,
        );
    }

    if primary_targets.is_empty() {
        let targets_list = target_locators
            .iter()
            .map(color::label)
            .collect::<Vec<_>>()
            .join(", ");

        if should_run_affected {
            let status_list = if args.status.is_empty() {
                color::symbol(TouchedStatus::All.to_string())
            } else {
                args.status
                    .iter()
                    .map(|s| color::symbol(s.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            console.out.write_line(
                format!("Target(s) {targets_list} not affected by touched files (using status {status_list})")
            )?;
        } else {
            console
                .out
                .write_line(format!("No tasks found for target(s) {targets_list}"))?;
        }

        if let Some(query_input) = &args.query {
            console
                .out
                .write_line(format!("Using query {}", color::shell(query_input)))?;
        }

        return Ok(());
    }

    // Process all tasks in the graph
    let context = ActionContext {
        affected_only: should_run_affected,
        initial_targets: FxHashSet::from_iter(target_locators.to_owned()),
        passthrough_args: args.passthrough.to_owned(),
        primary_targets: FxHashSet::from_iter(primary_targets),
        profile: args.profile.to_owned(),
        touched_files: touched_files.clone(),
        workspace_root: session.workspace_root.clone(),
        ..ActionContext::default()
    };

    run_action_pipeline(&session, action_graph_builder.build()?, Some(context)).await?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn run(session: CliSession, args: RunArgs) -> AppResult {
    run_target(&session, &args, &args.targets).await?;

    Ok(())
}
