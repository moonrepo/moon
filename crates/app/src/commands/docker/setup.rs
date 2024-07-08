use super::{disable_toolchain, docker_error::AppDockerError, DockerManifest, MANIFEST_NAME};
use crate::experiments::run_action_pipeline;
use crate::session::CliSession;
use starbase::AppResult;
use starbase_utils::json;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn setup(session: CliSession) -> AppResult {
    let manifest_path = session.workspace_root.join(MANIFEST_NAME);

    if !manifest_path.exists() {
        return Err(AppDockerError::MissingManifest.into());
    }

    disable_toolchain(&session);

    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let project_graph = session.get_project_graph().await?;
    let mut action_graph_builder = session.build_action_graph(&project_graph).await?;

    debug!(
        projects = ?manifest.focused_projects.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Installing tools and dependencies for focused projects"
    );

    for project_id in &manifest.focused_projects {
        let project = project_graph.get(project_id)?;

        action_graph_builder.install_deps(&project, None)?;
    }

    run_action_pipeline(&session, action_graph_builder.build()?, None).await?;

    Ok(())
}
