use crate::commands::docker::scaffold::DockerManifest;
use crate::helpers::AnyError;
use moon::{build_dep_graph, generate_project_graph, load_workspace_with_toolchain};
use moon_action_pipeline::Pipeline;
use moon_terminal::safe_exit;
use moon_utils::json;

pub async fn setup() -> Result<(), AnyError> {
    let mut workspace = load_workspace_with_toolchain().await?;
    let manifest_path = workspace.root.join("dockerManifest.json");

    if !manifest_path.exists() {
        eprintln!("Unable to setup, docker manifest missing. Has it been scaffolded with `moon docker scaffold`?");
        safe_exit(1);
    }

    let manifest: DockerManifest = json::read(manifest_path)?;
    let project_graph = generate_project_graph(&mut workspace).await?;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    for project_id in &manifest.focused_projects {
        dep_builder.install_deps(project_graph.get(project_id)?, None)?;
    }

    let dep_graph = dep_builder.build();

    Pipeline::new(workspace, project_graph)
        .run(dep_graph, None)
        .await?;

    Ok(())
}
