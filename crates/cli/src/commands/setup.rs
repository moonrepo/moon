use crate::helpers::{build_dep_graph, create_progress_bar, load_workspace};
use moon_platform::{Runtime, Version};
use moon_runner::Runner;
use moon_utils::is_test_env;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");

    let mut workspace = load_workspace().await?;
    let project_graph = workspace.generate_project_graph().await?;
    let mut dep_builder = build_dep_graph(&workspace, &project_graph);

    if let Some(node) = &workspace.toolchain.config.node {
        let runtime = Runtime::Node(Version(node.version.to_owned(), false));

        if is_test_env() {
            dep_builder.setup_tool(&runtime);
        } else {
            dep_builder.install_workspace_deps(&runtime);
        }
    }

    let dep_graph = dep_builder.build();

    Runner::new(workspace).run(dep_graph, None).await?;

    done("Setup complete", true);

    Ok(())
}
