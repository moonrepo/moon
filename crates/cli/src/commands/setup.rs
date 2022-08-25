use crate::helpers::{create_progress_bar, load_workspace};
use moon_action_runner::{ActionRunner, DepGraph};
use moon_contract::SupportedPlatform;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");

    let workspace = load_workspace().await?;
    let mut dep_graph = DepGraph::default();

    if workspace.config.node.is_some() {
        dep_graph.setup_tool(SupportedPlatform::Node);
    }

    ActionRunner::new(workspace).run(dep_graph, None).await?;

    done("Setup complete", true);

    Ok(())
}
