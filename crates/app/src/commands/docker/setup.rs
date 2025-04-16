use super::{DockerManifest, MANIFEST_NAME, docker_error::AppDockerError};
use crate::components::run_action_pipeline;
use crate::session::MoonSession;
use starbase::AppResult;
use starbase_utils::json;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn setup(session: MoonSession) -> AppResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let workspace_graph = session.get_workspace_graph().await?;
    let mut action_graph_builder = session.build_action_graph().await?;

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Installing tools and dependencies for focused projects"
    );

    for project_id in &manifest.focused_projects {
        let project = workspace_graph.get_project(project_id)?;

        action_graph_builder
            .install_dependencies_by_project(&project)
            .await?;
    }

    let (action_context, action_graph) = action_graph_builder.build();

    run_action_pipeline(&session, action_context, action_graph).await?;

    Ok(None)
}
