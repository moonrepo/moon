use crate::components::run_action_pipeline;
use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_action::ActionStatus;
use moon_action_graph::ActionGraphBuilderOptions;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_platform::{PlatformManager, ToolchainSpec};
use starbase::AppResult;
use tracing::instrument;

#[instrument]
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

    let mut toolchain_count = 0;

    // Add legacy platform toolchains (for backward compatibility)
    let platform_manager = PlatformManager::read();
    for platform in platform_manager.list() {
        // Legacy platforms don't expose runtime directly, we need to check if they have toolchains
        if platform.is_toolchain_enabled().unwrap_or(false) {
            let runtime = platform.get_runtime_from_config(None);
            // Only setup non-system runtimes that have specific versions
            if !runtime.is_system() && !runtime.requirement.is_global() {
                action_graph_builder
                    .setup_toolchain_legacy(&runtime)
                    .await?;
                toolchain_count += 1;
            }
        }
    }

    // Add new toolchain plugin setups
    for (toolchain_id, config) in &session.toolchain_config.plugins {
        // Check if plugin has a valid version configuration
        if let Some(version) = &config.version {
            let spec = ToolchainSpec::new(toolchain_id.to_owned(), version.to_owned());
            action_graph_builder.setup_toolchain(&spec).await?;
            toolchain_count += 1;
        } else {
            // For global toolchains, we still create the action but it will likely be skipped
            let spec = ToolchainSpec::new_global(toolchain_id.to_owned());
            action_graph_builder.setup_toolchain(&spec).await?;
            toolchain_count += 1;
        }
    }

    // Early exit if no toolchains are configured
    if toolchain_count == 0 {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Info) {
                    StyledText(content: "No toolchains are configured for setup")
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
            "Setup toolchains completed with {passed_count} success, {skipped_count} skipped, {failed_count} failed"
        )
    } else if passed_count > 0 {
        format!("Setup {passed_count} toolchain(s) successfully!")
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
