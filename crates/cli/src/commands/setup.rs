use crate::helpers::{create_progress_bar, load_workspace};
use moon_contract::Runtime;
use moon_runner::{ActionRunner, DepGraph};
use moon_utils::is_test_env;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");

    let workspace = load_workspace().await?;
    let mut dep_graph = DepGraph::default();

    if let Some(node) = &workspace.config.node {
        let platform = Runtime::Node(node.version.to_owned());

        dep_graph.setup_tool(&platform);

        if !is_test_env() {
            dep_graph.install_deps(&platform)?;
        }
    }

    ActionRunner::new(workspace).run(dep_graph, None).await?;

    done("Setup complete", true);

    Ok(())
}
