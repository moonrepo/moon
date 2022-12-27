use crate::helpers::{create_progress_bar, AnyError};
use moon::{build_dep_graph, generate_project_graph, load_workspace};
use moon_platform::{Runtime, Version};
use moon_runner::Runner;
use moon_utils::is_test_env;

pub async fn setup() -> Result<(), AnyError> {
    let done = create_progress_bar("Downloading and installing tools...");

    let mut workspace = load_workspace().await?;
    let project_graph = generate_project_graph(&mut workspace)?;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    if let Some(node_config) = &workspace.toolchain.config.node {
        if let Some(node_version) = &node_config.version {
            let runtime = Runtime::Node(Version(node_version.to_owned(), false));

            if is_test_env() {
                dep_builder.setup_tool(&runtime);
            } else {
                dep_builder.install_workspace_deps(&runtime);
            }
        }
    }

    let dep_graph = dep_builder.build();

    Runner::new(workspace)
        .run(dep_graph, project_graph, None)
        .await?;

    done("Setup complete", true);

    Ok(())
}
