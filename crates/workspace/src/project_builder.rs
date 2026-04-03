use crate::build_data::ProjectBuildData;
use crate::workspace_builder::WorkspaceBuilderContext;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use std::sync::Arc;

pub fn load_project_build_data(
    context: Arc<WorkspaceBuilderContext>,
    id: Id,
    source: WorkspaceRelativePathBuf,
) -> miette::Result<ProjectBuildData> {
    let config = context
        .config_loader
        .load_project_config_from_source(&context.workspace_root, &source)?;

    Ok(ProjectBuildData {
        config: Some(config),
        id: Some(id),
        source,
        ..Default::default()
    })
}
