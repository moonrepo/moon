use crate::helpers::run_action_pipeline;
use crate::session::{MoonSession, SessionResult};
use iocraft::prelude::element;
use moon_action_graph::ActionGraphBuilderOptions;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use tracing::instrument;

#[instrument(skip(session))]
pub async fn setup(session: MoonSession) -> SessionResult {
    let mut action_graph_builder = session
        .build_action_graph_with_options(ActionGraphBuilderOptions {
            // Only enable toolchain setup for this command
            install_dependencies: false.into(),
            setup_environment: true.into(),
            setup_toolchains: true.into(),
            sync_projects: false.into(),
            sync_project_dependencies: false,
            sync_workspace: false,
        })
        .await?;

    // First ensure proto is set up (this will be a dependency for toolchain setups)
    action_graph_builder.setup_proto();

    // Add new toolchain plugin setups, and their environments
    // in the workspace root, when the toolchain was set up
    let mut requests = vec![];

    for toolchain_id in session.toolchains_config.plugins.keys() {
        if let Some(spec) = action_graph_builder.get_workspace_spec(toolchain_id) {
            requests.push((
                action_graph_builder.setup_toolchain_and_environment(&spec),
                spec,
            ));
        }
    }

    let output = action_graph_builder.build().await?;
    let toolchain_count = requests
        .iter()
        .filter(|(request, spec)| !output.report.nodes(*request).is_empty() && !spec.is_system())
        .count();

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

    let (action_context, action_graph) = (output.context, output.graph);

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
    let results = run_action_pipeline(&session, action_context, action_graph, None).await?;

    // Analyze results and provide feedback
    let failed_count = results.iter().filter(|action| action.has_failed()).count();

    let message = if failed_count > 0 {
        "Failed to setup toolchains!".to_string()
    } else {
        "Setup toolchains successfully!".to_string()
    };

    // Return error code if any setup failed
    if failed_count > 0 {
        session.console.render_err(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: message)
                }
            }
        })?;

        return Ok(Some(1));
    }

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: message)
            }
        }
    })?;

    Ok(None)
}
