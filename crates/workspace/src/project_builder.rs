use crate::build_data::ProjectBuildData;
use crate::workspace_builder::WorkspaceBuilderContext;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_pdk_api::{ExtendProjectGraphInput, ExtendProjectGraphOutput};
use std::collections::BTreeMap;
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

pub async fn extend_projects_with_plugins(
    context: Arc<WorkspaceBuilderContext>,
    project_sources: BTreeMap<Id, String>,
) -> miette::Result<Vec<(Id, ExtendProjectGraphOutput, bool)>> {
    let mut outputs = vec![];

    // From toolchains
    for result in context
        .toolchain_registry
        .extend_project_graph_all(|registry, toolchain| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: project_sources.clone(),
            toolchain_config: registry.create_config(&toolchain.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, true));
    }

    // From extensions
    for result in context
        .extension_registry
        .extend_project_graph_all(|registry, extension| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: project_sources.clone(),
            extension_config: registry.create_config(&extension.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, false));
    }

    Ok(outputs)
}
