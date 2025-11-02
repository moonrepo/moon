use crate::helpers::run_action_pipeline;
use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_action::ActionStatus;
use moon_action_graph::ActionGraphBuilderOptions;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[instrument(skip(session))]
pub async fn setup(session: MoonSession) -> AppResult {
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions {
            // Only enable toolchain setup for this command
            install_dependencies: false.into(),
            setup_environment: false.into(),
            setup_toolchains: true.into(),
            sync_projects: false.into(),
            sync_project_dependencies: false,
            sync_workspace: false,
        })
        .await?;

    // First ensure proto is set up (this will be a dependency for toolchain setups)
    action_graph_builder.setup_proto().await?;

    // Add new toolchain plugin setups
    let mut toolchain_count = 0;

    for toolchain_id in session.toolchains_config.plugins.keys() {
        if let Some(spec) = action_graph_builder.get_workspace_spec(toolchain_id) {
            action_graph_builder.setup_toolchain(&spec, None).await?;

            if !spec.is_system() {
                toolchain_count += 1;
            }
        }
    }

    // Early exit if no toolchains are configured
    if toolchain_count == 0 {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Info) {
                    StyledText(content: "Unable to setup, no toolchains are configured!")
                }
            }
        })?;

        return Ok(None);
    }

    let (action_context, action_graph) = action_graph_builder.build();

    // Check if there are any actions to run
    if action_graph.get_node_count() == 0 {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Info) {
                    StyledText(content: "All toolchains are already up to date!")
                }
            }
        })?;

        return Ok(None);
    }

    // Run the action pipeline to set up all toolchains
    let results = run_action_pipeline(&session, action_context, action_graph).await?;

    // Analyze results and provide feedback
    let passed_count = results
        .iter()
        .filter(|action| matches!(action.status, ActionStatus::Passed))
        .count();
    let skipped_count = results
        .iter()
        .filter(|action| {
            matches!(
                action.status,
                ActionStatus::Skipped | ActionStatus::Cached | ActionStatus::CachedFromRemote
            )
        })
        .count();
    let failed_count = results.iter().filter(|action| action.has_failed()).count();

    let message = if failed_count > 0 {
        format!(
            "Setup toolchains with {passed_count} passed, {skipped_count} skipped, and {failed_count} failed"
        )
    } else if passed_count == 1 {
        format!("Setup {passed_count} toolchain successfully!")
    } else if passed_count > 0 {
        format!("Setup {passed_count} toolchains successfully!")
    } else {
        "All toolchains are already up to date!".to_string()
    };

    let variant = if failed_count > 0 {
        Variant::Caution
    } else {
        Variant::Success
    };

    session.console.render(element! {
        Container {
            Notice(variant: variant) {
                StyledText(content: message)
            }
        }
    })?;

    // Return error code if any setup failed
    if failed_count > 0 {
        return Ok(Some(1));
    }

    Ok(None)
}
