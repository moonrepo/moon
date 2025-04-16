use crate::components::run_action_pipeline;
use crate::queries::touched_files::{QueryTouchedFilesOptions, query_touched_files};
use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_action_context::{ActionContext, ProfileType};
use moon_action_graph::{ActionGraphBuilderOptions, RunRequirements};
use moon_affected::{DownstreamScope, UpstreamScope};
use moon_cache::CacheMode;
use moon_common::{is_ci, is_test_env};
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_task::TargetLocator;
use moon_vcs::TouchedStatus;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use tracing::instrument;

const HEADING_AFFECTED: &str = "Affected by";
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

    #[arg(
        long,
        help = "Run the task without including sync and setup related actions in the graph"
    )]
    pub no_actions: bool,

    #[arg(
        long,
        short = 'n',
        help = "When a task fails, continue executing other tasks instead of aborting immediately"
    )]
    pub no_bail: bool,

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
        help = "Only run target if affected by touched files or the environment",
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
    session: &MoonSession,
    args: &RunArgs,
    target_locators: &[TargetLocator],
) -> AppResult {
    let cache_engine = session.get_cache_engine()?;
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
    let mut action_graph_builder = if args.no_actions {
        session
            .build_action_graph_with_options(ActionGraphBuilderOptions::new(false))
            .await?
    } else {
        session.build_action_graph().await?
    };

    action_graph_builder.set_touched_files(touched_files)?;

    if let Some(query_input) = &args.query {
        action_graph_builder.set_query(query_input)?;
    }

    if should_run_affected {
        action_graph_builder.track_affected(UpstreamScope::Deep, DownstreamScope::Deep, false)?;
    }

    // Run targets, optionally based on affected files
    let reqs = RunRequirements {
        ci: is_ci(),
        ci_check: false,
        dependents: args.dependents,
        interactive: args.interactive,
    };
    let mut inserted_nodes = FxHashSet::default();

    for locator in target_locators {
        inserted_nodes.extend(
            action_graph_builder
                .run_task_by_target_locator(locator, &reqs)
                .await?,
        );
    }

    if inserted_nodes.is_empty() {
        let targets_list = target_locators
            .iter()
            .map(|target| format!("<id>{}</id>", target.as_str()))
            .collect::<Vec<_>>()
            .join(", ");

        let message = if should_run_affected {
            let status_list = if args.status.is_empty() {
                "<symbol>all</symbol>".into()
            } else {
                args.status
                    .iter()
                    .map(|status| format!("<symbol>{status}</symbol>"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            format!(
                "Target(s) {targets_list} not affected by touched files using status {status_list}"
            )
        } else {
            format!("No tasks found for target(s) {targets_list}")
        };

        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: message)

                    #(args.query.as_ref().map(|query| {
                        element! {
                            StyledText(content: format!("Using query <shell>{query}</shell>"))
                        }
                    }))
                }
            }
        })?;

        return Ok(None);
    }

    // Process all tasks in the graph
    let (action_context, action_graph) = action_graph_builder.build();

    let results = run_action_pipeline(
        session,
        ActionContext {
            passthrough_args: args.passthrough.to_owned(),
            profile: args.profile.to_owned(),
            ..action_context
        },
        action_graph,
    )
    .await?;

    if args.no_bail {
        let failed = results.iter().any(|result| {
            if result.has_failed() {
                !result.allow_failure
            } else {
                false
            }
        });

        if failed {
            return Ok(Some(1));
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn run(session: MoonSession, args: RunArgs) -> AppResult {
    return run_target(&session, &args, &args.targets).await;
}
