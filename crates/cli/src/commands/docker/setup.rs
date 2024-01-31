use super::MANIFEST_NAME;
use crate::commands::docker::scaffold::DockerManifest;
use miette::miette;
use moon::{build_action_graph, generate_project_graph};
use moon_action_pipeline::Pipeline;
use moon_app_components::AppConsole;
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;
use starbase_utils::json;

#[system]
pub async fn setup(resources: ResourcesMut) {
    let manifest_path = { resources.get::<Workspace>().root.join(MANIFEST_NAME) };

    if !manifest_path.exists() {
        return Err(miette!(
            code = "moon::docker::setup",
            "Unable to setup, docker manifest missing. Has it been scaffolded with {}?",
            color::shell("moon docker scaffold")
        ));
    }

    let manifest: DockerManifest = json::read_file(manifest_path)?;
    let project_graph = { generate_project_graph(resources.get_mut::<Workspace>()).await? };
    let mut action_graph_builder = build_action_graph(&project_graph)?;

    for project_id in &manifest.focused_projects {
        let project = project_graph.get(project_id)?;

        action_graph_builder.install_deps(&project, None)?;
    }

    let action_graph = action_graph_builder.build()?;

    Pipeline::new(resources.get::<Workspace>().to_owned(), project_graph)
        .run(
            action_graph,
            resources.get::<AppConsole>().into_inner(),
            None,
        )
        .await?;
}
